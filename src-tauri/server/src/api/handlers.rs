use crate::config::Config;
use crate::error::{AppError, Result};
use crate::models::*;
use crate::services::AppService;
use axum::{
    Json,
    body::Bytes,
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use uuid::Uuid;

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use tokio::fs;
use tokio::io::AsyncWriteExt;

fn media_subdir_for_ext(ext: &str) -> Option<&'static str> {
    match ext {
        "jpg" | "jpeg" | "png" | "webp" => Some("images"),
        "mp4" | "webm" | "mov" => Some("videos"),
        "mp3" | "wav" | "ogg" | "m4a" => Some("audio"),
        _ => None,
    }
}

fn public_static_url(path: &str) -> String {
    format!("/static/{}", path)
}

fn clean_static_path(path: &str, public_url: &str) -> String {
    path.strip_prefix(public_url.trim_end_matches('/'))
        .unwrap_or(path)
        .trim_start_matches("/static/")
        .trim_start_matches('/')
        .replace('\\', "/")
}

fn replacement_subdir_for_target(path: &str) -> Option<&'static str> {
    if path.starts_with("images/") {
        Some("images")
    } else if path.starts_with("videos/") {
        Some("videos")
    } else if path.starts_with("audio/") {
        Some("audio")
    } else if path.starts_with("backgrounds/") {
        Some("backgrounds")
    } else {
        None
    }
}

/// Max accepted raw upload size for an image before decoding (guards memory).
const MAX_IMAGE_BYTES: usize = 40 * 1024 * 1024; // 40 MB
/// Max decoded pixel count (guards against decompression bombs).
const MAX_IMAGE_PIXELS: u64 = 50_000_000; // 50 MP

/// Widths of the three served variants. `MEDIUM` is the one that makes phones cheap:
/// with only 420 and 1800 to choose from, a phone at DPR 2–3 needs ~500–1200 physical
/// pixels, so `srcset` correctly rejected the 420 thumb as too small and pulled the
/// full 1800 preview — a ~390 KB download to paint a 390 px-wide screen. 900 lands in
/// the gap, and the browser picks it.
const PREVIEW_PX: u32 = 1800;
const MEDIUM_PX: u32 = 900;
const THUMB_PX: u32 = 420;

/// Lossy WebP quality. WebP is only worth serving if it is *smaller* than the JPEG
/// beside it (see encode_webp_bytes).
const WEBP_QUALITY: f32 = 80.0;

/// All encoded representations of one uploaded image.
struct EncodedImageVariants {
    original_jpeg: Vec<u8>,
    preview_jpeg: Vec<u8>,
    medium_jpeg: Vec<u8>,
    thumb_jpeg: Vec<u8>,
    preview_webp: Vec<u8>,
    medium_webp: Vec<u8>,
    thumb_webp: Vec<u8>,
}

fn encode_jpeg_bytes(image: &image::RgbImage, quality: u8) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut bytes, quality);
    encoder
        .encode_image(image)
        .map_err(|e| AppError::Internal(format!("Failed to encode image: {}", e)))?;
    Ok(bytes)
}

/// LOSSY WebP, via libwebp.
///
/// This used to be `image`'s `WebPEncoder::new_lossless`, which is the only mode that
/// crate can encode — and lossless WebP of a photograph is not a saving, it is a
/// disaster. Measured on this server's own uploads before the change:
///
///     preview (1800px)   JPEG  392 KB   WebP  2480 KB    6.3x LARGER
///     thumb   (420px)    JPEG   31 KB   WebP   230 KB    7.2x LARGER
///
/// Anything that offered those files to a browser was handing mobile visitors megabytes
/// per photograph. Lossy q80 is what WebP is actually for: it comes out roughly 25-30%
/// *below* the equivalent JPEG, which is the whole reason to encode it at all.
fn encode_webp_bytes(image: &image::RgbImage) -> Result<Vec<u8>> {
    let encoder = webp::Encoder::from_rgb(image.as_raw(), image.width(), image.height());
    Ok(encoder.encode(WEBP_QUALITY).to_vec())
}

/// CPU-bound: decode + resize + encode. Runs inside spawn_blocking so it never
/// blocks an async runtime worker.
fn build_image_variants(data: &[u8]) -> Result<EncodedImageVariants> {
    let image = image::load_from_memory(data)
        .map_err(|e| AppError::BadRequest(format!("Invalid image file: {}", e)))?;

    if (image.width() as u64) * (image.height() as u64) > MAX_IMAGE_PIXELS {
        return Err(AppError::BadRequest(
            "Image dimensions are too large".into(),
        ));
    }

    let original = image.to_rgb8();
    let preview = image
        .resize(PREVIEW_PX, PREVIEW_PX, FilterType::Lanczos3)
        .to_rgb8();
    let medium = image
        .resize(MEDIUM_PX, MEDIUM_PX, FilterType::Lanczos3)
        .to_rgb8();
    let thumb = image
        .resize(THUMB_PX, THUMB_PX, FilterType::Lanczos3)
        .to_rgb8();

    Ok(EncodedImageVariants {
        original_jpeg: encode_jpeg_bytes(&original, 95)?,
        preview_jpeg: encode_jpeg_bytes(&preview, 86)?,
        medium_jpeg: encode_jpeg_bytes(&medium, 82)?,
        thumb_jpeg: encode_jpeg_bytes(&thumb, 78)?,
        preview_webp: encode_webp_bytes(&preview)?,
        medium_webp: encode_webp_bytes(&medium)?,
        thumb_webp: encode_webp_bytes(&thumb)?,
    })
}

async fn write_bytes(upload_dir: &str, relative_path: &str, bytes: &[u8]) -> Result<()> {
    let path = std::path::Path::new(upload_dir).join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await.map_err(AppError::Io)?;
    }
    let mut file = fs::File::create(&path).await.map_err(AppError::Io)?;
    file.write_all(bytes).await.map_err(AppError::Io)?;
    Ok(())
}

/// Longest slug prefix kept in an image filename. `slugify` is otherwise unbounded, and
/// figurine titles on this site run long and poetic — without a cap, a title alone can
/// push the filename past a filesystem's ~255-byte component limit and turn an upload
/// that always used to succeed into an ENAMETOOLONG failure. 60 bytes is generous for a
/// filename prefix (every char here is pure ASCII, since `slugify` never emits anything
/// else) while leaving headroom for the trailing `-{uuid}.jpg`.
const MAX_SLUG_LEN: usize = 60;

/// Cut `slug` to at most `max_len` bytes without splitting a word: back up to the last
/// `-` inside the budget. Safe to byte-slice directly — `slugify`'s output is ASCII only.
fn truncate_slug(slug: &str, max_len: usize) -> &str {
    if slug.len() <= max_len {
        return slug;
    }
    let cut = &slug[..max_len];
    match cut.rfind('-') {
        Some(i) if i > 0 => &slug[..i],
        _ => cut,
    }
}

/// Build the id segment of an uploaded image's on-disk filename.
///
/// Plain `Uuid::new_v4()` gives Google Images nothing to read (`3261c26a-…jpg`) — the
/// "descriptive file names" signal in Google's image SEO guidance wants the file's own
/// path to say what it depicts. When the caller supplies a name (the figurine's title,
/// at upload time), prefix the uuid with its transliterated, length-capped slug — same
/// `slugify` the figurine's own URL uses, so `"Хранительница порога"` yields
/// `hranitelnica-poroga-3261c26a-….jpg` instead of a bare id. The full uuid is kept
/// (not truncated) so collision-resistance is unchanged from before this existed; only
/// the slug prefix is capped (see MAX_SLUG_LEN). No name → identical to the old
/// bare-uuid behaviour, which is what every upload that isn't a figurine photo (avatars,
/// backgrounds, admin media library drops with no figurine context) still gets.
fn image_id_with_hint(name_hint: Option<&str>) -> String {
    let uuid = Uuid::new_v4().to_string();
    match name_hint.map(crate::slug::slugify).filter(|s| !s.is_empty()) {
        Some(slug) => format!("{}-{}", truncate_slug(&slug, MAX_SLUG_LEN), uuid),
        None => uuid,
    }
}

async fn save_image_variants(
    upload_dir: &str,
    data: &[u8],
    name_hint: Option<&str>,
) -> Result<serde_json::Value> {
    if data.len() > MAX_IMAGE_BYTES {
        return Err(AppError::BadRequest("Image file is too large".into()));
    }

    let id = image_id_with_hint(name_hint);
    let original_relative = format!("images/original/{}.jpg", id);
    let preview_relative = format!("images/preview/{}.jpg", id);
    let medium_relative = format!("images/medium/{}.jpg", id);
    let thumb_relative = format!("images/thumb/{}.jpg", id);
    let preview_webp = format!("images/preview/{}.webp", id);
    let medium_webp = format!("images/medium/{}.webp", id);
    let thumb_webp = format!("images/thumb/{}.webp", id);

    let data_owned = data.to_vec();
    let variants = tokio::task::spawn_blocking(move || build_image_variants(&data_owned))
        .await
        .map_err(|e| AppError::Internal(format!("Image processing task failed: {}", e)))??;

    write_bytes(upload_dir, &original_relative, &variants.original_jpeg).await?;
    write_bytes(upload_dir, &preview_relative, &variants.preview_jpeg).await?;
    write_bytes(upload_dir, &medium_relative, &variants.medium_jpeg).await?;
    write_bytes(upload_dir, &thumb_relative, &variants.thumb_jpeg).await?;
    write_bytes(upload_dir, &preview_webp, &variants.preview_webp).await?;
    write_bytes(upload_dir, &medium_webp, &variants.medium_webp).await?;
    write_bytes(upload_dir, &thumb_webp, &variants.thumb_webp).await?;

    // NB: only `relativePath` (the preview) and `thumbRelativePath` are persisted on the
    // image record. The medium and WebP siblings are deliberately NOT new DB columns —
    // they live at the same id under a different directory/extension, so the client
    // derives their URLs by rewriting the path (resolveMediumUrl / resolveWebpUrl in
    // api.ts). That keeps a new rendition from costing a migration every time.
    Ok(serde_json::json!({
        "url":                  public_static_url(&preview_relative),
        "relativePath":         preview_relative,
        "webpUrl":              public_static_url(&preview_webp),
        "mediumUrl":            public_static_url(&medium_relative),
        "mediumWebpUrl":        public_static_url(&medium_webp),
        "originalUrl":          public_static_url(&original_relative),
        "originalRelativePath": original_relative,
        "thumbUrl":             public_static_url(&thumb_relative),
        "thumbRelativePath":    thumb_relative,
        "thumbWebpUrl":         public_static_url(&thumb_webp)
    }))
}

