//! Hybrid retrieval for the keeper ("Хранитель").
//!
//! Three retrievers over a tiny in-memory cabinet, fused with Reciprocal Rank
//! Fusion so their scores are never compared as raw numbers:
//!
//! 1. Dense vectors supplied by the caller (multilingual-e5 cosine).
//! 2. Word BM25 on analyzed tokens. Query terms are expanded against the
//!    cabinet vocabulary with Elasticsearch `fuzziness: AUTO` (0 / 1 / 2 edits
//!    by term length, and only among similar-length words).
//! 3. Character trigrams scored like Postgres `pg_trgm.word_similarity` — the
//!    fraction of the query's trigrams that appear in a document token. This is
//!    how a partial stem ranks against a longer word; document-level Jaccard
//!    does not.
//!
//! Tokens are kept in their own script (after Unicode casefold). A second
//! Latin form from `deunicode` is indexed alongside, so a Latin query and a
//! Cyrillic name can meet without a hand-written transliteration table.
//! `visual_caption` / `pinterest_description` are searched here and never
//! serialised onto a public DTO.

use rust_stemmers::{Algorithm, Stemmer};
use std::collections::{HashMap, HashSet};

use crate::models::{Figurine, SemanticHit};

const RRF_K: f32 = 60.0;
const RETRIEVER_CAP: usize = 24;
const BM25_K1: f32 = 1.2;
/// Postgres `pg_trgm.word_similarity_threshold` default.
const WORD_SIM_FLOOR: f32 = 0.6;

const W_NAME: f32 = 4.0;
const W_SHORT: f32 = 2.2;
const W_DIM_MAT_TECH: f32 = 2.6;
const W_BODY: f32 = 1.0;
const W_BACKSTAGE: f32 = 1.2;

#[derive(Clone, Debug)]
pub struct SearchRecord {
    pub id: String,
    name: String,
    short_text: Option<String>,
    full_description: Option<String>,
    dimensions: Option<String>,
    material: Option<String>,
    technique: Option<String>,
    visual_caption: Option<String>,
    pinterest_description: Option<String>,
}

#[cfg_attr(not(test), allow(dead_code))] // `weight` читает только прежняя реализация (тесты)
struct WeightedField {
    /// Native-script tokens, one per occurrence (term frequency).
    tokens: Vec<String>,
    /// Native token plus its Latin fold — unique; matching and trigrams.
    keys: Vec<String>,
    weight: f32,
}

impl SearchRecord {
    pub fn from_figurine(f: &Figurine) -> Self {
        Self {
            id: f.id.to_string(),
            name: f.name.clone(),
            short_text: f.short_text.clone(),
            full_description: f.full_description.clone(),
            dimensions: f.dimensions.clone(),
            material: f.material.clone(),
            technique: f.technique.clone(),
            visual_caption: f.visual_caption.clone(),
            pinterest_description: f.pinterest_description.clone(),
        }
    }

    /// Дописать название и описание на других языках: поиск по словам находит
    /// вещь на любом из них, а не только на том, что лежит в основных полях.
    pub fn with_extra(mut self, name: &str, body: &str) -> Self {
        if !name.trim().is_empty() {
            self.name = format!("{} {}", self.name, name);
        }
        if !body.trim().is_empty() {
            self.full_description = Some(match self.full_description.take() {
                Some(d) => format!("{d} {body}"),
                None => body.to_string(),
            });
        }
        self
    }

    /// Запись из произвольного текста (каталог «Спросить сайт»): первая
    /// формулировка идёт в имя, остальные — в тело.
    pub fn from_text(id: &str, name: &str, body: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            short_text: None,
            full_description: Some(body.to_string()),
            dimensions: None,
            material: None,
            technique: None,
            visual_caption: None,
            pinterest_description: None,
        }
    }

    fn fields(&self) -> [(&str, f32); 8] {
        [
            (self.name.as_str(), W_NAME),
            (self.short_text.as_deref().unwrap_or(""), W_SHORT),
            (self.full_description.as_deref().unwrap_or(""), W_BODY),
            (self.dimensions.as_deref().unwrap_or(""), W_DIM_MAT_TECH),
            (self.material.as_deref().unwrap_or(""), W_DIM_MAT_TECH),
            (self.technique.as_deref().unwrap_or(""), W_DIM_MAT_TECH),
            (self.visual_caption.as_deref().unwrap_or(""), W_BACKSTAGE),
            (
                self.pinterest_description.as_deref().unwrap_or(""),
                W_BACKSTAGE,
            ),
        ]
    }

    #[allow(dead_code)] // прежняя реализация поиска в тестах (`legacy`)
    fn analyzed(&self) -> Vec<WeightedField> {
        self.fields()
            .into_iter()
            .map(|(text, weight)| analyze_field(text, weight))
            .collect()
    }
}

/// Сколько полей у записи (`SearchRecord::fields`).
const FIELDS: usize = 8;

/// Слово текста без повторов: его написания и где оно встретилось.
struct IndexedWord {
    forms: Vec<String>,
    /// (запись, поле, сколько раз слово стоит в поле)
    seen: Vec<(u32, u8, u32)>,
}

/// Набор записей, разобранный один раз.
///
/// Тексты режутся на слова, получают латинскую запись и основы при сборке индекса, а не на
/// каждый запрос: прежде каждый запрос заново разбирал все тексты (около 71 тысячи слов,
/// байки на шести языках) минимум пять раз. Теперь запрос разбирает только свои слова и
/// сверяет их с готовыми. Ответы те же, что у прежней реализации, до последнего знака:
/// это проверяет тест `index_gives_the_same_answers_as_the_per_query_analysis`.
pub struct SearchIndex {
    ids: Vec<String>,
    by_id: HashMap<String, usize>,
    /// Вес каждого из восьми полей (у всех записей одинаков).
    weights: [f32; FIELDS],
    words: Vec<IndexedWord>,
    /// Словарь для исправления опечаток: написания слов без основ.
    vocab: HashSet<String>,
    /// Все написания записи и написания её названия.
    keys: Vec<HashSet<String>>,
    name_keys: Vec<HashSet<String>>,
    /// Написания полей, которые могут пройти порог триграмм, без повторов, и их триграммы.
    trigram_keys: Vec<HashSet<String>>,
    /// У каждой записи: (вес поля относительно названия, номер написания в `trigram_keys`).
    trigram_owner: Vec<Vec<(f32, usize)>>,
}

