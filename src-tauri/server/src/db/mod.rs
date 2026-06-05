use sqlx::PgPool;
use uuid::Uuid;
use crate::error::{Result, AppError};
use crate::models::*;

#[derive(Clone)]
pub struct Repository {
    pg_pool: PgPool,
}

impl Repository {
    pub fn new(pg_pool: PgPool) -> Self {
        Self { pg_pool }
    }

    // === ORDERS (Postgres) ===

    pub async fn save_order(&self, order: &crate::models::OrderRequest) -> Result<crate::models::Order> {
        let rec = sqlx::query_as::<_, crate::models::Order>(
            "INSERT INTO orders (figurine_id, figurine_name, requester_name, requester_email, message, mode)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING *"
        )
        .bind(&order.figurine_id)
        .bind(&order.figurine_name)
        .bind(&order.requester_name)
        .bind(&order.requester_email)
        .bind(&order.message)
        .bind(&order.mode)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(rec)
    }

    pub async fn get_orders_page(
        &self,
        status_filter: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<crate::models::Order>, i64)> {
        let (items, total) = if let Some(status) = status_filter {
            let items = sqlx::query_as::<_, crate::models::Order>(
                "SELECT * FROM orders WHERE status = $1::order_status ORDER BY created_at DESC LIMIT $2 OFFSET $3"
            )
            .bind(status).bind(limit).bind(offset)
            .fetch_all(&self.pg_pool).await?;

            let (total,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM orders WHERE status = $1::order_status"
            )
            .bind(status)
            .fetch_one(&self.pg_pool).await?;

            (items, total)
        } else {
            let items = sqlx::query_as::<_, crate::models::Order>(
                "SELECT * FROM orders ORDER BY created_at DESC LIMIT $1 OFFSET $2"
            )
            .bind(limit).bind(offset)
            .fetch_all(&self.pg_pool).await?;

            let (total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM orders")
                .fetch_one(&self.pg_pool).await?;

            (items, total)
        };
        Ok((items, total))
    }

    pub async fn get_new_orders_count(&self) -> Result<i64> {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM orders WHERE status = 'new'"
        )
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(count)
    }

    pub async fn update_order_status(&self, id: uuid::Uuid, status: &crate::models::OrderStatus) -> Result<()> {
        let affected = sqlx::query(
            "UPDATE orders SET status = $1 WHERE id = $2"
        )
        .bind(status)
        .bind(id)
        .execute(&self.pg_pool)
        .await?
        .rows_affected();

        if affected == 0 {
            return Err(AppError::NotFound(format!("Order {} not found", id)));
        }
        Ok(())
    }

    // === SYSTEM (Postgres) ===

    pub async fn add_release(&self, file_path: &str, description: Option<String>) -> Result<Uuid> {
        let rec: (Uuid,) = sqlx::query_as(
            "INSERT INTO releases (file_path, description) VALUES ($1, $2) RETURNING id"
        )
        .bind(file_path)
        .bind(description)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(rec.0)
    }