/// Regenerate every derived rendition for images that predate the current pipeline.
///
/// Two things need backfilling on an existing install:
///   • the `medium` (900px) variant, which simply did not exist before;
///   • every `.webp`, which was written LOSSLESS and is therefore 6-7x larger than the
///     JPEG it was supposed to undercut.
///
/// Both are recoverable because `images/original/{id}.jpg` is kept for every upload, so
/// the renditions are rebuilt from the original rather than from an already-compressed
/// copy. An image is considered done when its medium JPEG exists, which makes this
/// idempotent and cheap to re-run on every boot.
///
/// Runs detached in a background task: it is pure CPU on spawn_blocking, must never hold
/// up the listener, and a failure here degrades quality, not correctness (the client
/// falls back to the preview when a medium is missing).
pub async fn backfill_image_variants(upload_dir: String) {
    let original_dir = std::path::Path::new(&upload_dir).join("images/original");
    let mut entries = match fs::read_dir(&original_dir).await {
        Ok(e) => e,
        Err(_) => return, // no uploads yet — nothing to do
    };

    let (mut done, mut failed) = (0u32, 0u32);
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jpg") {
            continue;
        }
        let Some(id) = path.file_stem().and_then(|s| s.to_str()).map(String::from) else {
            continue;
        };

        let medium_relative = format!("images/medium/{}.jpg", id);
        if std::path::Path::new(&upload_dir)
            .join(&medium_relative)
            .exists()
        {
            continue; // already migrated
        }

        let Ok(data) = fs::read(&path).await else {
            failed += 1;
            continue;
        };
        let variants =
            match tokio::task::spawn_blocking(move || build_image_variants(&data)).await {
                Ok(Ok(v)) => v,
                _ => {
                    failed += 1;
                    continue;
                }
            };

        // The original is left exactly as it is — it is the source of truth and
        // re-encoding it would only degrade it.
        let writes = [
            (format!("images/preview/{}.jpg", id), &variants.preview_jpeg),
            (medium_relative.clone(), &variants.medium_jpeg),
            (format!("images/thumb/{}.jpg", id), &variants.thumb_jpeg),
            (format!("images/preview/{}.webp", id), &variants.preview_webp),
            (format!("images/medium/{}.webp", id), &variants.medium_webp),
            (format!("images/thumb/{}.webp", id), &variants.thumb_webp),
        ];
        let mut ok = true;
        for (rel, bytes) in writes {
            if write_bytes(&upload_dir, &rel, bytes).await.is_err() {
                ok = false;
                break;
            }
        }
        if ok {
            done += 1;
        } else {
            failed += 1;
        }
    }

    if done > 0 || failed > 0 {
        tracing::info!(
            "image variant backfill: {} rebuilt, {} failed (medium 900px + lossy WebP)",
            done,
            failed
        );
    }
}

async fn save_regular_media_file(
    upload_dir: &str,
    subdir: &str,
    ext: &str,
    data: &[u8],
) -> Result<String> {
    let media_dir = format!("{}/{}", upload_dir, subdir);
    fs::create_dir_all(&media_dir).await.map_err(AppError::Io)?;

    let file_id = Uuid::new_v4();
    let file_name = format!("{}.{}", file_id, ext);
    let full_path = format!("{}/{}", media_dir, file_name);
    let mut file = fs::File::create(&full_path).await.map_err(AppError::Io)?;
    file.write_all(data).await.map_err(AppError::Io)?;
    Ok(format!("{}/{}", subdir, file_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_supported_extensions_to_media_subdirs() {
        assert_eq!(media_subdir_for_ext("jpg"), Some("images"));
        assert_eq!(media_subdir_for_ext("webp"), Some("images"));
        assert_eq!(media_subdir_for_ext("mp4"), Some("videos"));
        assert_eq!(media_subdir_for_ext("mp3"), Some("audio"));
        assert_eq!(media_subdir_for_ext("exe"), None);
    }

    #[test]
    fn builds_public_static_urls() {
        assert_eq!(
            public_static_url("images/preview/abc.jpg"),
            "/static/images/preview/abc.jpg"
        );
    }

    #[test]
    fn image_id_with_hint_caps_a_long_slug_at_a_word_boundary() {
        let long_name = "The Guardian of the Threshold Who Watches Over the Forgotten Gate at the Edge of Dreaming";
        let id = image_id_with_hint(Some(long_name));
        // A uuid is always exactly 36 chars; the slug prefix is everything before the
        // trailing "-<uuid>".
        let uuid_len = 36;
        assert!(id.len() > uuid_len);
        let slug_part = &id[..id.len() - uuid_len - 1];
        assert!(
            slug_part.len() <= MAX_SLUG_LEN,
            "slug prefix {} bytes exceeds cap",
            slug_part.len()
        );
        assert!(!slug_part.ends_with('-'), "truncation must not leave a trailing hyphen");
        assert!(Uuid::parse_str(&id[id.len() - uuid_len..]).is_ok(), "trailing segment must still be a full, untruncated uuid");
    }

    #[test]
    fn image_id_with_hint_leaves_a_short_slug_untouched() {
        let id = image_id_with_hint(Some("Raven"));
        assert!(id.starts_with("raven-"));
    }

    #[test]
    fn image_id_with_hint_falls_back_to_a_bare_uuid_without_a_hint() {
        let id = image_id_with_hint(None);
        assert!(Uuid::parse_str(&id).is_ok());
    }

    #[test]
    fn downsizes_and_recompresses_oversized_background() {
        // A synthetic 3000x2000 image, well above BACKGROUND_MAX_PX, encoded at
        // quality 100 — stands in for an unedited camera JPEG an admin might upload.
        let big = image::RgbImage::from_fn(3000, 2000, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8])
        });
        let raw = encode_jpeg_bytes(&big, 100).expect("encode source fixture");

        let variant = process_background_image(&raw).expect("process background");
        let decoded_jpeg =
            image::load_from_memory(&variant.jpeg).expect("decode processed jpeg background");
        let decoded_webp =
            image::load_from_memory(&variant.webp).expect("decode processed webp background");

        assert!(decoded_jpeg.width() <= BACKGROUND_MAX_PX);
        assert!(decoded_jpeg.height() <= BACKGROUND_MAX_PX);
        assert_eq!(decoded_webp.width(), decoded_jpeg.width());
        assert_eq!(decoded_webp.height(), decoded_jpeg.height());
        assert!(
            variant.jpeg.len() < raw.len(),
            "expected recompression to shrink the file: {} -> {}",
            raw.len(),
            variant.jpeg.len()
        );
    }

    #[tokio::test]
    async fn backfills_webp_sibling_for_a_pre_pipeline_background_and_then_stops() {
        let dir = std::env::temp_dir().join(format!(
            "gotiga-bg-backfill-test-{}",
            Uuid::new_v4()
        ));
        let bg_dir = dir.join("backgrounds");
        fs::create_dir_all(&bg_dir).await.unwrap();

        // Stands in for a background written by the *old* handler, which preserved
        // the uploaded file's original extension — hence .jpeg here, not .jpg, and
        // deliberately the extension the live site's current background actually has.
        // Raw bytes, no WebP sibling, not yet capped to BACKGROUND_MAX_PX.
        let big = image::RgbImage::from_fn(3000, 2000, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8])
        });
        let raw = encode_jpeg_bytes(&big, 100).unwrap();
        fs::write(bg_dir.join("cabinet-bg.jpeg"), &raw).await.unwrap();

        backfill_background_image(dir.to_string_lossy().to_string()).await;

        // The .jpeg path must be left as-is (no rename to .jpg) since renaming would
        // orphan the URL already stored in settings.
        let webp_path = bg_dir.join("cabinet-bg.webp");
        let webp_bytes = fs::read(&webp_path)
            .await
            .expect("backfill should have written a webp sibling next to the .jpeg");
        let jpeg_bytes = fs::read(bg_dir.join("cabinet-bg.jpeg")).await.unwrap();
        assert!(jpeg_bytes.len() < raw.len());

        let decoded = image::load_from_memory(&webp_bytes).expect("decode backfilled webp");
        assert!(decoded.width() <= BACKGROUND_MAX_PX);

        // Second run must be a no-op: touch cabinet-bg.jpeg with obviously-wrong bytes
        // and confirm backfill leaves it alone because the webp sibling already exists.
        fs::write(bg_dir.join("cabinet-bg.jpeg"), b"not a real image")
            .await
            .unwrap();
        backfill_background_image(dir.to_string_lossy().to_string()).await;
        let untouched = fs::read(bg_dir.join("cabinet-bg.jpeg")).await.unwrap();
        assert_eq!(untouched, b"not a real image");

        fs::remove_dir_all(&dir).await.ok();
    }
}

// === PUBLIC READ-ONLY HANDLERS ===

pub async fn health_check() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({ "status": "ok", "version": "1.0.0" })),
    )
}

pub async fn readiness_check(State(service): State<AppService>) -> Result<impl IntoResponse> {
    service.health_check().await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ready",
            "checks": {
                "postgres": "ok"
            }
        })),
    ))
}

