use crate::config::Config;
use crate::db::Repository;
use crate::error::{Result, AppError};
use crate::models::*;
use reqwest::Client;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use uuid::Uuid;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{SaltString, rand_core::OsRng};

type RateLimiter = Arc<Mutex<HashMap<String, Vec<Instant>>>>;

#[derive(Clone)]
pub struct AppService {
    repo: Repository,
    config: Config,
    comment_rate_limiter: RateLimiter,
    commission_rate_limiter: RateLimiter,
    /// Shared limiter for assorted public endpoints (auth, bookings, orders,
    /// waitlist, token lookups), keyed by "bucket|ip".
    general_rate_limiter: RateLimiter,
    /// Offline IP → country/city resolver (best-effort; may be a no-op).
    geoip: Arc<crate::geo::GeoIp>,
}

impl AppService {
    pub fn new(repo: Repository, config: Config) -> Self {
        let geoip = Arc::new(crate::geo::GeoIp::open(config.geoip_db_path.as_deref()));
        Self {
            repo,
            config,
            comment_rate_limiter: Arc::new(Mutex::new(HashMap::new())),
            commission_rate_limiter: Arc::new(Mutex::new(HashMap::new())),
            general_rate_limiter: Arc::new(Mutex::new(HashMap::new())),
            geoip,
        }
    }

    /// Delete login attempts older than the retention window (privacy / GDPR).
    pub async fn prune_login_history(&self, retention_days: i64) -> Result<u64> {
        self.repo.prune_old_login_attempts(retention_days).await
    }

    /// Assemble request metadata for storage, resolving geolocation from the IP.
    fn client_context(&self, ip: Option<String>, user_agent: Option<String>) -> crate::models::ClientContext {
        let (country_code, city) = ip
            .as_deref()
            .map(|i| self.geoip.lookup(i))
            .unwrap_or((None, None));
        crate::models::ClientContext { ip, user_agent, country_code, city }
    }

    /// Generic in-memory rate limit: at most `max` requests per `window_secs`
    /// for a given (bucket, ip). Best-effort (in-process, resets on restart) —
    /// a coarse abuse brake, not a hard quota.
    pub async fn check_rate_limit(&self, bucket: &str, ip: &str, max: usize, window_secs: u64) -> Result<()> {
        let now = Instant::now();
        let window = Duration::from_secs(window_secs);
        let key = format!("{bucket}|{ip}");
        let mut map = self.general_rate_limiter.lock().await;
        let entry = map.entry(key).or_default();
        entry.retain(|t: &Instant| now.duration_since(*t) < window);
        if entry.len() >= max {
            return Err(AppError::BadRequest(
                "Too many requests from this address. Please wait and try again.".into()
            ));
        }
        entry.push(now);
        Ok(())
    }

    pub async fn check_commission_rate_limit(&self, ip: &str) -> Result<()> {
        const MAX_PER_HOUR: usize = 6;
        let now = Instant::now();
        let cutoff_secs = Duration::from_secs(3600);
        let mut map = self.commission_rate_limiter.lock().await;
        let entry = map.entry(ip.to_string()).or_default();
        entry.retain(|t: &Instant| now.duration_since(*t) < cutoff_secs);
        if entry.len() >= MAX_PER_HOUR {
            return Err(AppError::BadRequest(
                "Too many requests from this address. Please wait before submitting again.".into()
            ));
        }
        entry.push(now);
        Ok(())
    }

    pub async fn initialize(&self) -> Result<()> {
        Ok(()) // Postgres is always ready, no pool to load
    }

    // === CONTENT API ===

    // Resolve a stored path/URL to a full URL for the frontend:
    // - "http..." → use as-is (external URL or legacy full URL)
    // - "/static/..." → prepend public_url (web-uploaded file, stored as relative path)
    // - anything else → treat as a relative path under /static (legacy stored paths)
    fn resolve_url(&self, file_path: &str, _table: &str, _id: &str) -> String {
        let base = self.config.public_url.trim_end_matches('/');
        if file_path.starts_with("http") {
            file_path.to_string()
        } else if file_path.starts_with("/static/") {
            format!("{}{}", base, file_path)
        } else {
            format!("{}/static/{}", base, file_path.trim_start_matches('/'))
        }
    }

    fn parse_uuid(s: &str) -> Result<Uuid> {
        Uuid::parse_str(s)
            .map_err(|_| crate::error::AppError::BadRequest(format!("Invalid ID: {}", s)))
    }

    /// Resolve the public URL for a figurine's face thumbnail (thumb if present,
    /// else the full image).
    fn face_image_url(&self, img: &Image) -> String {
        let i_id_str = img.id.to_string();
        img.thumb_path
            .as_ref()
            .map(|p| self.resolve_url(p, "images_thumb", &i_id_str))
            .unwrap_or_else(|| self.resolve_url(&img.file_path, "images", &i_id_str))
    }

    fn to_list_item(&self, f: Figurine, face: Option<&Image>) -> FigurineListItemDto {
        FigurineListItemDto {
            id: f.id.to_string(),
            name: f.name,
            status: f.status,
            face_image_url: face.map(|i| self.face_image_url(i)),
            year: f.year,
            sort_order: f.sort_order,
            series: None,
            technique: f.technique,
            material: f.material,
            is_featured: f.is_featured,
            created_at: f.created_at,
        }
    }

    pub async fn list_figurines(&self, visible_only: bool) -> Result<Vec<FigurineListItemDto>> {
        let figurines = self.repo.get_all_figurines(visible_only).await?;
        let ids: Vec<Uuid> = figurines.iter().map(|f| f.id).collect();
        let faces = self.repo.get_face_images_for_figurines(&ids).await?;
        Ok(figurines.into_iter()
            .map(|f| {
                let face = faces.get(&f.id);
                self.to_list_item(f, face)
            })
            .collect())
    }

    pub async fn list_in_progress_figurines(&self) -> Result<Vec<FigurineListItemDto>> {
        let all = self.repo.get_all_figurines(true).await?;
        let figurines: Vec<Figurine> = all.into_iter()
            .filter(|f| f.status == crate::models::FigurineStatus::InProgress)
            .collect();
        let ids: Vec<Uuid> = figurines.iter().map(|f| f.id).collect();
        let faces = self.repo.get_face_images_for_figurines(&ids).await?;
        Ok(figurines.into_iter()
            .map(|f| {
                let face = faces.get(&f.id);
                self.to_list_item(f, face)
            })
            .collect())
    }

    pub async fn get_figurine_details(&self, id: String) -> Result<FigurineDto> {
        let uuid = Self::parse_uuid(&id)?;
        let figurine = self.repo.get_figurine_by_id(uuid).await?
            .ok_or_else(|| crate::error::AppError::NotFound(format!("Figurine {} not found", id)))?;

        let images = self.repo.get_images_by_figurine(uuid).await?;
        let steps = self.repo.get_steps_by_figurine(uuid).await?;
        let related_entities = self.repo.get_related_figurines(uuid).await?;

        let fig_id_str = figurine.id.to_string();

        let related_ids: Vec<Uuid> = related_entities.iter().map(|r| r.id).collect();
        let related_faces = self.repo.get_face_images_for_figurines(&related_ids).await?;
        let related_items: Vec<FigurineListItemDto> = related_entities.into_iter()
            .map(|r| {
                let face = related_faces.get(&r.id);
                self.to_list_item(r, face)
            })
            .collect();

        let image_dtos = images.into_iter().map(|i| {
            let i_id_str = i.id.to_string();
            ImageDto {
                id: i_id_str.clone(),
                image_type: i.image_type,
                url: self.resolve_url(&i.file_path, "images", &i_id_str),
                original_url: i.original_path.as_ref()
                    .map(|p| self.resolve_url(p, "images_original", &i_id_str)),
                thumb_url: i.thumb_path.as_ref()
                    .map(|p| self.resolve_url(p, "images_thumb", &i_id_str)),
                alt_text: i.alt_text,
            }
        }).collect();

        let step_dtos = steps.into_iter().map(|s| {
            let s_id_str = s.id.to_string();
            ProcessStepDto {
                id: s_id_str.clone(),
                step_type: s.step_type,
                description: s.description,
                image_url: self.resolve_url(&s.image_path, "process_steps", &s_id_str),
            }
        }).collect();

        Ok(FigurineDto {
            id: fig_id_str.clone(),
            name: figurine.name,
            short_text: figurine.short_text,
            full_description: figurine.full_description,
            dimensions: figurine.dimensions,
            material: figurine.material,
            technique: figurine.technique,
            year: figurine.year,
            ambience_path: figurine.ambience_path.as_ref()
                .map(|p| self.resolve_url(p, "figurines_audio", &fig_id_str)),
            video_url: figurine.video_url.as_ref()
                .map(|p| self.resolve_url(p, "figurines_video", &fig_id_str)),
            secret_text: figurine.secret_text,
            status: figurine.status,
            sort_order: figurine.sort_order,
            is_visible: figurine.is_visible,
            is_featured: figurine.is_featured,
            images: image_dtos,
            process_steps: step_dtos,
            related_items,
        })
    }

    pub async fn get_author_texts(&self) -> Result<Vec<TextDto>> {
        let texts = self.repo.get_texts_by_category(TextCategory::Author).await?;
        Ok(texts.into_iter().map(|t| TextDto {
            id: t.id.to_string(),
            content: t.content
        }).collect())
    }

    pub async fn get_workshop_items(&self) -> Result<Vec<WorkshopItemDto>> {
        let texts = self.repo.get_texts_by_category(TextCategory::Workshop).await?;
        Ok(texts.into_iter().map(|t| {
            let t_id_str = t.id.to_string();
            WorkshopItemDto {
                id: t_id_str.clone(),
                content: t.content,
                caption: t.caption,
                image_url: t.image_path.as_ref().map(|p| self.resolve_url(p, "texts", &t_id_str)),
            }
        }).collect())
    }

    pub async fn get_cabinet_zones(&self) -> Result<Vec<CabinetZoneDto>> {
        let zones = self.repo.get_zones().await?;
        Ok(zones.into_iter().map(|z| CabinetZoneDto {
            id: z.id.to_string(),
            zone_type: z.zone_type,
            x: z.x_percent,
            y: z.y_percent,
            width: z.width_percent,
            height: z.height_percent,
            target_route: z.target_route,
        }).collect())
    }

    // === ADMIN WRITE ===

    pub async fn save_figurine(&self, req: crate::models::SaveFigurineRequest) -> Result<()> {
        let figurine_id = Self::parse_uuid(&req.id)?;
        validate_text("Name", &req.name, 200)?;
        if req.images.len() > 50 {
            return Err(AppError::BadRequest("Too many images (max 50)".into()));
        }
        if req.process_steps.len() > 50 {
            return Err(AppError::BadRequest("Too many process steps (max 50)".into()));
        }
        let prev_status = self.repo.get_figurine_by_id(figurine_id).await?.map(|f| f.status);
        self.repo.save_figurine_full(&req, &req.images, &req.process_steps).await?;
        // Just flipped to available → alert the author to everyone who was waiting
        // (queue + notify-me), so they can reach out personally.
        let became_available = req.status == crate::models::FigurineStatus::Available
            && prev_status.as_ref().is_some_and(|s| *s != crate::models::FigurineStatus::Available);
        if became_available {
            let _ = self.send_availability_digest(figurine_id, &req.name).await;
        }
        Ok(())
    }

    pub async fn delete_figurine(&self, id: String) -> Result<()> {
        let uuid = Self::parse_uuid(&id)?;
        self.repo.delete_figurine(uuid).await
    }

    pub async fn save_zone(&self, req: crate::models::SaveZoneRequest) -> Result<()> {
        let count = self.repo.get_zone_count().await?;
        self.repo.upsert_zone(&req, count).await
    }

    pub async fn delete_zone(&self, id: String) -> Result<()> {
        let uuid = Self::parse_uuid(&id)?;
        self.repo.delete_zone(uuid).await
    }

    pub async fn save_text(&self, category: crate::models::TextCategory, req: crate::models::SaveTextRequest) -> Result<()> {
        self.repo.upsert_text(&req, &category).await
    }

    pub async fn delete_text_item(&self, id: String) -> Result<()> {
        let uuid = Self::parse_uuid(&id)?;
        self.repo.delete_text(uuid).await
    }

    pub async fn get_background(&self) -> Result<Option<String>> {
        let path = self.repo.get_main_background().await?;
        Ok(path.map(|p| {
            let base = self.config.public_url.trim_end_matches('/');
            if p.starts_with("http") {
                p
            } else if p.starts_with("/static/") {
                format!("{}{}", base, p)
            } else {
                format!("{}/static/{}", base, p.trim_start_matches('/'))
            }
        }))
    }

    pub async fn set_background(&self, url: String) -> Result<()> {
        self.repo.set_main_background(&url).await
    }

    pub async fn get_home_content(&self) -> Result<HomeContent> {
        Ok(self.repo.get_home_content().await?.unwrap_or_default())
    }

