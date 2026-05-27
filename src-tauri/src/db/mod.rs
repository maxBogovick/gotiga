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
        add_column_if_missing(&conn, "images", "original_data", "BLOB")?;
        add_column_if_missing(&conn, "images", "thumb_data", "BLOB")?;

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
