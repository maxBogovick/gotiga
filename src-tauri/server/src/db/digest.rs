//! Чтения и записи недельного разбора быстрых уходов (`services/digest.rs`, `digest.rs`).

use super::Repository;
use crate::digest::{BrokenRoom, CellCount, Finding, SlowLoad};
use crate::error::Result;
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use sqlx::types::Json;

/// Сколько раз пробуем отправить разбор владельцу, прежде чем оставить его только в админке.
pub const DIGEST_MAX_TRIES: i32 = 6;

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DigestRow {
    pub week_start: NaiveDate,
    pub made_at: DateTime<Utc>,
    pub visits: i32,
    pub quick: i32,
    pub findings: Json<Vec<Finding>>,
    pub telegram_at: Option<DateTime<Utc>>,
}

impl Repository {
    /// Свёртка по источнику и устройству, сложенная за `from..=to`.
    pub async fn digest_cells(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<CellCount>> {
        Ok(sqlx::query_as::<_, CellCount>(
            r#"
            SELECT path_group, origin, device_class,
                   SUM(visits)::bigint AS visits,
                   SUM(quick)::bigint AS quick,
                   SUM(quick_unscrolled)::bigint AS quick_unscrolled
            FROM site_engagement_source_daily
            WHERE day BETWEEN $1 AND $2
            GROUP BY path_group, origin, device_class
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// LCP по комнатам и устройствам. Дневные 75-е процентили складываются взвешенно по
    /// числу замеров: это приближение, а не точный процентиль недели, и для приметы его хватает.
    pub async fn digest_slow_loads(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<SlowLoad>> {
        Ok(sqlx::query_as::<_, SlowLoad>(
            r#"
            SELECT path_group, device_class,
                   SUM(lcp_n)::bigint AS samples,
                   (SUM(lcp_p75::float8 * lcp_n) / SUM(lcp_n))::float8 AS lcp_p75
            FROM site_vitals_daily
            WHERE day BETWEEN $1 AND $2 AND lcp_p75 IS NOT NULL AND lcp_n > 0
            GROUP BY path_group, device_class
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn digest_broken(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<BrokenRoom>> {
        Ok(sqlx::query_as::<_, BrokenRoom>(
            r#"
            SELECT path_group, SUM(events)::bigint AS events
            FROM site_errors_daily
            WHERE day BETWEEN $1 AND $2
            GROUP BY path_group
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    pub async fn digest_exists(&self, week_start: NaiveDate) -> Result<bool> {
        Ok(
            sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS (SELECT 1 FROM insight_digests WHERE week_start = $1)",
            )
            .bind(week_start)
            .fetch_one(&self.pg_pool)
            .await?,
        )
    }

    /// Записать разбор недели. Вторая запись той же недели (две копии сервера) молча
    /// не пишется: возвращает, записал ли именно этот вызов.
    pub async fn digest_insert(
        &self,
        week_start: NaiveDate,
        visits: i64,
        quick: i64,
        findings: &[Finding],
    ) -> Result<bool> {
        let done = sqlx::query(
            r#"
            INSERT INTO insight_digests (week_start, visits, quick, findings)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (week_start) DO NOTHING
            "#,
        )
        .bind(week_start)
        .bind(visits as i32)
        .bind(quick as i32)
        .bind(Json(findings))
        .execute(&self.pg_pool)
        .await?;
        Ok(done.rows_affected() > 0)
    }

    /// Последние разборы, свежие сверху.
    pub async fn digest_recent(&self, limit: i64) -> Result<Vec<DigestRow>> {
        Ok(sqlx::query_as::<_, DigestRow>(
            r#"
            SELECT week_start, made_at, visits, quick, findings, telegram_at
            FROM insight_digests
            ORDER BY week_start DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Разборы, которые владелец ещё не получил. Старше двух недель не догоняем:
    /// сообщение про давнюю неделю уже ничего не говорит.
    pub async fn digest_unsent(&self, since: NaiveDate) -> Result<Vec<DigestRow>> {
        Ok(sqlx::query_as::<_, DigestRow>(
            r#"
            SELECT week_start, made_at, visits, quick, findings, telegram_at
            FROM insight_digests
            WHERE telegram_at IS NULL AND telegram_tries < $2 AND week_start >= $1
            ORDER BY week_start
            "#,
        )
        .bind(since)
        .bind(DIGEST_MAX_TRIES)
        .fetch_all(&self.pg_pool)
        .await?)
    }

    /// Отметить попытку до отправки: упавший посреди процесс не должен пробовать вечно.
    pub async fn digest_note_try(&self, week_start: NaiveDate) -> Result<()> {
        sqlx::query("UPDATE insight_digests SET telegram_tries = telegram_tries + 1 WHERE week_start = $1")
            .bind(week_start)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    pub async fn digest_mark_sent(&self, week_start: NaiveDate) -> Result<()> {
        sqlx::query("UPDATE insight_digests SET telegram_at = NOW() WHERE week_start = $1")
            .bind(week_start)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }

    /// Стереть разбор недели, чтобы проход сделал его заново (после дозаполнения свёртки).
    pub async fn digest_delete(&self, week_start: NaiveDate) -> Result<()> {
        sqlx::query("DELETE FROM insight_digests WHERE week_start = $1")
            .bind(week_start)
            .execute(&self.pg_pool)
            .await?;
        Ok(())
    }
}