    pub async fn save_home_content(&self, content: HomeContent) -> Result<()> {
        self.repo.save_home_content(&content).await
    }

    // === AUTHOR PROFILE ===

    pub async fn get_author_profile(&self) -> Result<AuthorProfile> {
        Ok(self.repo.get_author_profile().await?.unwrap_or_default())
    }

    pub async fn save_author_profile(&self, profile: AuthorProfile) -> Result<()> {
        self.repo.save_author_profile(&profile).await
    }

    // === ORDERS / NOTIFICATIONS ===

    pub async fn create_order(&self, order: &OrderRequest, user_id: Option<Uuid>) -> Result<Order> {
        if order.mode == OrderMode::Reserve {
            let figurine_id = uuid::Uuid::parse_str(&order.figurine_id)
                .map_err(|_| AppError::BadRequest("Invalid figurine ID".to_string()))?;
            let figurine = self.repo.get_figurine_by_id(figurine_id).await?
                .ok_or_else(|| AppError::NotFound("Figurine not found".to_string()))?;
            if figurine.status != FigurineStatus::Available {
                return Err(AppError::BadRequest(
                    "Reserve requests are available only for available works".to_string(),
                ));
            }
        }

        // Notify-me subscriptions are deduplicated and carry a cancel token;
        // request/question are plain one-off messages.
        let saved = if order.mode == OrderMode::Notify {
            self.repo.upsert_notify_order(order, user_id).await?
        } else {
            self.repo.save_order(order, user_id).await?
        };
        let _ = self.send_order_notification(&saved).await;
        Ok(saved)
    }

    pub async fn link_orders_to_user(&self, user_id: Uuid, email: &str) -> Result<u64> {
        self.repo.link_orders_to_user(user_id, email).await
    }

    /// View a notify subscription by its token (visitor's receipt lookup).
    pub async fn get_notify_by_token(&self, token: &str) -> Result<Option<crate::models::NotifyInfo>> {
        let Some(o) = self.repo.get_order_by_cancel_token(token).await? else {
            return Ok(None);
        };
        Ok(Some(crate::models::NotifyInfo {
            figurine_id: o.figurine_id,
            figurine_name: o.figurine_name,
        }))
    }

    /// Stop a notify subscription by token. Idempotent.
    pub async fn cancel_notify_by_token(&self, token: &str) -> Result<()> {
        self.repo.delete_order_by_cancel_token(token).await
    }

    pub async fn list_orders(
        &self,
        status_filter: Option<&str>,
        mode_filter: Option<&str>,
        page: i64,
        per_page: i64,
    ) -> Result<OrdersPage> {
        let offset = (page - 1) * per_page;
        let (items, total) = self.repo.get_orders_page(status_filter, mode_filter, per_page, offset).await?;
        let new_count = self.repo.get_new_orders_count().await?;
        Ok(OrdersPage { items, total, new_count, page, per_page })
    }

    pub async fn update_order_status(&self, id: uuid::Uuid, body: &UpdateOrderStatusRequest) -> Result<()> {
        self.repo.update_order_status(
            id,
            &body.status,
            body.admin_notes.as_deref(),
            body.reserve_status.as_ref(),
            body.reserve_expires_at.as_deref(),
            body.admin_terms_note.as_deref(),
            body.invoice_note.as_deref(),
        ).await?;
        if body.status == OrderStatus::Replied
            && let Ok(Some(order)) = self.repo.get_order_by_id(id).await
                && let Some(user_id) = order.user_id {
                    let subject = format!("Ответ на ваш запрос — {}", order.figurine_name);
                    let body = match order.admin_notes.as_deref() {
                        Some(n) if !n.is_empty() => format!(
                            "Ваш запрос по «{}» получил ответ.\n\n{}",
                            order.figurine_name, n
                        ),
                        _ => format!("Ваш запрос по «{}» получил ответ.", order.figurine_name),
                    };
                    let _ = self.repo.create_thread(user_id, "order", Some(order.id), &subject, &body, true, &[]).await;
                }
        Ok(())
    }

