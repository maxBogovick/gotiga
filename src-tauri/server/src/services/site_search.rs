//! Поиск по всему сайту: работы, небылицы, листы вестника и карты — один список
//! с одной оценкой.
//!
//! Раньше поиск в шапке искал по словам в браузере, а сервер подсказывал только
//! порядок работ. Небылицы и карты смысловым поиском не находились вовсе, а
//! вопрос вроде «gnomes with weapon» держался на том, что в тексте случайно
//! стоит нужное слово. Здесь всё лежит в одном месте: вектор (multilingual-e5),
//! слова с опечатками и триграммы, слитые через RRF, и бонус за слова запроса,
//! найденные в названии (`search::site_search`). Числа бонуса и порог подобраны
//! замером `examples/search_probe.rs` на `tools/search/cases.json`.
//!
//! Работы читают векторы из базы, как прежний `/search`; остальное (их десятки)
//! собирается в памяти и держится `INDEX_TTL`.

use super::*;
use crate::models::{SiteHit, SiteKind};
use crate::search::{site_search, SearchRecord, SiteBoost};

/// Бонусы за покрытие слов запроса и порог «молчать». Подбор — `search_probe --sweep`.
pub(crate) const BOOST: SiteBoost = SiteBoost { name: 0.01, any: 0.01, common: 0.25 };
/// Вещь с оценкой ниже этого подтверждена только одним из списков (вектор,
/// слова, триграммы): такой хвост — случайные соседи, и в ответ он не идёт.
/// Если не набрала порога даже лучшая вещь, запрос «не про сайт»: пусто. На 129
/// фразах порог 0,02 сокращает ответ с 32 до 11 вещей (без модели с 17 до 7),
/// верная остаётся (с моделью — кроме одной фразы из 129).
pub(crate) const FLOOR: f32 = 0.02;
const INDEX_TTL: Duration = Duration::from_secs(180);

/// Вещи, кроме работ: записи для поиска по словам и вектор каждой.
pub(super) struct SiteIndex {
    built: Instant,
    records: Vec<SearchRecord>,
    vectors: Vec<(String, Vec<f32>)>,
    kinds: HashMap<String, SiteKind>,
}