pub async fn record_analytics_event(
    State(service): State<AppService>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<impl IntoResponse> {
    const MAX_ANALYTICS_BODY: usize = 4096;
    if body.len() > MAX_ANALYTICS_BODY {
        return Err(AppError::BadRequest(
            "Analytics payload is too large".into(),
        ));
    }

    let content_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    let req: AnalyticsEventRequest = if content_type.starts_with("text/plain")
        || content_type.starts_with("application/json")
        || content_type.is_empty()
    {
        serde_json::from_slice(&body)
            .map_err(|_| AppError::BadRequest("Invalid analytics payload".into()))?
    } else {
        return Err(AppError::BadRequest(
            "Unsupported analytics content type".into(),
        ));
    };

    service.enqueue_analytics_event(req, &headers).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn admin_get_analytics_overview(
    State(service): State<AppService>,
    Query(query): Query<AdminAnalyticsQuery>,
) -> Result<Json<AdminAnalyticsOverview>> {
    service.refresh_analytics_hot_window_if_due().await?;
    Ok(Json(service.admin_get_analytics_overview(query).await?))
}

pub async fn admin_get_commission_funnel(
    State(service): State<AppService>,
    Query(query): Query<AdminAnalyticsQuery>,
) -> Result<Json<CommissionFunnel>> {
    service.refresh_analytics_hot_window_if_due().await?;
    Ok(Json(service.admin_get_commission_funnel(query).await?))
}

pub async fn admin_list_figurine_analytics(
    State(service): State<AppService>,
    Query(query): Query<AdminAnalyticsQuery>,
) -> Result<Json<AdminFigurineAnalyticsListPage>> {
    service.refresh_analytics_hot_window_if_due().await?;
    Ok(Json(service.admin_list_figurine_analytics(query).await?))
}

pub async fn admin_get_figurine_analytics(
    State(service): State<AppService>,
    Path(id): Path<String>,
    Query(query): Query<AdminAnalyticsQuery>,
) -> Result<Json<AdminFigurineAnalyticsDetail>> {
    service.refresh_analytics_hot_window_if_due().await?;
    Ok(Json(service.admin_get_figurine_analytics(id, query).await?))
}

/// Lightweight sibling of `admin_get_figurine_analytics`: just the permanent
/// per-day-per-country rollup for the geography map's "one figurine" mode —
/// the full detail endpoint does a lot of extra work (medians, CTA funnel,
/// browsers…) that a map-only view doesn't need.
pub async fn admin_get_figurine_geo_daily(
    State(service): State<AppService>,
    Path(id): Path<String>,
    Query(query): Query<AdminAnalyticsQuery>,
) -> Result<Json<Vec<FigurineGeoDailyPoint>>> {
    service.refresh_analytics_hot_window_if_due().await?;
    Ok(Json(service.admin_get_figurine_geo_daily(id, query).await?))
}

pub async fn admin_backfill_analytics(
    State(service): State<AppService>,
    body: Bytes,
) -> Result<Json<BackfillAnalyticsResponse>> {
    let req: BackfillAnalyticsRequest = if body.is_empty() {
        BackfillAnalyticsRequest::default()
    } else {
        serde_json::from_slice(&body)
            .map_err(|_| AppError::BadRequest("Invalid backfill payload".into()))?
    };
    Ok(Json(service.admin_backfill_analytics(req).await?))
}

pub async fn admin_get_life_of_house_trend(
    State(service): State<AppService>,
    Query(query): Query<AdminAnalyticsQuery>,
) -> Result<Json<LifeOfHouseTrend>> {
    Ok(Json(service.admin_get_life_of_house_trend(query).await?))
}

pub async fn admin_list_analytics_annotations(
    State(service): State<AppService>,
    Query(query): Query<AdminAnalyticsQuery>,
) -> Result<Json<Vec<AnalyticsAnnotation>>> {
    Ok(Json(service.admin_list_analytics_annotations(query).await?))
}

pub async fn admin_create_analytics_annotation(
    State(service): State<AppService>,
    Json(req): Json<CreateAnnotationRequest>,
) -> Result<Json<AnalyticsAnnotation>> {
    Ok(Json(service.admin_create_analytics_annotation(req).await?))
}

pub async fn admin_delete_analytics_annotation(
    State(service): State<AppService>,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    let id = uuid::Uuid::parse_str(&id)
        .map_err(|_| AppError::BadRequest("Invalid annotation id".into()))?;
    service.admin_delete_analytics_annotation(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/*pub async fn get_sync_manifest(
    State(service): State<AppService>
) -> Result<Json<Manifest>> {
    //let manifest = service.generate_manifest().await?;
    Ok(Json(manifest))
}*/

#[derive(serde::Deserialize)]
pub struct ListParams {
    visible: Option<bool>,
    status: Option<String>,
    search: Option<String>,
    sort: Option<String>,
    page: Option<i64>,
    #[serde(rename = "perPage")]
    per_page: Option<i64>,
    /// Accepted as an alias for `perPage`. The web client asked for `?limit=N` for a
    /// long time; serde dropped the unknown key without a word, `per_page` stayed None,
    /// and the service's `unwrap_or(i64::MAX)` then served the whole catalogue to a
    /// caller that had explicitly asked for thirty rows. The client now sends `perPage`,
    /// but a page cap silently meaning "no cap" is the kind of trap that should not be
    /// left armed for the next caller — or for a stale bundle still in someone's cache.
    limit: Option<i64>,
}

pub async fn list_figurines(
    State(service): State<AppService>,
    State(config): State<Config>,
    headers: HeaderMap,
    Query(params): Query<ListParams>,
) -> Result<Json<crate::models::FigurinesPage>> {
    let visible_only = params.visible.unwrap_or(true);

    // `visible=false` does not mean "show me the hidden ones" to the repository — it means
    // "drop the visibility filter ENTIRELY", and with it the first-look gate that holds a
    // work out of the public archive until its hour (see get_all_figurines). This route
    // carries no auth layer, so until now `GET /figurines?visible=false` handed any
    // anonymous caller the unpublished work and the not-yet-released work, and the client
    // was the only thing pretending otherwise (getAllFigurinesAdmin attaches a token). The
    // admin already sends that token; require it.
    if !visible_only {
        let authorized = bearer_token(&headers).is_some_and(|t| t == config.admin_api_key);
        if !authorized {
            tracing::warn!(
                target: "gotiga_server::auth",
                event = "list_figurines_unfiltered",
                outcome = "rejected",
                "anonymous request for the unfiltered figurine list"
            );
            return Err(AppError::Unauthorized);
        }
    }

    let query = crate::models::FigurineQuery {
        status: params.status,
        search: params.search,
        sort: params.sort,
        page: params.page,
        per_page: params
            .per_page
            .or(params.limit)
            .map(|n| n.clamp(1, 200)),
    };
    let page = service.list_figurines(visible_only, query).await?;
    Ok(Json(page))
}

pub async fn get_figurine(
    State(service): State<AppService>,
    Path(id): Path<String>,
) -> Result<Json<FigurineDto>> {
    let dto = service.get_figurine_details(id).await?;
    Ok(Json(dto))
}

pub async fn list_in_progress_figurines(
    State(service): State<AppService>,
) -> Result<Json<Vec<FigurineListItemDto>>> {
    let list = service.list_in_progress_figurines().await?;
    Ok(Json(list))
}

pub async fn list_first_look_figurines(
    State(service): State<AppService>,
) -> Result<Json<Vec<FigurineListItemDto>>> {
    let list = service.list_first_look_figurines().await?;
    Ok(Json(list))
}

// Combined GET dispatcher: /content/texts/:param (author | workshop)
pub async fn get_texts_by_param(
    State(service): State<AppService>,
    Path(param): Path<String>,
) -> Result<axum::response::Response> {
    match param.as_str() {
        "author" => {
            let texts = service.get_author_texts().await?;
            Ok(Json(texts).into_response())
        }
        "workshop" => {
            let items = service.get_workshop_items().await?;
            Ok(Json(items).into_response())
        }
        _ => Err(AppError::NotFound(format!(
            "Unknown text category: {}",
            param
        ))),
    }
}

pub async fn get_cabinet_zones(
    State(service): State<AppService>,
) -> Result<Json<Vec<CabinetZoneDto>>> {
    let zones = service.get_cabinet_zones().await?;
    Ok(Json(zones))
}

// === ADMIN AUTH ===

pub async fn admin_login(
    State(config): State<Config>,
    Json(creds): Json<LoginRequest>,
) -> Result<Json<serde_json::Value>> {
    if creds.login == config.admin_login && creds.password == config.admin_password {
        tracing::info!(
            target: "gotiga_server::auth",
            event = "admin_login",
            outcome = "ok",
            "admin login accepted"
        );
        Ok(Json(serde_json::json!({ "token": config.admin_api_key })))
    } else {
        tracing::warn!(
            target: "gotiga_server::auth",
            event = "admin_login",
            outcome = "rejected",
            "admin login rejected"
        );
        Err(AppError::Unauthorized)
    }
}

// === ADMIN FIGURINE CRUD ===

pub async fn save_figurine(
    State(service): State<AppService>,
    Json(req): Json<SaveFigurineRequest>,
) -> Result<StatusCode> {
    service.save_figurine(req).await?;
    Ok(StatusCode::OK)
}

pub async fn delete_figurine(
    State(service): State<AppService>,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    service.delete_figurine(id).await?;
    Ok(StatusCode::OK)
}

/// Admin: generate depth maps for a figurine's images on demand (the per-card
/// button). Runs Depth-Anything in-process (candle, CPU).
pub async fn admin_generate_figurine_depth(
    State(service): State<AppService>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse> {
    let summary = service.generate_figurine_depth(id).await?;
    Ok(Json(summary))
}

// === ADMIN MEDIA UPLOAD ===

pub async fn upload_file(
    State(_service): State<AppService>,
    State(config): State<Config>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>> {
    // Buffered first, acted on after the loop — same order-independent shape as
    // replace_media_everywhere below. An earlier version acted on `file` the moment it
    // saw it, which meant the optional `nameHint` field (the figurine's title, for
    // image_id_with_hint) was silently dropped with no error if a client happened to
    // send `file` first; buffering removes that footgun entirely rather than just
    // documenting it.
    let mut name_hint: Option<String> = None;
    let mut file_name: Option<String> = None;
    let mut file_data: Option<Bytes> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "nameHint" {
            name_hint = field.text().await.ok().filter(|s| !s.trim().is_empty());
        } else if name == "file" {
            file_name = Some(field.file_name().unwrap_or("file").to_string());
            file_data = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| AppError::Io(std::io::Error::other(e)))?,
            );
        }
    }

    let filename = file_name.ok_or_else(|| AppError::BadRequest("No file field found".to_string()))?;
    let data = file_data.ok_or_else(|| AppError::BadRequest("No file field found".to_string()))?;
    let ext = std::path::Path::new(&filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("bin")
        .to_lowercase();

    let subdir = media_subdir_for_ext(ext.as_str())
        .ok_or_else(|| AppError::BadRequest(format!("Unsupported media extension: {}", ext)))?;
    if subdir == "images" {
        let payload = save_image_variants(&config.upload_dir, &data, name_hint.as_deref()).await?;
        tracing::info!(
            target: "gotiga_server::media",
            event = "media_uploaded",
            media_type = "images",
            bytes = data.len(),
            outcome = "ok",
            "media uploaded"
        );
        return Ok(Json(payload));
    }

    let media_dir = format!("{}/{}", config.upload_dir, subdir);
    fs::create_dir_all(&media_dir).await.map_err(AppError::Io)?;

    let file_id = Uuid::new_v4();
    let file_name = format!("{}.{}", file_id, ext);
    let full_path = format!("{}/{}", media_dir, file_name);
    let mut file = fs::File::create(&full_path).await.map_err(AppError::Io)?;
    file.write_all(&data).await.map_err(AppError::Io)?;

    let relative_path = format!("{}/{}", subdir, file_name);
    let url = public_static_url(&relative_path);
    tracing::info!(
        target: "gotiga_server::media",
        event = "media_uploaded",
        media_type = subdir,
        path = %relative_path,
        bytes = data.len(),
        outcome = "ok",
        "media uploaded"
    );
    Ok(Json(serde_json::json!({
        "url": url,
        "relativePath": relative_path
    })))
}

pub async fn get_media_inventory(
    State(service): State<AppService>,
) -> Result<Json<MediaInventoryDto>> {
    Ok(Json(service.media_inventory().await?))
}

pub async fn get_unused_media_report(
    State(service): State<AppService>,
) -> Result<Json<MediaCleanupReportDto>> {
    Ok(Json(service.unused_media_report().await?))
}

pub async fn cleanup_unused_media(
    State(service): State<AppService>,
) -> Result<Json<serde_json::Value>> {
    let removed = service.cleanup_unused_media().await?;
    Ok(Json(serde_json::json!({ "removed": removed })))
}

pub async fn replace_media_everywhere(
    State(service): State<AppService>,
    State(config): State<Config>,
    mut multipart: Multipart,
) -> Result<Json<MediaReplaceResultDto>> {
    let mut target_path: Option<String> = None;
    let mut file_name: Option<String> = None;
    let mut file_data: Option<Bytes> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "targetPath" {
            let value = field
                .text()
                .await
                .map_err(|e| AppError::BadRequest(e.to_string()))?;
            target_path = Some(clean_static_path(&value, &config.public_url));
        } else if name == "file" {
            file_name = Some(field.file_name().unwrap_or("file").to_string());
            file_data = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| AppError::Io(std::io::Error::other(e)))?,
            );
        }
    }

    let target_path =
        target_path.ok_or_else(|| AppError::BadRequest("Missing targetPath".to_string()))?;
    let file_name = file_name.ok_or_else(|| AppError::BadRequest("Missing file".to_string()))?;
    let data = file_data.ok_or_else(|| AppError::BadRequest("Missing file data".to_string()))?;
    let target_subdir = replacement_subdir_for_target(&target_path).ok_or_else(|| {
        AppError::BadRequest(format!("Unsupported managed media path: {}", target_path))
    })?;
    let ext = std::path::Path::new(&file_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("bin")
        .to_lowercase();

    let result = if target_subdir == "images" {
        if media_subdir_for_ext(ext.as_str()) != Some("images") {
            return Err(AppError::BadRequest(format!(
                "Replacement must be an image, got {}",
                ext
            )));
        }
        let payload = save_image_variants(&config.upload_dir, &data, None).await?;
        let new_path = payload
            .get("relativePath")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Internal("Missing replacement relativePath".to_string()))?;
        let original_path = payload.get("originalRelativePath").and_then(|v| v.as_str());
        let thumb_path = payload.get("thumbRelativePath").and_then(|v| v.as_str());
        service
            .replace_media_everywhere(&target_path, new_path, original_path, thumb_path)
            .await?
    } else {
        let expected_subdir = media_subdir_for_ext(ext.as_str()).ok_or_else(|| {
            AppError::BadRequest(format!("Unsupported replacement extension: {}", ext))
        })?;
        if target_subdir != "backgrounds" && target_subdir != expected_subdir {
            return Err(AppError::BadRequest(format!(
                "Replacement type does not match target {}",
                target_path
            )));
        }
        let new_path =
            save_regular_media_file(&config.upload_dir, target_subdir, &ext, &data).await?;
        service
            .replace_media_everywhere(&target_path, &new_path, None, None)
            .await?
    };

    tracing::info!(
        target: "gotiga_server::media",
        event = "media_replace_uploaded",
        target_path = %target_path,
        updated_references = result.updated_references,
        bytes = data.len(),
        outcome = "ok",
        "media replacement uploaded"
    );
    Ok(Json(result))
}

// === ADMIN ZONE CRUD ===

pub async fn save_zone(
    State(service): State<AppService>,
    Json(req): Json<SaveZoneRequest>,
) -> Result<StatusCode> {
    service.save_zone(req).await?;
    Ok(StatusCode::OK)
}

pub async fn delete_zone(
    State(service): State<AppService>,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    service.delete_zone(id).await?;
    Ok(StatusCode::OK)
}

// === SHOWING ROOMS (the house wakes) ===

pub async fn get_showing_rooms(
    State(service): State<AppService>,
) -> Result<Json<Vec<crate::models::ShowingRoomDto>>> {
    let rooms = service.get_showing_rooms().await?;
    Ok(Json(rooms))
}

pub async fn save_showing_room(
    State(service): State<AppService>,
    Json(req): Json<crate::models::SaveShowingRoomRequest>,
) -> Result<StatusCode> {
    service.save_showing_room(req).await?;
    Ok(StatusCode::OK)
}

pub async fn delete_showing_room(
    State(service): State<AppService>,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    service.delete_showing_room(id).await?;
    Ok(StatusCode::OK)
}

// === ADMIN TEXT CRUD ===

pub async fn save_text(
    State(service): State<AppService>,
    Path(param): Path<String>,
    Json(req): Json<SaveTextRequest>,
) -> Result<StatusCode> {
    let cat = match param.as_str() {
        "author" => TextCategory::Author,
        "workshop" => TextCategory::Workshop,
        _ => return Err(AppError::BadRequest(format!("Unknown category: {}", param))),
    };
    service.save_text(cat, req).await?;
    Ok(StatusCode::OK)
}

pub async fn delete_text(
    State(service): State<AppService>,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    service.delete_text_item(id).await?;
    Ok(StatusCode::OK)
}

// === ADMIN BACKGROUND ===

pub async fn get_main_background(
    State(service): State<AppService>,
) -> Result<Json<serde_json::Value>> {
    let url = service.get_background().await?;
    Ok(Json(serde_json::json!({ "url": url })))
}

/// Cap + recompress an admin-uploaded main background so an unedited camera
/// JPEG or a lossless PNG export never ships to every visitor at whatever
/// size/quality it happened to be uploaded at — this was previously written
/// to disk byte-for-byte, which is exactly how a 275 KB background ended up
/// as the page's LCP element. Stays a single file, unlike figurine photos:
/// see resolveSrcset's doc comment in api.ts on why backgrounds deliberately
/// have no responsive thumb/medium/preview siblings.
const BACKGROUND_MAX_PX: u32 = 1800;
const BACKGROUND_JPEG_QUALITY: u8 = 86;

struct BackgroundVariant {
    jpeg: Vec<u8>,
    webp: Vec<u8>,
}

fn process_background_image(data: &[u8]) -> Result<BackgroundVariant> {
    if data.len() > MAX_IMAGE_BYTES {
        return Err(AppError::BadRequest("Image file is too large".into()));
    }
    let image = image::load_from_memory(data)
        .map_err(|e| AppError::BadRequest(format!("Invalid image file: {}", e)))?;
    if (image.width() as u64) * (image.height() as u64) > MAX_IMAGE_PIXELS {
        return Err(AppError::BadRequest(
            "Image dimensions are too large".into(),
        ));
    }
    // Never upscale: this same function reprocesses an already-capped background on
    // every server boot (see backfill_background_image below), and `resize` scales
    // *to fit* the box, which would blow up a smaller image back up to it.
    let fits_already = image.width() <= BACKGROUND_MAX_PX && image.height() <= BACKGROUND_MAX_PX;
    let sized = if fits_already {
        image.to_rgb8()
    } else {
        image
            .resize(BACKGROUND_MAX_PX, BACKGROUND_MAX_PX, FilterType::Lanczos3)
            .to_rgb8()
    };
    Ok(BackgroundVariant {
        jpeg: encode_jpeg_bytes(&sized, BACKGROUND_JPEG_QUALITY)?,
        webp: encode_webp_bytes(&sized)?,
    })
}

/// One-time startup fixup for a main background that predates this pipeline (i.e.
/// was written byte-for-byte by the old upload handler, with no WebP sibling).
/// Idempotent the same way backfill_image_variants is: presence of cabinet-bg.webp
/// means it's already done, so this only ever does real work once. Detached and
/// silent on failure — a missing WebP sibling degrades the format savings, not
/// correctness, and must never hold up the listener.
pub async fn backfill_background_image(upload_dir: String) {
    let bg_dir = std::path::Path::new(&upload_dir).join("backgrounds");

    // Find whatever cabinet-bg.* is already on disk. The *old* upload handler kept
    // the uploaded file's original extension (so this may be .jpeg, not .jpg — a
    // fresh upload today always writes .jpg, but this backfill is precisely for
    // whatever predates that). The path is left as-is rather than renamed to .jpg:
    // renaming would require updating the URL stored in settings too, and there is
    // no reason to touch that when recompressing in place works just as well.
    let mut entries = match fs::read_dir(&bg_dir).await {
        Ok(e) => e,
        Err(_) => return, // no backgrounds dir — nothing has ever been uploaded
    };
    let mut jpg_path = None;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        let is_cabinet_bg = path.file_stem().and_then(|s| s.to_str()) == Some("cabinet-bg");
        let is_jpeg = matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("jpg") | Some("jpeg")
        );
        if is_cabinet_bg && is_jpeg {
            jpg_path = Some(path);
            break;
        }
    }
    let Some(jpg_path) = jpg_path else {
        return; // no background uploaded yet (or it's some other format — leave it)
    };

    let webp_path = jpg_path.with_extension("webp");
    if fs::metadata(&webp_path).await.is_ok() {
        return; // already has a WebP sibling — uploaded (or backfilled) since this shipped
    }

    let Ok(data) = fs::read(&jpg_path).await else {
        return;
    };

    let variant = match tokio::task::spawn_blocking(move || process_background_image(&data)).await
    {
        Ok(Ok(v)) => v,
        _ => {
            tracing::warn!(
                "background backfill: failed to process {}",
                jpg_path.display()
            );
            return;
        }
    };

    if let Err(e) = fs::write(&jpg_path, &variant.jpeg).await {
        tracing::warn!(
            "background backfill: failed to write {}: {}",
            jpg_path.display(),
            e
        );
        return;
    }
    if let Err(e) = fs::write(&webp_path, &variant.webp).await {
        tracing::warn!(
            "background backfill: failed to write {}: {}",
            webp_path.display(),
            e
        );
    }
}