    async fn send_order_notification(&self, order: &Order) -> Result<()> {
        let (Some(token), Some(chat_id)) = (
            self.config.telegram_bot_token.as_deref(),
            self.config.telegram_chat_id.as_deref(),
        ) else {
            return Ok(());
        };

        let mode_label = match order.mode {
            OrderMode::Request  => "🛒 Запрос на покупку",
            OrderMode::Question => "❓ Вопрос",
            OrderMode::Notify   => "🔔 Уведомить о наличии",
            OrderMode::Reserve  => "🔒 Запрос резерва",
        };

        let admin_link = format!(
            "{}/admin#orders",
            self.config.public_url.trim_end_matches('/')
        );

        let text = format!(
            "{}\n\n\
            🏺 Фигурка: {}\n\
            👤 Имя: {}\n\
            📧 Email: {}\n\
            💬 Сообщение: {}\n\n\
            🔗 [Открыть в админке]({})",
            escape_markdown(mode_label),
            escape_markdown(&order.figurine_name),
            escape_markdown(&order.requester_name),
            escape_markdown(&order.requester_email),
            escape_markdown(order.message.as_deref().unwrap_or("—")),
            admin_link,
        );

        let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
        let client = Client::new();
        let _ = client.post(&url)
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "text": text,
                "parse_mode": "MarkdownV2"
            }))
            .send()
            .await;

        Ok(())
    }

    // === COMMISSIONS ===

    async fn commission_to_dto(&self, c: &Commission) -> Result<CommissionDto> {
        let attachments = self.repo.get_commission_attachments(c.id).await?;
        let thread = self.repo.find_thread_by_reference(c.id, "commission").await?;
        Ok(CommissionDto {
            id: c.id.to_string(),
            claim_token: c.claim_token.clone(),
            requester_name: c.requester_name.clone(),
            requester_email: c.requester_email.clone(),
            requester_phone: c.requester_phone.clone(),
            title: c.title.clone(),
            description: c.description.clone(),
            size_note: c.size_note.clone(),
            mood: c.mood.clone(),
            deadline: c.deadline.map(|d| d.to_string()),
            budget_note: c.budget_note.clone(),
            occasion: c.occasion.clone(),
            source_figurine_id: c.source_figurine_id.clone(),
            similar_keep_note: c.similar_keep_note.clone(),
            similar_change_note: c.similar_change_note.clone(),
            similar_tags: c.similar_tags.clone(),
            figurine_id: c.figurine_id.clone(),
            status: c.status.clone(),
            admin_notes: c.admin_notes.clone(),
            created_at: c.created_at.to_rfc3339(),
            updated_at: c.updated_at.to_rfc3339(),
            attachments: attachments.iter().map(AttachmentDto::from).collect(),
            thread_id: thread.map(|t| t.id.to_string()),
            started: c.status.is_started(),
        })
    }

    pub async fn create_commission(&self, req: &CommissionRequest) -> Result<CommissionCreatedResponse> {
        validate_attachments(&req.attachment_urls)?;
        if let Some(source_id) = req.source_figurine_id.as_deref().map(str::trim).filter(|id| !id.is_empty()) {
            let uuid = Self::parse_uuid(source_id)?;
            if self.repo.get_figurine_by_id(uuid).await?.is_none() {
                return Err(crate::error::AppError::BadRequest("Source figurine not found.".into()));
            }
        }
        let saved = self.repo.create_commission(req).await?;
        let _ = self.send_commission_notification(&saved).await;
        Ok(CommissionCreatedResponse {
            id: saved.id.to_string(),
            claim_token: saved.claim_token,
        })
    }

    pub async fn get_commission_by_token(&self, token: &str) -> Result<Option<CommissionDto>> {
        match self.repo.get_commission_by_token(token).await? {
            Some(c) => Ok(Some(self.commission_to_dto(&c).await?)),
            None => Ok(None),
        }
    }

    pub async fn claim_commission(&self, token: &str, user_id: Uuid) -> Result<CommissionDto> {
        let commission = self.repo.claim_commission(token, user_id).await?
            .ok_or_else(|| crate::error::AppError::NotFound("Commission not found".to_string()))?;

        // Seed a conversation thread with the original request, once.
        if self.repo.find_thread_by_reference(commission.id, "commission").await?.is_none() {
            let subject = if !commission.title.trim().is_empty() {
                commission.title.clone()
            } else if commission.lang == "en" {
                "A petition for a new figurine".to_string()
            } else {
                "Прошение о новой фигурке".to_string()
            };
            let _ = self.repo.create_thread(
                user_id, "commission", Some(commission.id), &subject, &commission.description, false, &[],
            ).await;
        }

        self.commission_to_dto(&commission).await
    }

    /// Edit a petition's content. `owner` limits the action to the petition's
    /// author (None ⇒ admin). Refused once work has started.
    pub async fn edit_commission(
        &self,
        id: Uuid,
        owner: Option<Uuid>,
        req: &EditCommissionRequest,
    ) -> Result<CommissionDto> {
        let existing = self.repo.get_commission_by_id(id).await?
            .ok_or_else(|| crate::error::AppError::NotFound("Commission not found".to_string()))?;
        if let Some(uid) = owner
            && existing.user_id != Some(uid) {
                return Err(crate::error::AppError::Unauthorized);
            }
        if existing.status.is_started() {
            return Err(crate::error::AppError::BadRequest(
                "Work has already begun on this petition — it can no longer be edited.".into()
            ));
        }
        if let Some(atts) = &req.attachment_urls {
            validate_attachments(atts)?;
        }
        let updated = self.repo.update_commission_content(id, req).await?
            .ok_or_else(|| crate::error::AppError::NotFound("Commission not found".to_string()))?;
        self.commission_to_dto(&updated).await
    }

    /// Delete a petition. `owner` limits the action to its author (None ⇒ admin).
    /// Refused once work has started.
    pub async fn delete_commission(&self, id: Uuid, owner: Option<Uuid>) -> Result<()> {
        let existing = self.repo.get_commission_by_id(id).await?
            .ok_or_else(|| crate::error::AppError::NotFound("Commission not found".to_string()))?;
        if let Some(uid) = owner
            && existing.user_id != Some(uid) {
                return Err(crate::error::AppError::Unauthorized);
            }
        if existing.status.is_started() {
            return Err(crate::error::AppError::BadRequest(
                "Work has already begun on this petition — it can no longer be deleted.".into()
            ));
        }

        // When the master removes a claimed petition, the petitioner must be told.
        // The commission's own conversation is cascade-deleted with it, so the
        // notice goes into a separate, persistent system thread (owner = None ⇒ admin).
        if owner.is_none()
            && let Some(user_id) = existing.user_id {
                let en = existing.lang == "en";
                let titled = !existing.title.trim().is_empty();
                let subject = if en { "Petition removed" } else { "Прошение снято" };
                let body = if en {
                    if titled {
                        format!("Your petition “{}” has been removed by the archive keeper. You are welcome to send a new one.", existing.title)
                    } else {
                        "Your petition has been removed by the archive keeper. You are welcome to send a new one.".to_string()
                    }
                } else if titled {
                    format!("Ваше прошение «{}» снято хранителем архива. Вы можете отправить новое.", existing.title)
                } else {
                    "Ваше прошение снято хранителем архива. Вы можете отправить новое.".to_string()
                };
                let _ = self.repo.create_thread(user_id, "system", None, subject, &body, true, &[]).await;
            }

        self.repo.delete_commission(id).await
    }

    pub async fn list_commissions(
        &self,
        status_filter: Option<&str>,
        similar_only: bool,
        page: i64,
        per_page: i64,
    ) -> Result<CommissionsPage> {
        let offset = (page - 1) * per_page;
        let (items, total) = self.repo.get_commissions_page(status_filter, similar_only, per_page, offset).await?;
        let new_count = self.repo.get_new_commissions_count().await?;
        let mut dtos = Vec::with_capacity(items.len());
        for c in &items {
            dtos.push(self.commission_to_dto(c).await?);
        }
        Ok(CommissionsPage { items: dtos, total, new_count, page, per_page })
    }

    pub async fn get_user_commissions(&self, user_id: Uuid) -> Result<Vec<CommissionDto>> {
        let items = self.repo.get_user_commissions(user_id).await?;
        let mut dtos = Vec::with_capacity(items.len());
        for c in &items {
            dtos.push(self.commission_to_dto(c).await?);
        }
        Ok(dtos)
    }

    /// Adopt unclaimed petitions sent from this account's email. Each goes through
    /// claim_commission so its conversation thread is seeded exactly as on manual
    /// claim. Covers guest petitions and the case where only the last claim token
    /// survived in the browser's localStorage.
    pub async fn adopt_commissions_by_email(&self, user_id: Uuid, email: &str) -> Result<()> {
        let tokens = self.repo.orphan_commission_tokens_by_email(email).await?;
        for token in tokens {
            let _ = self.claim_commission(&token, user_id).await;
        }
        Ok(())
    }

    pub async fn update_commission(
        &self,
        id: Uuid,
        status: &CommissionStatus,
        admin_notes: Option<&str>,
        figurine_id: Option<&str>,
    ) -> Result<Option<CommissionDto>> {
        let updated = self.repo.update_commission(id, status, admin_notes, figurine_id).await?;
        if let Some(ref c) = updated {
            // If the petitioner has an account, drop a note into their conversation,
            // in the language they wrote the petition in.
            if let Some(user_id) = c.user_id {
                let en = c.lang == "en";
                let label = match status {
                    CommissionStatus::Accepted   => Some(if en { "Your petition is accepted — the master takes up the work." } else { "Ваше прошение принято — мастер берётся за работу." }),
                    CommissionStatus::InProgress => Some(if en { "The master has begun your figurine." } else { "Мастер приступил к вашей фигурке." }),
                    CommissionStatus::Completed  => Some(if en { "Your figurine is finished." } else { "Ваша фигурка завершена." }),
                    CommissionStatus::Declined   => Some(if en { "Regrettably, the master will not take up this petition." } else { "К сожалению, мастер не возьмётся за это прошение." }),
                    _ => None,
                };
                if let Some(text) = label {
                    let body = match admin_notes {
                        Some(n) if !n.trim().is_empty() => format!("{}\n\n{}", text, n),
                        _ => text.to_string(),
                    };
                    if let Some(thread) = self.repo.find_thread_by_reference(c.id, "commission").await? {
                        let _ = self.repo.add_thread_reply(thread.id, uuid::Uuid::nil(), true, &body, &[]).await;
                    } else {
                        let subject = if !c.title.trim().is_empty() { c.title.clone() } else if en { "Your petition".to_string() } else { "Ваше прошение".to_string() };
                        let _ = self.repo.create_thread(user_id, "commission", Some(c.id), &subject, &body, true, &[]).await;
                    }
                }
            }
            Ok(Some(self.commission_to_dto(c).await?))
        } else {
            Ok(None)
        }
    }

    async fn send_commission_notification(&self, c: &Commission) -> Result<()> {
        let (Some(token), Some(chat_id)) = (
            self.config.telegram_bot_token.as_deref(),
            self.config.telegram_chat_id.as_deref(),
        ) else {
            return Ok(());
        };

        let admin_link = format!("{}/admin#commissions", self.config.public_url.trim_end_matches('/'));
        let title = if c.title.trim().is_empty() { "—" } else { c.title.as_str() };
        let text = format!(
            "🗝 Новое прошение о фигурке\n\n\
            ✒️ Идея: {}\n\
            📝 Описание: {}\n\
            👤 Имя: {}\n\
            📧 Email: {}\n\n\
            🔗 [Открыть в админке]({})",
            escape_markdown(title),
            escape_markdown(&c.description),
            escape_markdown(&c.requester_name),
            escape_markdown(&c.requester_email),
            admin_link,
        );

        let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
        let client = Client::new();
        let _ = client.post(&url)
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "text": text,
                "parse_mode": "MarkdownV2",
            }))
            .send()
            .await;
        Ok(())
    }

    // === SHOWINGS & BOOKINGS (PUBLIC) ===

    pub async fn get_figurine_schedule(&self, figurine_id: String) -> Result<FigurineScheduleDto> {
        let uuid = Self::parse_uuid(&figurine_id)?;
        let (showings, confirmed, pending) = self.repo.get_figurine_schedule(uuid).await?;

        let mut entries: Vec<ScheduleEntryDto> = Vec::new();

        for s in showings {
            entries.push(ScheduleEntryDto {
                entry_type: "showing".to_string(),
                title: Some(s.title),
                showing_type: Some(s.showing_type),
                venue: s.venue,
                starts_at: s.starts_at.to_string(),
                ends_at: s.ends_at.to_string(),
            });
        }

        for b in confirmed {
            entries.push(ScheduleEntryDto {
                entry_type: "booking".to_string(),
                title: None,
                showing_type: None,
                venue: None,
                starts_at: b.starts_at.to_string(),
                ends_at: b.ends_at.to_string(),
            });
        }

        for b in pending {
            entries.push(ScheduleEntryDto {
                entry_type: "pending".to_string(),
                title: None,
                showing_type: None,
                venue: None,
                starts_at: b.starts_at.to_string(),
                ends_at: b.ends_at.to_string(),
            });
        }

        entries.sort_by(|a, b| a.starts_at.cmp(&b.starts_at));
        Ok(FigurineScheduleDto { entries })
    }

    pub async fn get_booking_by_token(&self, token: &str) -> Result<Option<crate::models::BookingCancelInfo>> {
        Ok(self.repo.get_booking_by_cancel_token(token).await?.map(|b| crate::models::BookingCancelInfo {
            figurine_name: b.figurine_name,
            figurine_id: b.figurine_id.to_string(),
            starts_at: b.starts_at.to_string(),
            ends_at: b.ends_at.to_string(),
            status: b.status,
            admin_notes: b.admin_notes,
            curator_conditions: b.curator_conditions,
        }))
    }

    /// Batch variant — returns a map keyed by cancel token. Missing/invalid tokens are
    /// simply absent from the result (same "not found = omitted" semantics as the single GET).
    pub async fn get_bookings_by_tokens(
        &self,
        tokens: &[String],
    ) -> Result<std::collections::HashMap<String, crate::models::BookingCancelInfo>> {
        let bookings = self.repo.get_bookings_by_cancel_tokens(tokens).await?;
        Ok(bookings.into_iter().map(|b| {
            (b.cancel_token.clone(), crate::models::BookingCancelInfo {
                figurine_name: b.figurine_name,
                figurine_id: b.figurine_id.to_string(),
                starts_at: b.starts_at.to_string(),
                ends_at: b.ends_at.to_string(),
                status: b.status,
                admin_notes: b.admin_notes,
                curator_conditions: b.curator_conditions,
            })
        }).collect())
    }

    pub async fn cancel_booking_by_token(&self, token: &str) -> Result<()> {
        let booking = self.repo.cancel_booking_by_token(token).await?;
        if let Some(b) = booking {
            // If this was the only confirmed booking, revert figurine to available.
            // (token cancellation only works on 'pending' rows, so figurine status stays unchanged here.)
            let _ = b; // booking was pending — no status revert needed
        }
        // If None → booking not found or already not pending — treat as no-op (idempotent)
        Ok(())
    }

    /// Booking-rule validation (duration + advance notice), shared between the
    /// initial create and reschedule so a booking can't be created that could
    /// never be rescheduled.
    fn validate_booking_rules(rules: &BookingRules, starts_at: chrono::NaiveDate, ends_at: chrono::NaiveDate) -> Result<()> {
        let duration = (ends_at - starts_at).num_days() + 1;
        if duration < rules.min_days {
            return Err(AppError::BadRequest(format!("Minimum booking duration is {} day(s)", rules.min_days)));
        }
        if duration > rules.max_days {
            return Err(AppError::BadRequest(format!("Maximum booking duration is {} day(s)", rules.max_days)));
        }
        if rules.advance_days > 0 {
            let earliest = chrono::Utc::now().date_naive() + chrono::Duration::days(rules.advance_days);
            if starts_at < earliest {
                return Err(AppError::BadRequest(format!("Booking must start at least {} day(s) in advance", rules.advance_days)));
            }
        }
        Ok(())
    }

    pub async fn create_booking(&self, req: CreateBookingRequest, user_id: Option<Uuid>) -> Result<Booking> {
        let starts_at = chrono::NaiveDate::parse_from_str(&req.starts_at, "%Y-%m-%d")
            .map_err(|_| crate::error::AppError::BadRequest("Invalid starts_at date".to_string()))?;
        let ends_at = chrono::NaiveDate::parse_from_str(&req.ends_at, "%Y-%m-%d")
            .map_err(|_| crate::error::AppError::BadRequest("Invalid ends_at date".to_string()))?;

        if starts_at > ends_at {
            return Err(crate::error::AppError::BadRequest("starts_at must be before or equal to ends_at".to_string()));
        }

        // Apply the same booking rules as reschedule (was previously skipped on create).
        let rules = self.get_booking_rules().await?;
        Self::validate_booking_rules(&rules, starts_at, ends_at)?;

        let booking = self.repo.create_booking_atomic(&req, starts_at, ends_at, user_id).await?
            .ok_or_else(|| crate::error::AppError::Conflict(
                "These dates conflict with existing showings or confirmed bookings".to_string()
            ))?;
        let _ = self.send_booking_notification(&booking).await;
        Ok(booking)
    }

    async fn send_booking_notification(&self, booking: &Booking) -> Result<()> {
        let (Some(token), Some(chat_id)) = (
            self.config.telegram_bot_token.as_deref(),
            self.config.telegram_chat_id.as_deref(),
        ) else {
            return Ok(());
        };

        let admin_link = format!("{}/admin#bookings", self.config.public_url.trim_end_matches('/'));

        let text = format!(
            "📅 Запрос на бронирование\n\n\
            🏺 Фигурка: {}\n\
            📅 Период: {} — {}\n\
            👤 Имя: {}\n\
            📧 Email: {}\n\
            💬 Цель: {}\n\n\
            🔗 [Открыть в админке]({})",
            escape_markdown(&booking.figurine_name),
            escape_markdown(&booking.starts_at.to_string()),
            escape_markdown(&booking.ends_at.to_string()),
            escape_markdown(&booking.requester_name),
            escape_markdown(&booking.requester_email),
            escape_markdown(booking.purpose.as_deref().unwrap_or("—")),
            admin_link,
        );

        let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
        let client = Client::new();
        let _ = client.post(&url)
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "text": text,
                "parse_mode": "MarkdownV2"
            }))
            .send()
            .await;
        Ok(())
    }

    // === SHOWINGS (ADMIN) ===

    pub async fn list_showings(&self) -> Result<Vec<ShowingDto>> {
        let showings = self.repo.get_all_showings().await?;
        Ok(showings.into_iter().map(|s| ShowingDto {
            id: s.id.to_string(),
            figurine_id: s.figurine_id.to_string(),
            title: s.title,
            showing_type: s.showing_type,
            starts_at: s.starts_at.to_string(),
            ends_at: s.ends_at.to_string(),
            venue: s.venue,
            notes: s.notes,
        }).collect())
    }

    pub async fn save_showing(&self, req: SaveShowingRequest) -> Result<ShowingDto> {
        let id = self.repo.upsert_showing(&req).await?;
        Ok(ShowingDto {
            id: id.to_string(),
            figurine_id: req.figurine_id,
            title: req.title,
            showing_type: req.showing_type,
            starts_at: req.starts_at,
            ends_at: req.ends_at,
            venue: req.venue,
            notes: req.notes,
        })
    }

    pub async fn delete_showing(&self, id: String) -> Result<()> {
        let uuid = Self::parse_uuid(&id)?;
        self.repo.delete_showing(uuid).await
    }

    // === BOOKINGS (ADMIN) ===

    pub async fn list_bookings(&self, status_filter: Option<&str>, figurine_id: Option<uuid::Uuid>, page: i64, per_page: i64) -> Result<BookingsPage> {
        let offset = (page - 1) * per_page;
        let (items, total) = self.repo.get_bookings_page(status_filter, figurine_id, per_page, offset).await?;
        let pending_count = self.repo.get_pending_bookings_count().await?;
        let dtos = items.into_iter().map(|b| BookingDto {
            id: b.id.to_string(),
            figurine_id: b.figurine_id.to_string(),
            figurine_name: b.figurine_name,
            requester_name: b.requester_name,
            requester_email: b.requester_email,
            requester_phone: b.requester_phone,
            purpose: b.purpose,
            display_type: b.display_type,
            venue: b.venue,
            curator_conditions: b.curator_conditions,
            starts_at: b.starts_at.to_string(),
            ends_at: b.ends_at.to_string(),
            status: b.status,
            admin_notes: b.admin_notes,
            created_at: b.created_at.to_rfc3339(),
        }).collect();
        Ok(BookingsPage { items: dtos, total, pending_count, page, per_page })
    }

    pub async fn update_booking_status(&self, id: uuid::Uuid, status: BookingStatus, admin_notes: Option<String>, curator_conditions: Option<String>) -> Result<()> {
        let booking = self.repo.get_booking_by_id(id).await?
            .ok_or_else(|| crate::error::AppError::NotFound(format!("Booking {} not found", id)))?;

        if status == BookingStatus::Confirmed {
            // Conflict re-check + booking/figurine update happen atomically under a
            // per-figurine lock, so two confirmations can't race into a double-book.
            if let Some(reason) = self.repo.confirm_booking_atomic(
                id, booking.figurine_id, booking.starts_at, booking.ends_at,
                admin_notes.as_deref(), curator_conditions.as_deref(),
            ).await? {
                return Err(crate::error::AppError::Conflict(reason));
            }
            self.send_booking_status_message(&booking, &status, admin_notes.as_deref(), curator_conditions.as_deref()).await;
            return Ok(());
        }

        if (status == BookingStatus::Completed || status == BookingStatus::Cancelled || status == BookingStatus::Rejected)
            && booking.status == BookingStatus::Confirmed
        {
            self.repo.update_booking_status(id, &status, admin_notes.as_deref(), curator_conditions.as_deref()).await?;
            let has_others = self.repo.has_future_confirmed_bookings(booking.figurine_id, id).await?;
            if !has_others {
                self.repo.update_figurine_status(booking.figurine_id, &FigurineStatus::Available).await?;
            }
            self.send_booking_status_message(&booking, &status, admin_notes.as_deref(), curator_conditions.as_deref()).await;
            return Ok(());
        }

        self.repo.update_booking_status(id, &status, admin_notes.as_deref(), curator_conditions.as_deref()).await?;
        self.send_booking_status_message(&booking, &status, admin_notes.as_deref(), curator_conditions.as_deref()).await;
        Ok(())
    }

    async fn send_booking_status_message(&self, booking: &Booking, status: &BookingStatus, admin_notes: Option<&str>, curator_conditions: Option<&str>) {
        let Some(user_id) = booking.user_id else { return };
        let (subject, body) = match status {
            BookingStatus::Confirmed => (
                format!("Бронирование подтверждено — {}", booking.figurine_name),
                {
                    let base = format!(
                        "Ваш запрос на бронирование «{}» ({} — {}) подтверждён.",
                        booking.figurine_name,
                        booking.starts_at,
                        booking.ends_at,
                    );
                    match curator_conditions {
                        Some(c) if !c.is_empty() => format!("{}\n\nУсловия куратора: {}", base, c),
                        _ => base,
                    }
                },
            ),
            BookingStatus::Rejected => (
                format!("Бронирование отклонено — {}", booking.figurine_name),
                {
                    let base = format!(
                        "Ваш запрос на бронирование «{}» ({} — {}) отклонён.",
                        booking.figurine_name,
                        booking.starts_at,
                        booking.ends_at,
                    );
                    match admin_notes {
                        Some(n) if !n.is_empty() => format!("{}\n\nПримечание: {}", base, n),
                        _ => base,
                    }
                },
            ),
            BookingStatus::Cancelled => (
                format!("Бронирование отменено — {}", booking.figurine_name),
                format!(
                    "Бронирование «{}» ({} — {}) отменено.",
                    booking.figurine_name,
                    booking.starts_at,
                    booking.ends_at,
                ),
            ),
            BookingStatus::Completed => (
                format!("Бронирование завершено — {}", booking.figurine_name),
                format!(
                    "Бронирование «{}» ({} — {}) завершено. Спасибо!",
                    booking.figurine_name,
                    booking.starts_at,
                    booking.ends_at,
                ),
            ),
            _ => return,
        };
        let _ = self.repo.create_thread(user_id, "booking", Some(booking.id), &subject, &body, true, &[]).await;
    }

    fn clean_media_path(&self, path: &str) -> String {
        let base = self.config.public_url.trim_end_matches('/');
        path.strip_prefix(base)
            .unwrap_or(path)
            .trim_start_matches("/static/")
            .trim_start_matches('/')
            .replace('\\', "/")
    }

    fn public_media_url(&self, path: &str) -> String {
        let base = self.config.public_url.trim_end_matches('/');
        format!("{}/static/{}", base, path.trim_start_matches('/'))
    }

    fn is_managed_media_path(path: &str) -> bool {
        path.starts_with("images/") || path.starts_with("videos/") || path.starts_with("audio/") || path.starts_with("backgrounds/")
    }

    fn media_type_for_path(path: &str) -> String {
        if path.starts_with("images/") || path.starts_with("backgrounds/") {
            "image".to_string()
        } else if path.starts_with("videos/") {
            "video".to_string()
        } else if path.starts_with("audio/") {
            "audio".to_string()
        } else {
            "other".to_string()
        }
    }

    fn variant_for_path(path: &str) -> Option<String> {
        if path.starts_with("images/original/") {
            Some("original".to_string())
        } else if path.starts_with("images/preview/") {
            Some("preview".to_string())
        } else if path.starts_with("images/thumb/") {
            Some("thumb".to_string())
        } else {
            None
        }
    }

    fn collect_upload_files(&self) -> Result<Vec<(String, u64)>> {
        let mut files = Vec::new();
        for folder in ["images", "videos", "audio", "backgrounds"] {
            let dir = Path::new(&self.config.upload_dir).join(folder);
            if dir.exists() {
                Self::collect_files_recursive(Path::new(&self.config.upload_dir), &dir, &mut files)?;
            }
        }
        files.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(files)
    }

    fn collect_files_recursive(base: &Path, dir: &Path, files: &mut Vec<(String, u64)>) -> Result<()> {
        for entry in fs::read_dir(dir).map_err(crate::error::AppError::Io)? {
            let entry = entry.map_err(crate::error::AppError::Io)?;
            let path = entry.path();
            if path.is_dir() {
                Self::collect_files_recursive(base, &path, files)?;
                continue;
            }
            if !path.is_file() {
                continue;
            }
            let rel = path
                .strip_prefix(base)
                .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
                .to_string_lossy()
                .replace('\\', "/");
            let size = fs::metadata(&path).map_err(crate::error::AppError::Io)?.len();
            files.push((rel, size));
        }
        Ok(())
    }

    pub async fn media_inventory(&self) -> Result<MediaInventoryDto> {
        let mut usage_map: HashMap<String, Vec<MediaUsageDto>> = HashMap::new();
        for mut usage in self.repo.get_media_usages().await? {
            let cleaned = self.clean_media_path(&usage.path);
            if !Self::is_managed_media_path(&cleaned) {
                continue;
            }
            usage.path = cleaned;
            usage_map.entry(usage.path.clone()).or_default().push(usage);
        }

        let files_on_disk = self.collect_upload_files()?;
        let file_size_map: HashMap<String, u64> = files_on_disk.into_iter().collect();
        let mut known_paths: HashSet<String> = usage_map.keys().cloned().collect();
        known_paths.extend(file_size_map.keys().cloned());

        let mut files = known_paths.into_iter().map(|path| {
            let size_bytes = file_size_map.get(&path).copied().unwrap_or(0);
            let exists = file_size_map.contains_key(&path);
            let usages = usage_map.remove(&path).unwrap_or_default();
            MediaFileDto {
                url: self.public_media_url(&path),
                media_type: Self::media_type_for_path(&path),
                variant: Self::variant_for_path(&path),
                size_bytes,
                exists,
                path,
                usages,
            }
        }).collect::<Vec<_>>();

        files.sort_by(|a, b| {
            b.usages.len()
                .cmp(&a.usages.len())
                .then_with(|| a.path.cmp(&b.path))
        });

        let orphan_count = files.iter().filter(|file| file.usages.is_empty()).count();
        let used_count = files.len().saturating_sub(orphan_count);
        let total_size_bytes = files.iter().map(|file| file.size_bytes).sum();
        Ok(MediaInventoryDto { files, orphan_count, used_count, total_size_bytes })
    }

    pub async fn unused_media_report(&self) -> Result<MediaCleanupReportDto> {
        let inventory = self.media_inventory().await?;
        let files = inventory.files
            .into_iter()
            .filter(|file| file.exists && file.usages.is_empty())
            .collect::<Vec<_>>();
        let total_size_bytes = files.iter().map(|file| file.size_bytes).sum();
        Ok(MediaCleanupReportDto { files, total_size_bytes })
    }

    pub async fn cleanup_unused_media(&self) -> Result<Vec<String>> {
        let report = self.unused_media_report().await?;
        let mut removed = Vec::new();
        for file in report.files {
            let path = Path::new(&self.config.upload_dir).join(&file.path);
            if path.exists() && path.is_file() {
                fs::remove_file(&path).map_err(crate::error::AppError::Io)?;
                removed.push(file.path);
            }
        }
        Ok(removed)
    }

    pub async fn replace_media_everywhere(
        &self,
        old_path: &str,
        new_preview_path: &str,
        new_original_path: Option<&str>,
        new_thumb_path: Option<&str>,
    ) -> Result<MediaReplaceResultDto> {
        let old_path = self.clean_media_path(old_path);
        let base = self.config.public_url.trim_end_matches('/');
        let old_aliases = [
            old_path.clone(),
            format!("/static/{}", old_path),
            format!("{}/static/{}", base, old_path),
        ];
        let mut updated_references = 0usize;
        for alias in old_aliases {
            updated_references += self.repo.replace_media_path_everywhere(
                &alias,
                new_preview_path,
                new_original_path,
                new_thumb_path,
            ).await?;
        }
        let mut imported_paths = vec![new_preview_path.to_string()];
        if let Some(path) = new_original_path {
            imported_paths.push(path.to_string());
        }
        if let Some(path) = new_thumb_path {
            imported_paths.push(path.to_string());
        }
        Ok(MediaReplaceResultDto {
            old_path,
            new_path: new_preview_path.to_string(),
            updated_references,
            imported_paths,
        })
    }
}

