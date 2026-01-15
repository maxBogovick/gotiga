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

        Ok(())
    }

    pub fn seed_if_empty(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Проверить, есть ли данные
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM figurines",
            [],
            |row| row.get(0)
        )?;

        if count == 0 {
            conn.execute_batch(include_str!("seed.sql"))?;
        }

        Ok(())
    }
}
