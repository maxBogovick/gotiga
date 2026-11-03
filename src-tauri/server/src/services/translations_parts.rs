//! Массовый перевод не работ, а остального: байки, листы газеты, карты, правки
//! текстов сайта, поручения и этюды, расы, ключевые слова, рамки, движения.
//!
//! У работ переводы лежат в своих таблицах (`translations.rs`). Здесь тексты
//! лежат где придётся: в колонках `title_en … title_it` одной таблицы, в JSON
//! настройки (`battle_frames`), в JSON колонки карты (`traits`, `abilities`), в
//! общей карте правок (`copy_overrides`). Каждое такое место описано одной
//! «частью» (`PartDef`), а выгрузка и вставка одни на все: часть умеет только
//! прочитать свои единицы и записать одно поле одного языка.
//!
//! Правила те же, что у работ: вставка по умолчанию заполняет только пустое,
//! запись с ошибкой пропускается с пояснением, остальные сохраняются.

use super::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Все языки, в порядке, в котором их печатают словари.
const ALL_LANGS: [&str; 6] = ["en", "ru", "uk", "es", "de", "it"];
const DEFAULT_BATCH: usize = 20;
const MAX_BATCH: usize = 100;
const MAX_IMPORT_ITEMS: usize = 1500;

struct Field {
    name: &'static str,
    max: usize,
}

enum Kind {
    /// Колонки `{поле}_{язык}` одной таблицы; ключ — `id`.
    Cols { table: &'static str, filter: &'static str },
    /// JSON-массив в колонке строки (`battle_cards.traits`): поля
    /// `{поле}{Язык}` у элемента. Ключ — `{id строки}#{индекс или id элемента}`.
    RowArray {
        table: &'static str,
        column: &'static str,
        element_id: Option<&'static str>,
    },
    /// JSON-массив в настройке (`battle_frames` → `frames`).
    SettingArray {
        key: &'static str,
        array: &'static str,
        id_field: &'static str,
    },
    /// Правки текстов сайта: `{язык: {ключ словаря: текст}}`.
    Copy,
}

struct PartDef {
    code: &'static str,
    section: &'static str,
    kind: Kind,
    fields: &'static [Field],
    /// Колонка с названием для подписи в списке (для `Cols` — первое поле).
    label_col: Option<&'static str>,
}

const fn f(name: &'static str, max: usize) -> Field {
    Field { name, max }
}

static PARTS: &[PartDef] = &[
    PartDef {
        code: "tale",
        section: "tales",
        kind: Kind::Cols { table: "gazette_leaves", filter: "kind = 'tale'" },
        fields: &[f("title", 200), f("dek", 500), f("body", 12000)],
        label_col: None,
    },
    PartDef {
        code: "leaf",
        section: "gazette",
        kind: Kind::Cols { table: "gazette_leaves", filter: "kind <> 'tale'" },
        fields: &[f("title", 200), f("dek", 500), f("body", 12000)],
        label_col: None,
    },
    PartDef {
        code: "card",
        section: "cards",
        kind: Kind::Cols { table: "battle_cards", filter: "TRUE" },
        fields: &[f("title", 80), f("type", 40), f("effect", 400), f("lore", 400)],
        label_col: None,
    },
    PartDef {
        code: "trait",
        section: "cards",
        kind: Kind::RowArray { table: "battle_cards", column: "traits", element_id: None },
        fields: &[f("name", 60), f("text", 200)],
        label_col: Some("title_en"),
    },
    PartDef {
        code: "ability",
        section: "cards",
        kind: Kind::RowArray { table: "battle_cards", column: "abilities", element_id: Some("id") },
        fields: &[f("name", 60)],
        label_col: Some("title_en"),
    },
    PartDef {
        code: "copy",
        section: "site",
        kind: Kind::Copy,
        fields: &[f("text", 4000)],
        label_col: None,
    },
    PartDef {
        code: "errand",
        section: "errands",
        kind: Kind::Cols { table: "battle_errands", filter: "TRUE" },
        fields: &[f("title", 120), f("note", 400)],
        label_col: None,
    },
    PartDef {
        code: "challenge",
        section: "errands",
        kind: Kind::Cols { table: "battle_challenges", filter: "TRUE" },
        fields: &[f("title", 80), f("note", 400)],
        label_col: None,
    },
    PartDef {
        code: "race",
        section: "terms",
        kind: Kind::Cols { table: "battle_races", filter: "TRUE" },
        fields: &[f("name", 60), f("note", 200)],
        label_col: None,
    },
    PartDef {
        code: "studio_race",
        section: "terms",
        kind: Kind::Cols { table: "studio_races", filter: "TRUE" },
        fields: &[f("name", 60), f("note", 200)],
        label_col: None,
    },
    PartDef {
        code: "keyword",
        section: "terms",
        kind: Kind::Cols { table: "battle_keywords", filter: "TRUE" },
        fields: &[f("name", 60), f("rules", 300)],
        label_col: None,
    },
    PartDef {
        code: "frame",
        section: "terms",
        kind: Kind::SettingArray { key: "battle_frames", array: "frames", id_field: "tier" },
        fields: &[f("name", 60)],
        label_col: None,
    },
    PartDef {
        code: "motion",
        section: "terms",
        kind: Kind::SettingArray { key: "battle_motions", array: "motions", id_field: "id" },
        fields: &[f("name", 80)],
        label_col: None,
    },
];