// ── Input validation helpers ────────────────────────────────────────────────

const MAX_ATTACHMENTS: usize = 10;
const MAX_URL_LEN: usize = 2048;

/// Validate uploaded-reference URLs: bounded count, bounded length, and only
/// our own `/static/` paths or absolute http(s) URLs (no `javascript:` / `data:`
/// / `file:` smuggling).
fn validate_attachments(atts: &[AttachmentInput]) -> Result<()> {
    if atts.len() > MAX_ATTACHMENTS {
        return Err(AppError::BadRequest(format!("Too many attachments (max {MAX_ATTACHMENTS})")));
    }
    let ok_url = |u: &str| {
        u.len() <= MAX_URL_LEN
            && (u.starts_with("/static/") || u.starts_with("http://") || u.starts_with("https://"))
    };
    for a in atts {
        if !ok_url(&a.url) {
            return Err(AppError::BadRequest("Invalid attachment URL".into()));
        }
        if let Some(t) = &a.thumb_url
            && !ok_url(t) {
                return Err(AppError::BadRequest("Invalid attachment thumbnail URL".into()));
            }
    }
    Ok(())
}

/// Require a trimmed string within `[1, max]` characters.
fn validate_text(field: &str, value: &str, max: usize) -> Result<()> {
    let len = value.trim().chars().count();
    if len == 0 {
        return Err(AppError::BadRequest(format!("{field} is required")));
    }
    if len > max {
        return Err(AppError::BadRequest(format!("{field} is too long (max {max} characters)")));
    }
    Ok(())
}

/// Decode a JSON-stored setting. Missing → default, but a *corrupt* value is a
/// hard error instead of being silently reset to defaults (which would quietly
/// wipe SMTP creds / theme / booking rules).
fn parse_json_setting<T>(key: &str, json: Option<String>) -> Result<T>
where
    T: serde::de::DeserializeOwned + Default,
{
    match json {
        Some(j) => serde_json::from_str(&j)
            .map_err(|e| AppError::Internal(format!("Corrupt setting '{key}': {e}"))),
        None => Ok(T::default()),
    }
}

fn escape_markdown(s: &str) -> String {
    // Telegram MarkdownV2 special chars
    s.chars().fold(String::new(), |mut acc, c| {
        if matches!(c, '_'|'*'|'['|']'|'('|')'|'~'|'`'|'>'|'#'|'+'|'-'|'='|'|'|'{'|'}'|'.'|'!') {
            acc.push('\\');
        }
        acc.push(c);
        acc
    })
}

// ============================================================
// AUTH CONSTANTS
// ============================================================

const CATEGORIES: [&str; 4] = ["animals", "dishes", "seasons", "symbols"];

/// How many icons each user is shown per category (their personal subset).
const POOL_PER_CATEGORY: usize = 8;

/// Master icon pool. Each user is shown a random subset of POOL_PER_CATEGORY of
/// these per category (chosen at registration, persisted, replayed at login).
/// IDs must match `src/lib/data/visualIcons.ts` exactly on the frontend.
const ICONS: &[(&str, &[&str])] = &[
    ("animals", &[
        "wolf", "raven", "fox", "owl", "snake", "deer", "bat", "cat",
        "bear", "hare", "boar", "lynx", "crow", "moth", "spider", "frog",
        "hound", "horse", "goat", "ram", "hawk", "mouse", "beetle", "stag_beetle",
    ]),
    ("dishes", &[
        "mushroom", "apple", "bread", "cup", "fish", "berry", "honey", "herb",
        "pear", "plum", "egg", "cheese", "grapes", "carrot", "onion", "pumpkin",
        "walnut", "pie", "soup", "wine", "milk", "salt", "pepper", "garlic",
    ]),
    ("seasons", &[
        "snowflake", "bare_tree", "sprout", "rain", "sun", "wheat", "leaf", "acorn",
        "icicle", "frost_pane", "bud", "blossom", "cloud", "lightning", "mist", "pinecone",
        "fern", "sheaf", "crescent", "dewdrop", "hail", "gust", "ember", "catkin",
    ]),
    ("symbols", &[
        "key", "candle", "hourglass", "skull", "moon", "star", "cross", "anchor",
        "bell", "clock", "feather", "inkpot", "scroll", "dagger", "crown", "eye",
        "lantern", "mask", "book", "chalice", "compass", "keyhole", "ring", "coin",
    ]),
];