impl SearchIndex {
    pub fn new(records: &[SearchRecord]) -> Self {
        let mut index = SearchIndex {
            ids: Vec::with_capacity(records.len()),
            by_id: HashMap::new(),
            weights: [0.0; FIELDS],
            words: Vec::new(),
            vocab: HashSet::new(),
            keys: Vec::with_capacity(records.len()),
            name_keys: Vec::with_capacity(records.len()),
            trigram_keys: Vec::new(),
            trigram_owner: Vec::with_capacity(records.len()),
        };
        let mut word_of: HashMap<String, usize> = HashMap::new();
        let mut trigram_of: HashMap<String, usize> = HashMap::new();

        for (ri, rec) in records.iter().enumerate() {
            index.ids.push(rec.id.clone());
            index.by_id.insert(rec.id.clone(), ri); // при повторе id побеждает последняя запись
            let (mut all, mut name, mut owner) = (HashSet::new(), HashSet::new(), Vec::new());
            for (fi, (text, weight)) in rec.fields().into_iter().enumerate() {
                index.weights[fi] = weight;
                let field = analyze_field(text, weight);

                let mut counts: HashMap<usize, u32> = HashMap::new();
                for tok in &field.tokens {
                    let wi = *word_of.entry(tok.clone()).or_insert_with(|| {
                        index.words.push(IndexedWord { forms: token_forms(tok), seen: Vec::new() });
                        index.words.len() - 1
                    });
                    *counts.entry(wi).or_insert(0) += 1;
                }
                for (wi, count) in counts {
                    index.words[wi].seen.push((ri as u32, fi as u8, count));
                }

                // Порог триграмм 0,6, а сходство не больше 1: поле с весом ниже 0,6 от
                // названия (описание, текст, подпись) пройти его не может.
                let scale = weight / W_NAME;
                for key in &field.keys {
                    all.insert(key.clone());
                    if fi == 0 {
                        name.insert(key.clone());
                    }
                    if key.starts_with(STEM_MARK) {
                        continue;
                    }
                    index.vocab.insert(key.clone());
                    if scale >= WORD_SIM_FLOOR {
                        let ki = *trigram_of.entry(key.clone()).or_insert_with(|| {
                            index.trigram_keys.push(trigrams(key));
                            index.trigram_keys.len() - 1
                        });
                        owner.push((scale, ki));
                    }
                }
            }
            index.keys.push(all);
            index.name_keys.push(name);
            index.trigram_owner.push(owner);
        }
        index
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub fn hybrid(&self, query: &str, vector_scores: &[(String, f32)], limit: usize) -> Vec<SemanticHit> {
        if query.trim().chars().count() < 2 || self.ids.is_empty() {
            return Vec::new();
        }
        let q = fold_in_script(query);
        if q.chars().count() < 2 {
            return Vec::new();
        }
        let word_ids = self.word_bm25(&q);
        let tri_ids = self.trigram_rank(&q);
        let vector_ids = cluster_vector(vector_scores);
        rrf_merge(&[&vector_ids, &word_ids, &tri_ids], limit.clamp(1, 200))
    }

    /// Гибрид по всем видам вещей с бонусом за покрытие слов. Возвращает все вещи,
    /// у которых есть оценка, по убыванию (`limit` обрезает).
    pub fn site(
        &self,
        query: &str,
        vector_scores: &[(String, f32)],
        limit: usize,
        boost: SiteBoost,
    ) -> Vec<SemanticHit> {
        let mut hits = self.hybrid(query, vector_scores, self.ids.len().max(1));
        if hits.is_empty() || (boost.name == 0.0 && boost.any == 0.0) {
            hits.truncate(limit);
            return hits;
        }
        let q = fold_in_script(query);
        // Название принимает опечатки («frankenshtein»); «в каком-то поле» — только
        // точное слово: неточное совпадение с одним словом длинного текста делало
        // из «dragon» находку.
        let groups = self.content_groups(&q, boost.common, true);
        let exact = self.content_groups(&q, boost.common, false);
        if !groups.is_empty() {
            for h in hits.iter_mut() {
                let Some(&ri) = self.by_id.get(h.id.as_str()) else { continue };
                if groups.iter().all(|g| g.iter().any(|f| self.name_keys[ri].contains(f))) {
                    h.score += boost.name;
                } else if !exact.is_empty() && exact.iter().all(|g| g.iter().any(|f| self.keys[ri].contains(f))) {
                    h.score += boost.any;
                }
            }
            hits.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.id.cmp(&b.id))
            });
        }
        hits.truncate(limit);
        hits
    }

    /// Содержательные слова запроса, каждое со своими написаниями (латинская форма
    /// и, если `fuzzy`, опечатки). Пусто — запрос из одних общих слов.
    fn content_groups(&self, q: &str, common: f32, fuzzy: bool) -> Vec<Vec<String>> {
        let n = self.ids.len().max(1) as f32;
        let mut groups = Vec::new();
        for tok in tokens(q) {
            let mut forms: Vec<String> = Vec::new();
            for form in token_forms(&tok) {
                let variants = if fuzzy { expand_term(&form, &self.vocab) } else { vec![form] };
                for e in variants {
                    if !forms.contains(&e) {
                        forms.push(e);
                    }
                }
            }
            let seen = self.keys.iter().filter(|d| forms.iter().any(|f| d.contains(f))).count() as f32;
            if seen / n >= common {
                continue;
            }
            groups.push(forms);
        }
        groups
    }

    fn word_bm25(&self, q: &str) -> Vec<String> {
        let expanded = query_terms(q, &self.vocab);
        if expanded.is_empty() {
            return Vec::new();
        }
        let records = self.ids.len();
        let n = records as f32;

        // Для каждого слова запроса: сколько раз оно встретилось в каждом поле каждой записи.
        let mut terms: Vec<(f32, Vec<u32>)> = Vec::new();
        for tok in &expanded {
            let mut tf = vec![0u32; records * FIELDS];
            for word in &self.words {
                if word.forms.iter().any(|k| word_hit(tok, k)) {
                    for &(r, f, count) in &word.seen {
                        tf[r as usize * FIELDS + f as usize] += count;
                    }
                }
            }
            let df = (0..records)
                .filter(|r| tf[r * FIELDS..(r + 1) * FIELDS].iter().any(|&t| t > 0))
                .count() as f32;
            if df <= 0.0 {
                continue;
            }
            let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
            terms.push((idf, tf));
        }

        let mut scored: Vec<(String, f32)> = Vec::new();
        for r in 0..records {
            let mut score = 0.0_f32;
            for (idf, tf) in &terms {
                for fi in 0..FIELDS {
                    let count = tf[r * FIELDS + fi];
                    if count == 0 {
                        continue;
                    }
                    let tf = count as f32;
                    let tf_norm = (tf * (BM25_K1 + 1.0)) / (tf + BM25_K1);
                    score += idf * tf_norm * self.weights[fi];
                }
            }
            if score > 0.0 {
                scored.push((self.ids[r].clone(), score));
            }
        }
        sort_cap(scored)
    }

    fn trigram_rank(&self, q: &str) -> Vec<String> {
        let q_forms: Vec<String> = {
            let mut forms = vec![q.to_string()];
            let latin = latin_fold(q);
            if latin != q {
                forms.push(latin);
            }
            for tok in tokens(q) {
                for form in token_forms(&tok) {
                    if !form.starts_with(STEM_MARK) && !forms.contains(&form) {
                        forms.push(form);
                    }
                }
            }
            forms
        };
        let q_trigrams: Vec<HashSet<String>> = q_forms.iter().map(|f| trigrams(f)).collect();
        // Лучшее сходство каждого написания с запросом считается один раз.
        let key_similarity: Vec<f32> = self
            .trigram_keys
            .iter()
            .map(|tw| {
                q_trigrams
                    .iter()
                    .map(|tq| {
                        if tq.is_empty() {
                            0.0
                        } else {
                            tq.intersection(tw).count() as f32 / tq.len() as f32
                        }
                    })
                    .fold(0.0_f32, f32::max)
            })
            .collect();

        let mut scored: Vec<(String, f32)> = Vec::new();
        for (r, owner) in self.trigram_owner.iter().enumerate() {
            let mut best = 0.0_f32;
            for &(scale, ki) in owner {
                best = best.max(key_similarity[ki] * scale);
            }
            if best >= WORD_SIM_FLOOR {
                scored.push((self.ids[r].clone(), best));
            }
        }
        sort_cap(scored)
    }
}