pub async fn upload_main_background(
    State(service): State<AppService>,
    State(config): State<Config>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            let filename = field.file_name().unwrap_or("bg.jpg").to_string();
            let ext = std::path::Path::new(&filename)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("jpg")
                .to_lowercase();
            if media_subdir_for_ext(ext.as_str()) != Some("images") {
                return Err(AppError::BadRequest(format!(
                    "Unsupported background extension: {}",
                    ext
                )));
            }
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::Io(std::io::Error::other(e)))?;

            // Always re-encoded to JPEG+WebP below, so the files on disk are always
            // cabinet-bg.jpg/.webp regardless of what format was uploaded — this also
            // stops old uploads in a different format piling up as orphaned files.
            // The WebP sibling is found by the client via resolveWebpUrl's plain
            // .jpg→.webp rewrite (api.ts) — same trick as the figurine renditions,
            // just without the thumb/medium/preview multiplication (see the doc
            // comment on process_background_image above for why backgrounds stay
            // single-size).
            let data_owned = data.to_vec();
            let variant =
                tokio::task::spawn_blocking(move || process_background_image(&data_owned))
                    .await
                    .map_err(|e| {
                        AppError::Internal(format!("Image processing task failed: {}", e))
                    })??;

            let bg_dir = format!("{}/backgrounds", config.upload_dir);
            fs::create_dir_all(&bg_dir).await.map_err(AppError::Io)?;

            let file_name = "cabinet-bg.jpg".to_string();
            let full_path = format!("{}/{}", bg_dir, file_name);
            let mut file = fs::File::create(&full_path).await.map_err(AppError::Io)?;
            file.write_all(&variant.jpeg).await.map_err(AppError::Io)?;

            let webp_path = format!("{}/cabinet-bg.webp", bg_dir);
            let mut webp_file = fs::File::create(&webp_path).await.map_err(AppError::Io)?;
            webp_file
                .write_all(&variant.webp)
                .await
                .map_err(AppError::Io)?;

            // Sweep any cabinet-bg.* left by an older upload — the handler used to keep
            // the uploaded file's own extension, so a .jpeg (or .png) from before this
            // pipeline would otherwise sit here forever, unreferenced by settings yet
            // still served: the prerendered pages built while it was current still carry
            // its URL in their og:image, so leaving it behind means the site keeps
            // advertising last month's background to every link preview.
            if let Ok(mut entries) = fs::read_dir(&bg_dir).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();
                    let is_cabinet_bg =
                        path.file_stem().and_then(|s| s.to_str()) == Some("cabinet-bg");
                    let is_current = matches!(
                        path.extension().and_then(|e| e.to_str()),
                        Some("jpg") | Some("webp")
                    );
                    if is_cabinet_bg && !is_current {
                        let _ = fs::remove_file(&path).await;
                    }
                }
            }

            let relative = format!("/static/backgrounds/{}", file_name);
            service.set_background(relative.clone()).await?;
            let public_url = service.get_background().await?.unwrap_or(relative);
            return Ok(Json(serde_json::json!({ "url": public_url })));
        }
    }
    Err(AppError::BadRequest("No file field found".to_string()))
}

pub async fn get_home_content(State(service): State<AppService>) -> Result<Json<HomeContent>> {
    let content = service.get_home_content().await?;
    Ok(Json(content))
}

pub async fn save_home_content(
    State(service): State<AppService>,
    Json(content): Json<HomeContent>,
) -> Result<StatusCode> {
    service.save_home_content(content).await?;
    Ok(StatusCode::OK)
}

// === AUTHOR PROFILE ===

pub async fn get_author_profile(
    State(service): State<AppService>,
) -> Result<Json<crate::models::AuthorProfile>> {
    let profile = service.get_author_profile().await?;
    Ok(Json(profile))
}

pub async fn save_author_profile(
    State(service): State<AppService>,
    Json(profile): Json<crate::models::AuthorProfile>,
) -> Result<StatusCode> {
    service.save_author_profile(profile).await?;
    Ok(StatusCode::OK)
}

// === ORDERS ===

pub async fn create_order(
    State(service): State<AppService>,
    headers: HeaderMap,
    Json(order): Json<crate::models::OrderRequest>,
) -> Result<Json<crate::models::OrderCreatedResponse>> {
    service
        .check_rate_limit("order", &extract_ip(&headers), 15, 3600)
        .await?;
    // Tie the order to the account when the request carries a valid session, so it
    // shows up in the sender's profile — not only in the admin panel.
    let user_id = if let Some(token) = bearer_token(&headers) {
        service
            .get_user_from_session(token)
            .await
            .ok()
            .map(|u| u.id)
    } else {
        None
    };
    let saved = service.create_order(&order, user_id).await?;
    Ok(Json(crate::models::OrderCreatedResponse {
        cancel_token: saved.cancel_token,
    }))
}

pub async fn get_notify_by_token(
    State(service): State<AppService>,
    headers: HeaderMap,
    Path(token): Path<String>,
) -> Result<Json<crate::models::NotifyInfo>> {
    service
        .check_rate_limit("token", &extract_ip(&headers), 60, 3600)
        .await?;
    service
        .get_notify_by_token(&token)
        .await?
        .map(Json)
        .ok_or_else(|| crate::error::AppError::NotFound("Subscription not found".to_string()))
}

