use rusqlite::{params, Connection, Result};
use crate::models::*;

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
            "SELECT id, name, short_text, year, status, sort_order
             FROM figurines
             ORDER BY sort_order"
        )?;

        let iter = stmt.query_map([], |row| {
            Ok(Figurine {
                id: row.get(0)?,
                name: row.get(1)?,
                short_text: row.get(2)?,
                year: row.get(3)?,
                status: FigurineStatus::from_str(&row.get::<_, String>(4)?),
                sort_order: row.get(5)?,
            })
        })?;

        iter.collect()
    }

    pub fn get_figurine_by_id(&self, id: &str) -> Result<Option<Figurine>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, short_text, year, status, sort_order
             FROM figurines
             WHERE id = ?"
        )?;

        let mut rows = stmt.query(params![id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Figurine {
                id: row.get(0)?,
                name: row.get(1)?,
                short_text: row.get(2)?,
                year: row.get(3)?,
                status: FigurineStatus::from_str(&row.get::<_, String>(4)?),
                sort_order: row.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    // === IMAGES ===

    pub fn get_images_for_figurine(&self, figurine_id: &str) -> Result<Vec<Image>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, figurine_id, image_type, file_path, alt_text, sort_order
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
            })
        })?;

        iter.collect()
    }

    // === TEXTS ===

    pub fn get_texts_by_category(&self, category: &str) -> Result<Vec<Text>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, category, content, caption, image_path, sort_order
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
            })
        })?;

        iter.collect()
    }

    // === CABINET ZONES ===

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
}