/// Поиск по записям без готового индекса: собирает индекс на лету. Для вызовов, где
/// записи меняются от запроса к запросу или поиск редкий; частому поиску нужен
/// `SearchIndex`, собранный один раз.
pub fn hybrid_search(
    query: &str,
    records: &[SearchRecord],
    vector_scores: &[(String, f32)],
    limit: usize,
) -> Vec<SemanticHit> {
    if records.is_empty() {
        return Vec::new();
    }
    SearchIndex::new(records).hybrid(query, vector_scores, limit)
}

/// Поправки к оценке гибрида для поиска по всему сайту (`site_search`).
///
/// RRF смотрит только на места в трёх списках, поэтому точное совпадение слов
/// запроса с названием (то, что ищущий и имеет в виду, когда набирает имя) не
/// отличается от случайной близости по вектору. Бонус за покрытие возвращает это
/// различие. Значения подобраны замером (`examples/search_probe.rs`).
#[derive(Clone, Copy, Debug)]
pub struct SiteBoost {
    /// Все содержательные слова запроса нашлись в названии вещи.
    pub name: f32,
    /// Все содержательные слова запроса нашлись хоть в одном поле вещи.
    pub any: f32,
    /// Слово, которое стоит у такой доли вещей и больше, ничего не отличает
    /// («with», «doll», «the») и в покрытие не входит.
    pub common: f32,
}

impl SiteBoost {
    pub const NONE: SiteBoost = SiteBoost { name: 0.0, any: 0.0, common: 1.0 };
}

/// Гибрид по всем видам вещей с бонусом за покрытие слов (как `SearchIndex::site`), с индексом,
/// собранным на лету.
pub fn site_search(
    query: &str,
    records: &[SearchRecord],
    vector_scores: &[(String, f32)],
    limit: usize,
    boost: SiteBoost,
) -> Vec<SemanticHit> {
    if records.is_empty() {
        return Vec::new();
    }
    SearchIndex::new(records).site(query, vector_scores, limit, boost)
}

/// Какие находки показывать. Вещь показывается, если её подтвердили два списка из
/// трёх (оценка гибрида не ниже `floor`) или если вектор считает её очень близкой
/// (`vector_only`): вещь, которой нет слов запроса («weapon» → карта «Секироносец»),
/// иначе недостижима, потому что один список даёт не больше 1/61 ≈ 0,0164. Близость
/// `vector_only` ≤ 0 выключает второе условие.
pub fn keep_hits(hits: Vec<SemanticHit>, vector_scores: &[(String, f32)], floor: f32, vector_only: f32) -> Vec<SemanticHit> {
    let cos: HashMap<&str, f32> = vector_scores.iter().map(|(id, c)| (id.as_str(), *c)).collect();
    hits.into_iter()
        .filter(|h| h.score >= floor || (vector_only > 0.0 && cos.get(h.id.as_str()).is_some_and(|c| *c >= vector_only)))
        .collect()
}

pub fn records_from_figurines(figs: &[Figurine]) -> Vec<SearchRecord> {
    figs.iter().map(SearchRecord::from_figurine).collect()
}

/// Casefold, ё→е, collapse dimension separators. Script is preserved.
pub fn fold_in_script(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = true;
    let lower: String = s.chars().flat_map(char::to_lowercase).collect();
    let chars: Vec<char> = lower.chars().collect();
    let n = chars.len();
    let mut i = 0;
    while i < n {
        let c = chars[i];
        let mapped = if c == 'ё' {
            'е'
        } else if c == '×' {
            'x'
        } else if is_dim_separator(c) && neigh_digit(&chars, i) {
            'x'
        } else {
            c
        };
        if mapped.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(mapped);
            prev_space = false;
        }
        i += 1;
    }
    collapse_dim_x(out.trim())
}

fn latin_fold(s: &str) -> String {
    deunicode::deunicode(s).to_ascii_lowercase()
}

fn analyze_field(text: &str, weight: f32) -> WeightedField {
    let tokens = tokens(&fold_in_script(text));
    let mut keys = Vec::new();
    let mut seen = HashSet::new();
    for tok in &tokens {
        for form in token_forms(tok) {
            if seen.insert(form.clone()) {
                keys.push(form);
            }
        }
    }
    WeightedField {
        tokens,
        keys,
        weight,
    }
}

/// Все написания слова, по которым его ищут: само слово, его латинская запись
/// и основы (слово без окончания). Основа даёт другие формы того же слова:
/// «зеленая», «зеленый», «зелёного» → «зелен»; «witches» → «witch».
fn token_forms(tok: &str) -> Vec<String> {
    // Слово разбирают заново на каждый запрос для каждой пары «слово текста — слово
    // запроса», а разных слов на сайте тысячи: результат запоминается.
    thread_local! {
        static FORMS: std::cell::RefCell<HashMap<String, Vec<String>>> = std::cell::RefCell::new(HashMap::new());
    }
    FORMS.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(forms) = cache.get(tok) {
            return forms.clone();
        }
        let forms = compute_forms(tok);
        if cache.len() > 200_000 {
            cache.clear();
        }
        cache.insert(tok.to_string(), forms.clone());
        forms
    })
}