pub async fn cancel_notify_by_token(
    State(service): State<AppService>,
    headers: HeaderMap,
    Path(token): Path<String>,
) -> Result<StatusCode> {
    service
        .check_rate_limit("token", &extract_ip(&headers), 60, 3600)
        .await?;
    service.cancel_notify_by_token(&token).await?;
    Ok(StatusCode::OK)
}

pub async fn list_orders(
    State(service): State<AppService>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<crate::models::OrdersPage>> {
    let status = params.get("status").map(|s| s.as_str());
    let mode = params
        .get("mode")
        .map(|s| s.as_str())
        .filter(|s| !s.is_empty());
    if let Some(mode) = mode
        && !matches!(mode, "request" | "question" | "notify" | "reserve")
    {
        return Err(AppError::BadRequest(
            "Invalid order mode filter".to_string(),
        ));
    }
    let page = params
        .get("page")
        .and_then(|p| p.parse::<i64>().ok())
        .unwrap_or(1)
        .max(1);
    let per_page = params
        .get("perPage")
        .and_then(|p| p.parse::<i64>().ok())
        .unwrap_or(20)
        .clamp(1, 100);
    let page_data = service.list_orders(status, mode, page, per_page).await?;
    Ok(Json(page_data))
}

pub async fn update_order_status(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
    Json(body): Json<crate::models::UpdateOrderStatusRequest>,
) -> Result<StatusCode> {
    service.update_order_status(id, &body).await?;
    Ok(StatusCode::OK)
}

// === COMMISSIONS ===

fn looks_like_email(s: &str) -> bool {
    let s = s.trim();
    match s.split_once('@') {
        Some((local, domain)) => {
            !local.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
        }
        None => false,
    }
}

pub async fn create_commission(
    State(service): State<AppService>,
    headers: HeaderMap,
    Json(req): Json<crate::models::CommissionRequest>,
) -> Result<Json<crate::models::CommissionCreatedResponse>> {
    // Honeypot: real users never fill `website`. Pretend success, save nothing.
    if req
        .website
        .as_deref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
    {
        return Ok(Json(crate::models::CommissionCreatedResponse {
            id: Uuid::new_v4().to_string(),
            claim_token: Uuid::new_v4().to_string().replace('-', ""),
        }));
    }

    let ip = extract_ip(&headers);
    service.check_commission_rate_limit(&ip).await?;

    if !looks_like_email(&req.requester_email) {
        return Err(AppError::BadRequest("A valid email is required.".into()));
    }
    if req.description.trim().is_empty() {
        return Err(AppError::BadRequest("Description is required.".into()));
    }
    if req.description.chars().count() > 5000 {
        return Err(AppError::BadRequest("Description is too long.".into()));
    }
    if req.attachment_urls.len() > 5 {
        return Err(AppError::BadRequest("Too many attachments.".into()));
    }
    if req
        .source_figurine_id
        .as_deref()
        .map(|s| s.trim().chars().count() > 80)
        .unwrap_or(false)
    {
        return Err(AppError::BadRequest(
            "Source figurine id is too long.".into(),
        ));
    }
    if req
        .similar_keep_note
        .as_deref()
        .map(|s| s.chars().count() > 1000)
        .unwrap_or(false)
        || req
            .similar_change_note
            .as_deref()
            .map(|s| s.chars().count() > 1000)
            .unwrap_or(false)
    {
        return Err(AppError::BadRequest("Similar notes are too long.".into()));
    }
    if req.similar_tags.len() > 12 {
        return Err(AppError::BadRequest("Too many similar tags.".into()));
    }

    // If the requester is logged in, attach their account immediately.
    let user = if let Some(token) = bearer_token(&headers) {
        service.get_user_from_session(token).await.ok()
    } else {
        None
    };

    let created = service.create_commission(&req).await?;
    if let Some(u) = user {
        let _ = service.claim_commission(&created.claim_token, u.id).await;
    }
    Ok(Json(created))
}

pub async fn get_commission_by_token(
    State(service): State<AppService>,
    Path(token): Path<String>,
) -> Result<Json<crate::models::CommissionDto>> {
    service
        .get_commission_by_token(&token)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound("Commission not found".to_string()))
}

pub async fn user_claim_commission(
    State(service): State<AppService>,
    headers: HeaderMap,
    Json(body): Json<crate::models::ClaimCommissionRequest>,
) -> Result<Json<crate::models::CommissionDto>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    Ok(Json(
        service.claim_commission(&body.claim_token, user.id).await?,
    ))
}

pub async fn user_list_commissions(
    State(service): State<AppService>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    // Adopt orphan petitions sent from this account's email (guest petitions, or
    // ones whose claim token was lost when localStorage was cleared) before listing.
    let _ = service
        .adopt_commissions_by_email(user.id, &user.email)
        .await;
    let commissions = service.get_user_commissions(user.id).await?;
    Ok(Json(serde_json::json!({ "commissions": commissions })))
}

pub async fn user_edit_commission(
    State(service): State<AppService>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(body): Json<crate::models::EditCommissionRequest>,
) -> Result<Json<crate::models::CommissionDto>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    Ok(Json(
        service.edit_commission(id, Some(user.id), &body).await?,
    ))
}

pub async fn user_delete_commission(
    State(service): State<AppService>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    service.delete_commission(id, Some(user.id)).await?;
    Ok(StatusCode::OK)
}

/// Authenticated media upload for ordinary users (commission references, chat
/// attachments). Mirrors the admin /upload but gated on a user session and
/// restricted to images.
pub async fn user_upload_file(
    State(service): State<AppService>,
    State(config): State<Config>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let _user = service.get_user_from_session(token).await?;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        if field.name().unwrap_or("") == "file" {
            let filename = field.file_name().unwrap_or("file").to_string();
            let ext = std::path::Path::new(&filename)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::Io(std::io::Error::other(e)))?;

            if data.len() > 8 * 1024 * 1024 {
                return Err(AppError::BadRequest("Image is too large (max 8MB).".into()));
            }
            if media_subdir_for_ext(&ext) != Some("images") {
                return Err(AppError::BadRequest(
                    "Only image uploads are allowed.".into(),
                ));
            }
            let payload = save_image_variants(&config.upload_dir, &data, None).await?;
            return Ok(Json(payload));
        }
    }
    Err(AppError::BadRequest("No file field found".to_string()))
}

pub async fn admin_list_commissions(
    State(service): State<AppService>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<crate::models::CommissionsPage>> {
    let status = params.get("status").map(|s| s.as_str());
    let similar = params
        .get("similar")
        .is_some_and(|s| matches!(s.as_str(), "true" | "1"));
    let page = params
        .get("page")
        .and_then(|p| p.parse::<i64>().ok())
        .unwrap_or(1)
        .max(1);
    let per_page = params
        .get("perPage")
        .and_then(|p| p.parse::<i64>().ok())
        .unwrap_or(20)
        .clamp(1, 100);
    Ok(Json(
        service
            .list_commissions(status, similar, page, per_page)
            .await?,
    ))
}

pub async fn admin_update_commission(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
    Json(body): Json<crate::models::UpdateCommissionStatusRequest>,
) -> Result<Json<crate::models::CommissionDto>> {
    service
        .update_commission(
            id,
            &body.status,
            body.admin_notes.as_deref(),
            body.figurine_id.as_deref(),
        )
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound("Commission not found".to_string()))
}

pub async fn admin_delete_commission(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    service.delete_commission(id, None).await?;
    Ok(StatusCode::OK)
}

// === SCHEDULE & BOOKINGS (PUBLIC) ===

pub async fn get_figurine_schedule(
    State(service): State<AppService>,
    Path(id): Path<String>,
) -> Result<Json<crate::models::FigurineScheduleDto>> {
    let schedule = service.get_figurine_schedule(id).await?;
    Ok(Json(schedule))
}

pub async fn create_booking(
    State(service): State<AppService>,
    headers: HeaderMap,
    Path(_id): Path<String>,
    Json(req): Json<crate::models::CreateBookingRequest>,
) -> Result<Json<crate::models::BookingCreatedResponse>> {
    service
        .check_rate_limit("booking", &extract_ip(&headers), 15, 3600)
        .await?;
    // Tie the booking to the account when the request carries a valid session, so it
    // lands in the sender's profile without relying on a separate client-side link.
    let user_id = if let Some(token) = bearer_token(&headers) {
        service
            .get_user_from_session(token)
            .await
            .ok()
            .map(|u| u.id)
    } else {
        None
    };
    let booking = service.create_booking(req, user_id).await?;
    Ok(Json(crate::models::BookingCreatedResponse {
        cancel_token: booking.cancel_token,
    }))
}

/// Set or clear the visitor's wax-seal mark on a figurine. Anonymous,
/// idempotent (explicit-set, not a stateful toggle), rate-limited by IP like
/// the other public write endpoints — the response deliberately carries no
/// count, only this visitor's own resulting state.
pub async fn set_figurine_mark(
    State(service): State<AppService>,
    headers: HeaderMap,
    Path(handle): Path<String>,
    Json(req): Json<crate::models::MarkToggleRequest>,
) -> Result<Json<crate::models::MarkToggleResponse>> {
    service
        .check_rate_limit("mark", &extract_ip(&headers), 60, 3600)
        .await?;
    let figurine_id = service.resolve_figurine_uuid(&handle).await?;
    let tone = service
        .set_figurine_mark(figurine_id, &req.visitor_token, req.tone.as_deref())
        .await?;
    Ok(Json(crate::models::MarkToggleResponse {
        marked: tone.is_some(),
        tone,
    }))
}

/// Admin-only ranking of every figurine by mark count. Never exposed publicly.
pub async fn admin_get_mark_stats(
    State(service): State<AppService>,
) -> Result<Json<Vec<crate::models::AdminFigurineMarkStat>>> {
    Ok(Json(service.get_admin_mark_stats().await?))
}

/// Public "noticed by guests" shelf — admin pins + auto-ranked fill. Never
/// carries counts/tones, only the resolved figurine list.
pub async fn list_noticed_by_guests(
    State(service): State<AppService>,
) -> Result<Json<Vec<crate::models::FigurineListItemDto>>> {
    Ok(Json(service.list_noticed_by_guests().await?))
}

pub async fn admin_get_noticed_by_guests_settings(
    State(service): State<AppService>,
) -> Result<Json<crate::models::NoticedByGuestsSettings>> {
    Ok(Json(service.get_noticed_by_guests_settings().await?))
}

pub async fn admin_save_noticed_by_guests_settings(
    State(service): State<AppService>,
    Json(settings): Json<crate::models::NoticedByGuestsSettings>,
) -> Result<Json<crate::models::NoticedByGuestsSettings>> {
    service
        .save_noticed_by_guests_settings(settings.clone())
        .await?;
    Ok(Json(settings))
}

pub async fn get_booking_by_token(
    State(service): State<AppService>,
    headers: HeaderMap,
    Path(token): Path<String>,
) -> Result<Json<crate::models::BookingCancelInfo>> {
    service
        .check_rate_limit("token", &extract_ip(&headers), 60, 3600)
        .await?;
    service
        .get_booking_by_token(&token)
        .await?
        .map(Json)
        .ok_or_else(|| crate::error::AppError::NotFound("Booking not found".to_string()))
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\'', "&apos;")
        .replace('"', "&quot;")
}

fn image_sitemap_entry(url: Option<&str>) -> String {
    url.filter(|u| !u.trim().is_empty())
        .map(|u| {
            format!(
                "<image:image><image:loc>{}</image:loc></image:image>",
                xml_escape(u.trim())
            )
        })
        .unwrap_or_default()
}

