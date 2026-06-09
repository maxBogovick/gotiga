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

    pub fn pg_pool(&self) -> &PgPool {
        &self.pg_pool
    }

    // === ORDERS (Postgres) ===

    pub async fn save_order(&self, order: &crate::models::OrderRequest) -> Result<crate::models::Order> {
        let rec = sqlx::query_as::<_, crate::models::Order>(
            "INSERT INTO orders (figurine_id, figurine_name, requester_name, requester_email, requester_phone, message, mode)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING *"
        )
        .bind(&order.figurine_id)
        .bind(&order.figurine_name)
        .bind(&order.requester_name)
        .bind(&order.requester_email)
        .bind(&order.requester_phone)
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

    pub async fn get_order_by_id(&self, id: uuid::Uuid) -> Result<Option<crate::models::Order>> {
        Ok(sqlx::query_as::<_, crate::models::Order>(
            "SELECT * FROM orders WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pg_pool).await?)
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
            "INSERT INTO figurine_bookings (figurine_id, figurine_name, requester_name, requester_email, requester_phone, purpose, starts_at, ends_at, cancel_token)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING *"
        )
        .bind(figurine_id).bind(&req.figurine_name).bind(&req.requester_name)
        .bind(&req.requester_email).bind(&req.requester_phone).bind(&req.purpose)
        .bind(starts_at).bind(ends_at).bind(&cancel_token)
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
            "UPDATE figurine_bookings SET status = 'cancelled'
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

    // ============================================================
    // USER ACCOUNTS
    // ============================================================

    pub async fn create_user(&self, email: &str, display_name: &str, hash: &str) -> Result<crate::models::User> {
        let user = sqlx::query_as::<_, crate::models::User>(
            "INSERT INTO users (email, display_name, visual_password_hash)
             VALUES ($1, $2, $3) RETURNING *"
        )
        .bind(email)
        .bind(display_name)
        .bind(hash)
        .fetch_one(&self.pg_pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref dbe) = e {
                if dbe.constraint() == Some("users_email_key") || dbe.constraint() == Some("idx_users_email") {
                    return AppError::Conflict("Email already registered".into());
                }
            }
            AppError::Database(e)
        })?;
        Ok(user)
    }

    pub async fn find_user_by_email(&self, email: &str) -> Result<Option<crate::models::User>> {
        let user = sqlx::query_as::<_, crate::models::User>(
            "SELECT * FROM users WHERE email = $1"
        )
        .bind(email)
        .fetch_optional(&self.pg_pool)
        .await?;
        Ok(user)
    }

    pub async fn find_user_by_id(&self, id: Uuid) -> Result<Option<crate::models::User>> {
        let user = sqlx::query_as::<_, crate::models::User>(
            "SELECT * FROM users WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pg_pool)
        .await?;
        Ok(user)
    }

    // ── Sessions ─────────────────────────────────────────────

    pub async fn create_session(&self, user_id: Uuid, token: &str, expires_at: chrono::DateTime<chrono::Utc>) -> Result<()> {
        sqlx::query(
            "INSERT INTO user_sessions (user_id, token, expires_at) VALUES ($1, $2, $3)"
        )
        .bind(user_id)
        .bind(token)
        .bind(expires_at)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    pub async fn get_session_user(&self, token: &str) -> Result<Option<crate::models::User>> {
        let user = sqlx::query_as::<_, crate::models::User>(
            "SELECT u.* FROM users u
             JOIN user_sessions s ON s.user_id = u.id
             WHERE s.token = $1 AND s.expires_at > NOW()"
        )
        .bind(token)
        .fetch_optional(&self.pg_pool)
        .await?;
        Ok(user)
    }

    pub async fn delete_session(&self, token: &str) -> Result<()> {
        sqlx::query("DELETE FROM user_sessions WHERE token = $1")
            .bind(token)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    // ── Challenges ───────────────────────────────────────────

    pub async fn save_challenge(&self, email: &str, tokens_json: &serde_json::Value) -> Result<Uuid> {
        let rec: (Uuid,) = sqlx::query_as(
            "INSERT INTO login_challenges (email, tokens_json)
             VALUES ($1, $2) RETURNING id"
        )
        .bind(email)
        .bind(tokens_json)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(rec.0)
    }

    pub async fn get_challenge(&self, id: Uuid) -> Result<Option<(String, serde_json::Value)>> {
        let row: Option<(String, serde_json::Value)> = sqlx::query_as(
            "SELECT email, tokens_json FROM login_challenges
             WHERE id = $1 AND expires_at > NOW() AND used_at IS NULL"
        )
        .bind(id)
        .fetch_optional(&self.pg_pool)
        .await?;
        Ok(row)
    }

    pub async fn mark_challenge_used(&self, id: Uuid) -> Result<()> {
        sqlx::query("UPDATE login_challenges SET used_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    // ── Lockout ──────────────────────────────────────────────

    pub async fn record_attempt(&self, email: &str, success: bool) -> Result<()> {
        sqlx::query(
            "INSERT INTO login_attempts (email, success) VALUES ($1, $2)"
        )
        .bind(email)
        .bind(success)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    pub async fn count_recent_failures(&self, email: &str, window_minutes: i64) -> Result<i64> {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM login_attempts
             WHERE email = $1
               AND success = false
               AND attempted_at > NOW() - ($2 || ' minutes')::interval"
        )
        .bind(email)
        .bind(window_minutes)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(count)
    }

    // ── Profile data ─────────────────────────────────────────

    pub async fn link_bookings_to_user(&self, user_id: Uuid, cancel_tokens: &[String]) -> Result<usize> {
        if cancel_tokens.is_empty() { return Ok(0); }
        let result = sqlx::query(
            "UPDATE figurine_bookings SET user_id = $1 WHERE cancel_token = ANY($2) AND user_id IS NULL"
        )
        .bind(user_id)
        .bind(cancel_tokens)
        .execute(&self.pg_pool)
        .await?;
        Ok(result.rows_affected() as usize)
    }

    pub async fn prune_expired_sessions(&self, user_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM user_sessions WHERE user_id = $1 AND expires_at < NOW()")
            .bind(user_id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn get_user_bookings(&self, user_id: Uuid) -> Result<Vec<crate::models::Booking>> {
        let bookings = sqlx::query_as::<_, crate::models::Booking>(
            "SELECT * FROM figurine_bookings WHERE user_id = $1 ORDER BY created_at DESC"
        )
        .bind(user_id)
        .fetch_all(&self.pg_pool)
        .await?;
        Ok(bookings)
    }

    pub async fn get_user_orders(&self, user_id: Uuid) -> Result<Vec<crate::models::Order>> {
        let orders = sqlx::query_as::<_, crate::models::Order>(
            "SELECT * FROM orders WHERE user_id = $1 ORDER BY created_at DESC"
        )
        .bind(user_id)
        .fetch_all(&self.pg_pool)
        .await?;
        Ok(orders)
    }

    // ── Admin user management ────────────────────────────────

    pub async fn admin_list_users(&self, search: Option<&str>, limit: i64, offset: i64) -> Result<(Vec<crate::models::AdminUserListItem>, i64)> {
        let pattern = search.map(|s| format!("%{}%", s.to_lowercase()));
        let items = if let Some(ref p) = pattern {
            sqlx::query_as::<_, crate::models::AdminUserListItem>(
                "SELECT u.id::text, u.email, u.display_name, u.admin_notes,
                        u.created_at::text,
                        COUNT(DISTINCT b.id) AS booking_count,
                        COUNT(DISTINCT o.id) AS order_count
                 FROM users u
                 LEFT JOIN figurine_bookings b ON b.user_id = u.id
                 LEFT JOIN orders o ON o.user_id = u.id
                 WHERE LOWER(u.email) LIKE $1 OR LOWER(u.display_name) LIKE $1
                 GROUP BY u.id
                 ORDER BY u.created_at DESC
                 LIMIT $2 OFFSET $3"
            )
            .bind(p).bind(limit).bind(offset)
            .fetch_all(&self.pg_pool).await?
        } else {
            sqlx::query_as::<_, crate::models::AdminUserListItem>(
                "SELECT u.id::text, u.email, u.display_name, u.admin_notes,
                        u.created_at::text,
                        COUNT(DISTINCT b.id) AS booking_count,
                        COUNT(DISTINCT o.id) AS order_count
                 FROM users u
                 LEFT JOIN figurine_bookings b ON b.user_id = u.id
                 LEFT JOIN orders o ON o.user_id = u.id
                 GROUP BY u.id
                 ORDER BY u.created_at DESC
                 LIMIT $1 OFFSET $2"
            )
            .bind(limit).bind(offset)
            .fetch_all(&self.pg_pool).await?
        };

        let (total,): (i64,) = if let Some(ref p) = pattern {
            sqlx::query_as(
                "SELECT COUNT(*) FROM users WHERE LOWER(email) LIKE $1 OR LOWER(display_name) LIKE $1"
            ).bind(p).fetch_one(&self.pg_pool).await?
        } else {
            sqlx::query_as("SELECT COUNT(*) FROM users")
                .fetch_one(&self.pg_pool).await?
        };

        Ok((items, total))
    }

    pub async fn admin_get_user_sessions(&self, user_id: Uuid) -> Result<Vec<crate::models::AdminSessionDto>> {
        let rows: Vec<(Uuid, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)> =
            sqlx::query_as(
                "SELECT id, created_at, expires_at FROM user_sessions
                 WHERE user_id = $1 ORDER BY created_at DESC"
            )
            .bind(user_id)
            .fetch_all(&self.pg_pool)
            .await?;

        let now = chrono::Utc::now();
        Ok(rows.into_iter().map(|(id, created_at, expires_at)| crate::models::AdminSessionDto {
            id: id.to_string(),
            created_at: created_at.to_rfc3339(),
            expires_at: expires_at.to_rfc3339(),
            is_active: expires_at > now,
        }).collect())
    }

    pub async fn admin_revoke_all_sessions(&self, user_id: Uuid) -> Result<u64> {
        let result = sqlx::query("DELETE FROM user_sessions WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pg_pool)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn admin_update_user_notes(&self, user_id: Uuid, notes: Option<&str>) -> Result<()> {
        sqlx::query("UPDATE users SET admin_notes = $1 WHERE id = $2")
            .bind(notes)
            .bind(user_id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn admin_set_user_blocked(&self, user_id: Uuid, blocked: bool) -> Result<()> {
        sqlx::query("UPDATE users SET is_blocked = $1 WHERE id = $2")
            .bind(blocked)
            .bind(user_id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn admin_create_reset_token(&self, user_id: Uuid, token: &str, expires_at: chrono::DateTime<chrono::Utc>) -> Result<()> {
        sqlx::query(
            "UPDATE users SET password_reset_token = $1, password_reset_expires_at = $2 WHERE id = $3"
        )
        .bind(token)
        .bind(expires_at)
        .bind(user_id)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    /// Returns the user if token is valid and not yet expired.
    pub async fn find_user_by_reset_token(&self, token: &str) -> Result<Option<crate::models::User>> {
        let user = sqlx::query_as::<_, crate::models::User>(
            "SELECT * FROM users WHERE password_reset_token = $1 AND password_reset_expires_at > NOW()"
        )
        .bind(token)
        .fetch_optional(&self.pg_pool)
        .await?;
        Ok(user)
    }

    pub async fn apply_password_reset(&self, user_id: Uuid, new_hash: &str) -> Result<()> {
        sqlx::query(
            "UPDATE users SET visual_password_hash = $1, password_reset_token = NULL, password_reset_expires_at = NULL WHERE id = $2"
        )
        .bind(new_hash)
        .bind(user_id)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    // === COMMENTS ===

    pub async fn insert_comment(
        &self,
        figurine_id: Uuid,
        user_id: Option<Uuid>,
        author_name: &str,
        author_email: Option<&str>,
        body: &str,
    ) -> Result<crate::models::Comment> {
        let rec = sqlx::query_as::<_, crate::models::Comment>(
            "INSERT INTO figurine_comments (figurine_id, user_id, author_name, author_email, body)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING *"
        )
        .bind(figurine_id)
        .bind(user_id)
        .bind(author_name)
        .bind(author_email)
        .bind(body)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(rec)
    }

    pub async fn get_approved_comments(&self, figurine_id: Uuid, newest_first: bool) -> Result<Vec<crate::models::CommentWithAvatar>> {
        let order = if newest_first { "DESC" } else { "ASC" };
        let rows = sqlx::query_as::<_, crate::models::CommentWithAvatar>(
            &format!(
                "SELECT c.id, c.figurine_id, c.user_id, c.author_name, c.author_email, \
                        c.body, c.is_approved, c.admin_reply, c.created_at, \
                        u.avatar_url \
                 FROM figurine_comments c \
                 LEFT JOIN users u ON u.id = c.user_id \
                 WHERE c.figurine_id = $1 AND c.is_approved = true \
                 ORDER BY c.created_at {order}"
            )
        )
        .bind(figurine_id)
        .fetch_all(&self.pg_pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_pending_comments_count(&self) -> Result<i64> {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM figurine_comments WHERE is_approved = false"
        )
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(count)
    }

    pub async fn get_comments_admin_page(
        &self,
        only_pending: bool,
        figurine_filter: Option<Uuid>,
        newest_first: bool,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<(crate::models::Comment, String)>, i64)> {
        let mut conditions = Vec::new();
        if only_pending { conditions.push("c.is_approved = false"); }
        let figurine_cond;
        if figurine_filter.is_some() {
            figurine_cond = "c.figurine_id = $3".to_string();
            conditions.push(&figurine_cond);
        }
        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };
        let order = if newest_first { "DESC" } else { "ASC" };

        let items: Vec<(crate::models::Comment, String)> = {
            let query_str = format!(
                "SELECT c.*, f.name AS figurine_name
                 FROM figurine_comments c
                 JOIN figurines f ON f.id = c.figurine_id
                 {where_clause}
                 ORDER BY c.created_at {order}
                 LIMIT $1 OFFSET $2"
            );
            let mut q = sqlx::query(&query_str).bind(limit).bind(offset);
            if let Some(fid) = figurine_filter { q = q.bind(fid); }

            let rows = q.fetch_all(&self.pg_pool).await?;

            use sqlx::Row;
            rows.into_iter().map(|row| {
                let c = crate::models::Comment {
                    id:           row.get("id"),
                    figurine_id:  row.get("figurine_id"),
                    user_id:      row.get("user_id"),
                    author_name:  row.get("author_name"),
                    author_email: row.get("author_email"),
                    body:         row.get("body"),
                    is_approved:  row.get("is_approved"),
                    admin_reply:  row.get("admin_reply"),
                    created_at:   row.get("created_at"),
                };
                let name: String = row.get("figurine_name");
                (c, name)
            }).collect()
        };

        let count_str = format!("SELECT COUNT(*) FROM figurine_comments c JOIN figurines f ON f.id = c.figurine_id {where_clause}");
        let mut count_q = sqlx::query_as::<_, (i64,)>(&count_str);
        if let Some(fid) = figurine_filter { count_q = count_q.bind(fid); }
        let (total,) = count_q.fetch_one(&self.pg_pool).await?;

        Ok((items, total))
    }

    pub async fn moderate_comment(
        &self,
        id: Uuid,
        is_approved: bool,
        admin_reply: Option<&str>,
    ) -> Result<crate::models::Comment> {
        let rec = sqlx::query_as::<_, crate::models::Comment>(
            "UPDATE figurine_comments SET is_approved = $1, admin_reply = $2 WHERE id = $3 RETURNING *"
        )
        .bind(is_approved)
        .bind(admin_reply)
        .bind(id)
        .fetch_optional(&self.pg_pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Comment {} not found", id)))?;
        Ok(rec)
    }

    pub async fn delete_comment(&self, id: Uuid) -> Result<()> {
        let affected = sqlx::query("DELETE FROM figurine_comments WHERE id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?
            .rows_affected();
        if affected == 0 {
            return Err(AppError::NotFound(format!("Comment {} not found", id)));
        }
        Ok(())
    }

    // === SETTINGS ===

    pub async fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as("SELECT value FROM settings WHERE key = $1")
            .bind(key)
            .fetch_optional(&self.pg_pool)
            .await?;
        Ok(row.map(|(v,)| v))
    }

    pub async fn upsert_setting(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO settings (key, value) VALUES ($1, $2)
             ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value"
        )
        .bind(key)
        .bind(value)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    pub async fn update_user_display_name(&self, user_id: Uuid, display_name: &str) -> Result<crate::models::User> {
        let user = sqlx::query_as::<_, crate::models::User>(
            "UPDATE users SET display_name = $1 WHERE id = $2 RETURNING *"
        )
        .bind(display_name)
        .bind(user_id)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(user)
    }

    pub async fn update_user_avatar(&self, user_id: Uuid, avatar_url: &str) -> Result<crate::models::User> {
        let user = sqlx::query_as::<_, crate::models::User>(
            "UPDATE users SET avatar_url = $1 WHERE id = $2 RETURNING *"
        )
        .bind(avatar_url)
        .bind(user_id)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(user)
    }

    pub async fn delete_user(&self, user_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    // === CONFLICT CHECK EXCLUDING ONE BOOKING ===

    /// Same as check_booking_conflicts but excludes a specific booking ID (for reschedule).
    pub async fn check_booking_conflicts_excluding(
        &self,
        figurine_id: Uuid,
        exclude_booking_id: Uuid,
        starts_at: chrono::NaiveDate,
        ends_at: chrono::NaiveDate,
    ) -> Result<bool> {
        let (showing_conflict,): (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM figurine_showings WHERE figurine_id = $1 AND starts_at <= $3 AND ends_at >= $2)"
        )
        .bind(figurine_id).bind(starts_at).bind(ends_at)
        .fetch_one(&self.pg_pool).await?;

        if showing_conflict { return Ok(true); }

        let (booking_conflict,): (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM figurine_bookings WHERE figurine_id = $1 AND id != $2 AND status = 'confirmed' AND starts_at <= $4 AND ends_at >= $3)"
        )
        .bind(figurine_id).bind(exclude_booking_id).bind(starts_at).bind(ends_at)
        .fetch_one(&self.pg_pool).await?;

        Ok(booking_conflict)
    }

    // === RESCHEDULE BOOKING BY TOKEN ===

    /// Updates starts_at/ends_at for a pending booking identified by cancel token.
    /// Returns the updated booking, or None if not found / not pending.
    pub async fn reschedule_booking_by_token(
        &self,
        token: &str,
        starts_at: chrono::NaiveDate,
        ends_at: chrono::NaiveDate,
    ) -> Result<Option<crate::models::Booking>> {
        Ok(sqlx::query_as::<_, crate::models::Booking>(
            "UPDATE figurine_bookings SET starts_at = $1, ends_at = $2
             WHERE cancel_token = $3 AND status = 'pending'
             RETURNING *"
        )
        .bind(starts_at).bind(ends_at).bind(token)
        .fetch_optional(&self.pg_pool).await?)
    }

    // === WAITLIST ===

    pub async fn add_to_waitlist(
        &self,
        figurine_id: Uuid,
        req: &crate::models::CreateWaitlistRequest,
        user_id: Option<Uuid>,
    ) -> Result<crate::models::WaitlistEntry> {
        let rec = sqlx::query_as::<_, crate::models::WaitlistEntry>(
            "INSERT INTO figurine_waitlist (figurine_id, figurine_name, requester_name, requester_email, requester_phone, note, user_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *"
        )
        .bind(figurine_id)
        .bind(&req.figurine_name)
        .bind(&req.requester_name)
        .bind(&req.requester_email)
        .bind(&req.requester_phone)
        .bind(&req.note)
        .bind(user_id)
        .fetch_one(&self.pg_pool).await?;
        Ok(rec)
    }

    pub async fn get_waitlist_admin(
        &self,
        figurine_id: Option<Uuid>,
    ) -> Result<Vec<crate::models::WaitlistEntry>> {
        if let Some(fid) = figurine_id {
            Ok(sqlx::query_as::<_, crate::models::WaitlistEntry>(
                "SELECT * FROM figurine_waitlist WHERE figurine_id = $1 ORDER BY created_at ASC"
            )
            .bind(fid)
            .fetch_all(&self.pg_pool).await?)
        } else {
            Ok(sqlx::query_as::<_, crate::models::WaitlistEntry>(
                "SELECT * FROM figurine_waitlist ORDER BY created_at ASC"
            )
            .fetch_all(&self.pg_pool).await?)
        }
    }

    pub async fn remove_from_waitlist(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM figurine_waitlist WHERE id = $1")
            .bind(id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn get_waitlist_for_figurine(&self, figurine_id: Uuid) -> Result<Vec<crate::models::WaitlistEntry>> {
        Ok(sqlx::query_as::<_, crate::models::WaitlistEntry>(
            "SELECT * FROM figurine_waitlist WHERE figurine_id = $1 ORDER BY created_at ASC"
        )
        .bind(figurine_id)
        .fetch_all(&self.pg_pool).await?)
    }

    pub async fn mark_waitlist_notified(&self, figurine_id: Uuid) -> Result<u64> {
        Ok(sqlx::query(
            "DELETE FROM figurine_waitlist WHERE figurine_id = $1"
        )
        .bind(figurine_id)
        .execute(&self.pg_pool).await?
        .rows_affected())
    }

    // ── Message threads ────────────────────────────────────────

    pub async fn create_thread(
        &self,
        user_id: Uuid,
        category: &str,
        reference_id: Option<Uuid>,
        subject: &str,
        body: &str,
        from_admin: bool,
    ) -> Result<(crate::models::MessageThread, crate::models::ThreadMessage)> {
        let thread = sqlx::query_as::<_, crate::models::MessageThread>(
            "INSERT INTO message_threads (user_id, category, reference_id, subject)
             VALUES ($1, $2, $3, $4) RETURNING *"
        )
        .bind(user_id).bind(category).bind(reference_id).bind(subject)
        .fetch_one(&self.pg_pool).await?;

        let msg = sqlx::query_as::<_, crate::models::ThreadMessage>(
            "INSERT INTO thread_messages (thread_id, from_admin, body)
             VALUES ($1, $2, $3) RETURNING *"
        )
        .bind(thread.id).bind(from_admin).bind(body)
        .fetch_one(&self.pg_pool).await?;

        Ok((thread, msg))
    }

    pub async fn add_thread_reply(
        &self,
        thread_id: Uuid,
        _user_id: Uuid,
        from_admin: bool,
        body: &str,
    ) -> Result<crate::models::ThreadMessage> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM message_threads WHERE id = $1)"
        )
        .bind(thread_id)
        .fetch_one(&self.pg_pool).await?;

        if !exists {
            return Err(crate::error::AppError::NotFound(format!("Thread {} not found", thread_id)));
        }

        let msg = sqlx::query_as::<_, crate::models::ThreadMessage>(
            "INSERT INTO thread_messages (thread_id, from_admin, body)
             VALUES ($1, $2, $3) RETURNING *"
        )
        .bind(thread_id).bind(from_admin).bind(body)
        .fetch_one(&self.pg_pool).await?;

        sqlx::query(
            "UPDATE message_threads SET last_message_at = NOW(), status = 'open' WHERE id = $1"
        )
        .bind(thread_id)
        .execute(&self.pg_pool).await?;

        Ok(msg)
    }

    pub async fn get_user_threads(&self, user_id: Uuid) -> Result<Vec<(crate::models::MessageThread, i64, Option<String>)>> {
        let rows: Vec<(Uuid, Uuid, String, Option<Uuid>, String, String, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>, i64, Option<String>)> = sqlx::query_as(
            r#"SELECT
                t.id, t.user_id, t.category, t.reference_id, t.subject,
                t.status, t.created_at, t.last_message_at,
                COUNT(m.id) FILTER (WHERE m.read_at IS NULL AND m.from_admin = true)::bigint AS unread,
                (SELECT body FROM thread_messages WHERE thread_id = t.id ORDER BY created_at DESC LIMIT 1) AS preview
            FROM message_threads t
            LEFT JOIN thread_messages m ON m.thread_id = t.id
            WHERE t.user_id = $1
            GROUP BY t.id
            ORDER BY t.last_message_at DESC"#
        )
        .bind(user_id)
        .fetch_all(&self.pg_pool).await?;

        Ok(rows.into_iter().map(|(id, user_id, category, reference_id, subject, status, created_at, last_message_at, unread, preview)| {
            let thread = crate::models::MessageThread { id, user_id, category, reference_id, subject, status, created_at, last_message_at };
            (thread, unread, preview)
        }).collect())
    }

    pub async fn get_thread_messages(&self, thread_id: Uuid, user_id: Option<Uuid>) -> Result<(crate::models::MessageThread, Vec<crate::models::ThreadMessage>)> {
        let thread = sqlx::query_as::<_, crate::models::MessageThread>(
            "SELECT * FROM message_threads WHERE id = $1"
        )
        .bind(thread_id)
        .fetch_optional(&self.pg_pool).await?
        .ok_or_else(|| crate::error::AppError::NotFound(format!("Thread {} not found", thread_id)))?;

        if let Some(uid) = user_id {
            if thread.user_id != uid {
                return Err(crate::error::AppError::Unauthorized);
            }
        }

        let messages = sqlx::query_as::<_, crate::models::ThreadMessage>(
            "SELECT * FROM thread_messages WHERE thread_id = $1 ORDER BY created_at ASC"
        )
        .bind(thread_id)
        .fetch_all(&self.pg_pool).await?;

        Ok((thread, messages))
    }

    pub async fn mark_thread_read(&self, thread_id: Uuid, user_id: Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE thread_messages SET read_at = NOW()
             WHERE thread_id = $1 AND from_admin = true AND read_at IS NULL
             AND EXISTS (SELECT 1 FROM message_threads WHERE id = $1 AND user_id = $2)"
        )
        .bind(thread_id).bind(user_id)
        .execute(&self.pg_pool).await?;
        Ok(())
    }

    pub async fn mark_thread_read_admin(&self, thread_id: Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE thread_messages SET read_at = NOW()
             WHERE thread_id = $1 AND from_admin = false AND read_at IS NULL"
        )
        .bind(thread_id)
        .execute(&self.pg_pool).await?;
        Ok(())
    }

    pub async fn resolve_thread(&self, thread_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE message_threads SET status = 'resolved' WHERE id = $1")
            .bind(thread_id)
            .execute(&self.pg_pool).await?;
        Ok(())
    }

    pub async fn reopen_thread(&self, thread_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE message_threads SET status = 'open' WHERE id = $1")
            .bind(thread_id)
            .execute(&self.pg_pool).await?;
        Ok(())
    }

    pub async fn count_unread_threads(&self, user_id: Uuid) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(DISTINCT t.id)
               FROM message_threads t
               JOIN thread_messages m ON m.thread_id = t.id
               WHERE t.user_id = $1 AND m.from_admin = true AND m.read_at IS NULL"#
        )
        .bind(user_id)
        .fetch_one(&self.pg_pool).await?;
        Ok(row.0)
    }

    pub async fn admin_get_threads(
        &self,
        category: Option<&str>,
        status: Option<&str>,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<(crate::models::MessageThread, crate::models::User, i64, Option<String>)>, i64)> {
        let offset = (page - 1) * per_page;

        let mut where_parts: Vec<String> = Vec::new();
        if let Some(c) = category { where_parts.push(format!("t.category = '{}'", c.replace('\'', "''"))); }
        if let Some(s) = status   { where_parts.push(format!("t.status = '{}'", s.replace('\'', "''"))); }
        let where_clause = if where_parts.is_empty() { String::new() } else { format!("WHERE {}", where_parts.join(" AND ")) };

        let (total,): (i64,) = sqlx::query_as(&format!(
            "SELECT COUNT(DISTINCT t.id) FROM message_threads t {}",
            where_clause
        ))
        .fetch_one(&self.pg_pool).await?;

        let rows = sqlx::query(&format!(
            r#"SELECT
                t.id as thread_id, t.user_id, t.category, t.reference_id, t.subject,
                t.status, t.created_at as thread_created_at, t.last_message_at,
                u.id as u_id, u.email, u.display_name, u.visual_password_hash,
                u.admin_notes, u.is_blocked, u.password_reset_token, u.password_reset_expires_at,
                u.created_at as u_created_at, u.avatar_url,
                COUNT(m.id) FILTER (WHERE m.read_at IS NULL AND m.from_admin = false)::bigint AS unread,
                (SELECT body FROM thread_messages WHERE thread_id = t.id ORDER BY created_at DESC LIMIT 1) AS preview
            FROM message_threads t
            JOIN users u ON u.id = t.user_id
            LEFT JOIN thread_messages m ON m.thread_id = t.id
            {}
            GROUP BY t.id, u.id
            ORDER BY t.last_message_at DESC
            LIMIT $1 OFFSET $2"#,
            where_clause
        ))
        .bind(per_page).bind(offset)
        .fetch_all(&self.pg_pool).await?;

        use sqlx::Row;
        let items = rows.into_iter().map(|r| {
            let thread = crate::models::MessageThread {
                id: r.get("thread_id"),
                user_id: r.get("user_id"),
                category: r.get("category"),
                reference_id: r.get("reference_id"),
                subject: r.get("subject"),
                status: r.get("status"),
                created_at: r.get("thread_created_at"),
                last_message_at: r.get("last_message_at"),
            };
            let user = crate::models::User {
                id: r.get("u_id"),
                email: r.get("email"),
                display_name: r.get("display_name"),
                visual_password_hash: r.get("visual_password_hash"),
                admin_notes: r.get("admin_notes"),
                is_blocked: r.get("is_blocked"),
                password_reset_token: r.get("password_reset_token"),
                password_reset_expires_at: r.get("password_reset_expires_at"),
                created_at: r.get("u_created_at"),
                avatar_url: r.get("avatar_url"),
            };
            let unread: i64 = r.get("unread");
            let preview: Option<String> = r.get("preview");
            (thread, user, unread, preview)
        }).collect();

        Ok((items, total))
    }

    pub async fn get_user_threads_for_admin(&self, user_id: Uuid) -> Result<Vec<(crate::models::MessageThread, i64, Option<String>)>> {
        let rows: Vec<(Uuid, Uuid, String, Option<Uuid>, String, String, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>, i64, Option<String>)> = sqlx::query_as(
            r#"SELECT
                t.id, t.user_id, t.category, t.reference_id, t.subject,
                t.status, t.created_at, t.last_message_at,
                COUNT(m.id) FILTER (WHERE m.read_at IS NULL AND m.from_admin = false)::bigint AS unread,
                (SELECT body FROM thread_messages WHERE thread_id = t.id ORDER BY created_at DESC LIMIT 1) AS preview
            FROM message_threads t
            LEFT JOIN thread_messages m ON m.thread_id = t.id
            WHERE t.user_id = $1
            GROUP BY t.id
            ORDER BY t.last_message_at DESC"#
        )
        .bind(user_id)
        .fetch_all(&self.pg_pool).await?;

        Ok(rows.into_iter().map(|(id, user_id, category, reference_id, subject, status, created_at, last_message_at, unread, preview)| {
            let thread = crate::models::MessageThread { id, user_id, category, reference_id, subject, status, created_at, last_message_at };
            (thread, unread, preview)
        }).collect())
    }
}
