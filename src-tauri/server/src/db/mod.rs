use sqlx::{PgPool, SqlitePool};
use uuid::Uuid;
use crate::error::{Result, AppError};
use crate::models::*;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct Repository {
    pg_pool: PgPool,
    content_pool: Arc<RwLock<Option<SqlitePool>>>,
}

impl Repository {
    pub fn new(pg_pool: PgPool) -> Self {
        Self {
            pg_pool,
            content_pool: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_content_pool(&self, pool: SqlitePool) {
        let mut lock = self.content_pool.write().await;
        *lock = Some(pool);
    }

    pub async fn load_content_pool(&self, path: &str) -> Result<()> {
        let opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(false)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

        let pool = SqlitePool::connect_with(opts).await?;
        // Enable foreign keys
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await?;
        ensure_sqlite_column(&pool, "images", "original_path", "TEXT").await?;
        ensure_sqlite_column(&pool, "images", "thumb_path", "TEXT").await?;
        ensure_sqlite_column(&pool, "images", "original_data", "BLOB").await?;
        ensure_sqlite_column(&pool, "images", "thumb_data", "BLOB").await?;
        self.set_content_pool(pool).await;
        Ok(())
    }

    // Helper to get content pool or error
    async fn content(&self) -> Result<SqlitePool> {
        let lock = self.content_pool.read().await;
        lock.clone().ok_or_else(|| AppError::Internal("No active content database loaded".to_string()))
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
    
            // === CONTENT (SQLite) ===
    
        
    
            pub async fn get_all_figurines(&self, visible_only: bool) -> Result<Vec<Figurine>> {
    
                let pool = self.content().await?;
    
                let mut query = "SELECT * FROM figurines".to_string();
    
                if visible_only {
    
                    query.push_str(" WHERE is_visible = 1");
    
                }
    
                query.push_str(" ORDER BY sort_order");
    
        
    
                let figurines = sqlx::query_as::<_, Figurine>(&query)
    
                    .fetch_all(&pool)
    
                    .await?;
    
        
    
                Ok(figurines)
    
            }
    
        
    
            pub async fn get_figurine_by_id(&self, id: String) -> Result<Option<Figurine>> {
    
                let pool = self.content().await?;
    
                let figurine = sqlx::query_as::<_, Figurine>("SELECT * FROM figurines WHERE id = ?")
    
                    .bind(id)
    
                    .fetch_optional(&pool)
    
                    .await?;
    
                Ok(figurine)
    
            }
    
        
    
            pub async fn get_images_by_figurine(&self, figurine_id: String) -> Result<Vec<Image>> {
    
                let pool = self.content().await?;
    
                let images = sqlx::query_as::<_, Image>(
    
                    "SELECT * FROM images WHERE figurine_id = ? ORDER BY sort_order"
    
                )
    
                .bind(figurine_id)
    
                .fetch_all(&pool)
    
                .await?;
    
                Ok(images)
    
            }
    
        
    
            pub async fn get_steps_by_figurine(&self, figurine_id: String) -> Result<Vec<ProcessStep>> {
    
                let pool = self.content().await?;
    
                let steps = sqlx::query_as::<_, ProcessStep>(
    
                    "SELECT * FROM process_steps WHERE figurine_id = ? ORDER BY sort_order"
    
                )
    
                .bind(figurine_id)
    
                .fetch_all(&pool)
    
                .await?;
    
                Ok(steps)
    
            }
    
        
    
            pub async fn get_related_figurines(&self, current_id: String) -> Result<Vec<Figurine>> {
    
                let pool = self.content().await?;
    
                let current = match self.get_figurine_by_id(current_id.clone()).await? {
    
                    Some(c) => c,
    
                    None => return Ok(vec![]),
    
                };
    
        
    
                let material_hint = current.material.as_deref().map(|m| {
    
                     if m.len() >= 4 { &m[0..4] } else { m }
    
                }).unwrap_or("");
    
        
    
                // SQLite random is RANDOM()
    
                let query = r#"
    
                    SELECT * FROM figurines
    
                    WHERE id != ?
    
                    AND is_visible = 1
    
                    AND (
    
                        year = ?
    
                        OR (? != '' AND material LIKE '%' || ? || '%')
    
                    )
    
                    ORDER BY RANDOM()
    
                    LIMIT 3
    
                "#;
    
        
    
                let related = sqlx::query_as::<_, Figurine>(query)
    
                    .bind(current_id)
    
                    .bind(current.year)
    
                    .bind(material_hint)
    
                    .bind(material_hint)
    
                    .fetch_all(&pool)
    
                    .await?;
    
        
    
                Ok(related)
    
            }
    
        
    
            pub async fn get_texts_by_category(&self, category: TextCategory) -> Result<Vec<Text>> {
    
                let pool = self.content().await?;
    
                let texts = sqlx::query_as::<_, Text>(
    
                    "SELECT * FROM texts WHERE category = ? ORDER BY sort_order"
    
                )
    
                .bind(category)
    
                .fetch_all(&pool)
    
                .await?;
    
                Ok(texts)
    
            }
    
        
    
            pub async fn get_zones(&self) -> Result<Vec<CabinetZone>> {
    
                let pool = self.content().await?;
    
                let zones = sqlx::query_as::<_, CabinetZone>(
    
                    "SELECT * FROM cabinet_zones ORDER BY sort_order"
    
                )
    
                .fetch_all(&pool)
    
                .await?;
    
                Ok(zones)
    
            }
    
        
    
            // === ADMIN WRITE OPERATIONS ===

            pub async fn upsert_figurine(&self, f: &crate::models::SaveFigurineRequest) -> Result<()> {
                let pool = self.content().await?;
                sqlx::query(
                    "INSERT INTO figurines (id, name, short_text, full_description, dimensions, material, technique, year, ambience_path, video_url, secret_text, is_visible, status, sort_order, updated_at)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
                     ON CONFLICT(id) DO UPDATE SET
                       name=excluded.name, short_text=excluded.short_text, full_description=excluded.full_description,
                       dimensions=excluded.dimensions, material=excluded.material, technique=excluded.technique,
                       year=excluded.year, ambience_path=excluded.ambience_path, video_url=excluded.video_url,
                       secret_text=excluded.secret_text, is_visible=excluded.is_visible, status=excluded.status,
                       sort_order=excluded.sort_order, updated_at=datetime('now')"
                )
                .bind(&f.id).bind(&f.name).bind(&f.short_text).bind(&f.full_description)
                .bind(&f.dimensions).bind(&f.material).bind(&f.technique).bind(f.year)
                .bind(&f.ambience_path).bind(&f.video_url).bind(&f.secret_text)
                .bind(f.is_visible).bind(&f.status).bind(f.sort_order)
                .execute(&pool).await?;
                Ok(())
            }

            pub async fn delete_figurine(&self, id: &str) -> Result<()> {
                let pool = self.content().await?;
                sqlx::query("DELETE FROM figurines WHERE id = ?")
                    .bind(id).execute(&pool).await?;
                Ok(())
            }

            pub async fn replace_images(&self, figurine_id: &str, images: &[crate::models::SaveImageRequest]) -> Result<()> {
                let pool = self.content().await?;
                sqlx::query("DELETE FROM images WHERE figurine_id = ?")
                    .bind(figurine_id).execute(&pool).await?;
                for (idx, img) in images.iter().enumerate() {
                    let sort = img.sort_order.unwrap_or(idx as i32);
                    sqlx::query(
                        "INSERT INTO images (id, figurine_id, image_type, file_path, original_path, thumb_path, alt_text, sort_order) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
                    )
                    .bind(&img.id).bind(figurine_id).bind(&img.image_type)
                    .bind(&img.url).bind(&img.original_url).bind(&img.thumb_url)
                    .bind(&img.alt_text).bind(sort)
                    .execute(&pool).await?;
                }
                Ok(())
            }

            pub async fn replace_steps(&self, figurine_id: &str, steps: &[crate::models::SaveStepRequest]) -> Result<()> {
                let pool = self.content().await?;
                sqlx::query("DELETE FROM process_steps WHERE figurine_id = ?")
                    .bind(figurine_id).execute(&pool).await?;
                for (idx, step) in steps.iter().enumerate() {
                    let sort = step.sort_order.unwrap_or(idx as i32);
                    sqlx::query(
                        "INSERT INTO process_steps (id, figurine_id, step_type, description, image_path, sort_order) VALUES (?, ?, ?, ?, ?, ?)"
                    )
                    .bind(&step.id).bind(figurine_id).bind(&step.step_type)
                    .bind(&step.description).bind(&step.image_url).bind(sort)
                    .execute(&pool).await?;
                }
                Ok(())
            }

            pub async fn upsert_zone(&self, z: &crate::models::SaveZoneRequest, sort_order: i32) -> Result<()> {
                let pool = self.content().await?;
                sqlx::query(
                    "INSERT INTO cabinet_zones (id, zone_type, x_percent, y_percent, width_percent, height_percent, target_route, sort_order)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                     ON CONFLICT(id) DO UPDATE SET
                       zone_type=excluded.zone_type, x_percent=excluded.x_percent, y_percent=excluded.y_percent,
                       width_percent=excluded.width_percent, height_percent=excluded.height_percent,
                       target_route=excluded.target_route, sort_order=excluded.sort_order"
                )
                .bind(&z.id).bind(&z.zone_type).bind(z.x).bind(z.y)
                .bind(z.width).bind(z.height).bind(&z.target_route).bind(sort_order)
                .execute(&pool).await?;
                Ok(())
            }

            pub async fn delete_zone(&self, id: &str) -> Result<()> {
                let pool = self.content().await?;
                sqlx::query("DELETE FROM cabinet_zones WHERE id = ?")
                    .bind(id).execute(&pool).await?;
                Ok(())
            }

            pub async fn get_zone_count(&self) -> Result<i32> {
                let pool = self.content().await?;
                let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cabinet_zones")
                    .fetch_one(&pool).await?;
                Ok(row.0 as i32)
            }

            pub async fn upsert_text(&self, t: &crate::models::SaveTextRequest, category: &crate::models::TextCategory) -> Result<()> {
                let pool = self.content().await?;
                sqlx::query(
                    "INSERT INTO texts (id, category, content, caption, image_path, sort_order, updated_at)
                     VALUES (?, ?, ?, ?, ?, COALESCE((SELECT sort_order FROM texts WHERE id = ?), (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM texts WHERE category = ?)), datetime('now'))
                     ON CONFLICT(id) DO UPDATE SET
                       content=excluded.content, caption=excluded.caption,
                       image_path=excluded.image_path, updated_at=datetime('now')"
                )
                .bind(&t.id).bind(category).bind(&t.content).bind(&t.caption)
                .bind(&t.image_url).bind(&t.id).bind(category)
                .execute(&pool).await?;
                Ok(())
            }

            pub async fn delete_text(&self, id: &str) -> Result<()> {
                let pool = self.content().await?;
                sqlx::query("DELETE FROM texts WHERE id = ?")
                    .bind(id).execute(&pool).await?;
                Ok(())
            }

            pub async fn get_main_background(&self) -> Result<Option<String>> {
                let pool = self.content().await?;
                let row: Option<(String,)> = sqlx::query_as(
                    "SELECT file_path FROM app_resources WHERE key = 'main_background'"
                )
                .fetch_optional(&pool).await?;
                Ok(row.map(|r| r.0))
            }

            pub async fn set_main_background(&self, url: &str) -> Result<()> {
                let pool = self.content().await?;
                sqlx::query(
                    "INSERT INTO app_resources (key, file_path, updated_at) VALUES ('main_background', ?, datetime('now'))
                     ON CONFLICT(key) DO UPDATE SET file_path=excluded.file_path, updated_at=datetime('now')"
                )
                .bind(url).execute(&pool).await?;
                Ok(())
            }

            pub async fn get_home_content(&self) -> Result<Option<crate::models::HomeContent>> {
                let pool = self.content().await?;
                let row: Option<(String,)> = sqlx::query_as(
                    "SELECT file_path FROM app_resources WHERE key = 'home_content'"
                )
                .fetch_optional(&pool).await?;
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
                let pool = self.content().await?;
                let json = serde_json::to_string(content)
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                sqlx::query(
                    "INSERT INTO app_resources (key, file_path, updated_at) VALUES ('home_content', ?, datetime('now'))
                     ON CONFLICT(key) DO UPDATE SET file_path=excluded.file_path, updated_at=datetime('now')"
                )
                .bind(json).execute(&pool).await?;
                Ok(())
            }

            pub async fn get_author_profile(&self) -> Result<Option<crate::models::AuthorProfile>> {
                let pool = self.content().await?;
                let row: Option<(String,)> = sqlx::query_as(
                    "SELECT file_path FROM app_resources WHERE key = 'author_profile'"
                )
                .fetch_optional(&pool).await?;
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
                let pool = self.content().await?;
                let json = serde_json::to_string(profile)
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                sqlx::query(
                    "INSERT INTO app_resources (key, file_path, updated_at) VALUES ('author_profile', ?, datetime('now'))
                     ON CONFLICT(key) DO UPDATE SET file_path=excluded.file_path, updated_at=datetime('now')"
                )
                .bind(json).execute(&pool).await?;
                Ok(())
            }

            // === MEDIA STREAMING ===
    
            
    
            // Generic blob fetcher
    
            pub async fn get_blob(&self, table: &str, column: &str, id: String) -> Result<Option<Vec<u8>>> {
                let pool = self.content().await?;

                // app_resources uses `key` not `id` — handle separately
                if table == "app_resources" && column == "data" {
                    let row: Option<(Vec<u8>,)> = sqlx::query_as(
                        "SELECT data FROM app_resources WHERE key = ?"
                    ).bind(&id).fetch_optional(&pool).await?;
                    return Ok(row.map(|r| r.0));
                }

                // Validate table/column to prevent SQL injection (since we format the query string)
                match (table, column) {
                    ("images", "data") |
                    ("images", "original_data") |
                    ("images", "thumb_data") |
                    ("process_steps", "image_data") |
                    ("figurines", "video_data") |
                    ("figurines", "ambience_data") |
                    ("texts", "image_data") => {},
                    _ => return Err(AppError::BadRequest("Invalid media target".to_string())),
                }

                let query = format!("SELECT {} FROM {} WHERE id = ?", column, table);
                let row: Option<(Vec<u8>,)> = sqlx::query_as(&query)
                    .bind(id)
                    .fetch_optional(&pool)
                    .await?;
                Ok(row.map(|r| r.0))
            }

            pub async fn get_media_usages(&self) -> Result<Vec<MediaUsageDto>> {
                let pool = self.content().await?;
                let mut usages = Vec::new();

                let image_rows: Vec<(String, String, Option<String>, Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
                    "SELECT i.id, i.file_path, i.original_path, i.thumb_path, f.id, f.name
                     FROM images i
                     LEFT JOIN figurines f ON f.id = i.figurine_id"
                ).fetch_all(&pool).await?;
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
                    "SELECT ps.id, ps.image_path, f.id, f.name
                     FROM process_steps ps
                     LEFT JOIN figurines f ON f.id = ps.figurine_id"
                ).fetch_all(&pool).await?;
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
                    "SELECT id, category, caption, image_path FROM texts WHERE image_path IS NOT NULL"
                ).fetch_all(&pool).await?;
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
                    "SELECT id, name, ambience_path, video_url FROM figurines"
                ).fetch_all(&pool).await?;
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
                ).fetch_all(&pool).await?;
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

