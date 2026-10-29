//! Журнал фоновых проходов (`services/jobs.rs`, миграция `job_runs`).

use super::Repository;
use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobRow {
    pub job: String,
    pub every_secs: i32,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub duration_ms: i32,
    pub ok: bool,
    pub ok_at: Option<DateTime<Utc>>,
    pub did_at: Option<DateTime<Utc>>,
    pub note: Option<String>,
    pub error: Option<String>,
    pub runs: i64,
    pub failures: i64,
    pub in_a_row: i32,
    /// Сколько секунд прошло с начала последнего запуска (по часам базы).
    pub age_secs: i64,
}

impl Repository {
    /// Записать итог одного запуска. `note` — что проход сделал; пустая заметка прежнюю
    /// не стирает (проход, ничего не сделавший, не должен забывать, что делал вчера).
    pub async fn job_record(
        &self,
        job: &str,
        every_secs: i32,
        started_at: DateTime<Utc>,
        duration_ms: i32,
        ok: bool,
        note: Option<&str>,
        error: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO job_runs (
                job, every_secs, started_at, finished_at, duration_ms, ok,
                ok_at, did_at, note, error, runs, failures, in_a_row
            )
            VALUES (
                $1, $2, $3, NOW(), $4, $5,
                CASE WHEN $5 THEN NOW() END,
                CASE WHEN $5 AND $6::text IS NOT NULL THEN NOW() END,
                $6, $7, 1,
                CASE WHEN $5 THEN 0 ELSE 1 END,
                CASE WHEN $5 THEN 0 ELSE 1 END
            )
            ON CONFLICT (job) DO UPDATE SET
                every_secs  = EXCLUDED.every_secs,
                started_at  = EXCLUDED.started_at,
                finished_at = EXCLUDED.finished_at,
                duration_ms = EXCLUDED.duration_ms,
                ok          = EXCLUDED.ok,
                ok_at       = COALESCE(EXCLUDED.ok_at, job_runs.ok_at),
                did_at      = COALESCE(EXCLUDED.did_at, job_runs.did_at),
                note        = COALESCE(EXCLUDED.note, job_runs.note),
                error       = EXCLUDED.error,
                runs        = job_runs.runs + 1,
                failures    = job_runs.failures + CASE WHEN EXCLUDED.ok THEN 0 ELSE 1 END,
                in_a_row    = CASE WHEN EXCLUDED.ok THEN 0 ELSE job_runs.in_a_row + 1 END
            "#,
        )
        .bind(job)
        .bind(every_secs)
        .bind(started_at)
        .bind(duration_ms)
        .bind(ok)
        .bind(note)
        .bind(error)
        .execute(&self.pg_pool)
        .await?;
        Ok(())
    }

    pub async fn job_list(&self) -> Result<Vec<JobRow>> {
        Ok(sqlx::query_as::<_, JobRow>(
            r#"
            SELECT job, every_secs, started_at, finished_at, duration_ms, ok, ok_at, did_at,
                   note, error, runs, failures, in_a_row,
                   EXTRACT(EPOCH FROM (NOW() - started_at))::bigint AS age_secs
            FROM job_runs
            ORDER BY job
            "#,
        )
        .fetch_all(&self.pg_pool)
        .await?)
    }
}
