//! sqlx caches named prepared statements per connection. `SELECT *` / `RETURNING *`
//! bind the result row type into that plan, so an `ALTER TABLE` (another process,
//! restore, rolling migrate) makes the next execute fail with Postgres `0A000`
//! "cached plan must not change result type".
//!
//! The statement cache stays on. Recovery is idle until a real `0A000`: then each
//! pooled connection is cleared once via [`sqlx::Connection::clear_cached_statements`]
//! on checkout. sqlx's idle list is LIFO, so a discard-N-connections counter would
//! spend its budget on the hot connection and leave poisoned idle ones untouched.

use sqlx::Connection;
use sqlx::postgres::PgConnection;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

static PLAN_EPOCH: AtomicU64 = AtomicU64::new(0);
static LAST_CLEARED: LazyLock<Mutex<HashMap<usize, u64>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn is_stale_cached_plan(err: &sqlx::Error) -> bool {
    let sqlx::Error::Database(db_err) = err else {
        return false;
    };
    is_stale_cached_plan_parts(db_err.code().as_deref(), db_err.message())
}

fn is_stale_cached_plan_parts(code: Option<&str>, message: &str) -> bool {
    code == Some("0A000") && message.contains("cached plan must not change result type")
}

fn lock_last_cleared() -> std::sync::MutexGuard<'static, HashMap<usize, u64>> {
    LAST_CLEARED.lock().unwrap_or_else(|e| e.into_inner())
}

/// Called from `From<sqlx::Error>`. No-op for any other database error.
pub fn note_stale_cached_plan(err: &sqlx::Error) {
    if !is_stale_cached_plan(err) {
        return;
    }
    let prev = PLAN_EPOCH.fetch_add(1, Ordering::SeqCst);
    lock_last_cleared().clear();
    if prev == 0 {
        tracing::warn!(
            target: "gotiga_server::db",
            "postgres prepared-statement cache is stale; re-preparing on next checkout"
        );
    }
}

pub async fn clear_stale_prepared_statements(conn: &mut PgConnection) -> sqlx::Result<()> {
    let epoch = PLAN_EPOCH.load(Ordering::Acquire);
    if epoch == 0 {
        return Ok(());
    }

    let key = std::ptr::from_mut(conn) as usize;
    if lock_last_cleared().get(&key).copied() == Some(epoch) {
        return Ok(());
    }

    conn.clear_cached_statements().await?;

    let mut last = lock_last_cleared();
    // Recycled connections leave dead keys. Wiping only causes extra clears.
    if last.len() > 256 {
        last.clear();
    }
    last.insert(key, epoch);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_cached_plan_error() {
        assert!(is_stale_cached_plan_parts(
            Some("0A000"),
            "cached plan must not change result type"
        ));
        assert!(!is_stale_cached_plan_parts(
            Some("0A000"),
            "feature not supported"
        ));
        assert!(!is_stale_cached_plan_parts(
            Some("23505"),
            "cached plan must not change result type"
        ));
        assert!(!is_stale_cached_plan_parts(
            None,
            "cached plan must not change result type"
        ));
    }

    #[test]
    fn io_error_is_not_stale_plan() {
        let err = sqlx::Error::Io(std::io::Error::other("x"));
        assert!(!is_stale_cached_plan(&err));
    }
}
