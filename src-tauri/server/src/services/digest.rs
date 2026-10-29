//! Недельный разбор быстрых уходов: проход, отправка владельцу и чтение для админки.
//!
//! Правила и текст — `crate::digest`, здесь база и связь. Проход раз в час: делает разбор
//! последней законченной недели, если его ещё нет, и отправляет владельцу то, что не ушло.
//! Событий «неделя кончилась» нет, как нет и событий «опубликовано» у канала (§ 20),
//! поэтому проход сверяет журнал (`insight_digests`) с календарём.

use super::*;
use crate::db::DigestRow;
use crate::digest::{self, Digest};
use chrono::Duration;

/// Недели в сравнении: четыре прежние перед разбираемой.
const DIGEST_PAST_WEEKS: i64 = 4;
/// Разбор, не дошедший до владельца, догоняем не дольше этого.
const DIGEST_CATCH_UP_DAYS: i64 = 14;

impl AppService {
    /// Один проход. Возвращает, сделан ли разбор недели и сколько сообщений ушло владельцу.
    pub async fn weekly_digest_round(&self) -> Result<(bool, usize)> {
        let today = chrono::Utc::now().date_naive();
        let (start, end) = digest::last_full_week(today);

        let mut made = false;
        if !self.repo.digest_exists(start).await? {
            // Свёртка этих семи дней пересчитывается из сырых событий: сегодняшний «горячий
            // проход» держит только вчера и сегодня, а разбор нужен по всей неделе. Сырьё
            // живёт 30 дней, неделя не старше 13 — пересчёт безопасен.
            self.repo.refresh_signal_rollups_only(start, end).await?;
            let past_from = start - Duration::days(7 * DIGEST_PAST_WEEKS);
            let past_to = start - Duration::days(1);
            let (current, past, slow, broken) = tokio::try_join!(
                self.repo.digest_cells(start, end),
                self.repo.digest_cells(past_from, past_to),
                self.repo.digest_slow_loads(start, end),
                self.repo.digest_broken(start, end),
            )?;
            let d = digest::build(&current, &past, &slow, &broken);
            made = self
                .repo
                .digest_insert(start, d.visits, d.quick, &d.findings)
                .await?;
        }

        let mut sent = 0;
        let since = today - Duration::days(DIGEST_CATCH_UP_DAYS);
        for row in self.repo.digest_unsent(since).await? {
            let (Some(token), Some(chat_id)) = (
                self.config.telegram_bot_token.as_deref(),
                self.config.telegram_chat_id.as_deref(),
            ) else {
                // Бот не подключён: разбор остаётся в админке, а попыток не тратим.
                break;
            };
            let d = Digest {
                visits: row.visits as i64,
                quick: row.quick as i64,
                findings: row.findings.0.clone(),
            };
            let text = digest::telegram_text(
                row.week_start,
                row.week_start + Duration::days(6),
                &d,
                &format!("{}/admin", self.config.public_url.trim_end_matches('/')),
            );
            if self.config.public_url_is_local() {
                // Локальная база своя, а чат настоящий: печатаем в журнал, как объявления.
                tracing::info!("[local] digest for the owner:\n{text}");
                self.repo.digest_mark_sent(row.week_start).await?;
                continue;
            }
            self.repo.digest_note_try(row.week_start).await?;
            let url = format!("https://api.telegram.org/bot{token}/sendMessage");
            let reply = self
                .http_client
                .post(&url)
                .json(&serde_json::json!({
                    "chat_id": chat_id,
                    "text": text,
                    "disable_web_page_preview": true,
                }))
                .send()
                .await;
            match reply.and_then(|r| r.error_for_status()) {
                Ok(_) => {
                    self.repo.digest_mark_sent(row.week_start).await?;
                    sent += 1;
                }
                // Текст ответа Telegram в журнал не пишем: в адресе запроса токен бота.
                Err(e) => tracing::warn!(
                    "Digest for week {} did not reach Telegram (status {:?})",
                    row.week_start,
                    e.status()
                ),
            }
        }
        Ok((made, sent))
    }

    pub async fn admin_digests(&self) -> Result<Vec<DigestRow>> {
        self.repo.digest_recent(12).await
    }

    /// Сделать разбор последней недели заново и отправить его снова. Нужен после
    /// дозаполнения свёртки за прошлые дни: готовый разбор остаётся снимком на тот день.
    pub async fn admin_rebuild_digest(&self) -> Result<Vec<DigestRow>> {
        let (start, _) = digest::last_full_week(chrono::Utc::now().date_naive());
        self.repo.digest_delete(start).await?;
        self.weekly_digest_round().await?;
        self.admin_digests().await
    }
}
