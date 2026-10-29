//! «Спросить сайт»: каталог ответов в базе и подбор записи по вопросу.
//!
//! Вопрос превращается в вектор той же моделью, что ищет работы
//! (`embed::embed_query`), и сравнивается с формулировками каждой записи;
//! поиск по словам (`search::hybrid_search`) только подтверждает пограничный
//! случай. Решение принимает `ask::decide`. Выключенные записи (черновики)
//! участвуют в подборе, но не отвечают: узнанный вопрос без ответа даёт
//! `unanswered`. Если модели в сборке нет, подбор
//! идёт по словам и умеет только предлагать.

use super::*;
use crate::ask::{self, AskAnswer, AskRoute, Decision};
use crate::lang::Lang;
use crate::search::SearchRecord;

/// Каталог, готовый к подбору: все записи и вектор каждой формулировки.
pub(super) struct AskIndex {
    entries: Vec<AskAnswer>,
    /// (номер записи, вектор формулировки). Пусто, если модели нет.
    vectors: Vec<(usize, Vec<f32>)>,
    /// Формулировки, разобранные для поиска по словам: один раз при сборке каталога.
    search: crate::search::SearchIndex,
    embedded: bool,
}

const SELECT: &str = "SELECT id, key, handler, enabled, sort_order, topic, pages, phrases, question, answer, links FROM ask_answers";

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    key: String,
    handler: Option<String>,
    enabled: bool,
    sort_order: i32,
    topic: String,
    pages: sqlx::types::Json<Vec<String>>,
    phrases: sqlx::types::Json<Vec<String>>,
    question: sqlx::types::Json<ask::LangTexts>,
    answer: sqlx::types::Json<ask::LangTexts>,
    links: sqlx::types::Json<Vec<ask::AskLinkDef>>,
}

impl From<Row> for AskAnswer {
    fn from(r: Row) -> Self {
        AskAnswer {
            id: Some(r.id),
            key: r.key,
            handler: r.handler,
            enabled: r.enabled,
            sort_order: r.sort_order,
            topic: r.topic,
            pages: r.pages.0,
            phrases: r.phrases.0,
            question: r.question.0,
            answer: r.answer.0,
            links: r.links.0,
        }
    }
}

impl AppService {
    async fn ask_rows(&self, only_enabled: bool) -> Result<Vec<AskAnswer>> {
        let sql = format!(
            "{SELECT}{} ORDER BY sort_order, key",
            if only_enabled { " WHERE enabled" } else { "" }
        );
        let rows: Vec<Row> = sqlx::query_as(&sql)
            .fetch_all(self.repo.pg_pool())
            .await
            .map_err(AppError::Database)?;
        Ok(rows.into_iter().map(AskAnswer::from).collect())
    }