fn compute_forms(tok: &str) -> Vec<String> {
    let mut forms = vec![tok.to_string()];
    let latin = latin_fold(tok);
    if !latin.is_empty() && latin != tok {
        forms.push(latin);
    }
    for stem in stems(tok) {
        let stem_latin = latin_fold(&stem);
        for form in [format!("{STEM_MARK}{stem}"), format!("{STEM_MARK}{stem_latin}")] {
            if form.len() > STEM_MARK.len() && !forms.contains(&form) {
                forms.push(form);
            }
        }
    }
    forms
}

/// Основа слова записана с этим знаком в начале и совпадает только с такой же основой:
/// найденная как кусок внутри слова («курс» в любом «курсе») она давала бы случайные
/// находки.
const STEM_MARK: &str = "~";

/// Основы короче этого не берутся: «ста», «мет» совпали бы с тысячей слов.
const MIN_STEM: usize = 4;

/// Основа слова по правилам Snowball. Язык слова неизвестен (в одном тексте
/// склеены названия на шести языках), поэтому выбор по алфавиту: кириллица — русский
/// стеммер (он же работает и на украинских словах, как именно — в тестах), латиница —
/// английский, испанский, немецкий и итальянский сразу.
fn stems(tok: &str) -> Vec<String> {
    if !tok.chars().all(|c| c.is_alphabetic()) {
        return Vec::new();
    }
    let cyrillic = tok.chars().any(|c| ('\u{0400}'..='\u{04FF}').contains(&c));
    let algorithms: &[Algorithm] = if cyrillic {
        &[Algorithm::Russian]
    } else {
        &[Algorithm::English, Algorithm::Spanish, Algorithm::German, Algorithm::Italian]
    };
    // Русский стеммер не знает «і», «ї», «є», «ґ» и не находит у украинского слова
    // область окончания. Основа нужна только как ключ поиска, и у слова текста, и у слова
    // запроса она считается одинаково, поэтому буквы заменяются русскими до стемминга.
    let source: String = if cyrillic {
        tok.chars()
            .map(|c| match c {
                'і' | 'ї' => 'и',
                'є' => 'е',
                'ґ' => 'г',
                c => c,
            })
            .collect()
    } else {
        tok.to_string()
    };
    let mut out: Vec<String> = Vec::new();
    for algorithm in algorithms {
        let stem = Stemmer::create(*algorithm).stem(&source).to_string();
        if stem.chars().count() >= MIN_STEM && !out.contains(&stem) {
            out.push(stem);
        }
    }
    // Стеммер иногда оставляет слово как есть («зелена», «бабуся»): тогда у кириллического
    // слова отрезается последняя гласная, как делают простые стеммеры.
    if cyrillic && out.iter().all(|stem| stem == tok) && tok.chars().count() > MIN_STEM {
        let mut chars: Vec<char> = source.chars().collect();
        if chars.last().is_some_and(|c| "аеиоуыэюяіїє".contains(*c)) {
            chars.pop();
            out = vec![chars.into_iter().collect()];
        }
    }
    out
}

