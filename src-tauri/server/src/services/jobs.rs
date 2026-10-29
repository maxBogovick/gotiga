//! Журнал фоновых проходов: обёртка для запуска и чтение для вкладки «Механизмы».
//!
//! Сервер держит больше десятка проходов (`main.rs`), и сбой любого был виден только в
//! журнале. Каждый проход идёт через `track_job`: она записывает, когда он начался, чем
//! кончился и что сделал. Расписание (`every_secs`) записывается самим проходом, поэтому
//! второго списка «что должно идти раз в сколько» нет — он разошёлся бы с циклами.

use super::*;
use crate::db::JobRow;
use serde::Serialize;

/// Длина заметки и текста ошибки в журнале: это строка для глаза, не хранилище.
const JOB_TEXT_MAX: usize = 300;

/// Проход считается опоздавшим, когда с его начала прошло больше, чем это число расписаний.
/// Два с половиной, а не два: запуск может сдвинуться на тик, а сервер — перезапускаться.
const LATE_FACTOR_X10: i64 = 25;
/// Запас сверху к расписанию, секунды: проход сам занимает время.
const LATE_SLACK_SECS: i64 = 300;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobStatus {
    #[serde(flatten)]
    pub row: JobRow,
    /// `ok`, `failed` (последний запуск кончился ошибкой) или `late` (давно не шёл).
    pub state: &'static str,
}

/// Состояние прохода по последнему запуску. Ошибка важнее опоздания: она объясняет его.
pub fn job_state(ok: bool, every_secs: i32, age_secs: i64) -> &'static str {
    if !ok {
        "failed"
    } else if every_secs > 0 && age_secs > (every_secs as i64 * LATE_FACTOR_X10) / 10 + LATE_SLACK_SECS {
        "late"
    } else {
        "ok"
    }
}

/// Текст ошибки для журнала: адреса вырезаются целиком. Текст сетевой ошибки несёт адрес
/// запроса, а у бота Telegram токен стоит в самом адресе, и в таблице он не должен жить.
pub fn scrub_error(text: &str) -> String {
    let cleaned: Vec<&str> = text
        .split_whitespace()
        .map(|w| if w.contains("://") { "<url>" } else { w })
        .collect();
    let joined = cleaned.join(" ");
    joined.chars().take(JOB_TEXT_MAX).collect()
}

impl AppService {
    /// Выполнить проход и записать итог. Результат возвращается как есть: журнал не меняет
    /// того, что проход делал и как о нём пишет сервер. Запись в журнал — по возможности:
    /// сбой записи проход не роняет.
    ///
    /// `did` называет, что проход сделал, или `None`, если запуск был вхолостую.
    /// `every_secs == 0` — проход при запуске сервера, опоздать он не может.
    pub async fn track_job<R, Fut>(
        &self,
        key: &'static str,
        every_secs: u64,
        work: Fut,
        did: impl FnOnce(&R) -> Option<String>,
    ) -> Result<R>
    where
        Fut: std::future::Future<Output = Result<R>>,
    {
        let started = chrono::Utc::now();
        let clock = Instant::now();
        let out = work.await;
        let ms = clock.elapsed().as_millis().min(i32::MAX as u128) as i32;
        let (ok, note, error) = match &out {
            Ok(r) => (true, did(r), None),
            Err(e) => (false, None, Some(scrub_error(&e.to_string()))),
        };
        let note = note.map(|n| n.chars().take(JOB_TEXT_MAX).collect::<String>());
        if let Err(e) = self
            .repo
            .job_record(
                key,
                every_secs.min(i32::MAX as u64) as i32,
                started,
                ms,
                ok,
                note.as_deref(),
                error.as_deref(),
            )
            .await
        {
            tracing::warn!("Could not write the run of job {key} to the journal: {e}");
        }
        out
    }

    pub async fn admin_jobs(&self) -> Result<Vec<JobStatus>> {
        Ok(self
            .repo
            .job_list()
            .await?
            .into_iter()
            .map(|row| {
                let state = job_state(row.ok, row.every_secs, row.age_secs);
                JobStatus { row, state }
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failure_outweighs_lateness() {
        assert_eq!(job_state(false, 300, 10_000), "failed");
        assert_eq!(job_state(true, 300, 100), "ok");
    }

    #[test]
    fn late_means_more_than_two_and_a_half_schedules_and_a_slack() {
        // 300 с × 2,5 + 300 = 1050
        assert_eq!(job_state(true, 300, 1050), "ok");
        assert_eq!(job_state(true, 300, 1051), "late");
    }

    #[test]
    fn a_start_up_job_cannot_be_late() {
        assert_eq!(job_state(true, 0, 99_999_999), "ok");
    }

    #[test]
    fn urls_never_reach_the_journal() {
        let e = "error sending request for url (https://api.telegram.org/bot123:ABC-def/sendMessage): timed out";
        let s = scrub_error(e);
        assert!(!s.contains("123:ABC"), "{s}");
        assert!(!s.contains("telegram.org"), "{s}");
        assert!(s.contains("timed out"));
        assert!(scrub_error(&"x ".repeat(400)).chars().count() <= JOB_TEXT_MAX);
    }
}