/// Public sitemap. Absolute URLs are built from the forwarded Host/proto headers so the
/// same binary works across environments without a hard-coded domain.
pub async fn sitemap_xml(
    State(service): State<AppService>,
    headers: HeaderMap,
) -> Result<impl IntoResponse> {
    let host = headers
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost");
    let proto = headers
        .get("x-forwarded-proto")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("https");
    let base = format!("{proto}://{host}");

    let figurines = service.list_figurines(true, crate::models::FigurineQuery::default()).await?.items;

    let mut urls = String::new();
    for path in ["/", "/figurines", "/author", "/workshop", "/upcoming"] {
        urls.push_str(&format!("  <url><loc>{base}{path}</loc></url>\n"));
    }
    for f in &figurines {
        if f.status == FigurineStatus::InProgress {
            continue;
        }

        let detail = service.get_figurine_details(f.id.clone()).await.ok();
        let image_url = detail
            .as_ref()
            .and_then(|d| {
                d.images
                    .iter()
                    .find(|i| i.image_type == ImageType::Face)
                    .or_else(|| d.images.first())
            })
            .map(|i| i.url.as_str())
            .or(f.face_image_url.as_deref());
        let image = image_sitemap_entry(image_url);

        // Prefer the pretty slug — it is the canonical handle the detail pages
        // advertise; the UUID is only the fallback for works not yet re-saved.
        let handle = f.slug.as_deref().unwrap_or(&f.id);
        urls.push_str(&format!(
            "  <url><loc>{base}/figurines/{}</loc><lastmod>{}</lastmod>{image}</url>\n",
            xml_escape(handle),
            f.created_at.format("%Y-%m-%d")
        ));
    }

    let body = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\" xmlns:image=\"http://www.google.com/schemas/sitemap-image/1.1\">\n{urls}</urlset>\n"
    );
    Ok((
        [(
            axum::http::header::CONTENT_TYPE,
            "application/xml; charset=utf-8",
        )],
        body,
    ))
}

pub async fn get_bookings_by_tokens(
    State(service): State<AppService>,
    Json(req): Json<crate::models::BookingsByTokensRequest>,
) -> Result<Json<std::collections::HashMap<String, crate::models::BookingCancelInfo>>> {
    // Cap the batch to bound the query and guard against abuse.
    let tokens: Vec<String> = req.tokens.into_iter().take(100).collect();
    if tokens.is_empty() {
        return Ok(Json(std::collections::HashMap::new()));
    }
    Ok(Json(service.get_bookings_by_tokens(&tokens).await?))
}

pub async fn cancel_booking_by_token(
    State(service): State<AppService>,
    headers: HeaderMap,
    Path(token): Path<String>,
) -> Result<StatusCode> {
    service
        .check_rate_limit("token", &extract_ip(&headers), 60, 3600)
        .await?;
    service.cancel_booking_by_token(&token).await?;
    Ok(StatusCode::OK)
}

// === SHOWINGS (ADMIN) ===

pub async fn list_showings(
    State(service): State<AppService>,
) -> Result<Json<Vec<crate::models::ShowingDto>>> {
    Ok(Json(service.list_showings().await?))
}

pub async fn save_showing(
    State(service): State<AppService>,
    Json(req): Json<crate::models::SaveShowingRequest>,
) -> Result<Json<crate::models::ShowingDto>> {
    Ok(Json(service.save_showing(req).await?))
}

pub async fn delete_showing(
    State(service): State<AppService>,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    service.delete_showing(id).await?;
    Ok(StatusCode::OK)
}

pub async fn bulk_clear_showings(
    State(service): State<AppService>,
) -> Result<impl IntoResponse> {
    Ok(Json(service.bulk_clear_showings().await?))
}

// === BULK FIGURINE OPS (ADMIN) ===

pub async fn bulk_clear_darkness(State(service): State<AppService>) -> Result<impl IntoResponse> {
    Ok(Json(service.bulk_clear_darkness().await?))
}

pub async fn bulk_reset_parallax(State(service): State<AppService>) -> Result<impl IntoResponse> {
    Ok(Json(service.bulk_reset_parallax_intensity().await?))
}

pub async fn bulk_set_parallax(
    State(service): State<AppService>,
    Json(req): Json<crate::models::BulkSetParallaxRequest>,
) -> Result<impl IntoResponse> {
    Ok(Json(service.bulk_set_parallax_intensity(req.intensity).await?))
}

pub async fn bulk_recalculate_parallax(
    State(service): State<AppService>,
) -> Result<impl IntoResponse> {
    Ok(Json(service.bulk_recalculate_parallax().await?))
}

pub async fn bulk_set_second_angle(State(service): State<AppService>) -> Result<impl IntoResponse> {
    Ok(Json(service.bulk_set_second_angle().await?))
}

// === SLUGS (ADMIN) ===

/// Backfill: generate a transliterated URL slug for every work still missing one.
pub async fn backfill_slugs(State(service): State<AppService>) -> Result<impl IntoResponse> {
    Ok(Json(service.backfill_figurine_slugs().await?))
}

/// Set/regenerate a single work's URL slug. Blank/absent `slug` → regenerate from
/// the work's name. Returns `{ "slug": "<stored>" }`.
pub async fn set_figurine_slug(
    State(service): State<AppService>,
    Path(id): Path<String>,
    Json(req): Json<crate::models::SetSlugRequest>,
) -> Result<impl IntoResponse> {
    let slug = service.set_figurine_slug(id, req.slug).await?;
    Ok(Json(serde_json::json!({ "slug": slug })))
}

// === BOOKINGS (ADMIN) ===

pub async fn list_bookings(
    State(service): State<AppService>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<crate::models::BookingsPage>> {
    let status = params.get("status").map(|s| s.as_str());
    let figurine_id = params
        .get("figurineId")
        .and_then(|s| Uuid::parse_str(s).ok());
    let page = params
        .get("page")
        .and_then(|p| p.parse::<i64>().ok())
        .unwrap_or(1)
        .max(1);
    let per_page = params
        .get("perPage")
        .and_then(|p| p.parse::<i64>().ok())
        .unwrap_or(20)
        .clamp(1, 100);
    Ok(Json(
        service
            .list_bookings(status, figurine_id, page, per_page)
            .await?,
    ))
}

pub async fn update_booking_status(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
    Json(body): Json<crate::models::UpdateBookingStatusRequest>,
) -> Result<StatusCode> {
    service
        .update_booking_status(id, body.status, body.admin_notes, body.curator_conditions)
        .await?;
    Ok(StatusCode::OK)
}

// ============================================================
// USER AUTH HANDLERS
// ============================================================

pub async fn user_register(
    State(service): State<AppService>,
    headers: HeaderMap,
    Json(body): Json<RegisterRequest>,
) -> Result<Json<LoginVerifyResponse>> {
    let ip = extract_ip(&headers);
    service.check_rate_limit("register", &ip, 5, 3600).await?;
    let response = service
        .register_user(&body, client_ip(&ip), extract_user_agent(&headers))
        .await?;
    Ok(Json(response))
}

pub async fn user_login_challenge(
    State(service): State<AppService>,
    headers: HeaderMap,
    Json(body): Json<LoginChallengeRequest>,
) -> Result<Json<LoginChallengeResponse>> {
    service
        .check_rate_limit("login", &extract_ip(&headers), 20, 3600)
        .await?;
    let response = service.login_challenge(&body.email).await?;
    Ok(Json(response))
}

pub async fn user_login_verify(
    State(service): State<AppService>,
    headers: HeaderMap,
    Json(body): Json<LoginVerifyRequest>,
) -> Result<Json<LoginVerifyResponse>> {
    let ip = extract_ip(&headers);
    service.check_rate_limit("login", &ip, 30, 3600).await?;
    let response = service
        .login_verify(&body, client_ip(&ip), extract_user_agent(&headers))
        .await?;
    Ok(Json(response))
}

pub async fn user_logout(
    State(service): State<AppService>,
    headers: HeaderMap,
) -> Result<StatusCode> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    service.logout(token).await?;
    Ok(StatusCode::OK)
}

pub async fn user_me(
    State(service): State<AppService>,
    headers: HeaderMap,
) -> Result<Json<UserDto>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    Ok(Json(UserDto::from(&user)))
}

pub async fn user_link_bookings(
    State(service): State<AppService>,
    headers: HeaderMap,
    Json(body): Json<LinkBookingsRequest>,
) -> Result<Json<serde_json::Value>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    let linked = service.link_bookings(user.id, &body.cancel_tokens).await?;
    Ok(Json(serde_json::json!({ "linked": linked })))
}

pub async fn user_profile_bookings(
    State(service): State<AppService>,
    headers: HeaderMap,
) -> Result<Json<Vec<UserBookingDto>>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    let bookings = service.get_user_bookings(user.id).await?;
    Ok(Json(bookings))
}

pub async fn user_profile_orders(
    State(service): State<AppService>,
    headers: HeaderMap,
) -> Result<Json<Vec<UserOrderDto>>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    // Adopt any orphan guest orders sent from this account's email (legacy rows or
    // ones submitted while logged out) before listing.
    let _ = service.link_orders_to_user(user.id, &user.email).await;
    let orders = service.get_user_orders(user.id).await?;
    Ok(Json(orders))
}

pub async fn user_profile_certificates(
    State(service): State<AppService>,
    headers: HeaderMap,
) -> Result<Json<Vec<CollectorCertificateDto>>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    let certificates = service.get_user_certificates(user.id).await?;
    Ok(Json(certificates))
}

pub async fn get_public_certificate(
    State(service): State<AppService>,
    Path(token): Path<String>,
) -> Result<Json<PublicCertificateDto>> {
    service
        .get_public_certificate(&token)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound("Certificate not found".to_string()))
}

pub async fn issue_order_certificate(
    State(service): State<AppService>,
    Path(id): Path<String>,
) -> Result<Json<CollectorCertificateDto>> {
    let id =
        Uuid::parse_str(&id).map_err(|_| AppError::BadRequest("Invalid order ID".to_string()))?;
    let certificate = service.issue_order_certificate(id).await?;
    Ok(Json(certificate))
}

pub async fn revoke_order_certificate(
    State(service): State<AppService>,
    Path(id): Path<String>,
) -> Result<Json<CollectorCertificateDto>> {
    let id =
        Uuid::parse_str(&id).map_err(|_| AppError::BadRequest("Invalid order ID".to_string()))?;
    let certificate = service.revoke_order_certificate(id).await?;
    Ok(Json(certificate))
}

pub async fn issue_commission_certificate(
    State(service): State<AppService>,
    Path(id): Path<String>,
) -> Result<Json<CollectorCertificateDto>> {
    let id = Uuid::parse_str(&id)
        .map_err(|_| AppError::BadRequest("Invalid commission ID".to_string()))?;
    let certificate = service.issue_commission_certificate(id).await?;
    Ok(Json(certificate))
}

pub async fn revoke_commission_certificate(
    State(service): State<AppService>,
    Path(id): Path<String>,
) -> Result<Json<CollectorCertificateDto>> {
    let id = Uuid::parse_str(&id)
        .map_err(|_| AppError::BadRequest("Invalid commission ID".to_string()))?;
    let certificate = service.revoke_commission_certificate(id).await?;
    Ok(Json(certificate))
}

pub async fn user_get_wishlist(
    State(service): State<AppService>,
    headers: HeaderMap,
) -> Result<Json<Vec<String>>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    let ids = service.get_user_wishlist(user.id).await?;
    Ok(Json(ids))
}

pub async fn user_profile_waitlist(
    State(service): State<AppService>,
    headers: HeaderMap,
) -> Result<Json<Vec<WaitlistEntryDto>>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    let entries = service.get_user_waitlist(user.id).await?;
    Ok(Json(entries))
}

pub async fn user_set_wishlist(
    State(service): State<AppService>,
    headers: HeaderMap,
    Json(body): Json<SetWishlistRequest>,
) -> Result<Json<Vec<String>>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    let ids = service
        .set_user_wishlist(user.id, body.figurine_ids)
        .await?;
    Ok(Json(ids))
}

