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

    fn analyzed(&self) -> Vec<WeightedField> {
        self.fields()
            .into_iter()
            .map(|(text, weight)| analyze_field(text, weight))
            .collect()
    }
}

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

fn token_forms(tok: &str) -> Vec<String> {
    let mut forms = vec![tok.to_string()];
    let latin = latin_fold(tok);
    if !latin.is_empty() && latin != tok {
        forms.push(latin);
    }
    forms
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

fn cabinet_vocab(records: &[SearchRecord]) -> HashSet<String> {
    let mut vocab = HashSet::new();
    for rec in records {
        for field in rec.analyzed() {
            vocab.extend(field.keys);
        }
    }
    vocab
}

/// Elasticsearch `fuzziness: AUTO`: 0 for 1–2 chars, 1 for 3–5, 2 for 6+.
fn auto_edits(term_len: usize) -> usize {
    match term_len {
        0..=2 => 0,
        3..=5 => 1,
        _ => 2,
    }
}

fn expand_term(term: &str, vocab: &HashSet<String>) -> Vec<String> {
    let mut out = vec![term.to_string()];
    let qn = term.chars().count();
    let max_d = auto_edits(qn);
    if max_d == 0 {
        return out;
    }
    for v in vocab {
        let vn = v.chars().count();
        if vn.abs_diff(qn) > max_d {
            continue;
        }
        if strsim::damerau_levenshtein(term, v) <= max_d && !out.iter().any(|x| x == v) {
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

fn word_bm25(q: &str, records: &[SearchRecord], vocab: &HashSet<String>) -> Vec<String> {
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

fn word_hit(qtok: &str, doc: &str) -> bool {
    if doc == qtok {
        return true;
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

fn trigram_rank(q: &str, records: &[SearchRecord]) -> Vec<String> {
    let q_forms: Vec<String> = {
        let mut forms = vec![q.to_string()];
        let latin = latin_fold(q);
        if latin != q {
            forms.push(latin);
        }
        for tok in tokens(q) {
            for form in token_forms(&tok) {
                if !forms.contains(&form) {
                    forms.push(form);
                }
            }
        }
        forms
    };

    let mut scored: Vec<(String, f32)> = Vec::new();
    for rec in records {
        let mut best = 0.0_f32;
        for field in rec.analyzed() {
            let scale = field.weight / W_NAME;
            for key in &field.keys {
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
}