fn nonempty(s: Option<&String>) -> Option<String> {
    s.map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

fn first_of(values: &[Option<&String>]) -> String {
    values.iter().find_map(|v| nonempty(*v)).unwrap_or_default()
}

fn cut(text: &str, n: usize) -> String {
    text.chars().take(n).collect()
}

impl AppService {
    async fn site_index(&self) -> Result<Arc<SiteIndex>> {
        if let Some(i) = self.site_index.read().await.as_ref()
            && i.built.elapsed() < INDEX_TTL
        {
            return Ok(i.clone());
        }
        let mut slot = self.site_index.write().await;
        if let Some(i) = slot.as_ref()
            && i.built.elapsed() < INDEX_TTL
        {
            return Ok(i.clone());
        }

        let tales = self.list_tales_public().await?;
        let leaves = self.list_gazette_public(1, 500).await?.items;
        let cards = self.list_battle_cards_public().await?;

        let mut records = Vec::new();
        let mut passages: Vec<(String, String)> = Vec::new();
        let mut kinds = HashMap::new();

        for (list, kind) in [(&tales, SiteKind::Tale), (&leaves, SiteKind::Leaf)] {
            for l in list {
                if kind == SiteKind::Leaf && l.kind == "tale" {
                    continue;
                }
                let titles = [&l.title_en, &l.title_ru, &l.title_uk, &l.title_es, &l.title_de, &l.title_it];
                let deks = [&l.dek_en, &l.dek_ru, &l.dek_uk, &l.dek_es, &l.dek_de, &l.dek_it];
                let bodies = [&l.body_en, &l.body_ru, &l.body_uk, &l.body_es, &l.body_de, &l.body_it];
                let mut name: Vec<String> = titles.iter().filter(|t| !t.trim().is_empty()).map(|t| t.to_string()).collect();
                name.extend(l.figurine_name.clone());
                name.extend(l.figurine_name_translations.values().cloned());
                let body: Vec<String> = deks.iter().chain(bodies.iter()).filter_map(|d| nonempty(d.as_ref())).collect();
                records.push(SearchRecord::from_text(&l.id, &name.join(" "), &body.join(" ")));
                let title = first_of(&[Some(&l.title_en).filter(|t| !t.is_empty()), Some(&l.title_ru).filter(|t| !t.is_empty())]);
                let dek = first_of(&[l.dek_en.as_ref(), l.dek_ru.as_ref()]);
                let text = first_of(&[l.body_en.as_ref(), l.body_ru.as_ref()]);
                passages.push((l.id.clone(), format!("{title}. {dek}. {}", cut(&text, 500))));
                kinds.insert(l.id.clone(), kind);
            }
        }
        for c in cards.iter().filter(|c| c.kind != "structure") {
            let titles = [&c.title_en, &c.title_ru, &c.title_uk, &c.title_es, &c.title_de, &c.title_it];
            let races = [&c.race_name_en, &c.race_name_ru, &c.race_name_uk, &c.race_name_es, &c.race_name_de, &c.race_name_it];
            let effects = [&c.effect_en, &c.effect_ru, &c.effect_uk, &c.effect_es, &c.effect_de, &c.effect_it];
            let lores = [&c.lore_en, &c.lore_ru, &c.lore_uk, &c.lore_es, &c.lore_de, &c.lore_it];
            let mut name: Vec<String> = titles.iter().filter(|t| !t.trim().is_empty()).map(|t| t.to_string()).collect();
            name.extend(c.figurine_name.clone());
            name.extend(c.figurine_name_translations.values().cloned());
            let body: Vec<String> = races
                .iter()
                .chain(effects.iter())
                .chain(lores.iter())
                .filter_map(|d| nonempty(d.as_ref()))
                .collect();
            records.push(SearchRecord::from_text(&c.id, &name.join(" "), &body.join(" ")));
            let title = first_of(&[Some(&c.title_en).filter(|t| !t.is_empty()), Some(&c.title_ru).filter(|t| !t.is_empty())]);
            let race = first_of(&[c.race_name_en.as_ref(), c.race_name_ru.as_ref()]);
            let effect = first_of(&[c.effect_en.as_ref(), c.effect_ru.as_ref()]);
            let lore = first_of(&[c.lore_en.as_ref(), c.lore_ru.as_ref()]);
            passages.push((
                c.id.clone(),
                format!("{title} — {race}. {}. {}. {}", c.figurine_name.clone().unwrap_or_default(), cut(&effect, 200), cut(&lore, 200)),
            ));
            kinds.insert(c.id.clone(), SiteKind::Card);
        }

        let vectors = if crate::embed::is_available() && !passages.is_empty() {
            tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<(String, Vec<f32>)>> {
                passages.into_iter().map(|(id, p)| crate::embed::embed_passage(&p).map(|v| (id, v))).collect()
            })
            .await
            .map_err(|e| AppError::Internal(format!("site index task: {e}")))?
            .map_err(|e| AppError::Internal(format!("site index embedding: {e}")))?
        } else {
            Vec::new()
        };
        let index = Arc::new(SiteIndex { built: Instant::now(), records, vectors, kinds });
        *slot = Some(index.clone());
        Ok(index)
    }

    /// Построить индекс заранее (при запуске), чтобы первый поиск не ждал модель.
    pub async fn warm_site_index(&self) -> Result<usize> {
        Ok(self.site_index().await?.records.len())
    }

    /// Найти по всему сайту. Пусто, если запрос пуст или ничего не набрало порога.
    pub async fn site_search(&self, query: &str, limit: usize) -> Result<Vec<SiteHit>> {
        let query: String = query.split_whitespace().collect::<Vec<_>>().join(" ");
        if query.chars().count() < 2 {
            return Ok(Vec::new());
        }
        let key = format!("site|{limit}|{}", query.to_lowercase());
        if let Some((at, hits)) = self.search_cache.lock().await.get(&key)
            && at.elapsed() < SEARCH_CACHE_TTL
        {
            return self.site_hits(hits.clone()).await;
        }
        let hits = self.site_search_uncached(&query, limit).await?;
        let mut cache = self.search_cache.lock().await;
        if cache.len() >= SEARCH_CACHE_MAX {
            cache.clear();
        }
        cache.insert(key, (Instant::now(), hits.clone()));
        drop(cache);
        self.site_hits(hits).await
    }

    /// К оценкам добавляется вид вещи: работы — всё, чего нет среди остальных.
    async fn site_hits(&self, hits: Vec<SemanticHit>) -> Result<Vec<SiteHit>> {
        let index = self.site_index().await?;
        Ok(hits
            .into_iter()
            .map(|h| SiteHit {
                kind: index.kinds.get(&h.id).copied().unwrap_or(SiteKind::Work),
                id: h.id,
                score: h.score,
            })
            .collect())
    }

    async fn site_search_uncached(&self, query: &str, limit: usize) -> Result<Vec<SemanticHit>> {
        let index = self.site_index().await?;

        let fq = FigurineQuery { status: None, search: None, sort: None, page: None, per_page: None };
        let (figs, _) = self.repo.get_all_figurines(true, &fq).await?;
        let ids: Vec<Uuid> = figs.iter().map(|f| f.id).collect();
        let visible: HashSet<Uuid> = ids.iter().copied().collect();
        let translations = self.repo.get_figurine_translations(&ids).await?;
        let mut records: Vec<SearchRecord> = figs
            .iter()
            .map(|f| {
                let rec = SearchRecord::from_figurine(f);
                // Название и описание на других языках: работу находят на любом.
                let texts = translations.get(&f.id);
                let (mut names, mut bodies) = (Vec::new(), Vec::new());
                for t in texts.into_iter().flat_map(|m| m.values()) {
                    names.extend(nonempty(t.name.as_ref()));
                    bodies.extend(nonempty(t.short_text.as_ref()));
                }
                rec.with_extra(&names.join(" "), &bodies.join(" "))
            })
            .collect();
        records.extend(index.records.iter().cloned());

        let mut scores: Vec<(String, f32)> = Vec::new();
        if crate::embed::is_available() {
            let model = crate::embed::model_name();
            let q_owned = query.to_string();
            let qv = tokio::task::spawn_blocking(move || crate::embed::embed_query(&q_owned))
                .await
                .map_err(|e| AppError::Internal(format!("embed task join error: {e}")))?
                .map_err(|e| AppError::Internal(format!("embed failed: {e}")))?;
            let rows: Vec<(Uuid, Vec<u8>)> = sqlx::query_as("SELECT figurine_id, vec FROM figurine_embeddings WHERE model = $1")
                .bind(&model)
                .fetch_all(self.repo.pg_pool())
                .await?;
            scores = rows
                .into_iter()
                .filter(|(id, _)| visible.contains(id))
                .filter_map(|(id, bytes)| crate::embed::from_bytes(&bytes).map(|v| (id.to_string(), crate::embed::dot(&qv, &v))))
                .collect();
            scores.extend(index.vectors.iter().map(|(id, v)| (id.clone(), crate::embed::dot(&qv, v))));
        }

        let mut hits = site_search(query, &records, &scores, limit.clamp(1, 100), BOOST);
        // Порог один и с моделью, и без неё: без вектора оценки ниже, но вещь с
        // оценкой меньше 0,02 всё так же подтверждена одним списком из двух.
        hits.retain(|h| h.score >= FLOOR);
        Ok(hits)
    }
}