            pub async fn replace_media_path_everywhere(
                &self,
                old_path: &str,
                new_preview_path: &str,
                new_original_path: Option<&str>,
                new_thumb_path: Option<&str>,
            ) -> Result<usize> {
                let pool = self.content().await?;
                let mut updated = 0usize;

                if new_preview_path.starts_with("images/preview/") || new_preview_path.starts_with("/static/images/preview/") {
                    let result = sqlx::query(
                        "UPDATE images
                         SET file_path = ?, original_path = ?, thumb_path = ?
                         WHERE file_path = ? OR original_path = ? OR thumb_path = ?"
                    )
                    .bind(new_preview_path)
                    .bind(new_original_path)
                    .bind(new_thumb_path)
                    .bind(old_path)
                    .bind(old_path)
                    .bind(old_path)
                    .execute(&pool).await?;
                    updated += result.rows_affected() as usize;
                } else {
                    for (table, column) in [
                        ("images", "file_path"),
                        ("images", "original_path"),
                        ("images", "thumb_path"),
                    ] {
                        let query = format!("UPDATE {} SET {} = ? WHERE {} = ?", table, column, column);
                        let result = sqlx::query(&query)
                            .bind(new_preview_path)
                            .bind(old_path)
                            .execute(&pool).await?;
                        updated += result.rows_affected() as usize;
                    }
                }

                for (table, column) in [
                    ("process_steps", "image_path"),
                    ("texts", "image_path"),
                    ("figurines", "ambience_path"),
                    ("figurines", "video_url"),
                    ("app_resources", "file_path"),
                ] {
                    let extra = if table == "app_resources" { " AND key NOT IN ('author_profile', 'home_content')" } else { "" };
                    let query = format!("UPDATE {} SET {} = ? WHERE {} = ?{}", table, column, column, extra);
                    let result = sqlx::query(&query)
                        .bind(new_preview_path)
                        .bind(old_path)
                        .execute(&pool).await?;
                    updated += result.rows_affected() as usize;
                }

                Ok(updated)
            }
}

async fn ensure_sqlite_column(
    pool: &SqlitePool,
    table: &str,
    column: &str,
    column_type: &str,
) -> Result<()> {
    let rows: Vec<(String,)> = sqlx::query_as(&format!("SELECT name FROM pragma_table_info('{}')", table))
        .fetch_all(pool)
        .await?;

    if rows.iter().any(|(name,)| name == column) {
        return Ok(());
    }

    sqlx::query(&format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, column_type))
        .execute(pool)
        .await?;
    Ok(())
}
