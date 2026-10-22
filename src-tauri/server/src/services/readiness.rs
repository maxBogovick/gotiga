//! Помощник админки: что у работ и байек не доделано.
//!
//! Проверка одна, и считает её сервер: реестр (метка у работы), форма работы
//! (список «Готовность») и вкладка «Что не доделано» читают один ответ
//! `GET /admin/readiness`. Три места, считающие порознь, однажды показали бы
//! разное.
//!
//! Здесь же — фоновое обновление поиска (`refresh_stale_embeddings`): поиск
//! обновляется при сохранении работы, но фоном и без повтора, и сбой там
//! оставлял работу ненайденной молча. Проход раз в десять минут дочищает то,
//! что сохранение не успело.

use super::*;

/// Сколько похожих работ ждёт док на главной (`figurine_similar`, позиции 0..2).
const SIMILAR_WANTED: i64 = 3;

/// Языки байки, в порядке, в котором их печатает стол рассказов.
const TALE_LANGS: [&str; 3] = ["en", "ru", "uk"];

impl AppService {
    /// Какие из видимых работ не найдены поиском в нынешнем виде: вектора нет
    /// или он посчитан по другому тексту (другой моделью). `None` — модель
    /// поиска в этой сборке не лежит, и спрашивать нечего.
    async fn stale_search_ids(&self) -> Result<Option<HashSet<Uuid>>> {
        if !crate::embed::is_available() {
            return Ok(None);
        }
        let q = FigurineQuery {
            status: None,
            search: None,
            sort: None,
            page: None,
            per_page: None,
        };
        let (figs, _) = self.repo.get_all_figurines(true, &q).await?;
        let model = crate::embed::model_name();
        let rows: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT figurine_id, source_hash FROM figurine_embeddings WHERE model = $1",
        )
        .bind(&model)
        .fetch_all(self.repo.pg_pool())
        .await?;
        let have: HashMap<Uuid, String> = rows.into_iter().collect();
        let ids: Vec<Uuid> = figs.iter().map(|f| f.id).collect();
        let texts = self.repo.get_figurine_translations(&ids).await?;
        let stale = figs
            .iter()
            .filter(|f| {
                let hash = Self::text_hash(&model, &Self::curatorial_text(f, texts.get(&f.id)));
                have.get(&f.id) != Some(&hash)
            })
            .map(|f| f.id)
            .collect();
        Ok(Some(stale))
    }

    /// Фоновый проход: переиндексировать работы, у которых поиск отстал, и
    /// убрать векторы работ, снятых с сайта. Работы с прежним текстом
    /// пропускаются по отпечатку, поэтому проход почти ничего не стоит.
    /// Возвращает, сколько работ переписано.
    pub async fn refresh_stale_embeddings(&self) -> Result<usize> {
        let Some(stale) = self.stale_search_ids().await? else {
            return Ok(0);
        };
        let mut done = 0usize;
        for id in stale {
            match self.reindex_figurine_embedding(id).await {
                Ok(true) => done += 1,
                Ok(false) => {}
                Err(e) => tracing::warn!(
                    target: "gotiga_server::embed",
                    figurine = %id, error = %e,
                    "search index refresh failed"
                ),
            }
        }
        sqlx::query(
            "DELETE FROM figurine_embeddings e
              WHERE NOT EXISTS (
                SELECT 1 FROM figurines f
                 WHERE f.id = e.figurine_id AND f.is_visible
                   AND (f.first_look_until IS NULL OR f.first_look_until <= NOW())
              )",
        )
        .execute(self.repo.pg_pool())
        .await?;
        Ok(done)
    }

    /// Что не доделано. В ответе только то, у чего есть пропуски.
    pub async fn admin_readiness(&self) -> Result<ReadinessReport> {
        let q = FigurineQuery {
            status: None,
            search: None,
            sort: None,
            page: None,
            per_page: None,
        };
        let (figs, _) = self.repo.get_all_figurines(false, &q).await?;
        let total_works = figs.len() as i64;

        // Фото: сколько всего, есть ли обложка и второй ракурс — одним запросом.
        let photo_rows: Vec<(Uuid, i64, bool, bool)> = sqlx::query_as(
            "SELECT figurine_id, COUNT(*),
                    BOOL_OR(image_type = 'face'), BOOL_OR(image_type = 'detail')
               FROM images
              WHERE figurine_id IS NOT NULL
              GROUP BY figurine_id",
        )
        .fetch_all(self.repo.pg_pool())
        .await?;
        let photos: HashMap<Uuid, (i64, bool, bool)> = photo_rows
            .into_iter()
            .map(|(id, n, face, detail)| (id, (n, face, detail)))
            .collect();

        let similar_rows: Vec<(Uuid, i64)> = sqlx::query_as(
            "SELECT figurine_id, COUNT(*) FROM figurine_similar GROUP BY figurine_id",
        )
        .fetch_all(self.repo.pg_pool())
        .await?;
        let similar: HashMap<Uuid, i64> = similar_rows.into_iter().collect();

        let stale = self.stale_search_ids().await?;
        // Выбрать трёх похожих можно, только когда есть из кого: у работы
        // в собрании из трёх соседей двое.
        let similar_wanted = SIMILAR_WANTED.min(total_works - 1).max(0);

        let mut works = Vec::new();
        for f in &figs {
            let (n_photos, has_face, has_detail) = photos.get(&f.id).copied().unwrap_or_default();
            let n_similar = similar.get(&f.id).copied().unwrap_or(0);
            let mut gaps = Vec::new();
            if n_photos == 0 {
                gaps.push("photos".to_string());
            } else {
                if !has_face {
                    gaps.push("cover".to_string());
                }
                // Второй ракурс — другое фото, поэтому при одном фото его нет
                // и быть не может.
                if n_photos >= 2 && !has_detail {
                    gaps.push("angle".to_string());
                }
            }
            if n_similar < similar_wanted {
                gaps.push("similar".to_string());
            }
            if stale.as_ref().is_some_and(|s| s.contains(&f.id)) {
                gaps.push("search".to_string());
            }
            if !gaps.is_empty() {
                works.push(WorkGaps {
                    id: f.id.to_string(),
                    name: f.name.clone(),
                    hidden: !f.is_visible,
                    similar_count: n_similar,
                    gaps,
                });
            }
        }

        // Байки: только вышедшие и назначенные. Черновик для того и черновик,
        // чтобы быть недописанным.
        type TaleRow = (
            Uuid,
            String,
            String,
            String,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Vec<String>,
        );
        let tale_rows: Vec<TaleRow> = sqlx::query_as(
            "SELECT id, status, title_en, title_ru, title_uk,
                    body_en, body_ru, body_uk, image_url, image_urls
               FROM gazette_leaves
              WHERE kind = 'tale' AND status IN ('published', 'scheduled')
              ORDER BY shelf_order NULLS LAST, created_at DESC",
        )
        .fetch_all(self.repo.pg_pool())
        .await?;

        let mut tales = Vec::new();
        for (id, status, t_en, t_ru, t_uk, b_en, b_ru, b_uk, image, images) in tale_rows {
            let titles = [&t_en, &t_ru, &t_uk];
            let bodies = [&b_en, &b_ru, &b_uk];
            let mut gaps = Vec::new();
            for (i, lang) in TALE_LANGS.iter().enumerate() {
                if bodies[i].as_deref().unwrap_or("").trim().is_empty() {
                    gaps.push(format!("body_{lang}"));
                } else if titles[i].trim().is_empty() {
                    gaps.push(format!("title_{lang}"));
                }
            }
            let has_photo = images.iter().any(|u| !u.trim().is_empty())
                || image.as_deref().is_some_and(|u| !u.trim().is_empty());
            if !has_photo {
                gaps.push("photo".to_string());
            }
            if !gaps.is_empty() {
                let title = [&t_ru, &t_en, &t_uk]
                    .into_iter()
                    .find(|t| !t.trim().is_empty())
                    .cloned()
                    .unwrap_or_default();
                tales.push(TaleGaps {
                    id: id.to_string(),
                    title,
                    status,
                    gaps,
                });
            }
        }

        Ok(ReadinessReport {
            works,
            tales,
            search_checked: stale.is_some(),
        })
    }
}