fn valid_icon_ids(category: &str) -> Option<&'static [&'static str]> {
    ICONS.iter().find(|(c, _)| *c == category).map(|(_, ids)| *ids)
}

/// Deterministically derive a per-email decoy subset for emails that have no
/// account, so the login grid is stable and statistically indistinguishable
/// from a real stored pool (prevents account enumeration via the icon set).
/// Keyed by the admin secret so the subset can't be recomputed by an attacker.
fn decoy_pool(secret: &str, email: &str) -> Vec<Vec<String>> {
    use rand::seq::SliceRandom;
    use rand::SeedableRng;
    use sha2::{Digest, Sha256};

    CATEGORIES.iter().map(|category| {
        let mut hasher = Sha256::new();
        hasher.update(secret.as_bytes());
        hasher.update(b"\x00");
        hasher.update(email.as_bytes());
        hasher.update(b"\x00");
        hasher.update(category.as_bytes());
        let seed: [u8; 32] = hasher.finalize().into();
        let mut rng = rand::rngs::StdRng::from_seed(seed);

        let mut ids: Vec<&'static str> = valid_icon_ids(category).unwrap_or(&[]).to_vec();
        ids.shuffle(&mut rng);
        ids.into_iter().take(POOL_PER_CATEGORY).map(str::to_string).collect()
    }).collect()
}

/// Validate a client-proposed personal pool against the master pool and the
/// chosen selections, returning the `{category: [icon_id]}` JSON to persist.
/// Each category must contain exactly POOL_PER_CATEGORY distinct valid icon_ids,
/// and the selection for that category must be one of them.
fn validate_pool(pool: &[Vec<String>; 4], selections: &[String; 4]) -> Result<serde_json::Value> {
    let mut pool_obj = serde_json::Map::new();
    for (i, category) in CATEGORIES.iter().enumerate() {
        let master = valid_icon_ids(category)
            .ok_or_else(|| AppError::Internal("Unknown category".into()))?;
        let entries = &pool[i];

        if entries.len() != POOL_PER_CATEGORY {
            return Err(AppError::BadRequest(format!("Invalid pool size for {category}")));
        }
        let mut seen = std::collections::HashSet::new();
        for id in entries {
            if !master.contains(&id.as_str()) {
                return Err(AppError::BadRequest(format!("Invalid pool icon for {category}")));
            }
            if !seen.insert(id.as_str()) {
                return Err(AppError::BadRequest(format!("Duplicate pool icon for {category}")));
            }
        }
        if !entries.contains(&selections[i]) {
            return Err(AppError::BadRequest(format!("Selection not in pool for {category}")));
        }
        pool_obj.insert(category.to_string(), serde_json::json!(entries));
    }
    Ok(serde_json::Value::Object(pool_obj))
}

/// Parse a stored `visual_pool` JSON value ({category: [icon_id]}) into the
/// fixed category order. Returns None if the shape is unusable.
fn parse_stored_pool(value: &serde_json::Value) -> Option<Vec<Vec<String>>> {
    let obj = value.as_object()?;
    let mut out = Vec::with_capacity(CATEGORIES.len());
    for category in CATEGORIES {
        let arr = obj.get(category)?.as_array()?;
        let ids: Vec<String> = arr.iter().filter_map(|v| v.as_str().map(str::to_string)).collect();
        if ids.is_empty() { return None; }
        out.push(ids);
    }
    Some(out)
}

fn build_hash_input(selections: &[String; 4]) -> String {
    selections.iter().enumerate()
        .map(|(i, id)| format!("{}:{}", CATEGORIES[i], id))
        .collect::<Vec<_>>()
        .join("|")
}