pub async fn user_link_claim(
    State(service): State<AppService>,
    headers: HeaderMap,
    Json(body): Json<LinkClaimRequest>,
) -> Result<Json<LinkClaimResponse>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    let result = service.link_claim_by_token(&user, &body.token).await?;
    Ok(Json(result))
}

// ============================================================
// ADMIN USER MANAGEMENT HANDLERS
// ============================================================

pub async fn admin_list_users(
    State(service): State<AppService>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>> {
    let search = params.get("search").map(|s| s.as_str());
    let page = params
        .get("page")
        .and_then(|p| p.parse::<i64>().ok())
        .unwrap_or(1)
        .max(1);
    let per_page = params
        .get("perPage")
        .and_then(|p| p.parse::<i64>().ok())
        .unwrap_or(20)
        .clamp(1, 100);
    let (items, total) = service.admin_list_users(search, page, per_page).await?;
    Ok(Json(
        serde_json::json!({ "items": items, "total": total, "page": page, "perPage": per_page }),
    ))
}

pub async fn admin_get_user(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
) -> Result<Json<AdminUserDetail>> {
    let detail = service.admin_get_user_detail(id).await?;
    Ok(Json(detail))
}

pub async fn admin_revoke_user_sessions(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let revoked = service.admin_revoke_user_sessions(id).await?;
    Ok(Json(serde_json::json!({ "revoked": revoked })))
}

pub async fn admin_update_user_notes(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateUserAdminNotesRequest>,
) -> Result<StatusCode> {
    service
        .admin_update_user_notes(id, body.admin_notes.as_deref())
        .await?;
    Ok(StatusCode::OK)
}

pub async fn admin_set_user_blocked(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
    Json(body): Json<SetUserBlockedRequest>,
) -> Result<StatusCode> {
    service.admin_set_user_blocked(id, body.blocked).await?;
    Ok(StatusCode::OK)
}

pub async fn admin_generate_reset_token(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
) -> Result<Json<ResetTokenResponse>> {
    let resp = service.admin_generate_reset_token(id).await?;
    Ok(Json(resp))
}

pub async fn validate_reset_token(
    State(service): State<AppService>,
    Path(token): Path<String>,
) -> Result<Json<UserDto>> {
    let user = service.validate_reset_token(&token).await?;
    Ok(Json(user))
}

pub async fn apply_password_reset(
    State(service): State<AppService>,
    headers: HeaderMap,
    Json(body): Json<ApplyPasswordResetRequest>,
) -> Result<StatusCode> {
    let ip = extract_ip(&headers);
    service.check_rate_limit("reset", &ip, 10, 3600).await?;
    service
        .apply_password_reset(&body, client_ip(&ip), extract_user_agent(&headers))
        .await?;
    Ok(StatusCode::OK)
}

pub async fn forgot_password(
    State(service): State<AppService>,
    headers: HeaderMap,
    Json(body): Json<ForgotPasswordRequest>,
) -> Result<StatusCode> {
    let ip = extract_ip(&headers);
    service.check_rate_limit("forgot", &ip, 5, 3600).await?;
    // Always 200 — the response never reveals whether the account exists.
    service
        .request_password_reset(&body.email, client_ip(&ip), extract_user_agent(&headers))
        .await?;
    Ok(StatusCode::OK)
}

// ============================================================
// COMMENTS
// ============================================================

fn extract_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// The IP for storage: drop the `"unknown"` sentinel so we persist NULL rather
/// than a fake address (see `extract_ip`, which keeps it for rate-limit keys).
fn client_ip(ip: &str) -> Option<String> {
    (ip != "unknown").then(|| ip.to_string())
}

pub(crate) fn client_ip_from_headers(headers: &HeaderMap) -> Option<String> {
    client_ip(&extract_ip(headers))
}

/// Raw User-Agent header, truncated to a sane length for storage.
fn extract_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.chars().take(512).collect())
}

pub(crate) fn extract_user_agent_from_headers(headers: &HeaderMap) -> Option<String> {
    extract_user_agent(headers)
}

