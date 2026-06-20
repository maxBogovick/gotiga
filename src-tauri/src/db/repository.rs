use crate::models::*;
use chrono::{TimeZone, Utc};
use rusqlite::{params, Connection, Result};
use std::collections::HashSet;

fn get_iso_date(row: &rusqlite::Row, index: usize) -> Result<String> {
    use rusqlite::types::ValueRef;
    match row.get_ref(index)? {
        ValueRef::Integer(i) => {
            let dt = Utc.timestamp_opt(i, 0).unwrap();
            Ok(dt.to_rfc3339())
        }
        ValueRef::Text(s) => Ok(std::str::from_utf8(s).unwrap_or_default().to_string()),
        _ => Ok(Utc::now().to_rfc3339()),
    }
}

pub struct Repository<'a> {
    conn: &'a Connection,
}

impl<'a> Repository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    // === FIGURINES ===

    pub fn get_all_figurines(&self) -> Result<Vec<Figurine>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, short_text, full_description, dimensions, material, technique, year, passport_number, edition, created_period, care_instructions, provenance_note, authenticity_note, included_items, ambience_path, video_url, secret_text, status, sort_order, updated_at, is_visible, COALESCE(is_featured, 0)
             FROM figurines
             ORDER BY sort_order"
        )?;

        let iter = stmt.query_map([], |row| {
            Ok(Figurine {
                id: row.get(0)?,
                name: row.get(1)?,
                short_text: row.get(2)?,
                full_description: row.get(3)?,
                dimensions: row.get(4)?,
                material: row.get(5)?,
                technique: row.get(6)?,
                year: row.get(7)?,
                passport_number: row.get(8)?,
                edition: row.get(9)?,
                created_period: row.get(10)?,
                care_instructions: row.get(11)?,
                provenance_note: row.get(12)?,
                authenticity_note: row.get(13)?,
                included_items: row.get(14)?,
                ambience_path: row.get(15)?,
                video_url: row.get(16)?,
                secret_text: row.get(17)?,
                status: FigurineStatus::from_str(&row.get::<_, String>(18)?),
                sort_order: row.get(19)?,
                updated_at: get_iso_date(row, 20)?,
                is_visible: row.get(21)?,
                is_featured: row.get::<_, i32>(22).unwrap_or(0) != 0,
            })
        })?;

        iter.collect()
    }

    pub fn get_figurine_by_id(&self, id: &str) -> Result<Option<Figurine>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, short_text, full_description, dimensions, material, technique, year, passport_number, edition, created_period, care_instructions, provenance_note, authenticity_note, included_items, ambience_path, video_url, secret_text, status, sort_order, updated_at, is_visible, COALESCE(is_featured, 0)
             FROM figurines
             WHERE id = ?"
        )?;

        let mut rows = stmt.query(params![id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Figurine {
                id: row.get(0)?,
                name: row.get(1)?,
                short_text: row.get(2)?,
                full_description: row.get(3)?,
                dimensions: row.get(4)?,
                material: row.get(5)?,
                technique: row.get(6)?,
                year: row.get(7)?,
                passport_number: row.get(8)?,
                edition: row.get(9)?,
                created_period: row.get(10)?,
                care_instructions: row.get(11)?,
                provenance_note: row.get(12)?,
                authenticity_note: row.get(13)?,
                included_items: row.get(14)?,
                ambience_path: row.get(15)?,
                video_url: row.get(16)?,
                secret_text: row.get(17)?,
                status: FigurineStatus::from_str(&row.get::<_, String>(18)?),
                sort_order: row.get(19)?,
                updated_at: get_iso_date(row, 20)?,
                is_visible: row.get(21)?,
                is_featured: row.get::<_, i32>(22).unwrap_or(0) != 0,
            }))
        } else {
            Ok(None)
        }
    }

    // === WRITE OPERATIONS (SYNC) ===

    pub fn upsert_figurine(&self, f: &Figurine) -> Result<()> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO figurines (
                id, name, short_text, full_description, dimensions, material, technique,
                year, passport_number, edition, created_period, care_instructions, provenance_note, authenticity_note, included_items,
                ambience_path, video_url, secret_text, status, sort_order, is_visible, is_featured, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23)
            ON CONFLICT(id) DO UPDATE SET
                name=excluded.name,
                short_text=excluded.short_text,
                full_description=excluded.full_description,
                dimensions=excluded.dimensions,
                material=excluded.material,
                technique=excluded.technique,
                year=excluded.year,
                passport_number=excluded.passport_number,
                edition=excluded.edition,
                created_period=excluded.created_period,
                care_instructions=excluded.care_instructions,
                provenance_note=excluded.provenance_note,
                authenticity_note=excluded.authenticity_note,
                included_items=excluded.included_items,
                ambience_path=excluded.ambience_path,
                video_url=excluded.video_url,
                secret_text=excluded.secret_text,
                status=excluded.status,
                sort_order=excluded.sort_order,
                is_visible=excluded.is_visible,
                is_featured=excluded.is_featured,
                updated_at=excluded.updated_at"
        )?;

        stmt.execute(params![
            f.id,
            f.name,
            f.short_text,
            f.full_description,
            f.dimensions,
            f.material,
            f.technique,
            f.year,
            f.passport_number,
            f.edition,
            f.created_period,
            f.care_instructions,
            f.provenance_note,
            f.authenticity_note,
            f.included_items,
            f.ambience_path,
            f.video_url,
            f.secret_text,
            f.status.as_str(),
            f.sort_order,
            f.is_visible,
            f.is_featured as i32,
            f.updated_at
        ])?;

        Ok(())
    }

    pub fn delete_figurine(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM figurines WHERE id = ?", params![id])?;
        Ok(())
    }

    // === IMAGES ===

    pub fn replace_images(&self, figurine_id: &str, images: Vec<Image>) -> Result<()> {
        // Удаляем старые
        self.conn.execute(
            "DELETE FROM images WHERE figurine_id = ?",
            params![figurine_id],
        )?;

        // Вставляем новые
        let mut stmt = self.conn.prepare(
            "INSERT INTO images (id, figurine_id, image_type, file_path, original_path, thumb_path, alt_text, sort_order, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
        )?;

        for (i, img) in images.into_iter().enumerate() {
            stmt.execute(params![
                img.id,
                figurine_id,
                img.image_type.as_str(),
                img.file_path,
                img.original_path,
                img.thumb_path,
                img.alt_text,
                i as i32,
                img.updated_at // String
            ])?;
        }

        Ok(())
    }

    pub fn get_all_images(&self) -> Result<Vec<Image>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, figurine_id, image_type, file_path, original_path, thumb_path, alt_text, sort_order, updated_at FROM images"
        )?;
        let iter = stmt.query_map([], |row| {
            Ok(Image {
                id: row.get(0)?,
                figurine_id: row.get(1)?,
                image_type: ImageType::from_str(&row.get::<_, String>(2)?),
                file_path: row.get(3)?,
                original_path: row.get(4)?,
                thumb_path: row.get(5)?,
                alt_text: row.get(6)?,
                sort_order: row.get(7)?,
                updated_at: get_iso_date(row, 8)?,
            })
        })?;
        iter.collect()
    }

    pub fn get_all_media_paths(&self) -> Result<HashSet<String>> {
        let mut paths = HashSet::new();

        let mut stmt = self.conn.prepare("SELECT file_path FROM images")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for row in rows {
            paths.insert(row?);
        }

        let mut stmt = self
            .conn
            .prepare("SELECT original_path FROM images WHERE original_path IS NOT NULL")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for row in rows {
            paths.insert(row?);
        }

        let mut stmt = self
            .conn
            .prepare("SELECT thumb_path FROM images WHERE thumb_path IS NOT NULL")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for row in rows {
            paths.insert(row?);
        }

        let mut stmt = self.conn.prepare("SELECT image_path FROM process_steps")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for row in rows {
            paths.insert(row?);
        }

        let mut stmt = self
            .conn
            .prepare("SELECT image_path FROM texts WHERE image_path IS NOT NULL")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for row in rows {
            paths.insert(row?);
        }

        let mut stmt = self
            .conn
            .prepare("SELECT ambience_path FROM figurines WHERE ambience_path IS NOT NULL")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for row in rows {
            paths.insert(row?);
        }

        let mut stmt = self
            .conn
            .prepare("SELECT video_url FROM figurines WHERE video_url IS NOT NULL")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for row in rows {
            paths.insert(row?);
        }

        let mut stmt = self.conn.prepare("SELECT file_path FROM app_resources")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for row in rows {
            paths.insert(row?);
        }

        Ok(paths)
    }

    pub fn get_media_usages(&self) -> Result<Vec<MediaUsageDto>> {
        let mut usages = Vec::new();

        let mut stmt = self.conn.prepare(
            "SELECT i.id, i.file_path, i.original_path, i.thumb_path, f.id, f.name
             FROM images i
             LEFT JOIN figurines f ON f.id = i.figurine_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })?;
        for row in rows {
            let (image_id, preview, original, thumb, fig_id, fig_name) = row?;
            let label = format!(
                "Image for {}",
                fig_name.unwrap_or_else(|| "Unknown figurine".to_string())
            );
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

        let mut stmt = self.conn.prepare(
            "SELECT ps.id, ps.image_path, f.id, f.name
             FROM process_steps ps
             LEFT JOIN figurines f ON f.id = ps.figurine_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?;
        for row in rows {
            let (step_id, path, fig_id, fig_name) = row?;
            usages.push(MediaUsageDto {
                path,
                label: format!(
                    "Process step for {}",
                    fig_name.unwrap_or_else(|| "Unknown figurine".to_string())
                ),
                entity_type: "processStep".to_string(),
                entity_id: fig_id.unwrap_or(step_id),
                field: "image".to_string(),
            });
        }

        let mut stmt = self.conn.prepare(
            "SELECT id, category, caption, image_path FROM texts WHERE image_path IS NOT NULL",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        for row in rows {
            let (id, category, caption, path) = row?;
            usages.push(MediaUsageDto {
                path,
                label: caption.unwrap_or_else(|| format!("{} text", category)),
                entity_type: "text".to_string(),
                entity_id: id,
                field: "image".to_string(),
            });
        }

        let mut stmt = self
            .conn
            .prepare("SELECT id, name, ambience_path, video_url FROM figurines")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?;
        for row in rows {
            let (id, name, ambience, video) = row?;
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

        let mut stmt = self
            .conn
            .prepare("SELECT key, file_path FROM app_resources WHERE key NOT IN ('author_profile', 'home_content')")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (key, path) = row?;
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

    pub fn replace_media_path_everywhere(
        &self,
        old_path: &str,
        new_preview_path: &str,
        new_original_path: Option<&str>,
        new_thumb_path: Option<&str>,
    ) -> Result<usize> {
        let mut updated = 0usize;

        if new_preview_path.starts_with("images/preview/") {
            updated += self.conn.execute(
                "UPDATE images
                 SET file_path = ?1, original_path = ?2, thumb_path = ?3
                 WHERE file_path = ?4 OR original_path = ?4 OR thumb_path = ?4",
                params![
                    new_preview_path,
                    new_original_path,
                    new_thumb_path,
                    old_path
                ],
            )?;
        } else {
            updated += self.conn.execute(
                "UPDATE images SET file_path = ?1 WHERE file_path = ?2",
                params![new_preview_path, old_path],
            )?;
            updated += self.conn.execute(
                "UPDATE images SET original_path = ?1 WHERE original_path = ?2",
                params![new_preview_path, old_path],
            )?;
            updated += self.conn.execute(
                "UPDATE images SET thumb_path = ?1 WHERE thumb_path = ?2",
                params![new_preview_path, old_path],
            )?;
        }

        updated += self.conn.execute(
            "UPDATE process_steps SET image_path = ?1 WHERE image_path = ?2",
            params![new_preview_path, old_path],
        )?;
        updated += self.conn.execute(
            "UPDATE texts SET image_path = ?1 WHERE image_path = ?2",
            params![new_preview_path, old_path],
        )?;
        updated += self.conn.execute(
            "UPDATE figurines SET ambience_path = ?1 WHERE ambience_path = ?2",
            params![new_preview_path, old_path],
        )?;
        updated += self.conn.execute(
            "UPDATE figurines SET video_url = ?1 WHERE video_url = ?2",
            params![new_preview_path, old_path],
        )?;
        updated += self.conn.execute(
            "UPDATE app_resources SET file_path = ?1 WHERE file_path = ?2 AND key NOT IN ('author_profile', 'home_content')",
            params![new_preview_path, old_path],
        )?;

        Ok(updated)
    }

    pub fn get_images_for_figurine(&self, figurine_id: &str) -> Result<Vec<Image>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, figurine_id, image_type, file_path, original_path, thumb_path, alt_text, sort_order, updated_at
             FROM images
             WHERE figurine_id = ?
             ORDER BY sort_order"
        )?;

        let iter = stmt.query_map(params![figurine_id], |row| {
            Ok(Image {
                id: row.get(0)?,
                figurine_id: row.get(1)?,
                image_type: ImageType::from_str(&row.get::<_, String>(2)?),
                file_path: row.get(3)?,
                original_path: row.get(4)?,
                thumb_path: row.get(5)?,
                alt_text: row.get(6)?,
                sort_order: row.get(7)?,
                updated_at: get_iso_date(row, 8)?,
            })
        })?;

        iter.collect()
    }

    pub fn get_cabinet_zones(&self) -> Result<Vec<CabinetZone>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, zone_type, x_percent, y_percent, width_percent, height_percent, target_route
             FROM cabinet_zones
             ORDER BY sort_order"
        )?;

        let iter = stmt.query_map([], |row| {
            Ok(CabinetZone {
                id: row.get(0)?,
                zone_type: row.get(1)?,
                x_percent: row.get(2)?,
                y_percent: row.get(3)?,
                width_percent: row.get(4)?,
                height_percent: row.get(5)?,
                target_route: row.get(6)?,
            })
        })?;

        iter.collect()
    }

    pub fn upsert_cabinet_zone(&self, z: &CabinetZone) -> Result<()> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO cabinet_zones (id, zone_type, x_percent, y_percent, width_percent, height_percent, target_route, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET
                zone_type=excluded.zone_type,
                x_percent=excluded.x_percent,
                y_percent=excluded.y_percent,
                width_percent=excluded.width_percent,
                height_percent=excluded.height_percent,
                target_route=excluded.target_route,
                sort_order=excluded.sort_order"
        )?;

        stmt.execute(params![
            z.id,
            z.zone_type,
            z.x_percent,
            z.y_percent,
            z.width_percent,
            z.height_percent,
            z.target_route,
            0 // sort_order logic not implemented fully yet, default 0
        ])?;
        Ok(())
    }

    pub fn delete_cabinet_zone(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM cabinet_zones WHERE id = ?", params![id])?;
        Ok(())
    }

    // === TEXTS ===

    pub fn get_texts_by_category(&self, category: &str) -> Result<Vec<Text>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, category, content, caption, image_path, sort_order, updated_at
             FROM texts
             WHERE category = ?
             ORDER BY sort_order",
        )?;

        let iter = stmt.query_map(params![category], |row| {
            Ok(Text {
                id: row.get(0)?,
                category: TextCategory::from_str(&row.get::<_, String>(1)?),
                content: row.get(2)?,
                caption: row.get(3)?,
                image_path: row.get(4)?,
                sort_order: row.get(5)?,
                updated_at: get_iso_date(row, 6)?,
            })
        })?;

        iter.collect()
    }

    pub fn upsert_text(&self, t: &Text) -> Result<()> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO texts (id, category, content, caption, image_path, sort_order, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                category=excluded.category,
                content=excluded.content,
                caption=excluded.caption,
                image_path=excluded.image_path,
                sort_order=excluded.sort_order,
                updated_at=excluded.updated_at",
        )?;

        let category_str = match t.category {
            TextCategory::Author => "author",
            TextCategory::Workshop => "workshop",
        };

        stmt.execute(params![
            t.id,
            category_str,
            t.content,
            t.caption,
            t.image_path,
            t.sort_order,
            t.updated_at
        ])?;
        Ok(())
    }

    pub fn delete_text(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM texts WHERE id = ?", params![id])?;
        Ok(())
    }

    pub fn get_process_steps_for_figurine(&self, figurine_id: &str) -> Result<Vec<ProcessStep>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, figurine_id, step_type, description, image_path, sort_order, updated_at
             FROM process_steps
             WHERE figurine_id = ?
             ORDER BY sort_order",
        )?;

        let iter = stmt.query_map(params![figurine_id], |row| {
            Ok(ProcessStep {
                id: row.get(0)?,
                figurine_id: row.get(1)?,
                step_type: ProcessStepType::from_str(&row.get::<_, String>(2)?),
                description: row.get(3)?,
                image_path: row.get(4)?,
                sort_order: row.get(5)?,
                updated_at: get_iso_date(row, 6)?,
            })
        })?;

        iter.collect()
    }

    // === PROCESS STEPS WRITE ===
    pub fn replace_process_steps(&self, figurine_id: &str, steps: Vec<ProcessStep>) -> Result<()> {
        self.conn.execute(
            "DELETE FROM process_steps WHERE figurine_id = ?",
            params![figurine_id],
        )?;

        let mut stmt = self.conn.prepare(
            "INSERT INTO process_steps (id, figurine_id, step_type, description, image_path, sort_order, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
        )?;

        for (i, step) in steps.into_iter().enumerate() {
            stmt.execute(params![
                step.id,
                figurine_id,
                step.step_type.as_str(),
                step.description,
                step.image_path,
                i as i32,
                step.updated_at
            ])?;
        }

        Ok(())
    }

    // === RELATED ITEMS ===

    pub fn get_related_figurines(&self, id: &str) -> Result<Vec<Figurine>> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT f.id, f.name, f.short_text, f.full_description, f.dimensions, f.material, f.technique, f.year, f.passport_number, f.edition, f.created_period, f.care_instructions, f.provenance_note, f.authenticity_note, f.included_items, f.ambience_path, f.video_url, f.secret_text, f.status, f.sort_order, f.updated_at, f.is_visible, COALESCE(f.is_featured, 0)
             FROM figurines f
             JOIN figurines current ON current.id = ?1
             WHERE f.id != ?1
             AND (
                f.year = current.year 
                OR (current.material IS NOT NULL AND f.material LIKE '%' || substr(current.material, 1, 4) || '%')
             )
             AND f.is_visible = 1
             ORDER BY random()
             LIMIT 3"
        )?;

        let iter = stmt.query_map(params![id], |row| {
            Ok(Figurine {
                id: row.get(0)?,
                name: row.get(1)?,
                short_text: row.get(2)?,
                full_description: row.get(3)?,
                dimensions: row.get(4)?,
                material: row.get(5)?,
                technique: row.get(6)?,
                year: row.get(7)?,
                passport_number: row.get(8)?,
                edition: row.get(9)?,
                created_period: row.get(10)?,
                care_instructions: row.get(11)?,
                provenance_note: row.get(12)?,
                authenticity_note: row.get(13)?,
                included_items: row.get(14)?,
                ambience_path: row.get(15)?,
                video_url: row.get(16)?,
                secret_text: row.get(17)?,
                status: FigurineStatus::from_str(&row.get::<_, String>(18)?),
                sort_order: row.get(19)?,
                updated_at: get_iso_date(row, 20)?,
                is_visible: row.get(21)?,
                is_featured: row.get::<_, i32>(22).unwrap_or(0) != 0,
            })
        })?;

        iter.collect()
    }

    // === BLOB MANAGEMENT (FOR PORTABLE EXPORT) ===

    pub fn update_figurine_blobs(
        &self,
        id: &str,
        ambience: Option<Vec<u8>>,
        video: Option<Vec<u8>>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE figurines SET ambience_data = ?1, video_data = ?2 WHERE id = ?3",
            params![ambience, video, id],
        )?;
        Ok(())
    }

    pub fn figurine_has_ambience_blob(&self, id: &str) -> Result<bool> {
        self.conn.query_row(
            "SELECT ambience_data IS NOT NULL FROM figurines WHERE id = ?",
            params![id],
            |row| row.get(0),
        )
    }

    pub fn figurine_has_video_blob(&self, id: &str) -> Result<bool> {
        self.conn.query_row(
            "SELECT video_data IS NOT NULL FROM figurines WHERE id = ?",
            params![id],
            |row| row.get(0),
        )
    }

    pub fn update_image_blob(
        &self,
        id: &str,
        data: Vec<u8>,
        original_data: Option<Vec<u8>>,
        thumb_data: Option<Vec<u8>>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE images SET data = ?1, original_data = ?2, thumb_data = ?3 WHERE id = ?4",
            params![data, original_data, thumb_data, id],
        )?;
        Ok(())
    }

    pub fn image_has_blob(&self, id: &str) -> Result<bool> {
        self.conn.query_row(
            "SELECT data IS NOT NULL FROM images WHERE id = ?",
            params![id],
            |row| row.get(0),
        )
    }

    pub fn image_has_original_blob(&self, id: &str) -> Result<bool> {
        self.conn.query_row(
            "SELECT original_data IS NOT NULL FROM images WHERE id = ?",
            params![id],
            |row| row.get(0),
        )
    }

    pub fn image_has_thumb_blob(&self, id: &str) -> Result<bool> {
        self.conn.query_row(
            "SELECT thumb_data IS NOT NULL FROM images WHERE id = ?",
            params![id],
            |row| row.get(0),
        )
    }

    pub fn update_step_blob(&self, id: &str, data: Vec<u8>) -> Result<()> {
        self.conn.execute(
            "UPDATE process_steps SET image_data = ?1 WHERE id = ?2",
            params![data, id],
        )?;
        Ok(())
    }

    pub fn step_has_blob(&self, id: &str) -> Result<bool> {
        self.conn.query_row(
            "SELECT image_data IS NOT NULL FROM process_steps WHERE id = ?",
            params![id],
            |row| row.get(0),
        )
    }

    pub fn update_text_blob(&self, id: &str, data: Vec<u8>) -> Result<()> {
        self.conn.execute(
            "UPDATE texts SET image_data = ?1 WHERE id = ?2",
            params![data, id],
        )?;
        Ok(())
    }

    pub fn text_has_blob(&self, id: &str) -> Result<bool> {
        self.conn.query_row(
            "SELECT image_data IS NOT NULL FROM texts WHERE id = ?",
            params![id],
            |row| row.get(0),
        )
    }

    // === APP RESOURCES (SYSTEM ASSETS) ===

    pub fn get_app_resource(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT file_path FROM app_resources WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;

        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn upsert_app_resource(
        &self,
        key: &str,
        file_path: &str,
        data: Option<Vec<u8>>,
    ) -> Result<()> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO app_resources (key, file_path, data, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(key) DO UPDATE SET
                file_path=excluded.file_path,
                data=excluded.data,
                updated_at=excluded.updated_at",
        )?;

        stmt.execute(params![key, file_path, data, Utc::now().to_rfc3339()])?;
        Ok(())
    }

    pub fn get_app_resources(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT key, file_path FROM app_resources")?;
        let iter = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        iter.collect()
    }

    pub fn update_app_resource_blob(&self, key: &str, data: Vec<u8>) -> Result<()> {
        self.conn.execute(
            "UPDATE app_resources SET data = ?1, updated_at = ?2 WHERE key = ?3",
            params![data, Utc::now().to_rfc3339(), key],
        )?;
        Ok(())
    }

    pub fn app_resource_has_blob(&self, key: &str) -> Result<bool> {
        self.conn.query_row(
            "SELECT data IS NOT NULL FROM app_resources WHERE key = ?",
            params![key],
            |row| row.get(0),
        )
    }
}
