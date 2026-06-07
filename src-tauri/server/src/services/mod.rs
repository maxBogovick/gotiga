use crate::config::Config;
use crate::db::Repository;
use crate::error::Result;
use crate::models::*;
use reqwest::Client;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppService {
    repo: Repository,
    config: Config,
}

impl AppService {
    pub fn new(repo: Repository, config: Config) -> Self {
        Self { repo, config }
    }

    pub async fn initialize(&self) -> Result<()> {
        Ok(()) // Postgres is always ready, no pool to load
    }

    pub async fn list_releases(&self) -> Result<Vec<Release>> {
        self.repo.get_releases().await
    }

    pub async fn get_active_release_path(&self) -> Result<Option<String>> {
        Ok(None) // No SQLite content DB concept anymore
    }

    // === CONTENT API ===

    fn asset_url(&self, table: &str, id: &str) -> String {
        let base = self.config.public_url.trim_end_matches('/');
        format!("{}/api/v1/assets/{}/{}", base, table, id)
    }

    // Resolve a stored path/URL to a full URL for the frontend:
    // - "http..." → use as-is (external URL or legacy full URL)
    // - "/static/..." → prepend public_url (web-uploaded file, stored as relative path)
    // - anything else → serve via blob asset endpoint (Tauri-embedded BLOB path)
    fn resolve_url(&self, file_path: &str, table: &str, id: &str) -> String {
        if file_path.starts_with("http") {
            file_path.to_string()
        } else if file_path.starts_with("/static/") {
            let base = self.config.public_url.trim_end_matches('/');
            format!("{}{}", base, file_path)
        } else {
            self.asset_url(table, id)
        }
    }

    fn parse_uuid(s: &str) -> Result<Uuid> {
        Uuid::parse_str(s)
            .map_err(|_| crate::error::AppError::BadRequest(format!("Invalid ID: {}", s)))
    }

    pub async fn list_figurines(&self, visible_only: bool) -> Result<Vec<FigurineListItemDto>> {
        let figurines = self.repo.get_all_figurines(visible_only).await?;
        let mut result = Vec::new();

        for f in figurines {
            let images = self.repo.get_images_by_figurine(f.id).await?;
            let id_str = f.id.to_string();
            let face_img = images.iter()
                .find(|i| i.image_type == ImageType::Face)
                .map(|i| {
                    let i_id_str = i.id.to_string();
                    i.thumb_path
                        .as_ref()
                        .map(|p| self.resolve_url(p, "images_thumb", &i_id_str))
                        .unwrap_or_else(|| self.resolve_url(&i.file_path, "images", &i_id_str))
                });

            result.push(FigurineListItemDto {
                id: id_str,
                name: f.name,
                status: f.status,
                face_image_url: face_img,
                year: f.year,
                sort_order: f.sort_order,
                series: None,
                technique: f.technique,
                material: f.material,
                is_featured: f.is_featured,
            });
        }
        Ok(result)
    }

    pub async fn list_in_progress_figurines(&self) -> Result<Vec<FigurineListItemDto>> {
        let all = self.repo.get_all_figurines(true).await?;
        let mut result = Vec::new();
        for f in all.into_iter().filter(|f| f.status == crate::models::FigurineStatus::InProgress) {
            let images = self.repo.get_images_by_figurine(f.id).await?;
            let id_str = f.id.to_string();
            let face_img = images.iter()
                .find(|i| i.image_type == ImageType::Face)
                .map(|i| {
                    let i_id_str = i.id.to_string();
                    i.thumb_path
                        .as_ref()
                        .map(|p| self.resolve_url(p, "images_thumb", &i_id_str))
                        .unwrap_or_else(|| self.resolve_url(&i.file_path, "images", &i_id_str))
                });
            result.push(FigurineListItemDto {
                id: id_str,
                name: f.name,
                status: f.status,
                face_image_url: face_img,
                year: f.year,
                sort_order: f.sort_order,
                series: None,
                technique: f.technique,
                material: f.material,
                is_featured: f.is_featured,
            });
        }
        Ok(result)
    }