/// Разделы вкладки, кроме «Работ», в порядке показа.
pub const SECTIONS: [&str; 6] = ["tales", "gazette", "cards", "site", "errands", "terms"];

fn cap(lang: &str) -> String {
    let mut c = lang.chars();
    match c.next() {
        Some(h) => h.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn blank(v: Option<&str>) -> bool {
    v.map(|s| s.trim().is_empty()).unwrap_or(true)
}

/// Одна единица перевода: строка таблицы, элемент массива, ключ правок.
struct Unit {
    key: String,
    label: String,
    /// Поле → язык → непустой текст.
    texts: BTreeMap<&'static str, BTreeMap<&'static str, String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionTask {
    /// `{часть}:{ключ}` — по нему вставка находит единицу.
    pub id: String,
    pub part: String,
    pub name: String,
    /// Поле → язык → текст. Только поля, которые нужны хотя бы на одном языке.
    pub source: BTreeMap<String, BTreeMap<String, String>>,
    /// Язык → поля, которые надо перевести.
    pub need: BTreeMap<String, Vec<String>>,
    /// Поле → предел длины в знаках.
    pub limits: BTreeMap<String, usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionMissing {
    pub total: usize,
    pub items: Vec<SectionTask>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationSectionCount {
    pub id: &'static str,
    pub missing: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionImportItem {
    pub id: String,
    pub lang: String,
    #[serde(flatten)]
    pub fields: std::collections::HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionImport {
    pub section: String,
    pub items: Vec<SectionImportItem>,
    #[serde(default)]
    pub overwrite: bool,
}

fn parts_of(section: &str) -> Vec<&'static PartDef> {
    PARTS.iter().filter(|p| p.section == section).collect()
}

fn clean_langs(langs: &[String]) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for l in langs {
        let l = l.trim().to_lowercase();
        if let Some(known) = ALL_LANGS.iter().find(|k| **k == l)
            && !out.contains(known)
        {
            out.push(known);
        }
    }
    out
}

fn text_of(v: &Value) -> Option<String> {
    v.as_str().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string)
}

impl AppService {
    async fn load_units(&self, part: &PartDef) -> Result<Vec<Unit>> {
        let pool = self.repo.pg_pool();
        let mut out: Vec<Unit> = Vec::new();
        match &part.kind {
            Kind::Cols { table, filter } => {
                let cols: Vec<String> = part
                    .fields
                    .iter()
                    .flat_map(|fl| ALL_LANGS.iter().map(move |l| format!("\"{}_{}\"", fl.name, l)))
                    .collect();
                let sql = format!(
                    "SELECT row_to_json(t)::text FROM
                       (SELECT id::text AS id, {} FROM {table} WHERE {filter} ORDER BY id) t",
                    cols.join(", ")
                );
                let rows: Vec<(String,)> = sqlx::query_as(&sql).fetch_all(pool).await?;
                for (raw,) in rows {
                    let row: Value = serde_json::from_str(&raw)
                        .map_err(|e| AppError::Internal(e.to_string()))?;
                    let key = row["id"].as_str().unwrap_or_default().to_string();
                    let mut texts: BTreeMap<&'static str, BTreeMap<&'static str, String>> =
                        BTreeMap::new();
                    for fl in part.fields {
                        for l in ALL_LANGS {
                            if let Some(s) = text_of(&row[format!("{}_{}", fl.name, l)]) {
                                texts.entry(fl.name).or_default().insert(l, s);
                            }
                        }
                    }
                    let first = part.fields[0].name;
                    let label = ["en", "ru"]
                        .iter()
                        .find_map(|l| texts.get(first).and_then(|m| m.get(l)).cloned())
                        .unwrap_or_else(|| key.clone());
                    out.push(Unit { key, label, texts });
                }
            }
            Kind::RowArray { table, column, element_id } => {
                let label_col = part.label_col.unwrap_or("title_en");
                let sql = format!(
                    "SELECT row_to_json(t)::text FROM
                       (SELECT id::text AS id, {label_col} AS label, {column} AS data
                          FROM {table} WHERE {column} IS NOT NULL ORDER BY id) t"
                );
                let rows: Vec<(String,)> = sqlx::query_as(&sql).fetch_all(pool).await?;
                for (raw,) in rows {
                    let row: Value = serde_json::from_str(&raw)
                        .map_err(|e| AppError::Internal(e.to_string()))?;
                    let card = row["id"].as_str().unwrap_or_default();
                    let title = row["label"].as_str().unwrap_or_default();
                    let Some(data) = row["data"].as_str().and_then(|s| serde_json::from_str::<Value>(s).ok())
                    else {
                        continue;
                    };
                    let Some(list) = data.as_array() else { continue };
                    for (i, el) in list.iter().enumerate() {
                        let ek = match element_id {
                            Some(idf) => match el[*idf].as_str().filter(|s| !s.is_empty()) {
                                Some(s) => s.to_string(),
                                None => continue,
                            },
                            None => i.to_string(),
                        };
                        let unit = unit_from_object(part, el, format!("{card}#{ek}"), title);
                        out.push(unit);
                    }
                }
            }
            Kind::SettingArray { key, array, id_field } => {
                let raw = self.repo.get_setting(key).await?;
                let Some(v) = raw.and_then(|s| serde_json::from_str::<Value>(&s).ok()) else {
                    return Ok(out);
                };
                for el in v[*array].as_array().into_iter().flatten() {
                    let id = match &el[*id_field] {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        _ => continue,
                    };
                    out.push(unit_from_object(part, el, id.clone(), &id));
                }
            }
            Kind::Copy => {
                let raw = self.repo.get_setting("copy_overrides").await?;
                let v: Value = raw
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or(Value::Null);
                let mut keys: BTreeMap<String, BTreeMap<&'static str, String>> = BTreeMap::new();
                for l in ALL_LANGS {
                    for (k, t) in v[l].as_object().into_iter().flatten() {
                        if let Some(s) = text_of(t) {
                            keys.entry(k.clone()).or_default().insert(l, s);
                        }
                    }
                }
                for (k, by_lang) in keys {
                    let mut texts = BTreeMap::new();
                    texts.insert("text", by_lang);
                    out.push(Unit { label: k.clone(), key: k, texts });
                }
            }
        }
        Ok(out)
    }

    /// Единицы одного раздела со всем, чего им не хватает на `langs`.
    async fn section_tasks(&self, section: &str, langs: &[&'static str]) -> Result<Vec<SectionTask>> {
        let mut tasks = Vec::new();
        for part in parts_of(section) {
            for unit in self.load_units(part).await? {
                let mut need: BTreeMap<String, Vec<String>> = BTreeMap::new();
                for lang in langs {
                    let fields: Vec<String> = part
                        .fields
                        .iter()
                        .filter(|fl| {
                            let by_lang = unit.texts.get(fl.name);
                            let has_source = by_lang.is_some_and(|m| m.keys().any(|k| k != lang));
                            let has_own = by_lang.is_some_and(|m| m.contains_key(lang));
                            has_source && !has_own
                        })
                        .map(|fl| fl.name.to_string())
                        .collect();
                    if !fields.is_empty() {
                        need.insert(lang.to_string(), fields);
                    }
                }
                if need.is_empty() {
                    continue;
                }
                let wanted: std::collections::BTreeSet<&String> = need.values().flatten().collect();
                let source = unit
                    .texts
                    .iter()
                    .filter(|(k, _)| wanted.iter().any(|w| w.as_str() == **k))
                    .map(|(k, m)| {
                        (k.to_string(), m.iter().map(|(l, s)| (l.to_string(), s.clone())).collect())
                    })
                    .collect();
                tasks.push(SectionTask {
                    id: format!("{}:{}", part.code, unit.key),
                    part: part.code.to_string(),
                    name: unit.label,
                    source,
                    need,
                    limits: part.fields.iter().map(|fl| (fl.name.to_string(), fl.max)).collect(),
                });
            }
        }
        Ok(tasks)
    }

    /// Сколько единиц каждого раздела без перевода на `langs`.
    pub async fn translation_section_counts(&self, langs: &[String]) -> Result<Vec<TranslationSectionCount>> {
        let langs = clean_langs(langs);
        let mut out = Vec::new();
        for id in SECTIONS {
            out.push(TranslationSectionCount { id, missing: self.section_tasks(id, &langs).await?.len() });
        }
        Ok(out)
    }

    pub async fn missing_section(
        &self,
        section: &str,
        langs: &[String],
        limit: Option<usize>,
        skip: Option<usize>,
    ) -> Result<SectionMissing> {
        if !SECTIONS.contains(&section) {
            return Err(AppError::BadRequest("unknown section".into()));
        }
        let langs = clean_langs(langs);
        if langs.is_empty() {
            return Err(AppError::BadRequest("no languages".into()));
        }
        let limit = limit.unwrap_or(DEFAULT_BATCH).clamp(1, MAX_BATCH);
        let tasks = self.section_tasks(section, &langs).await?;
        let total = tasks.len();
        let items = tasks.into_iter().skip(skip.unwrap_or(0)).take(limit).collect();
        Ok(SectionMissing { total, items })
    }

    pub async fn import_section(
        &self,
        req: SectionImport,
    ) -> Result<crate::services::TranslationImportReport> {
        use crate::services::TranslationImportReport as Report;
        if !SECTIONS.contains(&req.section.as_str()) {
            return Err(AppError::BadRequest("unknown section".into()));
        }
        if req.items.len() > MAX_IMPORT_ITEMS {
            return Err(AppError::BadRequest(format!("too many items (max {MAX_IMPORT_ITEMS})")));
        }
        let parts = parts_of(&req.section);
        let mut report = Report::default();
        for (n, item) in req.items.into_iter().enumerate() {
            let at = format!("#{} ({} {})", n + 1, item.id, item.lang);
            let lang = item.lang.trim().to_lowercase();
            let Some(lang) = ALL_LANGS.iter().find(|l| **l == lang).copied() else {
                report.errors.push(format!("{at}: unknown language"));
                continue;
            };
            let Some((code, key)) = item.id.split_once(':') else {
                report.errors.push(format!("{at}: bad id"));
                continue;
            };
            let Some(part) = parts.iter().find(|p| p.code == code) else {
                report.errors.push(format!("{at}: unknown part"));
                continue;
            };
            let mut values: Vec<(&'static Field, String)> = Vec::new();
            let mut bad = false;
            for (name, v) in &item.fields {
                if name == "alts" {
                    continue;
                }
                let Some(fl) = part.fields.iter().find(|fl| fl.name == name) else {
                    continue;
                };
                let Some(s) = text_of(v) else { continue };
                if s.chars().count() > fl.max {
                    report.errors.push(format!("{at}: field {name} is too long"));
                    bad = true;
                    break;
                }
                values.push((fl, s));
            }
            if bad {
                continue;
            }
            if values.is_empty() {
                report.skipped += 1;
                continue;
            }
            match self.store_texts(part, key, lang, &values, req.overwrite).await {
                Ok(true) => report.saved += 1,
                Ok(false) => report.skipped += 1,
                Err(AppError::NotFound(m)) => report.errors.push(format!("{at}: {m}")),
                Err(e) => return Err(e),
            }
        }
        if report.saved > 0 {
            self.forget_site_lexical().await;
        }
        Ok(report)
    }

    /// Записать поля одной единицы на одном языке. `true` — что-то изменилось.
    async fn store_texts(
        &self,
        part: &PartDef,
        key: &str,
        lang: &'static str,
        values: &[(&'static Field, String)],
        overwrite: bool,
    ) -> Result<bool> {
        let pool = self.repo.pg_pool();
        match &part.kind {
            Kind::Cols { table, filter } => {
                let id = Uuid::parse_str(key).map_err(|_| AppError::NotFound("bad id".into()))?;
                let mut changed = false;
                let mut found = false;
                for (fl, text) in values {
                    let col = format!("\"{}_{}\"", fl.name, lang);
                    let guard = if overwrite {
                        String::new()
                    } else {
                        format!(" AND ({col} IS NULL OR btrim({col}) = '')")
                    };
                    let exists: Option<(Uuid,)> = sqlx::query_as(&format!(
                        "SELECT id FROM {table} WHERE id = $1 AND {filter}"
                    ))
                    .bind(id)
                    .fetch_optional(pool)
                    .await?;
                    found = exists.is_some();
                    if !found {
                        break;
                    }
                    let res = sqlx::query(&format!(
                        "UPDATE {table} SET {col} = $2 WHERE id = $1 AND {filter}{guard}"
                    ))
                    .bind(id)
                    .bind(text)
                    .execute(pool)
                    .await?;
                    changed |= res.rows_affected() > 0;
                }
                if !found {
                    return Err(AppError::NotFound("not found".into()));
                }
                Ok(changed)
            }
            Kind::RowArray { table, column, element_id } => {
                let (card, el_key) = key
                    .split_once('#')
                    .ok_or_else(|| AppError::NotFound("bad id".into()))?;
                let id = Uuid::parse_str(card).map_err(|_| AppError::NotFound("bad id".into()))?;
                let mut tx = pool.begin().await?;
                let row: Option<(Option<String>,)> = sqlx::query_as(&format!(
                    "SELECT {column} FROM {table} WHERE id = $1 FOR UPDATE"
                ))
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;
                let Some((Some(raw),)) = row else {
                    return Err(AppError::NotFound("not found".into()));
                };
                let mut data: Value = serde_json::from_str(&raw)
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                let list = data
                    .as_array_mut()
                    .ok_or_else(|| AppError::NotFound("not found".into()))?;
                let el = match element_id {
                    Some(idf) => list.iter_mut().find(|e| e[*idf].as_str() == Some(el_key)),
                    None => el_key.parse::<usize>().ok().and_then(|i| list.get_mut(i)),
                }
                .ok_or_else(|| AppError::NotFound("not found".into()))?;
                let changed = set_object_fields(el, lang, values, overwrite);
                if changed {
                    let json = serde_json::to_string(&data)
                        .map_err(|e| AppError::Internal(e.to_string()))?;
                    sqlx::query(&format!("UPDATE {table} SET {column} = $2 WHERE id = $1"))
                        .bind(id)
                        .bind(json)
                        .execute(&mut *tx)
                        .await?;
                }
                tx.commit().await?;
                Ok(changed)
            }
            Kind::SettingArray { key: setting, array, id_field } => {
                let raw = self.repo.get_setting(setting).await?;
                let mut v: Value = raw
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .ok_or_else(|| AppError::NotFound("not found".into()))?;
                let list = v[*array]
                    .as_array_mut()
                    .ok_or_else(|| AppError::NotFound("not found".into()))?;
                let el = list
                    .iter_mut()
                    .find(|e| match &e[*id_field] {
                        Value::String(s) => s == key,
                        Value::Number(n) => n.to_string() == key,
                        _ => false,
                    })
                    .ok_or_else(|| AppError::NotFound("not found".into()))?;
                let changed = set_object_fields(el, lang, values, overwrite);
                if changed {
                    let json = serde_json::to_string(&v)
                        .map_err(|e| AppError::Internal(e.to_string()))?;
                    self.repo.upsert_setting(setting, &json).await?;
                }
                Ok(changed)
            }
            Kind::Copy => {
                let raw = self.repo.get_setting("copy_overrides").await?;
                let mut v: Value = raw
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_else(|| Value::Object(Default::default()));
                // Новый ключ можно завести только там, где он уже есть на другом языке.
                let known = ALL_LANGS.iter().any(|l| v[*l].get(key).is_some());
                if !known {
                    return Err(AppError::NotFound("not found".into()));
                }
                let Some((_, text)) = values.first() else { return Ok(false) };
                let slot = &mut v[lang];
                if !slot.is_object() {
                    *slot = Value::Object(Default::default());
                }
                let map = slot.as_object_mut().expect("object");
                if !overwrite && !blank(map.get(key).and_then(Value::as_str)) {
                    return Ok(false);
                }
                map.insert(key.to_string(), Value::String(text.clone()));
                let json = serde_json::to_string(&v).map_err(|e| AppError::Internal(e.to_string()))?;
                if json.len() > 500 * 1024 {
                    return Err(AppError::BadRequest("Copy overrides are too large".into()));
                }
                self.repo.upsert_setting("copy_overrides", &json).await?;
                Ok(true)
            }
        }
    }
}

/// Единица из JSON-объекта с полями `{поле}{Язык}`.
fn unit_from_object(part: &PartDef, el: &Value, key: String, fallback: &str) -> Unit {
    let mut texts: BTreeMap<&'static str, BTreeMap<&'static str, String>> = BTreeMap::new();
    for fl in part.fields {
        for l in ALL_LANGS {
            if let Some(s) = text_of(&el[format!("{}{}", fl.name, cap(l))]) {
                texts.entry(fl.name).or_default().insert(l, s);
            }
        }
    }
    let first = part.fields[0].name;
    let own = ["en", "ru"]
        .iter()
        .find_map(|l| texts.get(first).and_then(|m| m.get(l)).cloned())
        .unwrap_or_default();
    let label = if own.is_empty() || own == fallback {
        fallback.to_string()
    } else if fallback.is_empty() {
        own
    } else {
        format!("{fallback} · {own}")
    };
    Unit { key, label, texts }
}

/// Записать `{поле}{Язык}` в объект; пустое заполняется всегда, занятое — при `overwrite`.
fn set_object_fields(el: &mut Value, lang: &str, values: &[(&Field, String)], overwrite: bool) -> bool {
    let Some(obj) = el.as_object_mut() else { return false };
    let mut changed = false;
    for (fl, text) in values {
        let name = format!("{}{}", fl.name, cap(lang));
        if !overwrite && !blank(obj.get(&name).and_then(Value::as_str)) {
            continue;
        }
        obj.insert(name, Value::String(text.clone()));
        changed = true;
    }
    changed
}
