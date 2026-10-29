//! Подписки на уведомления и журнал рассылки (`services/push.rs`).

use super::Repository;
use crate::error::Result;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Подписка, как она нужна рассылке.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PushSubscriber {
    pub id: Uuid,
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
    pub lang: String,
    pub topics: Vec<String>,
}

/// Подряд не доставленных, после которых подписка стирается.
pub const PUSH_MAX_FAILURES: i32 = 5;

impl Repository {
    /// Записать подписку. Пришла новая подписка взамен старой (`old_endpoint`) — старая
    /// стирается; темы и язык службе уже известны (из запроса или из старой записи).
    pub async fn push_upsert(
        &self,
        endpoint: &str,
        p256dh: &str,
        auth: &str,
        lang: &str,
        topics: &[String],
        old_endpoint: Option<&str>,
    ) -> Result<()> {
        let mut tx = self.pg_pool.begin().await?;
        if let Some(old) = old_endpoint.filter(|o| *o != endpoint) {
            sqlx::query("DELETE FROM push_subscriptions WHERE endpoint = $1")
                .bind(old)
                .execute(&mut *tx)
                .await?;
        }
        sqlx::query(
            "INSERT INTO push_subscriptions (endpoint, p256dh, auth, lang, topics)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (endpoint) DO UPDATE
                SET p256dh = EXCLUDED.p256dh, auth = EXCLUDED.auth, lang = EXCLUDED.lang,
                    topics = EXCLUDED.topics, failures = 0, updated_at = NOW()",
        )
        .bind(endpoint)
        .bind(p256dh)
        .bind(auth)
        .bind(lang)
        .bind(topics)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Язык и темы записанной подписки; `None` — такого адреса нет.
    pub async fn push_find(&self, endpoint: &str) -> Result<Option<(String, Vec<String>)>> {
        Ok(sqlx::query_as("SELECT lang, topics FROM push_subscriptions WHERE endpoint = $1")
            .bind(endpoint)
            .fetch_optional(&self.pg_pool)
            .await?)
    }

    pub async fn push_remove(&self, endpoint: &str) -> Result<u64> {
        Ok(sqlx::query("DELETE FROM push_subscriptions WHERE endpoint = $1")
            .bind(endpoint)
            .execute(&self.pg_pool)
            .await?
            .rows_affected())
    }

    /// Действующие подписки.
    pub async fn push_active(&self) -> Result<Vec<PushSubscriber>> {
        Ok(sqlx::query_as(
            "SELECT id, endpoint, p256dh, auth, lang, topics FROM push_subscriptions
              WHERE failures < $1",
        )
        .bind(PUSH_MAX_FAILURES)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn push_count(&self) -> Result<i64> {
        let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM push_subscriptions WHERE failures < $1")
            .bind(PUSH_MAX_FAILURES)
            .fetch_one(&self.pg_pool)
            .await?;
        Ok(n)
    }

    /// Итог рассылки: доставленным — обнулить счётчик, названным несуществующими — стереть,
    /// остальным — прибавить неудачу.
    pub async fn push_record(&self, delivered: &[Uuid], gone: &[Uuid], failed: &[Uuid]) -> Result<()> {
        if !delivered.is_empty() {
            sqlx::query(
                "UPDATE push_subscriptions SET failures = 0, last_sent_at = NOW()
                  WHERE id = ANY($1)",
            )
            .bind(delivered)
            .execute(&self.pg_pool)
            .await?;
        }
        if !gone.is_empty() {
            sqlx::query("DELETE FROM push_subscriptions WHERE id = ANY($1)")
                .bind(gone)
                .execute(&self.pg_pool)
                .await?;
        }
        if !failed.is_empty() {
            sqlx::query("UPDATE push_subscriptions SET failures = failures + 1 WHERE id = ANY($1)")
                .bind(failed)
                .execute(&self.pg_pool)
                .await?;
        }
        sqlx::query("DELETE FROM push_subscriptions WHERE failures >= $1")
            .bind(PUSH_MAX_FAILURES)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    /// Записать настройку, только если её ещё нет; `true` — записана эта. Две копии сервера,
    /// заводящие ключи одновременно, получают один и тот же.
    pub async fn setting_insert_if_absent(&self, key: &str, value: &str) -> Result<bool> {
        let n = sqlx::query("INSERT INTO settings (key, value) VALUES ($1, $2) ON CONFLICT (key) DO NOTHING")
            .bind(key)
            .bind(value)
            .execute(&self.pg_pool)
            .await?
            .rows_affected();
        Ok(n == 1)
    }

    /// Отметить вещи, замеченные на людях. Уже отмеченные не трогаются: строка журнала
    /// заводится один раз, поэтому скрыть и снова показать работу — не повод слать второй раз.
    pub async fn push_note_seen(&self, kind: &str, ids: &[Uuid], sent: bool) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let res = sqlx::query(
            "INSERT INTO push_posts (kind, target_id, sent_at)
             SELECT $1, id, CASE WHEN $3 THEN NOW() END FROM UNNEST($2::uuid[]) AS id
             ON CONFLICT (kind, target_id) DO NOTHING",
        )
        .bind(kind)
        .bind(ids)
        .bind(sent)
        .execute(&self.pg_pool)
        .await?;
        Ok(res.rows_affected())
    }

    /// Что пора слать: замечено не позже `grace_secs` назад и ещё не ушло. Старшие первыми.
    pub async fn push_due(&self, grace_secs: i64, limit: i64) -> Result<Vec<(String, Uuid, DateTime<Utc>)>> {
        Ok(sqlx::query_as(
            "SELECT kind, target_id, first_seen_at FROM push_posts
              WHERE sent_at IS NULL AND first_seen_at <= NOW() - make_interval(secs => $1)
              ORDER BY first_seen_at
              LIMIT $2",
        )
        .bind(grace_secs as f64)
        .bind(limit)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Записать неудачную попытку доставить вещь; возвращает, сколько их уже было.
    pub async fn push_attempt(&self, kind: &str, id: Uuid) -> Result<i32> {
        let (n,): (i32,) = sqlx::query_as(
            "UPDATE push_posts SET attempts = attempts + 1
              WHERE kind = $1 AND target_id = $2
          RETURNING attempts",
        )
        .bind(kind)
        .bind(id)
        .fetch_one(&self.pg_pool)
        .await?;
        Ok(n)
    }

    pub async fn push_mark_sent(&self, kind: &str, id: Uuid) -> Result<()> {
        sqlx::query("UPDATE push_posts SET sent_at = NOW() WHERE kind = $1 AND target_id = $2")
            .bind(kind)
            .bind(id)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }
}