    pub async fn get_figurine_details(&self, id: String) -> Result<FigurineDto> {
        let uuid = Self::parse_uuid(&id)?;
        let figurine = self.repo.get_figurine_by_id(uuid).await?
            .ok_or_else(|| crate::error::AppError::NotFound(format!("Figurine {} not found", id)))?;

        let images = self.repo.get_images_by_figurine(uuid).await?;
        let steps = self.repo.get_steps_by_figurine(uuid).await?;
        let related_entities = self.repo.get_related_figurines(uuid).await?;

        let fig_id_str = figurine.id.to_string();

        let mut related_items = Vec::new();
        for r in related_entities {
            let r_id_str = r.id.to_string();
            let r_imgs = self.repo.get_images_by_figurine(r.id).await?;
            let face = r_imgs.iter()
                .find(|i| i.image_type == ImageType::Face)
                .map(|i| {
                    let i_id_str = i.id.to_string();
                    i.thumb_path
                        .as_ref()
                        .map(|p| self.resolve_url(p, "images_thumb", &i_id_str))
                        .unwrap_or_else(|| self.resolve_url(&i.file_path, "images", &i_id_str))
                });

            related_items.push(FigurineListItemDto {
                id: r_id_str,
                name: r.name,
                status: r.status,
                face_image_url: face,
                year: r.year,
                sort_order: r.sort_order,
                series: None,
                technique: r.technique,
                material: r.material,
                is_featured: r.is_featured,
            });
        }

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
        self.repo.upsert_figurine(&req).await?;
        self.repo.replace_images(figurine_id, &req.images).await?;
        self.repo.replace_steps(figurine_id, &req.process_steps).await?;
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
            if p.starts_with("http") {
                p
            } else if p.starts_with("/static/") {
                let base = self.config.public_url.trim_end_matches('/');
                format!("{}{}", base, p)
            } else {
                // Tauri-embedded local path — serve BLOB from app_resources.data
                let base = self.config.public_url.trim_end_matches('/');
                format!("{}/api/v1/assets/background/main_background", base)
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

    pub async fn create_order(&self, order: &OrderRequest) -> Result<Order> {
        let saved = self.repo.save_order(order).await?;
        let _ = self.send_order_notification(&saved).await;
        Ok(saved)
    }

    pub async fn list_orders(
        &self,
        status_filter: Option<&str>,
        page: i64,
        per_page: i64,
    ) -> Result<OrdersPage> {
        let offset = (page - 1) * per_page;
        let (items, total) = self.repo.get_orders_page(status_filter, per_page, offset).await?;
        let new_count = self.repo.get_new_orders_count().await?;
        Ok(OrdersPage { items, total, new_count, page, per_page })
    }

    pub async fn update_order_status(&self, id: uuid::Uuid, status: &OrderStatus) -> Result<()> {
        self.repo.update_order_status(id, status).await
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
        }))
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

    pub async fn create_booking(&self, req: CreateBookingRequest) -> Result<Booking> {
        let figurine_id = Self::parse_uuid(&req.figurine_id)?;
        let starts_at = chrono::NaiveDate::parse_from_str(&req.starts_at, "%Y-%m-%d")
            .map_err(|_| crate::error::AppError::BadRequest("Invalid starts_at date".to_string()))?;
        let ends_at = chrono::NaiveDate::parse_from_str(&req.ends_at, "%Y-%m-%d")
            .map_err(|_| crate::error::AppError::BadRequest("Invalid ends_at date".to_string()))?;

        if starts_at > ends_at {
            return Err(crate::error::AppError::BadRequest("starts_at must be before or equal to ends_at".to_string()));
        }

        if self.repo.check_booking_conflicts(figurine_id, starts_at, ends_at).await? {
            return Err(crate::error::AppError::Conflict(
                "These dates conflict with existing showings or confirmed bookings".to_string()
            ));
        }

        let booking = self.repo.save_booking(&req).await?;
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
            starts_at: b.starts_at.to_string(),
            ends_at: b.ends_at.to_string(),
            status: b.status,
            admin_notes: b.admin_notes,
            created_at: b.created_at.to_rfc3339(),
        }).collect();
        Ok(BookingsPage { items: dtos, total, pending_count, page, per_page })
    }

    pub async fn update_booking_status(&self, id: uuid::Uuid, status: BookingStatus, admin_notes: Option<String>) -> Result<()> {
        let booking = self.repo.get_booking_by_id(id).await?
            .ok_or_else(|| crate::error::AppError::NotFound(format!("Booking {} not found", id)))?;

        if status == BookingStatus::Confirmed {
            // Conflict check
            if let Some(reason) = self.repo.check_admin_confirm_conflicts(
                id, booking.figurine_id, booking.starts_at, booking.ends_at
            ).await? {
                return Err(crate::error::AppError::Conflict(reason));
            }
            // Update booking status
            self.repo.update_booking_status(id, &status, admin_notes.as_deref()).await?;
            // Auto-set figurine to Reserved
            self.repo.update_figurine_status(booking.figurine_id, &FigurineStatus::Reserved).await?;
            return Ok(());
        }

        // If cancelling/rejecting a previously-confirmed booking → maybe revert figurine to Available
        if (status == BookingStatus::Cancelled || status == BookingStatus::Rejected)
            && booking.status == BookingStatus::Confirmed
        {
            self.repo.update_booking_status(id, &status, admin_notes.as_deref()).await?;
            let has_others = self.repo.has_future_confirmed_bookings(booking.figurine_id, id).await?;
            if !has_others {
                self.repo.update_figurine_status(booking.figurine_id, &FigurineStatus::Available).await?;
            }
            return Ok(());
        }

        self.repo.update_booking_status(id, &status, admin_notes.as_deref()).await
    }

    pub async fn get_asset(&self, table: &str, id: String) -> Result<Option<Vec<u8>>> {
        let (real_table, column) = match table {
            "images" => ("images", "data"),
            "images_original" => ("images", "original_data"),
            "images_thumb" => ("images", "thumb_data"),
            "process_steps" => ("process_steps", "image_data"),
            "figurines_video" => ("figurines", "video_data"),
            "figurines_audio" => ("figurines", "ambience_data"),
            "texts" => ("texts", "image_data"),
            "background" => ("app_resources", "data"),
            _ => return Err(crate::error::AppError::BadRequest("Invalid asset type".to_string())),
        };

        self.repo.get_blob(real_table, column, id).await
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