pub async fn get_figurine_comments(
    State(service): State<AppService>,
    Path(handle): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<CommentDto>>> {
    let figurine_id = service.resolve_figurine_uuid(&handle).await?;
    let newest_first = params.get("sort").map(|v| v == "newest").unwrap_or(false);
    let comments = service
        .get_figurine_comments(figurine_id, newest_first)
        .await?;
    Ok(Json(comments))
}

pub async fn submit_comment(
    State(service): State<AppService>,
    headers: HeaderMap,
    Path(handle): Path<String>,
    Json(body): Json<SubmitCommentRequest>,
) -> Result<StatusCode> {
    let figurine_id = service.resolve_figurine_uuid(&handle).await?;
    let user = if let Some(token) = bearer_token(&headers) {
        service.get_user_from_session(token).await.ok()
    } else {
        None
    };
    let ip = extract_ip(&headers);
    service
        .submit_comment(figurine_id, user.as_ref(), &body, &ip)
        .await?;
    Ok(StatusCode::CREATED)
}

pub async fn admin_list_comments(
    State(service): State<AppService>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<AdminCommentsPage>> {
    let only_pending = params.get("pending").map(|v| v == "true").unwrap_or(false);
    let newest_first = params.get("sort").map(|v| v == "newest").unwrap_or(true);
    let figurine_filter = params
        .get("figurineId")
        .and_then(|v| Uuid::parse_str(v).ok());
    let page = params
        .get("page")
        .and_then(|p| p.parse::<i64>().ok())
        .unwrap_or(1)
        .max(1);
    let per_page = params
        .get("perPage")
        .and_then(|p| p.parse::<i64>().ok())
        .unwrap_or(20)
        .clamp(1, 100);
    let page_data = service
        .admin_list_comments(only_pending, figurine_filter, newest_first, page, per_page)
        .await?;
    Ok(Json(page_data))
}

pub async fn admin_moderate_comment(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
    Json(body): Json<ModerateCommentRequest>,
) -> Result<Json<AdminCommentDto>> {
    let comment = service
        .admin_moderate_comment(id, body.is_approved, body.admin_reply.as_deref())
        .await?;
    Ok(Json(comment))
}

pub async fn admin_delete_comment(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    service.admin_delete_comment(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// === VISITOR IMPRESSIONS ("Book of Impressions") ===

pub async fn submit_impression(
    State(service): State<AppService>,
    headers: HeaderMap,
    Json(body): Json<SubmitImpressionRequest>,
) -> Result<StatusCode> {
    let ip = extract_ip(&headers);
    service.submit_impression(&body, &ip).await?;
    Ok(StatusCode::CREATED)
}

pub async fn get_featured_impressions(
    State(service): State<AppService>,
) -> Result<Json<Vec<ImpressionDto>>> {
    Ok(Json(service.get_featured_impressions().await?))
}

pub async fn admin_list_impressions(
    State(service): State<AppService>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<AdminImpressionsPage>> {
    let only_pending = params.get("pending").map(|v| v == "true").unwrap_or(false);
    let newest_first = params.get("sort").map(|v| v == "newest").unwrap_or(true);
    let page = params
        .get("page")
        .and_then(|p| p.parse::<i64>().ok())
        .unwrap_or(1)
        .max(1);
    let per_page = params
        .get("perPage")
        .and_then(|p| p.parse::<i64>().ok())
        .unwrap_or(20)
        .clamp(1, 100);
    let page_data = service
        .admin_list_impressions(only_pending, newest_first, page, per_page)
        .await?;
    Ok(Json(page_data))
}

pub async fn admin_moderate_impression(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
    Json(body): Json<ModerateImpressionRequest>,
) -> Result<Json<AdminImpressionDto>> {
    let impression = service
        .admin_moderate_impression(id, body.is_approved, body.is_featured)
        .await?;
    Ok(Json(impression))
}

pub async fn admin_delete_impression(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    service.admin_delete_impression(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn admin_get_smtp_settings(
    State(service): State<AppService>,
) -> Result<Json<SmtpSettings>> {
    let settings = service.get_smtp_settings().await?;
    Ok(Json(settings))
}

pub async fn admin_save_smtp_settings(
    State(service): State<AppService>,
    Json(body): Json<SmtpSettings>,
) -> Result<Json<SmtpSettings>> {
    service.save_smtp_settings(body.clone()).await?;
    Ok(Json(body))
}

pub async fn get_contact_settings(
    State(service): State<AppService>,
) -> Result<Json<ContactSettings>> {
    Ok(Json(service.get_contact_settings().await?))
}

pub async fn admin_save_contact_settings(
    State(service): State<AppService>,
    Json(body): Json<ContactSettings>,
) -> Result<Json<ContactSettings>> {
    service.save_contact_settings(body.clone()).await?;
    Ok(Json(body))
}

// === WORKSHOP FEATURE ===

pub async fn get_programme_settings(
    State(service): State<AppService>,
) -> Result<Json<ProgrammeSettings>> {
    Ok(Json(service.get_programme_settings().await?))
}

pub async fn save_programme_settings(
    State(service): State<AppService>,
    Json(body): Json<ProgrammeSettings>,
) -> Result<Json<ProgrammeSettings>> {
    service.save_programme_settings(body.clone()).await?;
    Ok(Json(body))
}

pub async fn get_workshop_feature(
    State(service): State<AppService>,
) -> Result<Json<WorkshopFeature>> {
    Ok(Json(service.get_workshop_feature().await?))
}

pub async fn save_workshop_feature(
    State(service): State<AppService>,
    Json(body): Json<WorkshopFeature>,
) -> Result<Json<WorkshopFeature>> {
    service.save_workshop_feature(body.clone()).await?;
    Ok(Json(body))
}

// === THEME CONFIG ===

pub async fn get_theme_config(State(service): State<AppService>) -> Result<Json<ThemeConfig>> {
    Ok(Json(service.get_theme_config().await?))
}

pub async fn save_theme_config(
    State(service): State<AppService>,
    Json(body): Json<ThemeConfig>,
) -> Result<Json<ThemeConfig>> {
    service.save_theme_config(body.clone()).await?;
    Ok(Json(body))
}

// === COPY OVERRIDES ===

pub async fn get_copy_overrides(State(service): State<AppService>) -> Result<Json<CopyOverrides>> {
    Ok(Json(service.get_copy_overrides().await?))
}

pub async fn save_copy_overrides(
    State(service): State<AppService>,
    Json(body): Json<CopyOverrides>,
) -> Result<Json<CopyOverrides>> {
    service.save_copy_overrides(body.clone()).await?;
    Ok(Json(body))
}

// === HOME LAYOUT CONFIG ===

pub async fn get_home_layout(State(service): State<AppService>) -> Result<Json<HomeLayoutConfig>> {
    Ok(Json(service.get_home_layout().await?))
}

pub async fn save_home_layout(
    State(service): State<AppService>,
    Json(body): Json<HomeLayoutConfig>,
) -> Result<Json<HomeLayoutConfig>> {
    service.save_home_layout(body.clone()).await?;
    Ok(Json(body))
}

// === REEL THEME ===

pub async fn get_reel_theme(State(service): State<AppService>) -> Result<Json<ReelTheme>> {
    Ok(Json(service.get_reel_theme().await?))
}

pub async fn save_reel_theme(
    State(service): State<AppService>,
    Json(body): Json<ReelTheme>,
) -> Result<Json<ReelTheme>> {
    service.save_reel_theme(body.clone()).await?;
    Ok(Json(body))
}

pub async fn admin_get_reel_theme_presets(
    State(service): State<AppService>,
) -> Result<Json<Vec<ReelThemePreset>>> {
    Ok(Json(service.get_reel_theme_presets().await?))
}

pub async fn admin_save_reel_theme_presets(
    State(service): State<AppService>,
    Json(body): Json<Vec<ReelThemePreset>>,
) -> Result<Json<Vec<ReelThemePreset>>> {
    service.save_reel_theme_presets(body.clone()).await?;
    Ok(Json(body))
}

pub async fn admin_get_home_layout_presets(
    State(service): State<AppService>,
) -> Result<Json<Vec<HomeLayoutPreset>>> {
    Ok(Json(service.get_home_layout_presets().await?))
}

pub async fn admin_save_home_layout_presets(
    State(service): State<AppService>,
    Json(body): Json<Vec<HomeLayoutPreset>>,
) -> Result<Json<Vec<HomeLayoutPreset>>> {
    service.save_home_layout_presets(body.clone()).await?;
    Ok(Json(body))
}

// === DISPLAY CONFIG PRESETS ===

pub async fn admin_get_display_presets(
    State(service): State<AppService>,
) -> Result<Json<Vec<crate::models::DisplayConfigPreset>>> {
    Ok(Json(service.get_display_presets().await?))
}

pub async fn admin_save_display_presets(
    State(service): State<AppService>,
    Json(body): Json<Vec<crate::models::DisplayConfigPreset>>,
) -> Result<Json<Vec<crate::models::DisplayConfigPreset>>> {
    service.save_display_presets(body.clone()).await?;
    Ok(Json(body))
}

pub async fn user_update_profile(
    State(service): State<AppService>,
    headers: HeaderMap,
    Json(body): Json<UpdateProfileRequest>,
) -> Result<Json<UserDto>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    let updated = service.update_profile(user.id, &body.display_name).await?;
    Ok(Json(updated))
}

pub async fn user_upload_avatar(
    State(service): State<AppService>,
    State(config): State<Config>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<UserDto>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        if field.name().unwrap_or("") == "file" {
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::Io(std::io::Error::other(e)))?;

            if data.len() > 10 * 1024 * 1024 {
                return Err(AppError::BadRequest(
                    "Avatar image is too large (max 10MB).".into(),
                ));
            }

            // Decode + resize + encode off the async runtime.
            let data_owned = data.to_vec();
            let buf = tokio::task::spawn_blocking(move || -> Result<Vec<u8>> {
                let img = image::load_from_memory(&data_owned)
                    .map_err(|e| AppError::BadRequest(format!("Invalid image: {}", e)))?;
                if (img.width() as u64) * (img.height() as u64) > MAX_IMAGE_PIXELS {
                    return Err(AppError::BadRequest(
                        "Image dimensions are too large".into(),
                    ));
                }
                let thumb = img.resize_to_fill(200, 200, FilterType::Lanczos3).to_rgb8();
                encode_jpeg_bytes(&thumb, 88)
            })
            .await
            .map_err(|e| AppError::Internal(format!("Image processing task failed: {}", e)))??;

            let id = Uuid::new_v4();
            let avatar_dir = format!("{}/avatars", config.upload_dir);
            fs::create_dir_all(&avatar_dir)
                .await
                .map_err(AppError::Io)?;

            let file_path = format!("{}/{}.jpg", avatar_dir, id);
            fs::write(&file_path, &buf).await.map_err(AppError::Io)?;

            let url = format!("/static/avatars/{}.jpg", id);
            let updated = service.set_user_avatar(user.id, &url).await?;
            return Ok(Json(updated));
        }
    }
    Err(AppError::BadRequest("No file field found".to_string()))
}

pub async fn user_delete_account(
    State(service): State<AppService>,
    headers: HeaderMap,
) -> Result<StatusCode> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    service.delete_account(user.id).await?;
    Ok(StatusCode::OK)
}

// === MESSAGING THREADS ===

pub async fn user_get_threads(
    State(service): State<AppService>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    let threads = service.get_user_threads(user.id).await?;
    let unread = service.count_unread_threads(user.id).await?;
    Ok(Json(
        serde_json::json!({ "threads": threads, "unread": unread }),
    ))
}

pub async fn user_get_thread(
    State(service): State<AppService>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ThreadDetailDto>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    Ok(Json(service.get_thread_detail(id, user.id).await?))
}

pub async fn user_create_thread(
    State(service): State<AppService>,
    headers: HeaderMap,
    Json(body): Json<CreateThreadRequest>,
) -> Result<Json<ThreadDetailDto>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    Ok(Json(
        service
            .user_create_thread(
                user.id,
                body.subject,
                body.body,
                body.category,
                body.attachment_urls,
            )
            .await?,
    ))
}

pub async fn user_reply_to_thread(
    State(service): State<AppService>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(body): Json<ReplyToThreadRequest>,
) -> Result<Json<ThreadMessageDto>> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    let user = service.get_user_from_session(token).await?;
    Ok(Json(
        service
            .user_reply_to_thread(id, user.id, body.body, body.attachment_urls)
            .await?,
    ))
}

pub async fn admin_list_threads(
    State(service): State<AppService>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>> {
    let category = params.get("category").cloned();
    let status = params.get("status").cloned();
    let page = params
        .get("page")
        .and_then(|p| p.parse::<i64>().ok())
        .unwrap_or(1)
        .max(1);
    let per_page = params
        .get("perPage")
        .and_then(|p| p.parse::<i64>().ok())
        .unwrap_or(25)
        .clamp(1, 100);
    Ok(Json(
        service
            .admin_list_threads(category, status, page, per_page)
            .await?,
    ))
}

pub async fn admin_get_thread(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
) -> Result<Json<ThreadDetailDto>> {
    Ok(Json(service.admin_get_thread_detail(id).await?))
}

pub async fn admin_create_thread_for_user(
    State(service): State<AppService>,
    Path(user_id): Path<Uuid>,
    Json(body): Json<CreateThreadRequest>,
) -> Result<Json<ThreadDetailDto>> {
    Ok(Json(
        service
            .admin_create_thread(
                user_id,
                body.subject,
                body.body,
                body.category,
                None,
                body.attachment_urls,
            )
            .await?,
    ))
}

pub async fn admin_reply_to_thread(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
    Json(body): Json<ReplyToThreadRequest>,
) -> Result<Json<ThreadMessageDto>> {
    Ok(Json(
        service
            .admin_reply_to_thread(id, body.body, body.attachment_urls)
            .await?,
    ))
}

pub async fn admin_resolve_thread(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    service.admin_resolve_thread(id).await?;
    Ok(StatusCode::OK)
}

pub async fn admin_reopen_thread(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    service.admin_reopen_thread(id).await?;
    Ok(StatusCode::OK)
}

// === BOOKING RULES ===

pub async fn get_booking_rules(State(service): State<AppService>) -> Result<Json<BookingRules>> {
    Ok(Json(service.get_booking_rules().await?))
}

pub async fn save_booking_rules(
    State(service): State<AppService>,
    Json(body): Json<BookingRules>,
) -> Result<StatusCode> {
    service.save_booking_rules(body).await?;
    Ok(StatusCode::OK)
}

// === RESCHEDULE BOOKING ===

pub async fn reschedule_booking_by_token(
    State(service): State<AppService>,
    Path(token): Path<String>,
    Json(body): Json<RescheduleBookingRequest>,
) -> Result<Json<BookingCancelInfo>> {
    Ok(Json(
        service.reschedule_booking_by_token(&token, body).await?,
    ))
}

// === WAITLIST ===

pub async fn join_waitlist(
    State(service): State<AppService>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<CreateWaitlistRequest>,
) -> Result<Json<crate::models::WaitlistCreatedResponse>> {
    service
        .check_rate_limit("waitlist", &extract_ip(&headers), 15, 3600)
        .await?;
    let user_id = if let Some(token) = bearer_token(&headers) {
        service
            .get_user_from_session(token)
            .await
            .ok()
            .map(|u| u.id)
    } else {
        None
    };
    Ok(Json(service.join_waitlist(id, body, user_id).await?))
}

pub async fn get_waitlist_by_token(
    State(service): State<AppService>,
    headers: HeaderMap,
    Path(token): Path<String>,
) -> Result<Json<crate::models::WaitlistCancelInfo>> {
    service
        .check_rate_limit("token", &extract_ip(&headers), 60, 3600)
        .await?;
    service
        .get_waitlist_by_token(&token)
        .await?
        .map(Json)
        .ok_or_else(|| crate::error::AppError::NotFound("Queue entry not found".to_string()))
}

pub async fn leave_waitlist_by_token(
    State(service): State<AppService>,
    headers: HeaderMap,
    Path(token): Path<String>,
) -> Result<StatusCode> {
    service
        .check_rate_limit("token", &extract_ip(&headers), 60, 3600)
        .await?;
    service.leave_waitlist_by_token(&token).await?;
    Ok(StatusCode::OK)
}

pub async fn admin_notify_waitlist(
    State(service): State<AppService>,
    Path(figurine_id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    Ok(Json(service.admin_notify_waitlist(figurine_id).await?))
}

pub async fn admin_list_waitlist(
    State(service): State<AppService>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<WaitlistEntryDto>>> {
    let figurine_id = params.get("figurineId").cloned();
    Ok(Json(service.list_waitlist_admin(figurine_id).await?))
}

pub async fn admin_remove_from_waitlist(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    service.remove_waitlist_entry(id).await?;
    Ok(StatusCode::OK)
}

// === NEWSLETTER ("visitor book") ===

pub async fn subscribe(
    State(service): State<AppService>,
    headers: HeaderMap,
    Json(body): Json<CreateSubscriptionRequest>,
) -> Result<Json<SubscriptionCreatedResponse>> {
    let ip = extract_ip(&headers);
    service.check_rate_limit("subscribe", &ip, 10, 3600).await?;
    Ok(Json(service.subscribe(body, Some(ip)).await?))
}

pub async fn get_subscription_by_token(
    State(service): State<AppService>,
    headers: HeaderMap,
    Path(token): Path<String>,
) -> Result<Json<SubscriberInfo>> {
    service
        .check_rate_limit("token", &extract_ip(&headers), 60, 3600)
        .await?;
    service
        .get_subscriber_by_token(&token)
        .await?
        .map(Json)
        .ok_or_else(|| crate::error::AppError::NotFound("Subscription not found".to_string()))
}

pub async fn unsubscribe_by_token(
    State(service): State<AppService>,
    headers: HeaderMap,
    Path(token): Path<String>,
) -> Result<StatusCode> {
    service
        .check_rate_limit("token", &extract_ip(&headers), 60, 3600)
        .await?;
    service.unsubscribe_by_token(&token).await?;
    Ok(StatusCode::OK)
}

pub async fn admin_list_subscribers(
    State(service): State<AppService>,
) -> Result<Json<Vec<SubscriberDto>>> {
    Ok(Json(service.list_subscribers_admin().await?))
}

pub async fn admin_remove_subscriber(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    service.remove_subscriber(id).await?;
    Ok(StatusCode::OK)
}

// === CONTACT MESSAGES ("write to the author") ===

pub async fn submit_contact_message(
    State(service): State<AppService>,
    headers: HeaderMap,
    Json(body): Json<CreateContactMessageRequest>,
) -> Result<StatusCode> {
    let ip = extract_ip(&headers);
    service
        .check_rate_limit("contact_message", &ip, 5, 3600)
        .await?;
    service.submit_contact_message(body, Some(ip)).await?;
    Ok(StatusCode::OK)
}

pub async fn admin_list_contact_messages(
    State(service): State<AppService>,
) -> Result<Json<Vec<ContactMessageDto>>> {
    Ok(Json(service.list_contact_messages_admin().await?))
}

pub async fn admin_mark_contact_message_read(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    service.mark_contact_message_read(id).await?;
    Ok(StatusCode::OK)
}

pub async fn admin_remove_contact_message(
    State(service): State<AppService>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    service.remove_contact_message(id).await?;
    Ok(StatusCode::OK)
}
