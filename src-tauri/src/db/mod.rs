pub mod repository;

use rusqlite::{Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Database {
    pub conn: Mutex<Connection>,
}

impl Database {
    /// Создать или открыть БД
    pub fn new(db_path: PathBuf) -> Result<Self> {
        // Создать директорию если не существует
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(&db_path)?;

        // Включить foreign keys
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;

        let db = Self {
            conn: Mutex::new(conn),
        };

        // Применить миграции
        db.migrate()?;

        Ok(db)
    }

    /// Применить миграции
    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(include_str!("schema.sql"))?;
        add_column_if_missing(&conn, "images", "original_path", "TEXT")?;
        add_column_if_missing(&conn, "images", "thumb_path", "TEXT")?;
        add_column_if_missing(&conn, "images", "depth_path", "TEXT")?;
        add_column_if_missing(&conn, "images", "original_data", "BLOB")?;
        add_column_if_missing(&conn, "images", "thumb_data", "BLOB")?;
        add_column_if_missing(
            &conn,
            "figurines",
            "is_featured",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        add_column_if_missing(&conn, "figurines", "passport_number", "TEXT")?;
        add_column_if_missing(&conn, "figurines", "edition", "TEXT")?;
        add_column_if_missing(&conn, "figurines", "created_period", "TEXT")?;
        add_column_if_missing(&conn, "figurines", "care_instructions", "TEXT")?;
        add_column_if_missing(&conn, "figurines", "provenance_note", "TEXT")?;
        add_column_if_missing(&conn, "figurines", "authenticity_note", "TEXT")?;
        add_column_if_missing(&conn, "figurines", "included_items", "TEXT")?;
        migrate_figurines_status_constraint(&conn)?;

        Ok(())
    }

    pub fn seed_if_empty(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Проверить, есть ли данные
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM figurines", [], |row| row.get(0))?;

        if count == 0 {
            conn.execute_batch(include_str!("seed.sql"))?;
        }

        Ok(())
    }
}

fn migrate_figurines_status_constraint(conn: &Connection) -> Result<()> {
    let result: rusqlite::Result<String> = conn.query_row(
        "SELECT COALESCE(sql, '') FROM sqlite_master WHERE type='table' AND name='figurines'",
        [],
        |row| row.get(0),
    );

    let sql = match result {
        Ok(s) => s,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(()),
        Err(e) => return Err(e),
    };

    if sql.contains("in_progress") {
        return Ok(());
    }

    // Drop leftover temp table from any previous failed migration
    conn.execute_batch("DROP TABLE IF EXISTS figurines_new;")?;

    conn.execute_batch(
        "
        PRAGMA foreign_keys = OFF;
        BEGIN;
        CREATE TABLE figurines_new (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            short_text TEXT,
            full_description TEXT,
            dimensions TEXT,
            material TEXT,
            technique TEXT,
            year INTEGER,
            passport_number TEXT,
            edition TEXT,
            created_period TEXT,
            care_instructions TEXT,
            provenance_note TEXT,
            authenticity_note TEXT,
            included_items TEXT,
            ambience_path TEXT,
            video_url TEXT,
            ambience_data BLOB,
            video_data BLOB,
            secret_text TEXT,
            is_visible BOOLEAN NOT NULL DEFAULT 1,
            is_featured INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'available'
                CHECK (status IN ('available', 'sold', 'reserved', 'in_progress')),
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT (datetime('now'))
        );
        INSERT INTO figurines_new
            SELECT id, name, short_text, full_description, dimensions, material, technique,
                   year, passport_number, edition, created_period, care_instructions,
                   provenance_note, authenticity_note, included_items,
                   ambience_path, video_url, ambience_data, video_data, secret_text,
                   is_visible, 0, status, sort_order, created_at, updated_at
            FROM figurines;
        DROP TABLE figurines;
        ALTER TABLE figurines_new RENAME TO figurines;
        CREATE INDEX IF NOT EXISTS idx_figurines_sort ON figurines(sort_order);
        COMMIT;
        PRAGMA foreign_keys = ON;
    ",
    )?;

    Ok(())
}

fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    column_type: &str,
) -> Result<()> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let existing: String = row.get(1)?;
        if existing == column {
            return Ok(());
        }
    }

    conn.execute(
        &format!(
            "ALTER TABLE {} ADD COLUMN {} {}",
            table, column, column_type
        ),
        [],
    )?;
    Ok(())
}
