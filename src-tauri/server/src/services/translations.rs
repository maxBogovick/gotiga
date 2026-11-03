//! Массовый перевод работ: выгрузка того, чего не хватает, и вставка готового.
//!
//! Работа написана по-английски в своих колонках, остальные языки лежат в
//! `figurine_translations` (тексты) и `figurine_image_translations` (подписи
//! фото). Вкладка «Переводы» в админке просит у сервера список работ, у которых
//! на нужных языках нет каких-то полей, отдаёт его модели и принимает назад
//! готовый перевод сразу для многих работ.
//!
//! Вставка по умолчанию заполняет только пустое: уже переведённое (в том числе
//! вручную в форме работы) не затирается. С `overwrite` присланные поля заменяют
//! прежние, а не присланные остаются как были.

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Сколько работ отдаёт одна выгрузка, если не просили иначе, и потолок.
const DEFAULT_BATCH: usize = 10;
const MAX_BATCH: usize = 50;
/// Сколько записей принимает одна вставка.
const MAX_IMPORT_ITEMS: usize = 600;
const ALT_MAX_CHARS: usize = 1000;

/// Колонки `figurine_translations` с текстом, в порядке запроса.
const TEXT_COLUMNS: [&str; 14] = [
    "name",
    "short_text",
    "full_description",
    "material",
    "technique",
    "dimensions",
    "series",
    "edition",
    "created_period",
    "care_instructions",
    "provenance_note",
    "authenticity_note",
    "included_items",
    "secret_text",
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationImageTask {
    pub id: Uuid,
    /// Английская подпись фото.
    pub alt_text: String,
    /// Языки, на которых подписи нет.
    pub langs: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationTask {
    pub id: Uuid,
    /// Английское название: чтобы список в админке был читаемым.
    pub name: String,
    pub slug: Option<String>,
    /// Английский текст, только поля, которых не хватает хотя бы на одном языке.
    pub source: serde_json::Value,
    /// Язык → поля, которые надо перевести.
    pub need: BTreeMap<String, Vec<String>>,
    pub images: Vec<TranslationImageTask>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingTranslations {
    /// Сколько всего работ, которым чего-то не хватает на выбранных языках.
    pub total: usize,
    pub items: Vec<TranslationTask>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationImportItem {
    pub id: String,
    pub lang: String,
    #[serde(flatten)]
    pub text: FigurineTextDto,
    /// Фото → подпись на этом языке.
    #[serde(default)]
    pub alts: std::collections::HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationImport {
    pub items: Vec<TranslationImportItem>,
    #[serde(default)]
    pub overwrite: bool,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationImportReport {
    /// Записей (работа + язык), в которых что-то сохранилось.
    pub saved: usize,
    /// Записей, в которых нечего было сохранять (всё уже переведено или пусто).
    pub skipped: usize,
    pub alts: usize,
    /// Отказы по записям: запись пропущена целиком, остальные сохранены.
    pub errors: Vec<String>,
}

fn clean_langs(langs: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for l in langs {
        let l = l.trim().to_lowercase();
        if FigurineTextDto::LANGS.contains(&l.as_str()) && !out.contains(&l) {
            out.push(l);
        }
    }
    out
}

fn text_object(t: &FigurineTextDto) -> serde_json::Map<String, serde_json::Value> {
    match serde_json::to_value(t) {
        Ok(serde_json::Value::Object(m)) => m,
        _ => serde_json::Map::new(),
    }
}

impl AppService {
    /// Работы, у которых на `langs` нет каких-то полей или подписей фото.
    /// Отдаёт `limit` работ, пропустив `skip`: вставленное выпадает из
    /// выгрузки само, `skip` нужен, только если часть пропустили намеренно.
    pub async fn missing_translations(
        &self,
        langs: &[String],
        limit: Option<usize>,
        skip: Option<usize>,
    ) -> Result<MissingTranslations> {
        let langs = clean_langs(langs);
        if langs.is_empty() {
            return Err(AppError::BadRequest("no languages".into()));
        }
        let limit = limit.unwrap_or(DEFAULT_BATCH).clamp(1, MAX_BATCH);
        let skip = skip.unwrap_or(0);

        let figs: Vec<Figurine> = sqlx::query_as(
            "SELECT * FROM figurines ORDER BY sort_order, created_at DESC, id",
        )
        .fetch_all(self.repo.pg_pool())
        .await?;
        let ids: Vec<Uuid> = figs.iter().map(|f| f.id).collect();
        let have = self.repo.get_figurine_translations(&ids).await?;

        let images: Vec<(Uuid, Uuid, String)> = sqlx::query_as(
            "SELECT id, figurine_id, btrim(alt_text) FROM images
              WHERE figurine_id = ANY($1) AND alt_text IS NOT NULL AND btrim(alt_text) <> ''
              ORDER BY figurine_id, id",
        )
        .bind(&ids)
        .fetch_all(self.repo.pg_pool())
        .await?;
        let alt_rows: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT image_id, lang FROM figurine_image_translations WHERE figurine_id = ANY($1)",
        )
        .bind(&ids)
        .fetch_all(self.repo.pg_pool())
        .await?;
        let mut alt_have: std::collections::HashMap<Uuid, BTreeSet<String>> =
            std::collections::HashMap::new();
        for (image, lang) in alt_rows {
            alt_have.entry(image).or_default().insert(lang);
        }
        let mut images_by_work: std::collections::HashMap<Uuid, Vec<(Uuid, String)>> =
            std::collections::HashMap::new();
        for (image, work, alt) in images {
            images_by_work.entry(work).or_default().push((image, alt));
        }

        let mut tasks: Vec<TranslationTask> = Vec::new();
        for f in &figs {
            let source_dto = FigurineTextDto {
                name: Some(f.name.clone()),
                short_text: f.short_text.clone(),
                full_description: f.full_description.clone(),
                material: f.material.clone(),
                technique: f.technique.clone(),
                dimensions: f.dimensions.clone(),
                series: f.series.clone(),
                edition: f.edition.clone(),
                created_period: f.created_period.clone(),
                care_instructions: f.care_instructions.clone(),
                provenance_note: f.provenance_note.clone(),
                authenticity_note: f.authenticity_note.clone(),
                included_items: f.included_items.clone(),
                secret_text: f.secret_text.clone(),
            }
            .cleaned();
            let source = text_object(&source_dto);

            let mut need: BTreeMap<String, Vec<String>> = BTreeMap::new();
            let mut wanted: BTreeSet<String> = BTreeSet::new();
            for lang in &langs {
                let existing = have
                    .get(&f.id)
                    .and_then(|m| m.get(lang))
                    .map(text_object)
                    .unwrap_or_default();
                let fields: Vec<String> = source
                    .keys()
                    .filter(|k| !existing.contains_key(*k))
                    .cloned()
                    .collect();
                if !fields.is_empty() {
                    wanted.extend(fields.iter().cloned());
                    need.insert(lang.clone(), fields);
                }
            }

            let mut image_tasks: Vec<TranslationImageTask> = Vec::new();
            for (image, alt) in images_by_work.get(&f.id).into_iter().flatten() {
                let done = alt_have.get(image);
                let lacking: Vec<String> = langs
                    .iter()
                    .filter(|l| !done.is_some_and(|d| d.contains(*l)))
                    .cloned()
                    .collect();
                if !lacking.is_empty() {
                    image_tasks.push(TranslationImageTask {
                        id: *image,
                        alt_text: alt.clone(),
                        langs: lacking,
                    });
                }
            }

            if need.is_empty() && image_tasks.is_empty() {
                continue;
            }
            let source: serde_json::Map<String, serde_json::Value> = source
                .into_iter()
                .filter(|(k, _)| wanted.contains(k))
                .collect();
            tasks.push(TranslationTask {
                id: f.id,
                name: f.name.clone(),
                slug: f.slug.clone(),
                source: serde_json::Value::Object(source),
                need,
                images: image_tasks,
            });
        }

        let total = tasks.len();
        let items = tasks.into_iter().skip(skip).take(limit).collect();
        Ok(MissingTranslations { total, items })
    }

    /// Сохранить переводы многих работ разом. Запись, которая не прошла
    /// проверку, пропускается с пояснением; остальные сохраняются.
    pub async fn import_translations(
        &self,
        req: TranslationImport,
    ) -> Result<TranslationImportReport> {
        if req.items.len() > MAX_IMPORT_ITEMS {
            return Err(AppError::BadRequest(format!(
                "too many items (max {MAX_IMPORT_ITEMS})"
            )));
        }
        // Пустое сохраняется как «не прислано»; непустое при заполнении пустого
        // не трогает прежнее, при замене — перекрывает его.
        let merge = |col: &str| {
            if req.overwrite {
                format!("{col} = COALESCE(EXCLUDED.{col}, figurine_translations.{col})")
            } else {
                format!(
                    "{col} = COALESCE(NULLIF(btrim(figurine_translations.{col}), ''), EXCLUDED.{col})"
                )
            }
        };
        let sets = TEXT_COLUMNS.iter().map(|c| merge(c)).collect::<Vec<_>>().join(", ");
        let upsert = format!(
            "INSERT INTO figurine_translations
                (figurine_id, lang, name, short_text, full_description, material,
                 technique, dimensions, series, edition, created_period,
                 care_instructions, provenance_note, authenticity_note,
                 included_items, secret_text, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, now())
             ON CONFLICT (figurine_id, lang) DO UPDATE SET {sets}, updated_at = now()"
        );
        // Строка затрагивается всегда, даже когда все поля уже заполнены, поэтому
        // «сохранено» считаем сравнением текстов до и после, а не числом строк.

        let mut report = TranslationImportReport::default();
        for (n, item) in req.items.into_iter().enumerate() {
            let at = format!("#{} ({} {})", n + 1, item.id, item.lang);
            let lang = item.lang.trim().to_lowercase();
            if !FigurineTextDto::LANGS.contains(&lang.as_str()) {
                report.errors.push(format!("{at}: unknown language"));
                continue;
            }
            let Ok(id) = Uuid::parse_str(item.id.trim()) else {
                report.errors.push(format!("{at}: bad id"));
                continue;
            };
            let text = item.text.cleaned();
            if let Some(bad) = text
                .limits()
                .iter()
                .find(|(_, v, max)| v.as_ref().is_some_and(|s| s.chars().count() > *max))
            {
                report.errors.push(format!("{at}: field {} is too long", bad.0));
                continue;
            }
            let exists: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM figurines WHERE id = $1")
                .bind(id)
                .fetch_optional(self.repo.pg_pool())
                .await?;
            if exists.is_none() {
                report.errors.push(format!("{at}: work not found"));
                continue;
            }

            let mut changed = false;
            if !text.is_empty() {
                let before = self.repo.get_figurine_translations(&[id]).await?.remove(&id);
                let before = before.and_then(|mut m| m.remove(&lang));
                sqlx::query(&upsert)
                    .bind(id)
                    .bind(&lang)
                    .bind(&text.name)
                    .bind(&text.short_text)
                    .bind(&text.full_description)
                    .bind(&text.material)
                    .bind(&text.technique)
                    .bind(&text.dimensions)
                    .bind(&text.series)
                    .bind(&text.edition)
                    .bind(&text.created_period)
                    .bind(&text.care_instructions)
                    .bind(&text.provenance_note)
                    .bind(&text.authenticity_note)
                    .bind(&text.included_items)
                    .bind(&text.secret_text)
                    .execute(self.repo.pg_pool())
                    .await?;
                let after = self.repo.get_figurine_translations(&[id]).await?.remove(&id);
                let after = after.and_then(|mut m| m.remove(&lang));
                changed = before != after;
            }

            let mut alts_saved = 0usize;
            for (image, alt) in &item.alts {
                let alt = alt.trim();
                let Ok(image) = Uuid::parse_str(image.trim()) else {
                    report.errors.push(format!("{at}: bad photo id {image}"));
                    continue;
                };
                if alt.is_empty() || alt.chars().count() > ALT_MAX_CHARS {
                    report.errors.push(format!("{at}: photo {image}: empty or too long"));
                    continue;
                }
                let on_conflict = if req.overwrite {
                    "DO UPDATE SET alt_text = EXCLUDED.alt_text, updated_at = now()"
                } else {
                    "DO NOTHING"
                };
                let res = sqlx::query(&format!(
                    "INSERT INTO figurine_image_translations (image_id, figurine_id, lang, alt_text)
                     SELECT i.id, i.figurine_id, $3, $4 FROM images i
                      WHERE i.id = $1 AND i.figurine_id = $2
                     ON CONFLICT (image_id, lang) {on_conflict}"
                ))
                .bind(image)
                .bind(id)
                .bind(&lang)
                .bind(alt)
                .execute(self.repo.pg_pool())
                .await?;
                alts_saved += res.rows_affected() as usize;
            }
            report.alts += alts_saved;

            if changed || alts_saved > 0 {
                report.saved += 1;
            } else {
                report.skipped += 1;
            }
        }

        if report.saved > 0 {
            self.forget_site_lexical().await;
        }
        Ok(report)
    }
}