    pub async fn activate_release(&self, id: Uuid) -> Result<()> {
        let mut tx = self.pg_pool.begin().await?;

        sqlx::query("UPDATE releases SET is_active = false WHERE is_active = true")
            .execute(&mut *tx).await?;

        sqlx::query("UPDATE releases SET is_active = true WHERE id = $1")
            .bind(id)
            .execute(&mut *tx).await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_active_release_path(&self) -> Result<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT file_path FROM releases WHERE is_active = true LIMIT 1"
        )
        .fetch_optional(&self.pg_pool)
        .await?;
        Ok(row.map(|r| r.0))
    }

    pub async fn get_releases(&self) -> Result<Vec<Release>> {
        let releases = sqlx::query_as::<_, Release>(
            "SELECT * FROM releases ORDER BY created_at DESC"
        )
        .fetch_all(&self.pg_pool)
        .await?;
        Ok(releases)
    }

    pub async fn get_release_by_id(&self, id: Uuid) -> Result<Option<Release>> {
        let release = sqlx::query_as::<_, Release>(
            "SELECT * FROM releases WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pg_pool)
        .await?;
        Ok(release)
    }

    // === CONTENT (Postgres) ===

    pub async fn get_all_figurines(&self, visible_only: bool) -> Result<Vec<Figurine>> {
        let figurines = if visible_only {
            sqlx::query_as::<_, Figurine>(
                "SELECT * FROM figurines WHERE is_visible = true ORDER BY sort_order"
            )
            .fetch_all(&self.pg_pool)
            .await?
        } else {
            sqlx::query_as::<_, Figurine>(
                "SELECT * FROM figurines ORDER BY sort_order"
            )
            .fetch_all(&self.pg_pool)
            .await?
        };
        Ok(figurines)
    }

    pub async fn get_figurine_by_id(&self, id: Uuid) -> Result<Option<Figurine>> {
        let figurine = sqlx::query_as::<_, Figurine>("SELECT * FROM figurines WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pg_pool)
            .await?;
        Ok(figurine)
    }

    pub async fn get_images_by_figurine(&self, figurine_id: Uuid) -> Result<Vec<Image>> {
        let images = sqlx::query_as::<_, Image>(
            "SELECT * FROM images WHERE figurine_id = $1 ORDER BY sort_order"
        )
        .bind(figurine_id)
        .fetch_all(&self.pg_pool)
        .await?;
        Ok(images)
    }

    pub async fn get_steps_by_figurine(&self, figurine_id: Uuid) -> Result<Vec<ProcessStep>> {
        let steps = sqlx::query_as::<_, ProcessStep>(
            "SELECT * FROM process_steps WHERE figurine_id = $1 ORDER BY sort_order"
        )
        .bind(figurine_id)
        .fetch_all(&self.pg_pool)
        .await?;
        Ok(steps)
    }

    pub async fn get_related_figurines(&self, current_id: Uuid) -> Result<Vec<Figurine>> {
        let current = match self.get_figurine_by_id(current_id).await? {
            Some(c) => c,
            None => return Ok(vec![]),
        };

        let material_hint = current.material.as_deref().map(|m| {
            if m.len() >= 4 { &m[0..4] } else { m }
        }).unwrap_or("").to_string();

        let related = sqlx::query_as::<_, Figurine>(
            "SELECT * FROM figurines
             WHERE id != $1
             AND is_visible = true
             AND (
                 year = $2
                 OR ($3 != '' AND material LIKE '%' || $4 || '%')
             )
             ORDER BY RANDOM()
             LIMIT 3"
        )
        .bind(current_id)
        .bind(current.year)
        .bind(&material_hint)
        .bind(&material_hint)
        .fetch_all(&self.pg_pool)
        .await?;

        Ok(related)
    }

    pub async fn get_texts_by_category(&self, category: TextCategory) -> Result<Vec<Text>> {
        let texts = sqlx::query_as::<_, Text>(
            "SELECT * FROM texts WHERE category = $1 ORDER BY sort_order"
        )
        .bind(category)
        .fetch_all(&self.pg_pool)
        .await?;
        Ok(texts)
    }

    pub async fn get_zones(&self) -> Result<Vec<CabinetZone>> {
        let zones = sqlx::query_as::<_, CabinetZone>(
            "SELECT * FROM cabinet_zones ORDER BY sort_order"
        )
        .fetch_all(&self.pg_pool)
        .await?;
        Ok(zones)
    }

    // === ADMIN WRITE OPERATIONS ===

    pub async fn upsert_figurine(&self, f: &crate::models::SaveFigurineRequest) -> Result<()> {
        let id = Uuid::parse_str(&f.id)
            .map_err(|_| AppError::BadRequest("Invalid figurine ID".to_string()))?;
        sqlx::query(
            "INSERT INTO figurines (id, name, short_text, full_description, dimensions, material, technique, year, ambience_path, video_url, secret_text, is_visible, is_featured, status, sort_order, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, NOW())
             ON CONFLICT (id) DO UPDATE SET
               name=EXCLUDED.name, short_text=EXCLUDED.short_text, full_description=EXCLUDED.full_description,
               dimensions=EXCLUDED.dimensions, material=EXCLUDED.material, technique=EXCLUDED.technique,
               year=EXCLUDED.year, ambience_path=EXCLUDED.ambience_path, video_url=EXCLUDED.video_url,
               secret_text=EXCLUDED.secret_text, is_visible=EXCLUDED.is_visible, is_featured=EXCLUDED.is_featured,
               status=EXCLUDED.status, sort_order=EXCLUDED.sort_order, updated_at=NOW()"
        )
        .bind(id).bind(&f.name).bind(&f.short_text).bind(&f.full_description)
        .bind(&f.dimensions).bind(&f.material).bind(&f.technique).bind(f.year)
        .bind(&f.ambience_path).bind(&f.video_url).bind(&f.secret_text)
        .bind(f.is_visible).bind(f.is_featured).bind(&f.status).bind(f.sort_order)
        .execute(&self.pg_pool).await?;
        Ok(())
    }

    pub async fn delete_figurine(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM figurines WHERE id = $1")
            .bind(id).execute(&self.pg_pool).await?;
        Ok(())
    }

    pub async fn replace_images(&self, figurine_id: Uuid, images: &[crate::models::SaveImageRequest]) -> Result<()> {
        sqlx::query("DELETE FROM images WHERE figurine_id = $1")
            .bind(figurine_id).execute(&self.pg_pool).await?;
        for (idx, img) in images.iter().enumerate() {
            let img_id = Uuid::parse_str(&img.id)
                .map_err(|_| AppError::BadRequest("Invalid image ID".to_string()))?;
            let sort = img.sort_order.unwrap_or(idx as i32);
            sqlx::query(
                "INSERT INTO images (id, figurine_id, image_type, file_path, original_path, thumb_path, alt_text, sort_order) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
            )
            .bind(img_id).bind(figurine_id).bind(&img.image_type)
            .bind(&img.url).bind(&img.original_url).bind(&img.thumb_url)
            .bind(&img.alt_text).bind(sort)
            .execute(&self.pg_pool).await?;
        }
        Ok(())
    }

    pub async fn replace_steps(&self, figurine_id: Uuid, steps: &[crate::models::SaveStepRequest]) -> Result<()> {
        sqlx::query("DELETE FROM process_steps WHERE figurine_id = $1")
            .bind(figurine_id).execute(&self.pg_pool).await?;
        for (idx, step) in steps.iter().enumerate() {
            let step_id = Uuid::parse_str(&step.id)
                .map_err(|_| AppError::BadRequest("Invalid step ID".to_string()))?;
            let sort = step.sort_order.unwrap_or(idx as i32);
            sqlx::query(
                "INSERT INTO process_steps (id, figurine_id, step_type, description, image_path, sort_order) VALUES ($1, $2, $3, $4, $5, $6)"
            )
            .bind(step_id).bind(figurine_id).bind(&step.step_type)
            .bind(&step.description).bind(&step.image_url).bind(sort)
            .execute(&self.pg_pool).await?;
        }
        Ok(())
    }

    pub async fn upsert_zone(&self, z: &crate::models::SaveZoneRequest, sort_order: i32) -> Result<()> {
        let id = Uuid::parse_str(&z.id)
            .map_err(|_| AppError::BadRequest("Invalid zone ID".to_string()))?;
        sqlx::query(
            "INSERT INTO cabinet_zones (id, zone_type, x_percent, y_percent, width_percent, height_percent, target_route, sort_order)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (id) DO UPDATE SET
               zone_type=EXCLUDED.zone_type, x_percent=EXCLUDED.x_percent, y_percent=EXCLUDED.y_percent,
               width_percent=EXCLUDED.width_percent, height_percent=EXCLUDED.height_percent,
               target_route=EXCLUDED.target_route, sort_order=EXCLUDED.sort_order"
        )
        .bind(id).bind(&z.zone_type).bind(z.x).bind(z.y)
        .bind(z.width).bind(z.height).bind(&z.target_route).bind(sort_order)
        .execute(&self.pg_pool).await?;
        Ok(())
    }

    pub async fn delete_zone(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM cabinet_zones WHERE id = $1")
            .bind(id).execute(&self.pg_pool).await?;
        Ok(())
    }

    pub async fn get_zone_count(&self) -> Result<i32> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cabinet_zones")
            .fetch_one(&self.pg_pool).await?;
        Ok(row.0 as i32)
    }

    pub async fn upsert_text(&self, t: &crate::models::SaveTextRequest, category: &crate::models::TextCategory) -> Result<()> {
        let id = Uuid::parse_str(&t.id)
            .map_err(|_| AppError::BadRequest("Invalid text ID".to_string()))?;
        sqlx::query(
            "INSERT INTO texts (id, category, content, caption, image_path, sort_order, updated_at)
             VALUES ($1, $2, $3, $4, $5, COALESCE((SELECT sort_order FROM texts WHERE id = $6), (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM texts WHERE category = $7)), NOW())
             ON CONFLICT (id) DO UPDATE SET
               content=EXCLUDED.content, caption=EXCLUDED.caption,
               image_path=EXCLUDED.image_path, updated_at=NOW()"
        )
        .bind(id).bind(category).bind(&t.content).bind(&t.caption)
        .bind(&t.image_url).bind(id).bind(category)
        .execute(&self.pg_pool).await?;
        Ok(())
    }

    pub async fn delete_text(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM texts WHERE id = $1")
            .bind(id).execute(&self.pg_pool).await?;
        Ok(())
    }

    pub async fn get_main_background(&self) -> Result<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT file_path FROM app_resources WHERE key = 'main_background'"
        )
        .fetch_optional(&self.pg_pool).await?;
        Ok(row.map(|r| r.0))
    }

    pub async fn set_main_background(&self, url: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO app_resources (key, file_path, updated_at) VALUES ('main_background', $1, NOW())
             ON CONFLICT (key) DO UPDATE SET file_path=EXCLUDED.file_path, updated_at=NOW()"
        )
        .bind(url).execute(&self.pg_pool).await?;
        Ok(())
    }

    pub async fn get_home_content(&self) -> Result<Option<crate::models::HomeContent>> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT file_path FROM app_resources WHERE key = 'home_content'"
        )
        .fetch_optional(&self.pg_pool).await?;
        match row {
            None => Ok(None),
            Some((json,)) => {
                let content = serde_json::from_str(&json)
                    .unwrap_or_default();
                Ok(Some(content))
            }
        }
    }

    pub async fn save_home_content(&self, content: &crate::models::HomeContent) -> Result<()> {
        let json = serde_json::to_string(content)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        sqlx::query(
            "INSERT INTO app_resources (key, file_path, updated_at) VALUES ('home_content', $1, NOW())
             ON CONFLICT (key) DO UPDATE SET file_path=EXCLUDED.file_path, updated_at=NOW()"
        )
        .bind(json).execute(&self.pg_pool).await?;
        Ok(())
    }

    pub async fn get_author_profile(&self) -> Result<Option<crate::models::AuthorProfile>> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT file_path FROM app_resources WHERE key = 'author_profile'"
        )
        .fetch_optional(&self.pg_pool).await?;
        match row {
            None => Ok(None),
            Some((json,)) => {
                let profile = serde_json::from_str(&json)
                    .unwrap_or_default();
                Ok(Some(profile))
            }
        }
    }

    pub async fn save_author_profile(&self, profile: &crate::models::AuthorProfile) -> Result<()> {
        let json = serde_json::to_string(profile)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        sqlx::query(
            "INSERT INTO app_resources (key, file_path, updated_at) VALUES ('author_profile', $1, NOW())
             ON CONFLICT (key) DO UPDATE SET file_path=EXCLUDED.file_path, updated_at=NOW()"
        )
        .bind(json).execute(&self.pg_pool).await?;
        Ok(())
    }

    // === MEDIA ===

    // No blobs in Postgres — files are served from disk via /static/
    pub async fn get_blob(&self, _table: &str, _column: &str, _id: String) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }

    pub async fn get_media_usages(&self) -> Result<Vec<MediaUsageDto>> {
        let mut usages = Vec::new();

        let image_rows: Vec<(String, String, Option<String>, Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT i.id::text, i.file_path, i.original_path, i.thumb_path, f.id::text, f.name
             FROM images i
             LEFT JOIN figurines f ON f.id = i.figurine_id"
        ).fetch_all(&self.pg_pool).await?;
        for (image_id, preview, original, thumb, fig_id, fig_name) in image_rows {
            let label = format!("Image for {}", fig_name.unwrap_or_else(|| "Unknown figurine".to_string()));
            let entity_id = fig_id.unwrap_or_else(|| image_id.clone());
            usages.push(MediaUsageDto {
                path: preview,
                label: label.clone(),
                entity_type: "figurineImage".to_string(),
                entity_id: entity_id.clone(),
                field: "preview".to_string(),
            });
            if let Some(path) = original {
                usages.push(MediaUsageDto {
                    path,
                    label: label.clone(),
                    entity_type: "figurineImage".to_string(),
                    entity_id: entity_id.clone(),
                    field: "original".to_string(),
                });
            }
            if let Some(path) = thumb {
                usages.push(MediaUsageDto {
                    path,
                    label: label.clone(),
                    entity_type: "figurineImage".to_string(),
                    entity_id: entity_id.clone(),
                    field: "thumb".to_string(),
                });
            }
        }

        let step_rows: Vec<(String, String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT ps.id::text, ps.image_path, f.id::text, f.name
             FROM process_steps ps
             LEFT JOIN figurines f ON f.id = ps.figurine_id"
        ).fetch_all(&self.pg_pool).await?;
        for (step_id, path, fig_id, fig_name) in step_rows {
            usages.push(MediaUsageDto {
                path,
                label: format!("Process step for {}", fig_name.unwrap_or_else(|| "Unknown figurine".to_string())),
                entity_type: "processStep".to_string(),
                entity_id: fig_id.unwrap_or(step_id),
                field: "image".to_string(),
            });
        }

        let text_rows: Vec<(String, String, Option<String>, String)> = sqlx::query_as(
            "SELECT id::text, category::text, caption, image_path FROM texts WHERE image_path IS NOT NULL"
        ).fetch_all(&self.pg_pool).await?;
        for (id, category, caption, path) in text_rows {
            usages.push(MediaUsageDto {
                path,
                label: caption.unwrap_or_else(|| format!("{} text", category)),
                entity_type: "text".to_string(),
                entity_id: id,
                field: "image".to_string(),
            });
        }

        let figurine_rows: Vec<(String, String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT id::text, name, ambience_path, video_url FROM figurines"
        ).fetch_all(&self.pg_pool).await?;
        for (id, name, ambience, video) in figurine_rows {
            if let Some(path) = ambience {
                usages.push(MediaUsageDto {
                    path,
                    label: format!("Audio for {}", name),
                    entity_type: "figurine".to_string(),
                    entity_id: id.clone(),
                    field: "ambience".to_string(),
                });
            }
            if let Some(path) = video {
                usages.push(MediaUsageDto {
                    path,
                    label: format!("Video for {}", name),
                    entity_type: "figurine".to_string(),
                    entity_id: id.clone(),
                    field: "video".to_string(),
                });
            }
        }

        let resource_rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT key, file_path FROM app_resources WHERE key NOT IN ('author_profile', 'home_content')"
        ).fetch_all(&self.pg_pool).await?;
        for (key, path) in resource_rows {
            usages.push(MediaUsageDto {
                path,
                label: format!("App resource {}", key),
                entity_type: "appResource".to_string(),
                entity_id: key,
                field: "file".to_string(),
            });
        }

        Ok(usages)
    }

    // === SHOWINGS ===

    pub async fn get_showings_by_figurine(&self, figurine_id: Uuid) -> Result<Vec<Showing>> {
        let rows = sqlx::query_as::<_, Showing>(
            "SELECT * FROM figurine_showings WHERE figurine_id = $1 ORDER BY starts_at"
        )
        .bind(figurine_id)
        .fetch_all(&self.pg_pool).await?;
        Ok(rows)
    }

    pub async fn get_all_showings(&self) -> Result<Vec<Showing>> {
        let rows = sqlx::query_as::<_, Showing>(
            "SELECT * FROM figurine_showings ORDER BY starts_at DESC"
        )
        .fetch_all(&self.pg_pool).await?;
        Ok(rows)
    }

    pub async fn upsert_showing(&self, req: &crate::models::SaveShowingRequest) -> Result<Uuid> {
        let id = match &req.id {
            Some(s) => Uuid::parse_str(s).map_err(|_| AppError::BadRequest("Invalid showing ID".to_string()))?,
            None => Uuid::new_v4(),
        };
        let figurine_id = Uuid::parse_str(&req.figurine_id)
            .map_err(|_| AppError::BadRequest("Invalid figurine ID".to_string()))?;
        let starts_at = chrono::NaiveDate::parse_from_str(&req.starts_at, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest("Invalid starts_at date".to_string()))?;
        let ends_at = chrono::NaiveDate::parse_from_str(&req.ends_at, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest("Invalid ends_at date".to_string()))?;

        sqlx::query(
            "INSERT INTO figurine_showings (id, figurine_id, title, showing_type, starts_at, ends_at, venue, notes)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (id) DO UPDATE SET
               figurine_id=EXCLUDED.figurine_id, title=EXCLUDED.title,
               showing_type=EXCLUDED.showing_type, starts_at=EXCLUDED.starts_at,
               ends_at=EXCLUDED.ends_at, venue=EXCLUDED.venue, notes=EXCLUDED.notes"
        )
        .bind(id).bind(figurine_id).bind(&req.title).bind(&req.showing_type)
        .bind(starts_at).bind(ends_at).bind(&req.venue).bind(&req.notes)
        .execute(&self.pg_pool).await?;
        Ok(id)
    }

    pub async fn delete_showing(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM figurine_showings WHERE id = $1")
            .bind(id).execute(&self.pg_pool).await?;
        Ok(())
    }

    pub async fn get_figurine_schedule(&self, figurine_id: Uuid) -> Result<(Vec<Showing>, Vec<Booking>, Vec<Booking>)> {
        let today = chrono::Utc::now().date_naive();
        let showings = sqlx::query_as::<_, Showing>(
            "SELECT * FROM figurine_showings WHERE figurine_id = $1 AND ends_at >= $2 ORDER BY starts_at"
        )
        .bind(figurine_id).bind(today)
        .fetch_all(&self.pg_pool).await?;

        let confirmed = sqlx::query_as::<_, Booking>(
            "SELECT * FROM figurine_bookings WHERE figurine_id = $1 AND status = 'confirmed' AND ends_at >= $2 ORDER BY starts_at"
        )
        .bind(figurine_id).bind(today)
        .fetch_all(&self.pg_pool).await?;

        let pending = sqlx::query_as::<_, Booking>(
            "SELECT * FROM figurine_bookings WHERE figurine_id = $1 AND status = 'pending' AND ends_at >= $2 ORDER BY starts_at"
        )
        .bind(figurine_id).bind(today)
        .fetch_all(&self.pg_pool).await?;

        Ok((showings, confirmed, pending))
    }

    pub async fn check_booking_conflicts(&self, figurine_id: Uuid, starts_at: chrono::NaiveDate, ends_at: chrono::NaiveDate) -> Result<bool> {
        let (showing_conflict,): (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM figurine_showings WHERE figurine_id = $1 AND starts_at <= $3 AND ends_at >= $2)"
        )
        .bind(figurine_id).bind(starts_at).bind(ends_at)
        .fetch_one(&self.pg_pool).await?;

        if showing_conflict { return Ok(true); }

        let (booking_conflict,): (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM figurine_bookings WHERE figurine_id = $1 AND status = 'confirmed' AND starts_at <= $3 AND ends_at >= $2)"
        )
        .bind(figurine_id).bind(starts_at).bind(ends_at)
        .fetch_one(&self.pg_pool).await?;

        Ok(booking_conflict)
    }

    // === BOOKINGS ===

    pub async fn save_booking(&self, req: &crate::models::CreateBookingRequest) -> Result<Booking> {
        let figurine_id = Uuid::parse_str(&req.figurine_id)
            .map_err(|_| AppError::BadRequest("Invalid figurine ID".to_string()))?;
        let starts_at = chrono::NaiveDate::parse_from_str(&req.starts_at, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest("Invalid starts_at".to_string()))?;
        let ends_at = chrono::NaiveDate::parse_from_str(&req.ends_at, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest("Invalid ends_at".to_string()))?;

        let cancel_token = Self::generate_cancel_token();

        let rec = sqlx::query_as::<_, Booking>(
            "INSERT INTO figurine_bookings (figurine_id, figurine_name, requester_name, requester_email, purpose, starts_at, ends_at, cancel_token)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *"
        )
        .bind(figurine_id).bind(&req.figurine_name).bind(&req.requester_name)
        .bind(&req.requester_email).bind(&req.purpose).bind(starts_at).bind(ends_at)
        .bind(&cancel_token)
        .fetch_one(&self.pg_pool).await?;
        Ok(rec)
    }

    fn generate_cancel_token() -> String {
        let raw = Uuid::new_v4().to_string().replace('-', "").to_uppercase();
        format!("{}-{}", &raw[..4], &raw[4..8])
    }

    pub async fn get_booking_by_cancel_token(&self, token: &str) -> Result<Option<Booking>> {
        Ok(sqlx::query_as::<_, Booking>(
            "SELECT * FROM figurine_bookings WHERE cancel_token = $1"
        )
        .bind(token)
        .fetch_optional(&self.pg_pool).await?)
    }

    pub async fn cancel_booking_by_token(&self, token: &str) -> Result<Option<Booking>> {
        Ok(sqlx::query_as::<_, Booking>(
            "UPDATE figurine_bookings SET status = 'cancelled', updated_at = NOW()
             WHERE cancel_token = $1 AND status = 'pending'
             RETURNING *"
        )
        .bind(token)
        .fetch_optional(&self.pg_pool).await?)
    }

    pub async fn get_bookings_page(
        &self,
        status_filter: Option<&str>,
        figurine_id_filter: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<Booking>, i64)> {
        // Build WHERE clauses dynamically
        let mut conditions: Vec<String> = Vec::new();
        if status_filter.is_some()    { conditions.push(format!("status = ${}::booking_status", conditions.len() + 1)); }
        if figurine_id_filter.is_some() { conditions.push(format!("figurine_id = ${}", conditions.len() + 1)); }
        let where_clause = if conditions.is_empty() { String::new() } else { format!("WHERE {}", conditions.join(" AND ")) };

        let items_sql = format!("SELECT * FROM figurine_bookings {} ORDER BY created_at DESC LIMIT ${} OFFSET ${}", where_clause, conditions.len() + 1, conditions.len() + 2);
        let count_sql = format!("SELECT COUNT(*) FROM figurine_bookings {}", where_clause);

        macro_rules! bind_filters {
            ($q:expr) => {{
                let mut q = $q;
                if let Some(s) = status_filter      { q = q.bind(s); }
                if let Some(f) = figurine_id_filter  { q = q.bind(f); }
                q
            }};
        }

        let items = bind_filters!(sqlx::query_as::<_, Booking>(&items_sql))
            .bind(limit).bind(offset)
            .fetch_all(&self.pg_pool).await?;

        let (total,): (i64,) = bind_filters!(sqlx::query_as::<_, (i64,)>(&count_sql))
            .fetch_one(&self.pg_pool).await?;

        Ok((items, total))
    }

    pub async fn update_figurine_status(&self, figurine_id: Uuid, status: &crate::models::FigurineStatus) -> Result<()> {
        sqlx::query("UPDATE figurines SET status = $1, updated_at = NOW() WHERE id = $2")
            .bind(status).bind(figurine_id)
            .execute(&self.pg_pool).await?;
        Ok(())
    }

    pub async fn has_future_confirmed_bookings(&self, figurine_id: Uuid, exclude_id: Uuid) -> Result<bool> {
        let today = chrono::Utc::now().date_naive();
        let (exists,): (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM figurine_bookings WHERE id != $1 AND figurine_id = $2 AND status = 'confirmed' AND ends_at >= $3)"
        )
        .bind(exclude_id).bind(figurine_id).bind(today)
        .fetch_one(&self.pg_pool).await?;
        Ok(exists)
    }

    pub async fn get_booking_by_id(&self, id: Uuid) -> Result<Option<Booking>> {
        Ok(sqlx::query_as::<_, Booking>(
            "SELECT * FROM figurine_bookings WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pg_pool).await?)
    }

    // Check conflicts for admin confirmation: showings + other confirmed bookings (excluding the booking being confirmed)
    pub async fn check_admin_confirm_conflicts(
        &self,
        booking_id: Uuid,
        figurine_id: Uuid,
        starts_at: chrono::NaiveDate,
        ends_at: chrono::NaiveDate,
    ) -> Result<Option<String>> {
        let (showing_conflict,): (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM figurine_showings WHERE figurine_id = $1 AND starts_at <= $3 AND ends_at >= $2)"
        )
        .bind(figurine_id).bind(starts_at).bind(ends_at)
        .fetch_one(&self.pg_pool).await?;

        if showing_conflict {
            return Ok(Some("Даты пересекаются с показом фигурки".to_string()));
        }

        let (booking_conflict,): (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM figurine_bookings WHERE id != $1 AND figurine_id = $2 AND status = 'confirmed' AND starts_at <= $4 AND ends_at >= $3)"
        )
        .bind(booking_id).bind(figurine_id).bind(starts_at).bind(ends_at)
        .fetch_one(&self.pg_pool).await?;

        if booking_conflict {
            return Ok(Some("На эти даты уже есть подтверждённая бронь".to_string()));
        }

        Ok(None)
    }

    pub async fn get_pending_bookings_count(&self) -> Result<i64> {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM figurine_bookings WHERE status = 'pending'"
        )
        .fetch_one(&self.pg_pool).await?;
        Ok(count)
    }

    pub async fn update_booking_status(&self, id: Uuid, status: &crate::models::BookingStatus, admin_notes: Option<&str>) -> Result<()> {
        let affected = sqlx::query(
            "UPDATE figurine_bookings SET status = $1, admin_notes = COALESCE($2, admin_notes) WHERE id = $3"
        )
        .bind(status).bind(admin_notes).bind(id)
        .execute(&self.pg_pool).await?.rows_affected();

        if affected == 0 {
            return Err(AppError::NotFound(format!("Booking {} not found", id)));
        }
        Ok(())
    }

    pub async fn replace_media_path_everywhere(
        &self,
        old_path: &str,
        new_preview_path: &str,
        new_original_path: Option<&str>,
        new_thumb_path: Option<&str>,
    ) -> Result<usize> {
        let mut updated = 0usize;

        if new_preview_path.starts_with("images/preview/") || new_preview_path.starts_with("/static/images/preview/") {
            let result = sqlx::query(
                "UPDATE images
                 SET file_path = $1, original_path = $2, thumb_path = $3
                 WHERE file_path = $4 OR original_path = $5 OR thumb_path = $6"
            )
            .bind(new_preview_path)
            .bind(new_original_path)
            .bind(new_thumb_path)
            .bind(old_path)
            .bind(old_path)
            .bind(old_path)
            .execute(&self.pg_pool).await?;
            updated += result.rows_affected() as usize;
        } else {
            for column in ["file_path", "original_path", "thumb_path"] {
                let query = format!("UPDATE images SET {} = $1 WHERE {} = $2", column, column);
                let result = sqlx::query(&query)
                    .bind(new_preview_path)
                    .bind(old_path)
                    .execute(&self.pg_pool).await?;
                updated += result.rows_affected() as usize;
            }
        }

        for (table, column) in [
            ("process_steps", "image_path"),
            ("texts", "image_path"),
            ("figurines", "ambience_path"),
            ("figurines", "video_url"),
        ] {
            let query = format!("UPDATE {} SET {} = $1 WHERE {} = $2", table, column, column);
            let result = sqlx::query(&query)
                .bind(new_preview_path)
                .bind(old_path)
                .execute(&self.pg_pool).await?;
            updated += result.rows_affected() as usize;
        }

        // app_resources — skip JSON-stored keys
        let result = sqlx::query(
            "UPDATE app_resources SET file_path = $1 WHERE file_path = $2 AND key NOT IN ('author_profile', 'home_content')"
        )
        .bind(new_preview_path)
        .bind(old_path)
        .execute(&self.pg_pool).await?;
        updated += result.rows_affected() as usize;

        Ok(updated)
    }
}