    /// Каталог с векторами. Строится один раз и стоит, пока его не сбросит правка.
    async fn ask_index(&self) -> Result<Arc<AskIndex>> {
        if let Some(index) = self.ask_index.read().await.as_ref() {
            return Ok(index.clone());
        }
        let mut slot = self.ask_index.write().await;
        // Пока ждали замок, соседний вопрос мог уже построить.
        if let Some(index) = slot.as_ref() {
            return Ok(index.clone());
        }
        // Выключенные записи тоже в подборе: вопрос, на который ответа ещё нет,
        // должен узнаваться, а не падать на соседнюю запись.
        let entries = self.ask_rows(false).await?;
        let embedded = crate::embed::is_available() && !entries.is_empty();
        let records: Vec<SearchRecord> = entries
            .iter()
            .map(|e| {
                let phrases = ask::index_phrases(e);
                let first = phrases.first().cloned().unwrap_or_default();
                SearchRecord::from_text(&e.key, &first, &phrases.iter().skip(1).cloned().collect::<Vec<_>>().join(" \n "))
            })
            .collect();
        let vectors = if embedded {
            let phrases: Vec<(usize, String)> = entries
                .iter()
                .enumerate()
                .flat_map(|(i, e)| ask::index_phrases(e).into_iter().map(move |p| (i, p)))
                .collect();
            tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<(usize, Vec<f32>)>> {
                phrases
                    .into_iter()
                    .map(|(i, p)| crate::embed::embed_query(&p).map(|v| (i, v)))
                    .collect()
            })
            .await
            .map_err(|e| AppError::Internal(format!("ask index task: {e}")))?
            .map_err(|e| AppError::Internal(format!("ask index embedding: {e}")))?
        } else {
            Vec::new()
        };
        let index = Arc::new(AskIndex { entries, vectors, search: crate::search::SearchIndex::new(&records), embedded });
        *slot = Some(index.clone());
        Ok(index)
    }

    /// Построить каталог заранее. Возвращает число включённых записей.
    pub async fn warm_ask_index(&self) -> Result<usize> {
        Ok(self.ask_index().await?.entries.len())
    }

    /// Выбрать запись каталога по вопросу. Текст вопроса нигде не пишется.
    pub async fn ask_route(&self, question: &str, lang: Lang) -> Result<AskRoute> {
        let q: String = question.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(ask::PHRASE_LEN).collect();
        if q.chars().count() < 2 {
            return Ok(AskRoute::none());
        }
        let index = self.ask_index().await?;
        if index.entries.is_empty() {
            return Ok(AskRoute::none());
        }

        // Вопрос, напечатанный на кнопке, совпадает с формулировкой слово в слово:
        // отвечаем без подбора, он на таком не должен ошибаться (и не умеет без модели).
        // Поиск по словам получает вопрос без служебных слов; кодировщик — целиком.
        let words = ask::content_words(&q);
        let decision = if let Some(i) = ask::exact_match(&q, &index.entries) {
            Decision::Answer(i)
        } else if index.embedded {
            let q2 = q.clone();
            let qv = tokio::task::spawn_blocking(move || crate::embed::embed_query(&q2))
                .await
                .map_err(|e| AppError::Internal(format!("ask embed task: {e}")))?
                .map_err(|e| AppError::Internal(format!("ask embed: {e}")))?;
            let mut levels = vec![f32::NEG_INFINITY; index.entries.len()];
            for (i, v) in &index.vectors {
                let s = crate::embed::dot(&qv, v);
                if s > levels[*i] {
                    levels[*i] = s;
                }
            }
            let scores: Vec<(String, f32)> =
                index.entries.iter().zip(&levels).map(|(e, s)| (e.key.clone(), *s)).collect();
            let hits = index.search.hybrid(&words, &scores, index.entries.len());
            let lexical_top = hits
                .first()
                .and_then(|h| index.entries.iter().position(|e| e.key == h.id));
            ask::decide(&levels, lexical_top)
        } else {
            let hits = index.search.hybrid(&words, &[], index.entries.len());
            let ranked: Vec<usize> = hits
                .iter()
                .filter_map(|h| index.entries.iter().position(|e| e.key == h.id))
                .collect();
            ask::decide_lexical(&ranked)
        };

        Ok(match decision {
            Decision::Answer(i) if index.entries[i].enabled => AskRoute::answer(&index.entries[i], lang),
            Decision::Answer(i) => AskRoute::unanswered(&index.entries[i].key),
            Decision::Suggest(v) => {
                // Кнопкой предлагается только то, на что есть ответ.
                let entries: Vec<&AskAnswer> = v.iter().map(|&i| &index.entries[i]).filter(|e| e.enabled).collect();
                AskRoute::suggest(&entries, lang)
            }
            Decision::Nothing => AskRoute::none(),
        })
    }

    /// Вопросы для списка в окне: напечатанные на языке читателя, с темой и
    /// страницами. Берутся из того же каталога в памяти, что и подбор.
    pub async fn ask_questions(&self, lang: Lang) -> Result<Vec<ask::AskQuestion>> {
        Ok(ask::public_questions(&self.ask_index().await?.entries, lang))
    }

    // ── Стол админа ─────────────────────────────────────────────────────────

    pub async fn admin_list_ask_answers(&self) -> Result<Vec<AskAnswer>> {
        self.ask_rows(false).await
    }

    /// Записать запись каталога: без `id` — новую, с `id` — правку. Возвращает
    /// как легло.
    pub async fn admin_save_ask_answer(&self, a: AskAnswer) -> Result<AskAnswer> {
        let a = ask::normalize(a).map_err(AppError::BadRequest)?;
        let pool = self.repo.pg_pool();
        let phrases = sqlx::types::Json(&a.phrases);
        let question = sqlx::types::Json(&a.question);
        let answer = sqlx::types::Json(&a.answer);
        let links = sqlx::types::Json(&a.links);
        let pages = sqlx::types::Json(&a.pages);
        let result: std::result::Result<Option<Row>, sqlx::Error> = match a.id {
            Some(id) => {
                sqlx::query_as(&format!(
                    "UPDATE ask_answers SET key = $2, handler = $3, enabled = $4, sort_order = $5,
                            phrases = $6, question = $7, answer = $8, links = $9,
                            topic = $10, pages = $11, updated_at = now()
                     WHERE id = $1
                     RETURNING id, key, handler, enabled, sort_order, topic, pages, phrases, question, answer, links"
                ))
                .bind(id)
                .bind(&a.key)
                .bind(&a.handler)
                .bind(a.enabled)
                .bind(a.sort_order)
                .bind(phrases)
                .bind(question)
                .bind(answer)
                .bind(links)
                .bind(&a.topic)
                .bind(pages)
                .fetch_optional(pool)
                .await
            }
            None => {
                sqlx::query_as(
                    "INSERT INTO ask_answers (key, handler, enabled, sort_order, phrases, question, answer, links, topic, pages)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                     RETURNING id, key, handler, enabled, sort_order, topic, pages, phrases, question, answer, links",
                )
                .bind(&a.key)
                .bind(&a.handler)
                .bind(a.enabled)
                .bind(a.sort_order)
                .bind(phrases)
                .bind(question)
                .bind(answer)
                .bind(links)
                .bind(&a.topic)
                .bind(pages)
                .fetch_optional(pool)
                .await
            }
        };
        let row = match result {
            Ok(row) => row,
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                return Err(AppError::BadRequest(format!("The key “{}” is already used by another answer.", a.key)));
            }
            Err(e) => return Err(AppError::Database(e)),
        };
        let row = row.ok_or_else(|| AppError::NotFound("ask answer".into()))?;
        *self.ask_index.write().await = None;
        Ok(row.into())
    }

    pub async fn admin_delete_ask_answer(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM ask_answers WHERE id = $1")
            .bind(id)
            .execute(self.repo.pg_pool())
            .await
            .map_err(AppError::Database)?;
        *self.ask_index.write().await = None;
        Ok(())
    }
}
