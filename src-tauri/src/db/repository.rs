use rusqlite::{params, Connection, Result};
use crate::models::*;
use chrono::{Utc, TimeZone};

fn get_iso_date(row: &rusqlite::Row, index: usize) -> Result<String> {
    use rusqlite::types::ValueRef;
    match row.get_ref(index)? {
        ValueRef::Integer(i) => {
            let dt = Utc.timestamp_opt(i, 0).unwrap();
            Ok(dt.to_rfc3339())
        },
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
            "SELECT id, name, short_text, full_description, dimensions, material, technique, year, ambience_path, video_url, secret_text, status, sort_order, updated_at, is_visible
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
                ambience_path: row.get(8)?,
                video_url: row.get(9)?,
                secret_text: row.get(10)?,
                status: FigurineStatus::from_str(&row.get::<_, String>(11)?),
                sort_order: row.get(12)?,
                updated_at: get_iso_date(row, 13)?,
                is_visible: row.get(14)?,
            })
        })?;

        iter.collect()
    }

    pub fn get_figurine_by_id(&self, id: &str) -> Result<Option<Figurine>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, short_text, full_description, dimensions, material, technique, year, ambience_path, video_url, secret_text, status, sort_order, updated_at, is_visible
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
                ambience_path: row.get(8)?,
                video_url: row.get(9)?,
                secret_text: row.get(10)?,
                status: FigurineStatus::from_str(&row.get::<_, String>(11)?),
                sort_order: row.get(12)?,
                updated_at: get_iso_date(row, 13)?,
                is_visible: row.get(14)?,
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
                year, ambience_path, video_url, secret_text, status, sort_order, is_visible, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            ON CONFLICT(id) DO UPDATE SET
                name=excluded.name,
                short_text=excluded.short_text,
                full_description=excluded.full_description,
                dimensions=excluded.dimensions,
                material=excluded.material,
                technique=excluded.technique,
                year=excluded.year,
                ambience_path=excluded.ambience_path,
                video_url=excluded.video_url,
                secret_text=excluded.secret_text,
                status=excluded.status,
                sort_order=excluded.sort_order,
                is_visible=excluded.is_visible,
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
            f.ambience_path,
            f.video_url,
            f.secret_text,
            f.status.as_str(),
            f.sort_order,
            f.is_visible,
            f.updated_at // This is String now
        ])?;

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
            "INSERT INTO images (id, figurine_id, image_type, file_path, alt_text, sort_order, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
        )?;

        for (i, img) in images.into_iter().enumerate() {
            stmt.execute(params![
                img.id,
                figurine_id,
                img.image_type.as_str(),
                img.file_path,
                img.alt_text,
                i as i32,
                img.updated_at // String
            ])?;
        }

        Ok(())
    }

    pub fn get_all_images(&self) -> Result<Vec<Image>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, figurine_id, image_type, file_path, alt_text, sort_order, updated_at FROM images"
        )?;
        let iter = stmt.query_map([], |row| {
            Ok(Image {
                id: row.get(0)?,
                figurine_id: row.get(1)?,
                image_type: ImageType::from_str(&row.get::<_, String>(2)?),
                file_path: row.get(3)?,
                alt_text: row.get(4)?,
                sort_order: row.get(5)?,
                updated_at: get_iso_date(row, 6)?,
            })
        })?;
        iter.collect()
    }

    pub fn get_images_for_figurine(&self, figurine_id: &str) -> Result<Vec<Image>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, figurine_id, image_type, file_path, alt_text, sort_order, updated_at
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
                alt_text: row.get(4)?,
                sort_order: row.get(5)?,
                updated_at: get_iso_date(row, 6)?,
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
        self.conn.execute("DELETE FROM cabinet_zones WHERE id = ?", params![id])?;
        Ok(())
    }

    // === TEXTS ===

    pub fn get_texts_by_category(&self, category: &str) -> Result<Vec<Text>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, category, content, caption, image_path, sort_order, updated_at
             FROM texts
             WHERE category = ?
             ORDER BY sort_order"
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
                updated_at=excluded.updated_at"
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
        self.conn.execute("DELETE FROM texts WHERE id = ?", params![id])?;
        Ok(())
    }

    pub fn get_process_steps_for_figurine(&self, figurine_id: &str) -> Result<Vec<ProcessStep>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, figurine_id, step_type, description, image_path, sort_order, updated_at
             FROM process_steps
             WHERE figurine_id = ?
             ORDER BY sort_order"
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
            "SELECT DISTINCT f.id, f.name, f.short_text, f.full_description, f.dimensions, f.material, f.technique, f.year, f.ambience_path, f.video_url, f.secret_text, f.status, f.sort_order, f.updated_at, f.is_visible
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
                ambience_path: row.get(8)?,
                video_url: row.get(9)?,
                secret_text: row.get(10)?,
                status: FigurineStatus::from_str(&row.get::<_, String>(11)?),
                sort_order: row.get(12)?,
                updated_at: get_iso_date(row, 13)?,
                is_visible: row.get(14)?,
            })
        })?;

        iter.collect()
    }

    // === BLOB MANAGEMENT (FOR PORTABLE EXPORT) ===

    pub fn update_figurine_blobs(&self, id: &str, ambience: Option<Vec<u8>>, video: Option<Vec<u8>>) -> Result<()> {
        self.conn.execute(
            "UPDATE figurines SET ambience_data = ?1, video_data = ?2 WHERE id = ?3",
            params![ambience, video, id],
        )?;
        Ok(())
    }

    pub fn update_image_blob(&self, id: &str, data: Vec<u8>) -> Result<()> {
        self.conn.execute(
            "UPDATE images SET data = ?1 WHERE id = ?2",
            params![data, id],
        )?;
        Ok(())
    }

    pub fn update_step_blob(&self, id: &str, data: Vec<u8>) -> Result<()> {
        self.conn.execute(
            "UPDATE process_steps SET image_data = ?1 WHERE id = ?2",
            params![data, id],
        )?;
        Ok(())
    }

    pub fn update_text_blob(&self, id: &str, data: Vec<u8>) -> Result<()> {
        self.conn.execute(
            "UPDATE texts SET image_data = ?1 WHERE id = ?2",
            params![data, id],
        )?;
        Ok(())
    }

    // === APP RESOURCES (SYSTEM ASSETS) ===

    pub fn get_app_resource(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare("SELECT file_path FROM app_resources WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;

        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn upsert_app_resource(&self, key: &str, file_path: &str, data: Option<Vec<u8>>) -> Result<()> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO app_resources (key, file_path, data, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(key) DO UPDATE SET
                file_path=excluded.file_path,
                data=excluded.data,
                updated_at=excluded.updated_at"
        )?;

        stmt.execute(params![
            key,
            file_path,
            data,
            Utc::now().to_rfc3339()
        ])?;
        Ok(())
    }
}
