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
            .read_only(true);

        let pool = SqlitePool::connect_with(opts).await?;
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
    
        
    
            // === MEDIA STREAMING ===
    
            
    
            // Generic blob fetcher
    
            pub async fn get_blob(&self, table: &str, column: &str, id: String) -> Result<Option<Vec<u8>>> {
    
                let pool = self.content().await?;
    
                // Validate table/column to prevent SQL injection (since we format string)
    
                match (table, column) {
    
                    ("images", "data") |
    
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
}