fn hash_password(input: &str) -> std::result::Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(input.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

fn verify_password(input: &str, hash: &str) -> bool {
    let parsed = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default().verify_password(input.as_bytes(), &parsed).is_ok()
}

// ============================================================
// AppService — AUTH METHODS
// ============================================================

impl AppService {
    pub async fn register_user(&self, req: &RegisterRequest, ip: Option<String>, user_agent: Option<String>) -> Result<UserDto> {
        if !req.email.contains('@') {
            return Err(AppError::BadRequest("Invalid email".into()));
        }
        if req.display_name.trim().is_empty() {
            return Err(AppError::BadRequest("Display name required".into()));
        }

        // Validate the personal pool (alphabet shown at login) and that each
        // selection is drawn from it.
        let pool_json = validate_pool(&req.pool, &req.selections)?;

        let hash_input = build_hash_input(&req.selections);
        let hash = hash_password(&hash_input)
            .map_err(|e| AppError::Internal(format!("Hash error: {e}")))?;

        let ctx = self.client_context(ip, user_agent);
        let user = self.repo.create_user(&req.email.to_lowercase(), &req.display_name, &hash, &pool_json, &ctx).await?;
        Ok(UserDto::from(&user))
    }

    pub async fn login_challenge(&self, email: &str) -> Result<LoginChallengeResponse> {
        use rand::seq::SliceRandom;
        let email_lower = email.to_lowercase();

        // A challenge is always issued — for unknown, valid, AND blocked emails —
        // so the response never reveals whether an account exists. The actual
        // user/password/blocked checks happen in login_verify, which returns the
        // same generic Unauthorized for every failure. (Avoids account enumeration.)

        // Check lockout before issuing challenge
        let failures = self.repo.count_recent_failures(&email_lower, 15).await?;
        if failures >= 5 {
            return Err(AppError::BadRequest("Too many failed attempts. Try again in 15 minutes.".into()));
        }

        // Determine the icon set to show. For a real account it's the personal
        // pool stored at registration; for an unknown email (or a legacy account
        // with no pool) it's a deterministic decoy keyed by the admin secret, so
        // the grid is stable per email and reveals nothing about account
        // existence. (.await before building tokens — ThreadRng is !Send.)
        let user = self.repo.find_user_by_email(&email_lower).await?;
        let pool: Vec<Vec<String>> = user
            .as_ref()
            .and_then(|u| u.visual_pool.as_ref())
            .and_then(parse_stored_pool)
            .unwrap_or_else(|| decoy_pool(&self.config.admin_api_key, &email_lower));

        // Build tokens synchronously in a block so ThreadRng (!Send) is dropped before any .await
        let (all_tokens, steps) = {
            let mut rng = rand::thread_rng();
            let mut all_tokens: Vec<ChallengeToken> = Vec::new();
            let mut steps: Vec<ChallengeStepDto> = Vec::new();

            for (i, category) in CATEGORIES.iter().enumerate() {
                let mut icons_shuffled: Vec<&String> = pool[i].iter().collect();
                icons_shuffled.shuffle(&mut rng);

                let mut step_icons: Vec<ChallengeIconDto> = Vec::new();
                for icon_id in icons_shuffled {
                    let token = Uuid::new_v4().to_string();
                    all_tokens.push(ChallengeToken {
                        token: token.clone(),
                        category: category.to_string(),
                        icon_id: icon_id.clone(),
                    });
                    step_icons.push(ChallengeIconDto {
                        token,
                        icon_id: icon_id.clone(),
                    });
                }
                steps.push(ChallengeStepDto {
                    category: category.to_string(),
                    icons: step_icons,
                });
            }
            (all_tokens, steps)
        }; // rng dropped here, before any .await

        let tokens_json = serde_json::to_value(&all_tokens)
            .map_err(|e| AppError::Internal(format!("Serialize error: {e}")))?;

        let challenge_id = self.repo.save_challenge(&email_lower, &tokens_json).await?;

        Ok(LoginChallengeResponse {
            challenge_id: challenge_id.to_string(),
            steps,
        })
    }

    pub async fn login_verify(&self, req: &LoginVerifyRequest, ip: Option<String>, user_agent: Option<String>) -> Result<LoginVerifyResponse> {
        // Resolve geo once up front; the same context is stored on every attempt
        // (success or failure) and on the created session.
        let ctx = self.client_context(ip, user_agent);

        let challenge_id = Uuid::parse_str(&req.challenge_id)
            .map_err(|_| AppError::BadRequest("Invalid challenge ID".into()))?;

        let (email, tokens_json) = self.repo.get_challenge(challenge_id).await?
            .ok_or_else(|| AppError::BadRequest("Challenge expired or not found".into()))?;

        // Check lockout
        let failures = self.repo.count_recent_failures(&email, 15).await?;
        if failures >= 5 {
            return Err(AppError::BadRequest("Too many failed attempts. Try again in 15 minutes.".into()));
        }

        let token_map: Vec<ChallengeToken> = serde_json::from_value(tokens_json)
            .map_err(|e| AppError::Internal(format!("Deserialize error: {e}")))?;

        // Resolve each submitted token to icon_id, in category order
        let mut resolved_selections: [String; 4] = Default::default();
        for (i, submitted_token) in req.tokens.iter().enumerate() {
            let expected_category = CATEGORIES[i];
            let entry = token_map.iter()
                .find(|t| &t.token == submitted_token && t.category == expected_category)
                .ok_or_else(|| AppError::BadRequest("Invalid selection".into()))?;
            resolved_selections[i] = entry.icon_id.clone();
        }

        // Mark challenge as used before verifying (prevent replay regardless of outcome)
        self.repo.mark_challenge_used(challenge_id).await?;

        let user = match self.repo.find_user_by_email(&email).await? {
            Some(u) => u,
            None => {
                self.repo.record_attempt(&email, false, &ctx).await?;
                return Err(AppError::Unauthorized);
            }
        };

        let hash_input = build_hash_input(&resolved_selections);
        if !verify_password(&hash_input, &user.visual_password_hash) {
            self.repo.record_attempt(&email, false, &ctx).await?;
            return Err(AppError::Unauthorized);
        }

        // Blocked accounts fail with the same generic Unauthorized (no enumeration).
        if user.is_blocked {
            self.repo.record_attempt(&email, false, &ctx).await?;
            return Err(AppError::Unauthorized);
        }

        // record_attempt failure must not abort a successful login — use .ok()
        self.repo.record_attempt(&email, true, &ctx).await.ok();

        // Create 30-day session; prune expired sessions for this user at the same time
        let session_token = Uuid::new_v4().to_string();
        let expires_at = chrono::Utc::now() + chrono::Duration::days(30);
        self.repo.create_session(user.id, &session_token, expires_at, &ctx).await?;
        self.repo.prune_expired_sessions(user.id).await.ok();

        Ok(LoginVerifyResponse {
            session_token,
            user: UserDto::from(&user),
        })
    }

    pub async fn get_user_from_session(&self, token: &str) -> Result<User> {
        self.repo.get_session_user(token).await?
            .ok_or(AppError::Unauthorized)
    }

    pub async fn logout(&self, token: &str) -> Result<()> {
        self.repo.delete_session(token).await
    }

    pub async fn link_bookings(&self, user_id: Uuid, cancel_tokens: &[String]) -> Result<usize> {
        let user = self.repo.find_user_by_id(user_id).await?
            .ok_or(AppError::Unauthorized)?;
        self.repo.link_bookings_to_user(user_id, &user.email, cancel_tokens).await
    }

    pub async fn get_user_bookings(&self, user_id: Uuid) -> Result<Vec<UserBookingDto>> {
        let bookings = self.repo.get_user_bookings(user_id).await?;
        Ok(bookings.into_iter().map(|b| UserBookingDto {
            id: b.id.to_string(),
            figurine_id: b.figurine_id.to_string(),
            figurine_name: b.figurine_name,
            starts_at: b.starts_at.to_string(),
            ends_at: b.ends_at.to_string(),
            status: b.status,
            created_at: b.created_at.to_rfc3339(),
            cancel_token: b.cancel_token,
            display_type: b.display_type,
            venue: b.venue,
            curator_conditions: b.curator_conditions,
        }).collect())
    }

    pub async fn get_user_wishlist(&self, user_id: Uuid) -> Result<Vec<String>> {
        self.repo.get_user_wishlist(user_id).await
    }

    pub async fn set_user_wishlist(&self, user_id: Uuid, ids: Vec<String>) -> Result<Vec<String>> {
        // Dedupe and cap so the column stays bounded even if a client misbehaves.
        let mut seen = std::collections::HashSet::new();
        let cleaned: Vec<String> = ids
            .into_iter()
            .filter(|s| !s.trim().is_empty() && seen.insert(s.clone()))
            .take(500)
            .collect();
        self.repo.set_user_wishlist(user_id, &cleaned).await?;
        Ok(cleaned)
    }

    /// Attach a guest request to the account by its secret code. Tries each request
    /// type in turn (booking → waitlist → notify → commission); the first table that
    /// recognises the token decides the outcome.
    pub async fn link_claim_by_token(&self, user: &User, token: &str) -> Result<LinkClaimResponse> {
        let token = token.trim();
        if token.is_empty() {
            return Ok(LinkClaimResponse { result: "not_found".into(), kind: None, name: None });
        }
        if let Some(m) = self.repo.link_booking_by_token(user.id, &user.email, token).await? {
            return Ok(Self::claim_response("booking", m));
        }
        if let Some(m) = self.repo.link_waitlist_by_token(user.id, &user.email, token).await? {
            return Ok(Self::claim_response("waitlist", m));
        }
        if let Some(m) = self.repo.link_notify_order_by_token(user.id, &user.email, token).await? {
            return Ok(Self::claim_response("notify", m));
        }
        if self.repo.commission_claimable_by(token, user.id).await? {
            // Token-only (no email guard). Reuse claim_commission so the conversation
            // thread is seeded exactly as it is for the in-app claim flow.
            let dto = self.claim_commission(token, user.id).await?;
            let name = if dto.title.trim().is_empty() { dto.requester_name.clone() } else { dto.title.clone() };
            return Ok(LinkClaimResponse { result: "linked".into(), kind: Some("commission".into()), name: Some(name) });
        }
        Ok(LinkClaimResponse { result: "not_found".into(), kind: None, name: None })
    }

    fn claim_response(kind: &str, m: crate::db::ClaimMatch) -> LinkClaimResponse {
        let result = if m.linked {
            "linked"
        } else if !m.email_ok {
            "email_mismatch"
        } else {
            "already_linked"
        };
        LinkClaimResponse { result: result.to_string(), kind: Some(kind.to_string()), name: Some(m.name) }
    }

    pub async fn get_user_waitlist(&self, user_id: Uuid) -> Result<Vec<WaitlistEntryDto>> {
        let entries = self.repo.get_user_waitlist(user_id).await?;
        let mut dtos = Vec::with_capacity(entries.len());
        for e in entries {
            let position = self.repo.waitlist_position(e.figurine_id, e.created_at).await?;
            dtos.push(WaitlistEntryDto {
                id: e.id.to_string(),
                figurine_id: e.figurine_id.to_string(),
                figurine_name: e.figurine_name,
                requester_name: e.requester_name,
                requester_email: e.requester_email,
                requester_phone: e.requester_phone,
                note: e.note,
                created_at: e.created_at.to_rfc3339(),
                user_id: e.user_id.map(|u| u.to_string()),
                position,
            });
        }
        Ok(dtos)
    }

    pub async fn get_user_orders(&self, user_id: Uuid) -> Result<Vec<UserOrderDto>> {
        let orders = self.repo.get_user_orders(user_id).await?;
        Ok(orders.into_iter().map(|o| UserOrderDto {
            id: o.id.to_string(),
            figurine_id: o.figurine_id,
            figurine_name: o.figurine_name,
            mode: o.mode,
            status: o.status,
            created_at: o.created_at.to_rfc3339(),
            admin_notes: o.admin_notes,
            reserve_status: o.reserve_status,
            reserve_expires_at: o.reserve_expires_at.map(|d| d.to_string()),
            admin_terms_note: o.admin_terms_note,
            invoice_note: o.invoice_note,
        }).collect())
    }

    // === ADMIN USER MANAGEMENT ===

    pub async fn admin_list_users(&self, search: Option<&str>, page: i64, per_page: i64) -> Result<(Vec<AdminUserListItem>, i64)> {
        let offset = (page - 1) * per_page;
        self.repo.admin_list_users(search, per_page, offset).await
    }

    pub async fn admin_get_user_detail(&self, user_id: Uuid) -> Result<AdminUserDetail> {
        let user = self.repo.find_user_by_id(user_id).await?
            .ok_or_else(|| AppError::NotFound(format!("User {} not found", user_id)))?;

        let bookings = self.get_user_bookings(user_id).await?;
        let orders = self.get_user_orders(user_id).await?;
        let sessions = self.repo.admin_get_user_sessions(user_id).await?;
        let recent_failures = self.repo.count_recent_failures(&user.email, 24 * 60).await?;
        let messages = self.admin_get_user_threads(user_id).await?;

        Ok(AdminUserDetail {
            id: user.id.to_string(),
            email: user.email,
            display_name: user.display_name,
            admin_notes: user.admin_notes,
            created_at: user.created_at.to_rfc3339(),
            signup_ip: user.signup_ip,
            signup_country_code: user.signup_country_code,
            signup_city: user.signup_city,
            last_reset_ip: user.last_reset_ip,
            last_reset_country_code: user.last_reset_country_code,
            last_reset_city: user.last_reset_city,
            last_reset_at: user.last_reset_at.map(|t| t.to_rfc3339()),
            last_reset_request_ip: user.last_reset_request_ip,
            last_reset_request_country_code: user.last_reset_request_country_code,
            last_reset_request_city: user.last_reset_request_city,
            last_reset_request_at: user.last_reset_request_at.map(|t| t.to_rfc3339()),
            bookings,
            orders,
            sessions,
            recent_failures,
            messages,
        })
    }

    pub async fn admin_revoke_user_sessions(&self, user_id: Uuid) -> Result<u64> {
        self.repo.admin_revoke_all_sessions(user_id).await
    }

    pub async fn admin_update_user_notes(&self, user_id: Uuid, notes: Option<&str>) -> Result<()> {
        self.repo.admin_update_user_notes(user_id, notes).await
    }

    pub async fn admin_set_user_blocked(&self, user_id: Uuid, blocked: bool) -> Result<()> {
        // Revoke all active sessions when blocking so the user is immediately logged out
        if blocked {
            self.repo.admin_revoke_all_sessions(user_id).await?;
        }
        self.repo.admin_set_user_blocked(user_id, blocked).await
    }

    pub async fn admin_generate_reset_token(&self, user_id: Uuid) -> Result<ResetTokenResponse> {
        // Verify user exists
        self.repo.find_user_by_id(user_id).await?
            .ok_or_else(|| AppError::NotFound(format!("User {} not found", user_id)))?;

        let token = Uuid::new_v4().to_string();
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(48);
        self.repo.admin_create_reset_token(user_id, &token, expires_at).await?;
        Ok(ResetTokenResponse { token, expires_at: expires_at.to_rfc3339() })
    }

    pub async fn validate_reset_token(&self, token: &str) -> Result<UserDto> {
        let user = self.repo.find_user_by_reset_token(token).await?
            .ok_or_else(|| AppError::BadRequest("Reset link is invalid or has expired.".into()))?;
        Ok(UserDto::from(&user))
    }

    pub async fn apply_password_reset(&self, req: &ApplyPasswordResetRequest, ip: Option<String>, user_agent: Option<String>) -> Result<()> {
        let user = self.repo.find_user_by_reset_token(&req.token).await?
            .ok_or_else(|| AppError::BadRequest("Reset link is invalid or has expired.".into()))?;

        // Reset regenerates the personal pool, so validate it the same way as
        // registration and persist it alongside the new hash.
        let pool_json = validate_pool(&req.pool, &req.selections)?;

        let hash_input = build_hash_input(&req.selections);
        let new_hash = hash_password(&hash_input)
            .map_err(|e| AppError::Internal(format!("Hash error: {e}")))?;

        // Record where the reset was applied from (audit), then persist.
        let ctx = self.client_context(ip, user_agent);

        // Invalidate all existing sessions so old password can't be used
        self.repo.admin_revoke_all_sessions(user.id).await?;
        self.repo.apply_password_reset(user.id, &new_hash, &pool_json, &ctx).await
    }

    /// Self-service "forgot password": issue a reset token and email the link to
    /// the account owner. Always succeeds from the caller's view — whether or not
    /// the email maps to an account is never revealed (anti-enumeration), and the
    /// email is sent out of band so response latency doesn't leak existence.
    pub async fn request_password_reset(&self, email: &str, ip: Option<String>, user_agent: Option<String>) -> Result<()> {
        let email_lower = email.trim().to_lowercase();

        // Unknown or blocked accounts: silently do nothing, same response.
        let Some(user) = self.repo.find_user_by_email(&email_lower).await? else {
            return Ok(());
        };
        if user.is_blocked {
            return Ok(());
        }

        let token = Uuid::new_v4().to_string();
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(48);
        let ctx = self.client_context(ip, user_agent);
        self.repo.create_self_reset_token(user.id, &token, expires_at, &ctx).await?;

        // Fire-and-forget the email so the slow SMTP path can't be timed to infer
        // whether the account exists.
        let svc = self.clone();
        let to = user.email.clone();
        tokio::spawn(async move {
            if let Err(e) = svc.send_password_reset_email(&to, &token).await {
                tracing::warn!("Password reset email failed: {e}");
            }
        });

        Ok(())
    }

    async fn send_password_reset_email(&self, to: &str, token: &str) -> Result<()> {
        use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
        use lettre::transport::smtp::authentication::Credentials;
        use lettre::message::header::ContentType;

        // DB settings take precedence over env config (same resolution as replies).
        let db = self.get_smtp_settings().await.unwrap_or_default();
        let host = db.host.as_deref().or(self.config.smtp_host.as_deref());
        let user = db.user.as_deref().or(self.config.smtp_user.as_deref());
        let pass = db.pass.as_deref().or(self.config.smtp_pass.as_deref());
        let from = db.from.as_deref().or(self.config.smtp_from.as_deref());
        let port = db.port.or(self.config.smtp_port).unwrap_or(587);

        let (Some(host), Some(user), Some(pass), Some(from)) = (host, user, pass, from) else {
            tracing::warn!("SMTP not configured — password reset link not sent to {to}");
            return Ok(());
        };

        let link = format!(
            "{}/set-password?token={}",
            self.config.public_url.trim_end_matches('/'),
            token
        );
        let body_text = format!(
            "Someone asked to restore the way into the archive for this address.\n\n\
            If it was you, follow this passage within 48 hours:\n\n\
            {link}\n\n\
            You will be asked to choose your signs anew.\n\n\
            If it was not you, no harm is done — ignore this letter and nothing changes.",
        );

        let email = Message::builder()
            .from(from.parse().map_err(|_| AppError::Internal("Invalid SMTP from address".into()))?)
            .to(to.parse().map_err(|_| AppError::Internal("Invalid recipient address".into()))?)
            .subject("A key to the archive")
            .header(ContentType::TEXT_PLAIN)
            .body(body_text)
            .map_err(|e| AppError::Internal(format!("Email build error: {e}")))?;

        let creds = Credentials::new(user.to_string(), pass.to_string());
        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(host)
            .map_err(|e| AppError::Internal(format!("SMTP relay error: {e}")))?
            .port(port)
            .credentials(creds)
            .build();

        mailer.send(email).await
            .map_err(|e| AppError::Internal(format!("SMTP send error: {e}")))?;
        Ok(())
    }

    // === COMMENTS ===

    async fn check_comment_rate_limit(&self, ip: &str) -> Result<()> {
        const MAX_PER_HOUR: usize = 5;
        let now = Instant::now();
        let cutoff_secs = Duration::from_secs(3600);
        let mut map = self.comment_rate_limiter.lock().await;
        let entry = map.entry(ip.to_string()).or_default();
        entry.retain(|t: &Instant| now.duration_since(*t) < cutoff_secs);
        if entry.len() >= MAX_PER_HOUR {
            return Err(AppError::BadRequest(
                "Too many comments from this address. Please wait before submitting again.".into()
            ));
        }
        entry.push(now);
        Ok(())
    }

    pub async fn get_smtp_settings(&self) -> Result<SmtpSettings> {
        parse_json_setting("smtp", self.repo.get_setting("smtp").await?)
    }

    pub async fn save_smtp_settings(&self, s: SmtpSettings) -> Result<()> {
        let json = serde_json::to_string(&s)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        self.repo.upsert_setting("smtp", &json).await
    }

    pub async fn get_contact_settings(&self) -> Result<ContactSettings> {
        parse_json_setting("contact", self.repo.get_setting("contact").await?)
    }

    pub async fn save_contact_settings(&self, s: ContactSettings) -> Result<()> {
        let json = serde_json::to_string(&s)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        self.repo.upsert_setting("contact", &json).await
    }

    // === WORKSHOP FEATURE (home page) ===

    pub async fn get_workshop_feature(&self) -> Result<WorkshopFeature> {
        parse_json_setting("workshop_feature", self.repo.get_setting("workshop_feature").await?)
    }

    pub async fn save_workshop_feature(&self, feature: WorkshopFeature) -> Result<()> {
        let json = serde_json::to_string(&feature)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        if json.len() > 64 * 1024 {
            return Err(AppError::BadRequest("Workshop feature is too large".into()));
        }
        self.repo.upsert_setting("workshop_feature", &json).await
    }

    // === BOOKING RULES ===

    pub async fn get_booking_rules(&self) -> Result<BookingRules> {
        parse_json_setting("booking_rules", self.repo.get_setting("booking_rules").await?)
    }

    pub async fn save_booking_rules(&self, rules: BookingRules) -> Result<()> {
        let json = serde_json::to_string(&rules)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        self.repo.upsert_setting("booking_rules", &json).await
    }

    // === THEME CONFIG ===

    pub async fn get_theme_config(&self) -> Result<ThemeConfig> {
        parse_json_setting("theme_config", self.repo.get_setting("theme_config").await?)
    }

    pub async fn save_theme_config(&self, config: ThemeConfig) -> Result<()> {
        let json = serde_json::to_string(&config)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        if json.len() > 100 * 1024 {
            return Err(AppError::BadRequest("Theme config is too large".into()));
        }
        self.repo.upsert_setting("theme_config", &json).await
    }

    // === COPY OVERRIDES ===

    pub async fn get_copy_overrides(&self) -> Result<CopyOverrides> {
        parse_json_setting("copy_overrides", self.repo.get_setting("copy_overrides").await?)
    }

    pub async fn save_copy_overrides(&self, overrides: CopyOverrides) -> Result<()> {
        let json = serde_json::to_string(&overrides)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        if json.len() > 500 * 1024 {
            return Err(AppError::BadRequest("Copy overrides are too large".into()));
        }
        self.repo.upsert_setting("copy_overrides", &json).await
    }

    // === RESCHEDULE ===

    pub async fn reschedule_booking_by_token(&self, token: &str, req: RescheduleBookingRequest) -> Result<BookingCancelInfo> {
        let rules = self.get_booking_rules().await?;

        let starts_at = chrono::NaiveDate::parse_from_str(&req.starts_at, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest("Invalid starts_at date".to_string()))?;
        let ends_at = chrono::NaiveDate::parse_from_str(&req.ends_at, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest("Invalid ends_at date".to_string()))?;

        if starts_at > ends_at {
            return Err(AppError::BadRequest("starts_at must be ≤ ends_at".to_string()));
        }

        Self::validate_booking_rules(&rules, starts_at, ends_at)?;

        // Fetch the current booking to get figurine_id for conflict check
        let current = self.repo.get_booking_by_cancel_token(token).await?
            .ok_or_else(|| AppError::NotFound("Booking not found".to_string()))?;

        if current.status != BookingStatus::Pending {
            return Err(AppError::BadRequest("Only pending bookings can be rescheduled".to_string()));
        }

        // Check for conflicts, excluding this booking itself
        if self.repo.check_booking_conflicts_excluding(current.figurine_id, current.id, starts_at, ends_at).await? {
            return Err(AppError::Conflict("These dates conflict with an existing showing or confirmed booking".to_string()));
        }

        let updated = self.repo.reschedule_booking_by_token(token, starts_at, ends_at).await?
            .ok_or_else(|| AppError::NotFound("Booking not found or already processed".to_string()))?;

        Ok(BookingCancelInfo {
            figurine_name: updated.figurine_name,
            figurine_id: updated.figurine_id.to_string(),
            starts_at: updated.starts_at.to_string(),
            ends_at: updated.ends_at.to_string(),
            status: updated.status,
            admin_notes: updated.admin_notes,
            curator_conditions: updated.curator_conditions,
        })
    }

    // === WAITLIST ===

    pub async fn join_waitlist(&self, figurine_id: String, req: CreateWaitlistRequest, user_id: Option<Uuid>) -> Result<crate::models::WaitlistCreatedResponse> {
        let uuid = Self::parse_uuid(&figurine_id)?;
        validate_text("Name", &req.requester_name, 100)?;
        if !req.requester_email.contains('@') || req.requester_email.len() > 200 {
            return Err(AppError::BadRequest("Valid email is required".to_string()));
        }
        if let Some(note) = &req.note
            && note.chars().count() > 1000 {
                return Err(AppError::BadRequest("Note is too long (max 1000 characters)".to_string()));
            }
        let (entry, position) = self.repo.add_to_waitlist(uuid, &req, user_id).await?;
        let _ = self.send_waitlist_notification(&entry).await;
        Ok(crate::models::WaitlistCreatedResponse { cancel_token: entry.cancel_token, position })
    }

    /// View a queue place by its cancel token (visitor's receipt lookup).
    pub async fn get_waitlist_by_token(&self, token: &str) -> Result<Option<crate::models::WaitlistCancelInfo>> {
        let Some(e) = self.repo.get_waitlist_by_cancel_token(token).await? else {
            return Ok(None);
        };
        let position = self.repo.waitlist_position(e.figurine_id, e.created_at).await?;
        Ok(Some(crate::models::WaitlistCancelInfo {
            figurine_id: e.figurine_id.to_string(),
            figurine_name: e.figurine_name,
            position,
            created_at: e.created_at.to_rfc3339(),
        }))
    }

    /// Leave the queue by token. Idempotent.
    pub async fn leave_waitlist_by_token(&self, token: &str) -> Result<()> {
        let _ = self.repo.remove_waitlist_by_token(token).await?;
        Ok(())
    }

    pub async fn admin_notify_waitlist(&self, figurine_id: String) -> Result<serde_json::Value> {
        let uuid = Self::parse_uuid(&figurine_id)?;
        let entries = self.repo.get_waitlist_for_figurine(uuid).await?;
        if entries.is_empty() {
            return Ok(serde_json::json!({ "notified": 0 }));
        }
        let figurine_name = entries[0].figurine_name.clone();
        let subject = format!("Фигурина «{}» снова доступна", figurine_name);
        let body = format!(
            "Хорошие новости — фигурина «{}», которую вы ждали, снова доступна.\n\nПосетите архив, чтобы узнать подробности.",
            figurine_name
        );
        let mut notified = 0u64;
        for entry in &entries {
            if let Some(uid) = entry.user_id {
                let _ = self.repo.create_thread(uid, "waitlist", None, &subject, &body, true, &[]).await;
                notified += 1;
            }
        }
        // Registered users get an in-app message; anonymous visitors have no
        // account, so also send the author a digest with everyone's contacts
        // to reach out personally. Runs before entries are cleared.
        let _ = self.send_availability_digest(uuid, &figurine_name).await;
        // Remove all entries for this figurine after notification
        self.repo.mark_waitlist_notified(uuid).await?;
        Ok(serde_json::json!({ "notified": notified, "total": entries.len() }))
    }

    pub async fn list_waitlist_admin(&self, figurine_id: Option<String>) -> Result<Vec<WaitlistEntryDto>> {
        let fid = match figurine_id {
            Some(s) => Some(Self::parse_uuid(&s)?),
            None => None,
        };
        let entries = self.repo.get_waitlist_admin(fid).await?;
        // Entries arrive ordered by join time, so a per-figurine running counter
        // yields each row's 1-based position in its queue.
        let mut counters: std::collections::HashMap<uuid::Uuid, i64> = std::collections::HashMap::new();
        Ok(entries.into_iter().map(|e| {
            let counter = counters.entry(e.figurine_id).or_insert(0);
            *counter += 1;
            WaitlistEntryDto {
                id: e.id.to_string(),
                figurine_id: e.figurine_id.to_string(),
                figurine_name: e.figurine_name,
                requester_name: e.requester_name,
                requester_email: e.requester_email,
                requester_phone: e.requester_phone,
                note: e.note,
                created_at: e.created_at.to_rfc3339(),
                user_id: e.user_id.map(|id| id.to_string()),
                position: *counter,
            }
        }).collect())
    }

    pub async fn remove_waitlist_entry(&self, id: uuid::Uuid) -> Result<()> {
        self.repo.remove_from_waitlist(id).await
    }

    async fn send_waitlist_notification(&self, entry: &WaitlistEntry) -> Result<()> {
        let (Some(token), Some(chat_id)) = (
            self.config.telegram_bot_token.as_deref(),
            self.config.telegram_chat_id.as_deref(),
        ) else {
            return Ok(());
        };

        let admin_link = format!("{}/admin#waitlist", self.config.public_url.trim_end_matches('/'));
        let text = format!(
            "👁 Лист ожидания\n\n\
            🏺 {}\n\
            👤 {}\n\
            📧 {}\n\
            📝 {}\n\n\
            🔗 [Открыть в админке]({})",
            escape_markdown(&entry.figurine_name),
            escape_markdown(&entry.requester_name),
            escape_markdown(&entry.requester_email),
            escape_markdown(entry.note.as_deref().unwrap_or("—")),
            admin_link,
        );

        let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
        let _ = Client::new().post(&url)
            .json(&serde_json::json!({ "chat_id": chat_id, "text": text, "parse_mode": "MarkdownV2" }))
            .send().await;
        Ok(())
    }

    /// When a work becomes reachable again, send the author one Telegram digest
    /// listing everyone waiting — queue + "notify me" requests, including
    /// anonymous visitors who have no account — so the author can reach out
    /// personally. This is the handmade/author-model alternative to an
    /// automated e-commerce mailshot.
    async fn send_availability_digest(&self, figurine_id: Uuid, figurine_name: &str) -> Result<()> {
        let (Some(token), Some(chat_id)) = (
            self.config.telegram_bot_token.as_deref(),
            self.config.telegram_chat_id.as_deref(),
        ) else {
            return Ok(());
        };

        let queue = self.repo.get_waitlist_for_figurine(figurine_id).await.unwrap_or_default();
        let notify = self.repo.get_notify_orders_for_figurine(figurine_id).await.unwrap_or_default();
        if queue.is_empty() && notify.is_empty() {
            return Ok(());
        }

        let fmt_contact = |name: &str, email: &str, phone: Option<&str>, note: Option<&str>| {
            let phone = phone.filter(|p| !p.trim().is_empty()).unwrap_or("—");
            let note = note.filter(|n| !n.trim().is_empty()).unwrap_or("—");
            format!(
                "• {} — {} — {} — {}",
                escape_markdown(name),
                escape_markdown(email),
                escape_markdown(phone),
                escape_markdown(note),
            )
        };

        let mut sections: Vec<String> = Vec::new();
        if !queue.is_empty() {
            let lines: Vec<String> = queue.iter().enumerate()
                .map(|(i, e)| format!(
                    "{}\\. {}",
                    i + 1,
                    fmt_contact(&e.requester_name, &e.requester_email, e.requester_phone.as_deref(), e.note.as_deref())
                ))
                .collect();
            sections.push(format!("🪑 *Очередь* \\({}\\)\n{}", queue.len(), lines.join("\n")));
        }
        if !notify.is_empty() {
            let lines: Vec<String> = notify.iter()
                .map(|o| fmt_contact(&o.requester_name, &o.requester_email, o.requester_phone.as_deref(), o.message.as_deref()))
                .collect();
            sections.push(format!("🔔 *Просили уведомить* \\({}\\)\n{}", notify.len(), lines.join("\n")));
        }

        let admin_link = format!("{}/admin#waitlist", self.config.public_url.trim_end_matches('/'));
        let text = format!(
            "✨ Работа «{}» снова доступна\n\nСвяжитесь лично с теми, кто ждал:\n\n{}\n\n🔗 [Открыть в админке]({})",
            escape_markdown(figurine_name),
            sections.join("\n\n"),
            admin_link,
        );

        let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
        let _ = Client::new().post(&url)
            .json(&serde_json::json!({ "chat_id": chat_id, "text": text, "parse_mode": "MarkdownV2" }))
            .send().await;
        Ok(())
    }

    async fn send_reply_email(
        &self,
        to: &str,
        figurine_name: &str,
        figurine_id: &str,
        comment_body: &str,
        reply: &str,
    ) -> Result<()> {
        use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
        use lettre::transport::smtp::authentication::Credentials;
        use lettre::message::header::ContentType;

        // DB settings take precedence over env config
        let db = self.get_smtp_settings().await.unwrap_or_default();
        let host = db.host.as_deref().or(self.config.smtp_host.as_deref());
        let user = db.user.as_deref().or(self.config.smtp_user.as_deref());
        let pass = db.pass.as_deref().or(self.config.smtp_pass.as_deref());
        let from = db.from.as_deref().or(self.config.smtp_from.as_deref());
        let port = db.port.or(self.config.smtp_port).unwrap_or(587);

        let (Some(host), Some(user), Some(pass), Some(from)) = (host, user, pass, from) else {
            return Ok(());
        };

        let figurine_url = format!(
            "{}/figurines/{}",
            self.config.public_url.trim_end_matches('/'),
            figurine_id
        );
        let body_text = format!(
            "Your impression of «{figurine_name}»:\n\n\
            {comment_body}\n\n\
            — — —\n\n\
            Author's reply:\n\n\
            {reply}\n\n\
            View the figurine: {figurine_url}",
        );

        let email = Message::builder()
            .from(from.parse().map_err(|_| AppError::Internal("Invalid SMTP from address".into()))?)
            .to(to.parse().map_err(|_| AppError::Internal("Invalid recipient address".into()))?)
            .subject(format!("Re: Your impression of «{figurine_name}»"))
            .header(ContentType::TEXT_PLAIN)
            .body(body_text)
            .map_err(|e| AppError::Internal(format!("Email build error: {e}")))?;

        let creds = Credentials::new(user.to_string(), pass.to_string());
        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(host)
            .map_err(|e| AppError::Internal(format!("SMTP relay error: {e}")))?
            .port(port)
            .credentials(creds)
            .build();

        let _ = mailer.send(email).await;
        Ok(())
    }

    async fn send_comment_telegram_notification(&self, figurine_name: &str, author_name: &str, body: &str) {
        let (Some(token), Some(chat_id)) = (
            self.config.telegram_bot_token.as_deref(),
            self.config.telegram_chat_id.as_deref(),
        ) else { return; };

        let admin_link = format!("{}/admin#comments", self.config.public_url.trim_end_matches('/'));
        let text = format!(
            "💬 Новый комментарий\n\n\
            🏺 {}\n\
            👤 {}\n\
            📝 {}\n\n\
            🔗 [Открыть в админке]({})",
            escape_markdown(figurine_name),
            escape_markdown(author_name),
            escape_markdown(&body.chars().take(200).collect::<String>()),
            admin_link,
        );
        let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
        let _ = Client::new().post(&url)
            .json(&serde_json::json!({ "chat_id": chat_id, "text": text, "parse_mode": "MarkdownV2" }))
            .send().await;
    }

    pub async fn submit_comment(
        &self,
        figurine_id: Uuid,
        user: Option<&User>,
        req: &SubmitCommentRequest,
        ip: &str,
    ) -> Result<()> {
        if user.is_none() {
            self.check_comment_rate_limit(ip).await?;
        }

        let body = req.body.trim();
        if body.is_empty() {
            return Err(AppError::BadRequest("Comment body cannot be empty".into()));
        }
        if body.chars().count() > 1000 {
            return Err(AppError::BadRequest("Comment is too long (max 1000 characters)".into()));
        }

        let (author_name, author_email, user_id) = if let Some(u) = user {
            (u.display_name.clone(), None::<String>, Some(u.id))
        } else {
            let name = req.author_name.as_deref()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| AppError::BadRequest("Author name is required for anonymous comments".into()))?;
            if name.chars().count() > 100 {
                return Err(AppError::BadRequest("Name is too long (max 100 characters)".into()));
            }
            (name, req.author_email.clone(), None)
        };

        self.repo.insert_comment(figurine_id, user_id, &author_name, author_email.as_deref(), body).await?;

        let figurine_name = self.repo.get_figurine_by_id(figurine_id).await?
            .map(|f| f.name).unwrap_or_default();
        self.send_comment_telegram_notification(&figurine_name, &author_name, body).await;

        Ok(())
    }

    pub async fn get_figurine_comments(&self, figurine_id: Uuid, newest_first: bool) -> Result<Vec<CommentDto>> {
        let rows = self.repo.get_approved_comments(figurine_id, newest_first).await?;
        Ok(rows.into_iter().map(|c| CommentDto {
            id: c.id.to_string(),
            author_name: c.author_name,
            author_avatar_url: c.avatar_url,
            body: c.body,
            admin_reply: c.admin_reply,
            created_at: c.created_at.to_rfc3339(),
        }).collect())
    }

    pub async fn admin_list_comments(
        &self,
        only_pending: bool,
        figurine_filter: Option<Uuid>,
        newest_first: bool,
        page: i64,
        per_page: i64,
    ) -> Result<AdminCommentsPage> {
        let offset = (page - 1) * per_page;
        let (rows, total) = self.repo.get_comments_admin_page(only_pending, figurine_filter, newest_first, per_page, offset).await?;
        let pending_count = self.repo.get_pending_comments_count().await?;

        let items = rows.into_iter().map(|(c, figurine_name)| AdminCommentDto {
            id: c.id.to_string(),
            figurine_id: c.figurine_id.to_string(),
            figurine_name,
            author_name: c.author_name,
            author_email: c.author_email,
            body: c.body,
            is_approved: c.is_approved,
            admin_reply: c.admin_reply,
            created_at: c.created_at.to_rfc3339(),
            user_id: c.user_id.map(|id| id.to_string()),
        }).collect();

        Ok(AdminCommentsPage { items, total, pending_count, page, per_page })
    }

    pub async fn admin_moderate_comment(
        &self,
        id: Uuid,
        is_approved: bool,
        admin_reply: Option<&str>,
    ) -> Result<AdminCommentDto> {
        let prev = self.repo.moderate_comment(id, is_approved, admin_reply).await?;
        let figurine = self.repo.get_figurine_by_id(prev.figurine_id).await?;
        let figurine_name = figurine.as_ref().map(|f| f.name.clone()).unwrap_or_default();

        // Send email to commenter if reply was just set and they have an email
        let reply_is_new = admin_reply.map(|r| !r.trim().is_empty()).unwrap_or(false);
        if reply_is_new
            && let Some(email) = prev.author_email.as_deref() {
                let fid = prev.figurine_id.to_string();
                let _ = self.send_reply_email(
                    email,
                    &figurine_name,
                    &fid,
                    &prev.body,
                    admin_reply.unwrap_or(""),
                ).await;
            }

        Ok(AdminCommentDto {
            id: prev.id.to_string(),
            figurine_id: prev.figurine_id.to_string(),
            figurine_name,
            author_name: prev.author_name,
            author_email: prev.author_email,
            body: prev.body,
            is_approved: prev.is_approved,
            admin_reply: prev.admin_reply,
            created_at: prev.created_at.to_rfc3339(),
            user_id: prev.user_id.map(|id| id.to_string()),
        })
    }

    pub async fn admin_delete_comment(&self, id: Uuid) -> Result<()> {
        self.repo.delete_comment(id).await
    }

    pub async fn update_profile(&self, user_id: Uuid, display_name: &str) -> Result<UserDto> {
        if display_name.trim().is_empty() {
            return Err(AppError::BadRequest("Display name required".into()));
        }
        let user = self.repo.update_user_display_name(user_id, display_name).await?;
        Ok(UserDto::from(&user))
    }

    pub async fn set_user_avatar(&self, user_id: Uuid, avatar_url: &str) -> Result<UserDto> {
        let user = self.repo.update_user_avatar(user_id, avatar_url).await?;
        Ok(UserDto::from(&user))
    }

    pub async fn delete_account(&self, user_id: Uuid) -> Result<()> {
        self.repo.delete_user(user_id).await
    }

    // ── Message threads ─────────────────────────────────────────

    fn thread_dto(thread: &MessageThread, unread: i64, preview: Option<String>) -> MessageThreadDto {
        MessageThreadDto {
            id: thread.id.to_string(),
            category: thread.category.clone(),
            reference_id: thread.reference_id.map(|id| id.to_string()),
            subject: thread.subject.clone(),
            status: thread.status.clone(),
            unread,
            last_message_at: thread.last_message_at.to_rfc3339(),
            created_at: thread.created_at.to_rfc3339(),
            preview: preview.map(|p| if p.chars().count() > 80 { format!("{}…", &p.chars().take(80).collect::<String>()) } else { p }),
        }
    }

    pub async fn get_user_threads(&self, user_id: Uuid) -> Result<Vec<MessageThreadDto>> {
        let rows = self.repo.get_user_threads(user_id).await?;
        Ok(rows.iter().map(|(t, unread, preview)| Self::thread_dto(t, *unread, preview.clone())).collect())
    }

    pub async fn count_unread_threads(&self, user_id: Uuid) -> Result<i64> {
        self.repo.count_unread_threads(user_id).await
    }

    /// Build message DTOs, loading per-message attachments.
    async fn messages_with_attachments(&self, messages: &[ThreadMessage]) -> Result<Vec<ThreadMessageDto>> {
        let mut out = Vec::with_capacity(messages.len());
        for m in messages {
            let atts = self.repo.get_message_attachments(m.id).await?;
            out.push(ThreadMessageDto::from_with_attachments(m, atts.iter().map(AttachmentDto::from).collect()));
        }
        Ok(out)
    }

    async fn message_dto_with_attachments(&self, msg: &ThreadMessage) -> Result<ThreadMessageDto> {
        let atts = self.repo.get_message_attachments(msg.id).await?;
        Ok(ThreadMessageDto::from_with_attachments(msg, atts.iter().map(AttachmentDto::from).collect()))
    }

    pub async fn get_thread_detail(&self, thread_id: Uuid, user_id: Uuid) -> Result<ThreadDetailDto> {
        let (thread, messages) = self.repo.get_thread_messages(thread_id, Some(user_id)).await?;
        self.repo.mark_thread_read(thread_id, user_id).await?;
        let preview = messages.last().map(|m| m.body.clone());
        Ok(ThreadDetailDto {
            thread: Self::thread_dto(&thread, 0, preview),
            messages: self.messages_with_attachments(&messages).await?,
            user: None,
        })
    }

    pub async fn user_create_thread(&self, user_id: Uuid, subject: String, body: String, category: Option<String>, attachments: Vec<AttachmentInput>) -> Result<ThreadDetailDto> {
        validate_text("Subject", &subject, 200)?;
        validate_text("Message", &body, 5000)?;
        validate_attachments(&attachments)?;
        let category = category.unwrap_or_else(|| "general".to_string());
        let (thread, msg) = self.repo.create_thread(user_id, &category, None, &subject, &body, false, &attachments).await?;
        Ok(ThreadDetailDto {
            thread: Self::thread_dto(&thread, 0, Some(msg.body.clone())),
            messages: vec![self.message_dto_with_attachments(&msg).await?],
            user: None,
        })
    }

    pub async fn user_reply_to_thread(&self, thread_id: Uuid, user_id: Uuid, body: String, attachments: Vec<AttachmentInput>) -> Result<ThreadMessageDto> {
        validate_text("Message", &body, 5000)?;
        validate_attachments(&attachments)?;
        let (thread, _) = self.repo.get_thread_messages(thread_id, Some(user_id)).await?;
        if thread.status == "resolved" {
            self.repo.reopen_thread(thread_id).await?;
        }
        let msg = self.repo.add_thread_reply(thread_id, user_id, false, &body, &attachments).await?;
        self.message_dto_with_attachments(&msg).await
    }

    pub async fn admin_create_thread(
        &self,
        user_id: Uuid,
        subject: String,
        body: String,
        category: Option<String>,
        reference_id: Option<Uuid>,
        attachments: Vec<AttachmentInput>,
    ) -> Result<ThreadDetailDto> {
        validate_attachments(&attachments)?;
        let category = category.unwrap_or_else(|| "general".to_string());
        let (thread, msg) = self.repo.create_thread(user_id, &category, reference_id, &subject, &body, true, &attachments).await?;
        let user = self.repo.find_user_by_id(user_id).await?;
        Ok(ThreadDetailDto {
            thread: Self::thread_dto(&thread, 0, Some(msg.body.clone())),
            messages: vec![self.message_dto_with_attachments(&msg).await?],
            user: user.map(|u| ThreadUserDto { id: u.id.to_string(), display_name: u.display_name, email: u.email }),
        })
    }

    pub async fn admin_reply_to_thread(&self, thread_id: Uuid, body: String, attachments: Vec<AttachmentInput>) -> Result<ThreadMessageDto> {
        validate_attachments(&attachments)?;
        let msg = self.repo.add_thread_reply(thread_id, uuid::Uuid::nil(), true, &body, &attachments).await?;
        self.message_dto_with_attachments(&msg).await
    }

    pub async fn admin_get_thread_detail(&self, thread_id: Uuid) -> Result<ThreadDetailDto> {
        let (thread, messages) = self.repo.get_thread_messages(thread_id, None).await?;
        self.repo.mark_thread_read_admin(thread_id).await?;
        let user = self.repo.find_user_by_id(thread.user_id).await?;
        let preview = messages.last().map(|m| m.body.clone());
        Ok(ThreadDetailDto {
            thread: Self::thread_dto(&thread, 0, preview),
            messages: self.messages_with_attachments(&messages).await?,
            user: user.map(|u| ThreadUserDto { id: u.id.to_string(), display_name: u.display_name, email: u.email }),
        })
    }

    pub async fn admin_list_threads(&self, category: Option<String>, status: Option<String>, page: i64, per_page: i64) -> Result<serde_json::Value> {
        let (rows, total) = self.repo.admin_get_threads(
            category.as_deref(),
            status.as_deref(),
            page,
            per_page,
        ).await?;
        let items: Vec<serde_json::Value> = rows.iter().map(|(thread, user, unread, preview)| {
            let dto = Self::thread_dto(thread, *unread, preview.clone());
            serde_json::json!({
                "thread": dto,
                "user": { "id": user.id.to_string(), "displayName": user.display_name, "email": user.email }
            })
        }).collect();
        Ok(serde_json::json!({ "items": items, "total": total, "page": page, "perPage": per_page }))
    }

    pub async fn admin_resolve_thread(&self, thread_id: Uuid) -> Result<()> {
        self.repo.resolve_thread(thread_id).await
    }

    pub async fn admin_reopen_thread(&self, thread_id: Uuid) -> Result<()> {
        self.repo.reopen_thread(thread_id).await
    }

    pub async fn admin_get_user_threads(&self, user_id: Uuid) -> Result<Vec<MessageThreadDto>> {
        let rows = self.repo.get_user_threads_for_admin(user_id).await?;
        Ok(rows.iter().map(|(t, unread, preview)| Self::thread_dto(t, *unread, preview.clone())).collect())
    }
}