fn collapse_dim_x(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            out.push(chars[i]);
            i += 1;
            let mut j = i;
            while j < chars.len() && chars[j] == ' ' {
                j += 1;
            }
            if j < chars.len() && chars[j] == 'x' {
                let mut k = j + 1;
                while k < chars.len() && chars[k] == ' ' {
                    k += 1;
                }
                if k < chars.len() && chars[k].is_ascii_digit() {
                    out.push('x');
                    i = k;
                    continue;
                }
            }
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn is_dim_separator(c: char) -> bool {
    matches!(c, 'x' | 'х' | '*')
}

fn neigh_digit(chars: &[char], i: usize) -> bool {
    let left = (0..i).rev().find_map(|j| {
        let c = chars[j];
        if c.is_whitespace() { None } else { Some(c) }
    });
    let right = ((i + 1)..chars.len()).find_map(|j| {
        let c = chars[j];
        if c.is_whitespace() { None } else { Some(c) }
    });
    left.map(|c| c.is_ascii_digit()).unwrap_or(false)
        && right.map(|c| c.is_ascii_digit()).unwrap_or(false)
}

fn tokens(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in s.chars() {
        if c.is_alphanumeric() {
            cur.push(c);
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Сколько правок допускает слово запроса: 0 для 1–2 знаков, 1 для 3–7, 2 для 8+.
/// У слов из 6–7 знаков вторая правка разрешена только в последних двух знаках
/// (`expand_term`): так находятся другие формы слова («зеленая» — «зеленый»).
/// Elasticsearch берёт две правки уже с шести знаков, но словарь здесь на шести
/// языках, и у шестибуквенного слова две замены находят постороннее: «weapon» →
/// «season», «reason» — и по этим словам в переводах небылиц всплывали небылицы,
/// не имеющие к запросу отношения. На 129 фразах (`tools/search`) порог 6, 7, 8 и 9
/// дают одни и те же числа, опечатки («frankenshtein», 13 знаков) исправляются как
/// прежде.
fn auto_edits(term_len: usize) -> usize {
    match term_len {
        0..=2 => 0,
        3..=7 => 1,
        _ => 2,
    }
}

/// Сколько первых знаков совпадает у двух слов.
fn common_prefix(a: &str, b: &str) -> usize {
    a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count()
}

fn expand_term(term: &str, vocab: &HashSet<String>) -> Vec<String> {
    let mut out = vec![term.to_string()];
    let qn = term.chars().count();
    let max_d = auto_edits(qn);
    if max_d == 0 || term.starts_with(STEM_MARK) {
        return out;
    }
    for v in vocab {
        let vn = v.chars().count();
        // У слова из 6–7 знаков вторая правка допустима только в окончании:
        // «зеленая» → «зеленый» (другая форма того же слова), но не «weapon» → «season».
        let ending_only = (6..=7).contains(&qn) && common_prefix(term, v) + 2 >= qn;
        let allowed = if ending_only { max_d + 1 } else { max_d };
        if vn.abs_diff(qn) > allowed {
            continue;
        }
        if strsim::damerau_levenshtein(term, v) <= allowed && !out.iter().any(|x| x == v) {
            out.push(v.clone());
        }
    }
    out
}

fn query_terms(q: &str, vocab: &HashSet<String>) -> Vec<String> {
    let mut terms = Vec::new();
    let mut seen = HashSet::new();
    for tok in tokens(q) {
        for form in token_forms(&tok) {
            for expanded in expand_term(&form, vocab) {
                if seen.insert(expanded.clone()) {
                    terms.push(expanded);
                }
            }
        }
    }
    terms
}

fn word_hit(qtok: &str, doc: &str) -> bool {
    if doc == qtok {
        return true;
    }
    // Основа сравнивается только с основой.
    if qtok.starts_with(STEM_MARK) || doc.starts_with(STEM_MARK) {
        return false;
    }
    // Infix of a token once the fragment is long enough to be a term, not a
    // letter — same idea as a wildcard on an analyzed token, not edit distance.
    qtok.chars().count() >= 3 && doc.contains(qtok)
}

/// pg_trgm trigrams: pad with two spaces so the start and end of a word count.
fn trigrams(s: &str) -> HashSet<String> {
    let padded: Vec<char> = [' ', ' ']
        .into_iter()
        .chain(s.chars())
        .chain([' ', ' '])
        .collect();
    if padded.len() < 3 {
        return HashSet::new();
    }
    padded
        .windows(3)
        .map(|w| w.iter().copied().collect::<String>())
        .collect()
}

#[cfg(test)]
/// pg_trgm `word_similarity(query, word)`: share of the query's trigrams that
/// appear in the word. Short queries against a longer token stay meaningful.
fn word_similarity(query: &str, word: &str) -> f32 {
    let tq = trigrams(query);
    if tq.is_empty() {
        return 0.0;
    }
    let tw = trigrams(word);
    tq.intersection(&tw).count() as f32 / tq.len() as f32
}

fn cluster_vector(scores: &[(String, f32)]) -> Vec<String> {
    let mut hits: Vec<(String, f32)> = scores
        .iter()
        .filter(|(_, s)| s.is_finite())
        .cloned()
        .collect();
    hits.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    hits.truncate(RETRIEVER_CAP);
    hits.into_iter().map(|(id, _)| id).collect()
}

fn sort_cap(mut scored: Vec<(String, f32)>) -> Vec<String> {
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(RETRIEVER_CAP);
    scored.into_iter().map(|(id, _)| id).collect()
}

fn rrf_merge(lists: &[&[String]], limit: usize) -> Vec<SemanticHit> {
    let mut scores: HashMap<String, f32> = HashMap::new();
    for list in lists {
        for (rank, id) in list.iter().enumerate() {
            *scores.entry(id.clone()).or_insert(0.0) += 1.0 / (RRF_K + rank as f32 + 1.0);
        }
    }
    let mut hits: Vec<SemanticHit> = scores
        .into_iter()
        .map(|(id, score)| SemanticHit { id, score })
        .collect();
    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.id.cmp(&b.id))
    });
    hits.truncate(limit);
    hits
}

/// Прежняя реализация: на каждый запрос заново разбирает все тексты. Остаётся только в
/// тестах как эталон, с которым сверяется `SearchIndex`.
#[cfg(test)]
mod legacy {
    use super::*;

    pub fn hybrid_search(
        query: &str,
        records: &[SearchRecord],
        vector_scores: &[(String, f32)],
        limit: usize,
    ) -> Vec<SemanticHit> {
        if query.trim().chars().count() < 2 || records.is_empty() {
            return Vec::new();
        }
        let q = fold_in_script(query);
        if q.chars().count() < 2 {
            return Vec::new();
        }

        let vocab = cabinet_vocab(records);
        let word_ids = word_bm25(&q, records, &vocab);
        let tri_ids = trigram_rank(&q, records);
        let vector_ids = cluster_vector(vector_scores);

        rrf_merge(&[&vector_ids, &word_ids, &tri_ids], limit.clamp(1, 200))
    }

    pub fn content_groups(q: &str, records: &[SearchRecord], vocab: &HashSet<String>, common: f32, fuzzy: bool) -> Vec<Vec<String>> {
        let docs: Vec<HashSet<String>> = records
            .iter()
            .map(|r| r.analyzed().into_iter().flat_map(|f| f.keys).collect())
            .collect();
        let n = records.len().max(1) as f32;
        let mut groups = Vec::new();
        for tok in tokens(q) {
            let mut forms: Vec<String> = Vec::new();
            for form in token_forms(&tok) {
                let variants = if fuzzy { expand_term(&form, vocab) } else { vec![form] };
                for e in variants {
                    if !forms.contains(&e) {
                        forms.push(e);
                    }
                }
            }
            let seen = docs.iter().filter(|d| forms.iter().any(|f| d.contains(f))).count() as f32;
            if seen / n >= common {
                continue;
            }
            groups.push(forms);
        }
        groups
    }

    pub fn site_search(
        query: &str,
        records: &[SearchRecord],
        vector_scores: &[(String, f32)],
        limit: usize,
        boost: SiteBoost,
    ) -> Vec<SemanticHit> {
        let mut hits = hybrid_search(query, records, vector_scores, records.len().max(1));
        if hits.is_empty() || (boost.name == 0.0 && boost.any == 0.0) {
            hits.truncate(limit);
            return hits;
        }
        let q = fold_in_script(query);
        let vocab = cabinet_vocab(records);
        // Название принимает опечатки («frankenshtein»); «в каком-то поле» — только
        // точное слово: неточное совпадение с одним словом длинного текста делало
        // из «dragon» находку.
        let groups = content_groups(&q, records, &vocab, boost.common, true);
        let exact = content_groups(&q, records, &vocab, boost.common, false);
        if !groups.is_empty() {
            let by_id: HashMap<&str, &SearchRecord> = records.iter().map(|r| (r.id.as_str(), r)).collect();
            for h in hits.iter_mut() {
                let Some(rec) = by_id.get(h.id.as_str()) else { continue };
                let fields = rec.analyzed();
                let name_keys: HashSet<&String> = fields[0].keys.iter().collect();
                let any_keys: HashSet<&String> = fields.iter().flat_map(|f| f.keys.iter()).collect();
                if groups.iter().all(|g| g.iter().any(|f| name_keys.contains(f))) {
                    h.score += boost.name;
                } else if !exact.is_empty() && exact.iter().all(|g| g.iter().any(|f| any_keys.contains(f))) {
                    h.score += boost.any;
                }
            }
            hits.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.id.cmp(&b.id))
            });
        }
        hits.truncate(limit);
        hits
    }

    pub fn cabinet_vocab(records: &[SearchRecord]) -> HashSet<String> {
        let mut vocab = HashSet::new();
        for rec in records {
            for field in rec.analyzed() {
                vocab.extend(field.keys.into_iter().filter(|k| !k.starts_with(STEM_MARK)));
            }
        }
        vocab
    }

    pub fn query_terms(q: &str, vocab: &HashSet<String>) -> Vec<String> {
        let mut terms = Vec::new();
        let mut seen = HashSet::new();
        for tok in tokens(q) {
            for form in token_forms(&tok) {
                for expanded in expand_term(&form, vocab) {
                    if seen.insert(expanded.clone()) {
                        terms.push(expanded);
                    }
                }
            }
        }
        terms
    }

    pub fn word_bm25(q: &str, records: &[SearchRecord], vocab: &HashSet<String>) -> Vec<String> {
        let expanded = query_terms(q, vocab);
        if expanded.is_empty() {
            return Vec::new();
        }
        let n = records.len() as f32;
        let docs: Vec<Vec<WeightedField>> = records.iter().map(|r| r.analyzed()).collect();

        let mut df: HashMap<String, f32> = HashMap::new();
        for tok in &expanded {
            let mut seen = 0.0;
            for fields in &docs {
                if fields
                    .iter()
                    .any(|f| f.keys.iter().any(|k| word_hit(tok, k)))
                {
                    seen += 1.0;
                }
            }
            df.insert(tok.clone(), seen);
        }

        let mut scored: Vec<(String, f32)> = Vec::new();
        for (rec, fields) in records.iter().zip(docs.iter()) {
            let mut score = 0.0_f32;
            for tok in &expanded {
                let dfi = *df.get(tok).unwrap_or(&0.0);
                if dfi <= 0.0 {
                    continue;
                }
                let idf = ((n - dfi + 0.5) / (dfi + 0.5) + 1.0).ln();
                for field in fields {
                    let tf = field
                        .tokens
                        .iter()
                        .filter(|t| token_forms(t).iter().any(|k| word_hit(tok, k)))
                        .count() as f32;
                    if tf <= 0.0 {
                        continue;
                    }
                    let tf_norm = (tf * (BM25_K1 + 1.0)) / (tf + BM25_K1);
                    score += idf * tf_norm * field.weight;
                }
            }
            if score > 0.0 {
                scored.push((rec.id.clone(), score));
            }
        }
        sort_cap(scored)
    }

    pub fn trigram_rank(q: &str, records: &[SearchRecord]) -> Vec<String> {
        let q_forms: Vec<String> = {
            let mut forms = vec![q.to_string()];
            let latin = latin_fold(q);
            if latin != q {
                forms.push(latin);
            }
            for tok in tokens(q) {
                for form in token_forms(&tok) {
                    if !form.starts_with(STEM_MARK) && !forms.contains(&form) {
                        forms.push(form);
                    }
                }
            }
            forms
        };

        let mut scored: Vec<(String, f32)> = Vec::new();
        for rec in records {
            let mut best = 0.0_f32;
            for (text, weight) in rec.fields() {
                let scale = weight / W_NAME;
                // Сходство не больше 1, а порог 0,6: поле с весом ниже 0,6 от названия
                // (описание, текст, подпись) пройти его не может. Такие поля не разбираются
                // вовсе: тело байки на шести языках — основная часть времени поиска.
                if scale < WORD_SIM_FLOOR {
                    continue;
                }
                let field = analyze_field(text, weight);
                for key in field.keys.iter().filter(|k| !k.starts_with(STEM_MARK)) {
                    for qf in &q_forms {
                        best = best.max(word_similarity(qf, key) * scale);
                    }
                }
            }
            if best >= WORD_SIM_FLOOR {
                scored.push((rec.id.clone(), best));
            }
        }
        sort_cap(scored)
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(id: &str, name: &str, extra: &str) -> SearchRecord {
        SearchRecord {
            id: id.into(),
            name: name.into(),
            short_text: None,
            full_description: Some(extra.into()),
            dimensions: None,
            material: None,
            technique: None,
            visual_caption: None,
            pinterest_description: None,
        }
    }

    fn rec_full(
        id: &str,
        name: &str,
        material: &str,
        dimensions: &str,
        caption: &str,
    ) -> SearchRecord {
        SearchRecord {
            id: id.into(),
            name: name.into(),
            short_text: None,
            full_description: None,
            dimensions: Some(dimensions.into()),
            material: Some(material.into()),
            technique: None,
            visual_caption: Some(caption.into()),
            pinterest_description: None,
        }
    }

    #[test]
    fn fold_yo_and_times_keeps_script() {
        assert_eq!(fold_in_script("  Ёлка  20×15  "), "елка 20x15");
        assert_eq!(fold_in_script("20 х 15"), "20x15");
    }

    #[test]
    fn latin_fold_is_deunicode_not_a_hand_map() {
        assert_eq!(
            latin_fold("елка"),
            deunicode::deunicode("елка").to_ascii_lowercase()
        );
        assert_ne!(latin_fold("елка"), "елка");
    }

    #[test]
    fn trigram_coverage_ranks_a_partial_stem() {
        let records = vec![
            rec("a", "Франкенштейн", "готическая кукла"),
            rec("b", "Ворон", "nevermore"),
        ];
        let hits = hybrid_search("fraken", &records, &[], 8);
        assert!(!hits.is_empty());
        assert_eq!(hits[0].id, "a");
        let hits_en = hybrid_search("fraken", &[rec("c", "Frankenstein", "")], &[], 8);
        assert_eq!(hits_en[0].id, "c");
    }

    #[test]
    fn auto_fuzziness_does_not_bridge_a_length_gap() {
        let vocab = HashSet::from(["frankenstein".into(), "хранителница".into()]);
        let expanded = expand_term("fraken", &vocab);
        assert_eq!(
            expanded,
            vec!["fraken".to_string()],
            "a 6-letter stem is not an AUTO edit of a 12-letter word"
        );
    }

    #[test]
    fn similar_length_typo_ranks_first() {
        let records = vec![
            rec("a", "Хранительница порога", "пыль"),
            rec("b", "Монах со свечой", "кабинет"),
        ];
        let hits = hybrid_search("хрантельница порога", &records, &[], 8);
        assert_eq!(hits[0].id, "a");
    }

    #[test]
    fn cyrillic_query_hits_cyrillic_name_without_folding() {
        let records = vec![rec("a", "Хранительница порога", ""), rec("b", "Ворон", "")];
        let hits = hybrid_search("хранительница", &records, &[], 8);
        assert_eq!(hits[0].id, "a");
    }

    #[test]
    fn dimensions_and_caption_are_findable() {
        let records = vec![
            rec_full(
                "a",
                "Страж",
                "полимерная глина",
                "20×15×10 cm",
                "монах со свечой",
            ),
            rec_full("b", "Птица", "фарфор", "4 cm", "синяя птица"),
        ];
        let by_dim = hybrid_search("20x15", &records, &[], 8);
        assert_eq!(by_dim[0].id, "a");
        let by_cap = hybrid_search("монах со свечой", &records, &[], 8);
        assert_eq!(by_cap[0].id, "a");
        let by_mat = hybrid_search("полимерная глина", &records, &[], 8);
        assert_eq!(by_mat[0].id, "a");
    }

    #[test]
    fn rrf_prefers_agreement() {
        let hits = rrf_merge(&[&["a".into(), "b".into()], &["a".into(), "c".into()]], 3);
        assert_eq!(hits[0].id, "a");
    }

    #[test]
    fn blank_and_short_queries_are_empty() {
        let records = vec![rec("a", "Имя", "")];
        assert!(hybrid_search(" ", &records, &[], 8).is_empty());
        assert!(hybrid_search("я", &records, &[], 8).is_empty());
    }

    /// Есть ли у двух написаний общая форма: одно и то же слово в разных формах.
    fn same_word(a: &str, b: &str) -> bool {
        let fa = token_forms(&fold_in_script(a));
        token_forms(&fold_in_script(b)).iter().any(|f| fa.contains(f))
    }

    #[test]
    fn other_forms_of_a_word_share_a_stem() {
        let pairs = [
            // русский
            ("зеленая", "зелёный"), ("зелёного", "зеленая"), ("красная", "красному"), ("ведьма", "ведьмы"),
            ("метла", "метлой"), ("бабушка", "бабушки"), ("платка", "платком"), ("гномы", "гнома"),
            // английский
            ("witches", "witch"), ("weapons", "weapon"), ("brooms", "broom"), ("walking", "walks"),
            // испанский
            ("brujas", "bruja"), ("gatos", "gato"), ("escobas", "escoba"),
            // немецкий
            ("Zwerge", "Zwergen"), ("Kapuze", "Kapuzen"), ("Mützen", "Mütze"),
            // итальянский
            ("streghe", "strega"), ("gnomi", "gnomo"), ("scope", "scopa"),
        ];
        for (a, b) in pairs {
            assert!(same_word(a, b), "{a} и {b} должны быть одним словом");
        }
        // Известные ограничения: у коротких слов («Hexe» — «Hexen») основа короче MIN_STEM
        // и не берётся, чтобы не совпасть с тысячей слов; «беглая гласная» («платок» — «платка») не склеивается.
        assert!(!same_word("платок", "платком"));
        // разные слова не склеиваются
        for (a, b) in [("weapon", "season"), ("ведьма", "ведро"), ("гном", "гнев"), ("cat", "cart")] {
            assert!(!same_word(a, b), "{a} и {b} — разные слова");
        }
    }

    #[test]
    fn ukrainian_forms_share_a_stem() {
        for (a, b) in [("зелена", "зелений"), ("зелені", "зелена"), ("відьма", "відьми"), ("мітла", "мітли"),
                       ("бабуся", "бабусі"), ("хустка", "хустку"), ("гноми", "гнома"), ("сокира", "сокири")] {
            assert!(same_word(a, b), "{a} і {b} мають бути одним словом");
        }
        assert!(!same_word("відьма", "відро"));
    }

    #[test]
    fn other_form_of_a_six_or_seven_letter_word_is_found() {
        let vocab: HashSet<String> = ["зеленый", "красный", "season", "reason"].iter().map(|w| w.to_string()).collect();
        // Окончание «-ая» / «-ый»: две замены, но основа та же.
        assert!(expand_term("зеленая", &vocab).contains(&"зеленый".to_string()));
        assert!(expand_term("красная", &vocab).contains(&"красный".to_string()));
        // Основа другая: не форма того же слова.
        let weapon = expand_term("weapon", &vocab);
        assert!(!weapon.contains(&"season".to_string()) && !weapon.contains(&"reason".to_string()));
    }

    #[test]
    fn short_words_do_not_expand_into_unrelated_words() {
        let vocab: HashSet<String> = ["season", "reason", "weapons", "frankenstein"].iter().map(|w| w.to_string()).collect();
        // Шесть знаков, две замены: «season» и «reason» — не опечатки «weapon».
        let expanded = expand_term("weapon", &vocab);
        assert!(!expanded.contains(&"season".to_string()) && !expanded.contains(&"reason".to_string()));
        assert!(expanded.contains(&"weapons".to_string()));
        // Одна опечатка в коротком слове по-прежнему исправляется.
        assert!(expand_term("weapns", &vocab).contains(&"weapons".to_string()));
        // Длинное слово терпит две правки.
        assert!(expand_term("frankenshtein", &vocab).contains(&"frankenstein".to_string()));
    }

    #[test]
    fn word_similarity_is_query_coverage_not_document_jaccard() {
        let sim = word_similarity("fraken", "frankenstein");
        assert!(
            sim >= WORD_SIM_FLOOR,
            "pg_trgm-style coverage should accept a partial stem, got {sim}"
        );
        assert!(word_similarity("zzzzzz", "frankenstein") < WORD_SIM_FLOOR);
        let jac_would_be_low = {
            let tq = trigrams("fraken");
            let tw = trigrams("frankenstein");
            tq.intersection(&tw).count() as f32 / tq.union(&tw).count() as f32
        };
        assert!(
            jac_would_be_low < WORD_SIM_FLOOR,
            "document Jaccard would reject the same stem; word_similarity must not"
        );
    }

    #[test]
    fn site_search_prefers_words_found_in_the_name() {
        // Вектора нет: оценку даёт только лексика, поэтому бонус виден чище.
        let records = vec![
            rec("a", "Gnome with axe", "small doll made of clay"),
            rec("b", "Cooking gnome", "gnome with a pot and an axe on the wall"),
            rec("c", "Witch", "witch with broom"),
        ];
        let boost = SiteBoost { name: 0.05, any: 0.01, common: 0.9 };
        let hits = site_search("gnome axe", &records, &[], 10, boost);
        assert_eq!(hits.first().map(|h| h.id.as_str()), Some("a"), "{hits:?}");
        let plain = site_search("gnome axe", &records, &[], 10, SiteBoost::NONE);
        let gap = |hs: &[SemanticHit]| {
            let s = |id: &str| hs.iter().find(|h| h.id == id).map(|h| h.score).unwrap_or(0.0);
            s("a") - s("b")
        };
        assert!(gap(&hits) > gap(&plain), "бонус должен разводить название и текст");
    }

    #[test]
    fn site_search_ignores_words_that_every_item_has() {
        // «doll» стоит почти у каждой вещи и запрос «doll witch» ничем не сужает:
        // покрытием считается только «witch».
        let records = vec![
            rec("a", "Small doll Witch", ""),
            rec("b", "Small doll Gnome", ""),
            rec("c", "Small doll Granny", ""),
            rec("d", "Small doll Vampire", ""),
        ];
        let boost = SiteBoost { name: 0.05, any: 0.0, common: 0.5 };
        let hits = site_search("doll witch", &records, &[], 10, boost);
        assert_eq!(hits.first().map(|h| h.id.as_str()), Some("a"));
        assert!(hits.first().unwrap().score > 0.05, "название покрыто — бонус выдан");
    }

    #[test]
    fn site_search_without_boost_is_plain_hybrid() {
        let records = vec![rec("a", "Gnome with axe", ""), rec("b", "Witch", "")];
        let a = site_search("gnome", &records, &[], 10, SiteBoost::NONE);
        let b = hybrid_search("gnome", &records, &[], records.len());
        assert_eq!(a.iter().map(|h| &h.id).collect::<Vec<_>>(), b.iter().map(|h| &h.id).collect::<Vec<_>>());
    }

    #[test]
    fn with_extra_adds_other_languages_to_the_lexical_side() {
        let r = rec("a", "Forester", "").with_extra("Лесник", "лесник с белками");
        let records = vec![r, rec("b", "Witch", "")];
        let hits = site_search("лесник", &records, &[], 10, SiteBoost::NONE);
        assert_eq!(hits.first().map(|h| h.id.as_str()), Some("a"));
    }

    /// Записи так же, как собирает их `examples/search_probe.rs` (`docs()`).
    fn corpus_records(corpus: &serde_json::Value) -> Vec<SearchRecord> {
        use serde_json::Value;
        fn text(v: &Value) -> String {
            v.as_str().unwrap_or("").to_string()
        }
        fn all(v: &Value) -> Vec<String> {
            v.as_object()
                .map(|o| o.values().filter_map(|x| x.as_str().map(str::to_string)).filter(|x| !x.is_empty()).collect())
                .unwrap_or_default()
        }
        fn list(v: &Value) -> Vec<String> {
            v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default()
        }
        let mut out = Vec::new();
        for w in corpus["works"].as_array().unwrap() {
            let name = text(&w["name"]);
            let craft = [text(&w["material"]), text(&w["technique"]), text(&w["series"])].join(" ");
            out.push(SearchRecord::from_text(
                &text(&w["id"]),
                &[vec![name], all(&w["names"])].concat().join(" "),
                &[all(&w["short"]).join(" "), craft].join(" "),
            ));
        }
        for key in ["tales", "gazette"] {
            for l in corpus[key].as_array().unwrap() {
                out.push(SearchRecord::from_text(
                    &text(&l["id"]),
                    &[all(&l["title"]), vec![text(&l["work"])], list(&l["works"])].concat().join(" "),
                    &[all(&l["dek"]).join(" "), all(&l["body"]).join(" ")].join(" "),
                ));
            }
        }
        for c in corpus["cards"].as_array().unwrap() {
            out.push(SearchRecord::from_text(
                &text(&c["id"]),
                &[all(&c["title"]), vec![text(&c["work"])], list(&c["works"])].concat().join(" "),
                &[all(&c["race"]).join(" "), all(&c["effect"]).join(" "), all(&c["lore"]).join(" ")].join(" "),
            ));
        }
        out
    }

    /// Оценки «вектора» для проверки слияния: свои у каждой пары «запрос — вещь», но всегда те же.
    fn fake_vector(query: &str, records: &[SearchRecord]) -> Vec<(String, f32)> {
        use std::hash::{Hash, Hasher};
        records
            .iter()
            .map(|r| {
                let mut h = std::collections::hash_map::DefaultHasher::new();
                (query, &r.id).hash(&mut h);
                (r.id.clone(), (h.finish() % 1000) as f32 / 1000.0)
            })
            .collect()
    }

    /// Эталонная проверка индекса: прежняя реализация (`legacy`) и `SearchIndex` дают одинаковые
    /// списки и одинаковые оценки до последнего знака на всех фразах `tools/search/cases.json`.
    /// Медленная (прежняя реализация разбирает тексты на каждый запрос, около минуты), поэтому
    /// в обычный прогон не входит; запускать после любой правки индекса:
    /// `cargo test --release --lib index_gives -- --ignored`.
    #[test]
    #[ignore = "медленная: прогоняет прежнюю реализацию"]
    fn index_gives_the_same_answers_as_the_per_query_analysis() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/search");
        let (Ok(corpus), Ok(cases)) = (
            std::fs::read_to_string(dir.join("corpus.json")),
            std::fs::read_to_string(dir.join("cases.json")),
        ) else {
            return; // нет снимка данных (`tools/search`): нечего сверять
        };
        let corpus: serde_json::Value = serde_json::from_str(&corpus).unwrap();
        let cases: Vec<serde_json::Value> = serde_json::from_str(&cases).unwrap();
        let records = corpus_records(&corpus);
        let index = SearchIndex::new(&records);
        assert_eq!(index.len(), records.len());

        let mut queries: Vec<String> = cases.iter().map(|c| c["q"].as_str().unwrap().to_string()).collect();
        queries.extend(
            ["weapon", "weapons", "оружие", "зеленая", "зелёный", "бабушка", "pizza recipe", "how to fix a car", "football score",
             "курс доллара", "a", "ё", "  ", "gnomes with axes", "Hexen", "ведьмы с метлами", "x", "30 x 20", "30х20", "красную шапочку",
             "frankenshtein", "dragon", "gnomo con hacha", "ascia"]
                .map(String::from),
        );
        let boosts = [SiteBoost::NONE, SiteBoost { name: 0.01, any: 0.01, common: 0.25 }];
        let mut compared = 0;
        for q in &queries {
            for scores in [Vec::new(), fake_vector(q, &records)] {
                let (old, new) = (legacy::hybrid_search(q, &records, &scores, 40), index.hybrid(q, &scores, 40));
                assert_eq!(old.len(), new.len(), "hybrid: число находок у {q:?}");
                for (a, b) in old.iter().zip(&new) {
                    assert_eq!(a.id, b.id, "hybrid: порядок у {q:?}");
                    assert_eq!(a.score.to_bits(), b.score.to_bits(), "hybrid: оценка {} у {q:?}", a.id);
                }
                for boost in boosts {
                    let (old, new) = (legacy::site_search(q, &records, &scores, 40, boost), index.site(q, &scores, 40, boost));
                    assert_eq!(old.len(), new.len(), "site: число находок у {q:?}");
                    for (a, b) in old.iter().zip(&new) {
                        assert_eq!(a.id, b.id, "site: порядок у {q:?}");
                        assert_eq!(a.score.to_bits(), b.score.to_bits(), "site: оценка {} у {q:?}", a.id);
                    }
                    compared += 1;
                }
            }
        }
        println!("сверено ответов: {compared} (запросов {})", queries.len());
    }
}
