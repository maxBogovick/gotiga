use sqlx::PgPool;
use uuid::Uuid;
use crate::error::Result;
use crate::models::*;

#[derive(Clone)]
pub struct Repository {
    pool: PgPool,
}

impl Repository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_pool(&self) -> &PgPool {
        &self.pool
    }

    // === FIGURINES ===

    pub async fn get_all_figurines(&self, visible_only: bool) -> Result<Vec<Figurine>> {
        let mut query = "SELECT * FROM figurines".to_string();
        if visible_only {
            query.push_str(" WHERE is_visible = true");
        }
        query.push_str(" ORDER BY sort_order");

        let figurines = sqlx::query_as::<_, Figurine>(&query)
            .fetch_all(&self.pool)
            .await?;

        Ok(figurines)
    }

    pub async fn get_figurine_by_id(&self, id: Uuid) -> Result<Option<Figurine>> {
        let figurine = sqlx::query_as::<_, Figurine>("SELECT * FROM figurines WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(figurine)
    }

    pub async fn upsert_figurine(&self, f: &Figurine) -> Result<Figurine> {
        let rec = sqlx::query_as::<_, Figurine>(
            r#"
            INSERT INTO figurines (
                id, name, short_text, full_description, dimensions, material, technique, 
                year, ambience_path, video_url, secret_text, status, sort_order, is_visible,
                updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, NOW())
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                short_text = EXCLUDED.short_text,
                full_description = EXCLUDED.full_description,
                dimensions = EXCLUDED.dimensions,
                material = EXCLUDED.material,
                technique = EXCLUDED.technique,
                year = EXCLUDED.year,
                ambience_path = EXCLUDED.ambience_path,
                video_url = EXCLUDED.video_url,
                secret_text = EXCLUDED.secret_text,
                status = EXCLUDED.status,
                sort_order = EXCLUDED.sort_order,
                is_visible = EXCLUDED.is_visible,
                updated_at = NOW()
            RETURNING *
            "#
        )
        .bind(f.id)
        .bind(&f.name)
        .bind(&f.short_text)
        .bind(&f.full_description)
        .bind(&f.dimensions)
        .bind(&f.material)
        .bind(&f.technique)
        .bind(f.year)
        .bind(&f.ambience_path)
        .bind(&f.video_url)
        .bind(&f.secret_text)
        .bind(&f.status)
        .bind(f.sort_order)
        .bind(f.is_visible)
        .fetch_one(&self.pool)
        .await?;

        Ok(rec)
    }

    pub async fn delete_figurine(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM figurines WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // === IMAGES ===

    pub async fn get_images_by_figurine(&self, figurine_id: Uuid) -> Result<Vec<Image>> {
        let images = sqlx::query_as::<_, Image>(
            "SELECT * FROM images WHERE figurine_id = $1 ORDER BY sort_order"
        )
        .bind(figurine_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(images)
    }

    pub async fn replace_images(&self, figurine_id: Uuid, images: Vec<Image>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Delete existing
        sqlx::query("DELETE FROM images WHERE figurine_id = $1")
            .bind(figurine_id)
            .execute(&mut *tx)
            .await?;

        // Insert new
        for img in images {
            sqlx::query(
                r#"
                INSERT INTO images (id, figurine_id, image_type, file_path, alt_text, sort_order)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#
            )
            .bind(img.id)
            .bind(figurine_id)
            .bind(img.image_type)
            .bind(img.file_path)
            .bind(img.alt_text)
            .bind(img.sort_order)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

     // === PROCESS STEPS ===

    pub async fn get_steps_by_figurine(&self, figurine_id: Uuid) -> Result<Vec<ProcessStep>> {
        let steps = sqlx::query_as::<_, ProcessStep>(
            "SELECT * FROM process_steps WHERE figurine_id = $1 ORDER BY sort_order"
        )
        .bind(figurine_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(steps)
    }

    pub async fn replace_steps(&self, figurine_id: Uuid, steps: Vec<ProcessStep>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM process_steps WHERE figurine_id = $1")
            .bind(figurine_id)
            .execute(&mut *tx)
            .await?;

        for step in steps {
            sqlx::query(
                r#"
                INSERT INTO process_steps (id, figurine_id, step_type, description, image_path, sort_order)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#
            )
            .bind(step.id)
            .bind(figurine_id)
            .bind(step.step_type)
            .bind(step.description)
            .bind(step.image_path)
            .bind(step.sort_order)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    // === RELATED ITEMS ===

    pub async fn get_related_figurines(&self, current_id: Uuid) -> Result<Vec<Figurine>> {
        // Fetch current to get year/material
        let current = match self.get_figurine_by_id(current_id).await? {
            Some(c) => c,
            None => return Ok(vec![]),
        };

        // Logic: Same Year OR Material contains substring(0,4)
        // PostgreSQL: SUBSTRING(material, 1, 4)
        let material_hint = current.material.as_deref().map(|m| {
             if m.len() >= 4 { &m[0..4] } else { m }
        }).unwrap_or("");

        let query = r#"
            SELECT * FROM figurines
            WHERE id != $1
            AND is_visible = true
            AND (
                year = $2
                OR ($3 != '' AND material ILIKE '%' || $3 || '%')
            )
            ORDER BY RANDOM()
            LIMIT 3
        "#;

        let related = sqlx::query_as::<_, Figurine>(query)
            .bind(current_id)
            .bind(current.year)
            .bind(material_hint)
            .fetch_all(&self.pool)
            .await?;

        Ok(related)
    }

    // === CONTENT & ZONES ===

    pub async fn get_texts_by_category(&self, category: TextCategory) -> Result<Vec<Text>> {
        let texts = sqlx::query_as::<_, Text>(
            "SELECT * FROM texts WHERE category = $1 ORDER BY sort_order"
        )
        .bind(category)
        .fetch_all(&self.pool)
        .await?;
        Ok(texts)
    }

    pub async fn get_zones(&self) -> Result<Vec<CabinetZone>> {
        let zones = sqlx::query_as::<_, CabinetZone>(
            "SELECT * FROM cabinet_zones ORDER BY sort_order"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(zones)
    }

    // === MANIFEST DATA ===

    pub async fn get_all_images(&self) -> Result<Vec<Image>> {
        let images = sqlx::query_as::<_, Image>("SELECT * FROM images").fetch_all(&self.pool).await?;
        Ok(images)
    }

    pub async fn get_all_steps(&self) -> Result<Vec<ProcessStep>> {
        let steps = sqlx::query_as::<_, ProcessStep>("SELECT * FROM process_steps").fetch_all(&self.pool).await?;
        Ok(steps)
    }

    // === FULL RELEASE OVERWRITE ===

    pub async fn replace_full_state(
        &self,
        figurines: Vec<Figurine>,
        images: Vec<Image>,
        steps: Vec<ProcessStep>,
        texts: Vec<Text>,
        zones: Vec<CabinetZone>
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // 1. Truncate (Cascading deletes handles children)
        sqlx::query("TRUNCATE TABLE images, process_steps, texts, cabinet_zones, figurines CASCADE")
            .execute(&mut *tx).await?;

        // 2. Insert Figurines
        for f in figurines {
            sqlx::query(
                r#"INSERT INTO figurines (
                    id, name, short_text, full_description, dimensions, material, technique, 
                    year, ambience_path, video_url, secret_text, status, sort_order, is_visible,
                    created_at, updated_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, NOW(), NOW())"#
            )
            .bind(f.id).bind(&f.name).bind(&f.short_text).bind(&f.full_description)
            .bind(&f.dimensions).bind(&f.material).bind(&f.technique).bind(f.year)
            .bind(&f.ambience_path).bind(&f.video_url).bind(&f.secret_text)
            .bind(f.status).bind(f.sort_order).bind(f.is_visible)
            .execute(&mut *tx).await?;
        }

        // 3. Insert Images
        for i in images {
            sqlx::query(
                r#"INSERT INTO images (id, figurine_id, image_type, file_path, alt_text, sort_order, created_at, updated_at)
                   VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"#
            )
            .bind(i.id).bind(i.figurine_id).bind(i.image_type).bind(i.file_path).bind(i.alt_text).bind(i.sort_order)
            .execute(&mut *tx).await?;
        }

        // 4. Insert Steps
        for s in steps {
            sqlx::query(
                r#"INSERT INTO process_steps (id, figurine_id, step_type, description, image_path, sort_order, created_at, updated_at)
                   VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"#
            )
            .bind(s.id).bind(s.figurine_id).bind(s.step_type).bind(s.description).bind(s.image_path).bind(s.sort_order)
            .execute(&mut *tx).await?;
        }

        // 5. Insert Texts
        for t in texts {
            sqlx::query(
                r#"INSERT INTO texts (id, category, content, caption, image_path, sort_order, created_at, updated_at)
                   VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"#
            )
            .bind(t.id).bind(t.category).bind(t.content).bind(t.caption).bind(t.image_path).bind(t.sort_order)
            .execute(&mut *tx).await?;
        }

        // 6. Insert Zones
        for z in zones {
            sqlx::query(
                r#"INSERT INTO cabinet_zones (id, zone_type, x_percent, y_percent, width_percent, height_percent, target_route, sort_order)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#
            )
            .bind(z.id).bind(z.zone_type).bind(z.x_percent).bind(z.y_percent).bind(z.width_percent).bind(z.height_percent).bind(z.target_route).bind(z.sort_order)
            .execute(&mut *tx).await?;
        }

        tx.commit().await?;
        Ok(())
    }
}
