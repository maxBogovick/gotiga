//! Замер единого ранжирования для поиска в шапке на наборе фраз `tools/search/cases.json`
//! поверх снимка данных `tools/search/corpus.json` (его делает `fetch_corpus.py`).
//!
//! Все вещи (работы, небылицы, листы вестника, карты) лежат в одном списке и
//! получают одну оценку: вектор (multilingual-e5) + слова (BM25 с опечатками) +
//! триграммы через RRF — тот же `search::hybrid_search`, что ищет работы. Для
//! сравнения считается и «только вектор».
//!
//! Мера — место первой верной вещи в общем списке: попала в первую / в три / в
//! шесть. Фразы, на которые ничего находиться не должно (`expect: []`), в эти
//! числа не входят: для них печатается, какую оценку получила лучшая вещь, —
//! по ней подбирается порог «молчать».
//!
//!   EMBED_WEIGHTS_DIR=/путь/к/весам \
//!     cargo run --release --example search_probe -- [-v] [--tag paraphrase]
//!
//! Тот же замер для прежнего поиска в браузере — `tools/search/baseline.md`.
use std::collections::HashMap;

use gotiga_server::embed;
use gotiga_server::search::{hybrid_search, keep_hits, SearchIndex, SearchRecord, SiteBoost};
use serde_json::Value;

struct Doc {
    id: String,
    kind: &'static str,
    label: String,
    name: String,
    body: String,
    /// Что вектор видит: заглавие, пояснение, начало текста.
    passage: String,
}

fn s(v: &Value) -> String {
    v.as_str().unwrap_or("").to_string()
}

/// Все значения объекта «язык → текст» подряд.
fn all(v: &Value) -> Vec<String> {
    v.as_object()
        .map(|o| o.values().filter_map(|x| x.as_str().map(str::to_string)).filter(|x| !x.is_empty()).collect())
        .unwrap_or_default()
}

fn first(v: &Value, langs: &[&str]) -> String {
    for l in langs {
        if let Some(t) = v.get(*l).and_then(Value::as_str) {
            if !t.is_empty() {
                return t.to_string();
            }
        }
    }
    String::new()
}

fn cut(text: &str, n: usize) -> String {
    text.chars().take(n).collect()
}

fn docs(corpus: &Value) -> Vec<Doc> {
    let mut out = Vec::new();
    for w in corpus["works"].as_array().unwrap() {
        let name = s(&w["name"]);
        let names = all(&w["names"]);
        let shorts = all(&w["short"]);
        let craft = [s(&w["material"]), s(&w["technique"]), s(&w["series"])].join(" ");
        let en = first(&w["short"], &["en", "ru"]);
        out.push(Doc {
            id: s(&w["id"]),
            kind: "work",
            label: name.clone(),
            name: [vec![name.clone()], names.clone()].concat().join(" "),
            body: [shorts.join(" "), craft.clone()].join(" "),
            passage: format!("{}. {}. {}", name, cut(&en, 300), craft.trim()),
        });
    }
    for (key, kind) in [("tales", "tale"), ("gazette", "leaf")] {
        for l in corpus[key].as_array().unwrap() {
            let titles = all(&l["title"]);
            let works = vec![s(&l["work"])];
            let works_all: Vec<String> = l["works"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
            let en_title = first(&l["title"], &["en", "ru"]);
            let dek = first(&l["dek"], &["en", "ru"]);
            let body = first(&l["body"], &["en", "ru"]);
            out.push(Doc {
                id: s(&l["id"]),
                kind,
                label: en_title.clone(),
                name: [titles, works, works_all].concat().join(" "),
                body: [all(&l["dek"]).join(" "), all(&l["body"]).join(" ")].join(" "),
                passage: format!("{}. {}. {}", en_title, dek, cut(&body, 500)),
            });
        }
    }
    for c in corpus["cards"].as_array().unwrap() {
        let titles = all(&c["title"]);
        let works_all: Vec<String> = c["works"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
        let en_title = first(&c["title"], &["en", "ru"]);
        let race = first(&c["race"], &["en", "ru"]);
        let effect = first(&c["effect"], &["en", "ru"]);
        let lore = first(&c["lore"], &["en", "ru"]);
        out.push(Doc {
            id: s(&c["id"]),
            kind: "card",
            label: format!("{} ({})", en_title, race),
            name: [titles, vec![s(&c["work"])], works_all].concat().join(" "),
            body: [all(&c["race"]).join(" "), all(&c["effect"]).join(" "), all(&c["lore"]).join(" ")].join(" "),
            passage: format!("{} — {}. {}. {}. {}", en_title, race, s(&c["work"]), cut(&effect, 200), cut(&lore, 200)),
        });
    }
    out
}

struct Tally {
    n: usize,
    at1: usize,
    at3: usize,
    at6: usize,
    rr: f32,
}

impl Tally {
    fn new() -> Self {
        Tally { n: 0, at1: 0, at3: 0, at6: 0, rr: 0.0 }
    }
    fn add(&mut self, rank: Option<usize>) {
        self.n += 1;
        if let Some(r) = rank {
            if r == 0 { self.at1 += 1; }
            if r < 3 { self.at3 += 1; }
            if r < 6 { self.at6 += 1; }
            self.rr += 1.0 / (r as f32 + 1.0);
        }
    }
    fn line(&self, name: &str) -> String {
        let p = |x: usize| 100.0 * x as f32 / self.n.max(1) as f32;
        format!(
            "{:<12} {:>3}  первая {:>5.1}%  в трёх {:>5.1}%  в шести {:>5.1}%  MRR {:.3}",
            name, self.n, p(self.at1), p(self.at3), p(self.at6), self.rr / self.n.max(1) as f32
        )
    }
}

fn rank_of(order: &[String], expect: &[String]) -> Option<usize> {
    order.iter().position(|id| expect.iter().any(|e| id.starts_with(e.as_str())))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let verbose = args.iter().any(|a| a == "-v");
    let only_tag = args.iter().position(|a| a == "--tag").and_then(|i| args.get(i + 1)).cloned();
    let dir = std::env::var("SEARCH_DIR").unwrap_or_else(|_| "../../tools/search".into());
    let corpus: Value = serde_json::from_str(&std::fs::read_to_string(format!("{dir}/corpus.json")).unwrap()).unwrap();
    let cases: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(format!("{dir}/cases.json")).unwrap()).unwrap();
    assert!(embed::is_available(), "нет весов: задайте EMBED_WEIGHTS_DIR");

    let docs = docs(&corpus);
    eprintln!("вещей: {}; считаю векторы…", docs.len());
    let vecs: Vec<Vec<f32>> = docs.iter().map(|d| embed::embed_passage(&d.passage).unwrap()).collect();
    let records: Vec<SearchRecord> = docs.iter().map(|d| SearchRecord::from_text(&d.id, &d.name, &d.body)).collect();
    let built = std::time::Instant::now();
    let index = SearchIndex::new(&records);
    eprintln!("индекс собран за {:.0} мс", built.elapsed().as_secs_f64() * 1000.0);
    let by_id: HashMap<&str, &Doc> = docs.iter().map(|d| (d.id.as_str(), d)).collect();

    // Прежний поиск в браузере дополнял найденное словарём порядком смыслового
    // `/search` — только по работам. Его порядок выгружается для замера прежнего
    // поиска (`baseline.js`), потому что локальный сервер в отладочной сборке
    // считает один запрос секунд пятнадцать.
    if let Some(path) = args.iter().position(|a| a == "--dump-meaning").and_then(|i| args.get(i + 1)) {
        let work_idx: Vec<usize> = docs.iter().enumerate().filter(|(_, d)| d.kind == "work").map(|(i, _)| i).collect();
        let work_records: Vec<SearchRecord> = work_idx.iter().map(|&i| records[i].clone()).collect();
        let mut out = serde_json::Map::new();
        for case in &cases {
            let q = s(&case["q"]);
            if q.trim().chars().count() < 3 {
                continue;
            }
            let qv = embed::embed_query(&q).unwrap();
            let scores: Vec<(String, f32)> = work_idx.iter().map(|&i| (docs[i].id.clone(), embed::dot(&qv, &vecs[i]))).collect();
            let hits = hybrid_search(&q, &work_records, &scores, 24);
            out.insert(q, Value::Array(hits.into_iter().map(|h| Value::String(h.id)).collect()));
        }
        std::fs::write(path, serde_json::to_string(&Value::Object(out)).unwrap()).unwrap();
        eprintln!("порядок выгружен: {path}");
        return;
    }

    // `--bench N`: сколько времени уходит на запрос: вектор запроса отдельно, слова отдельно.
    if let Some(n) = args.iter().position(|a| a == "--bench").and_then(|i| args.get(i + 1)).and_then(|v| v.parse::<usize>().ok()) {
        let qs = ["ведьма с метлой", "гном с топором", "bruja con escoba", "красная шапочка", "pizza recipe", "old woman with cats"];
        let t = std::time::Instant::now();
        let vecs_q: Vec<Vec<f32>> = qs.iter().map(|q| embed::embed_query(q).unwrap()).collect();
        println!("вектор запроса: {:.0} мс на запрос (e5-small, CPU)", t.elapsed().as_secs_f64() * 1000.0 / qs.len() as f64);
        let scored: Vec<Vec<(String, f32)>> = vecs_q.iter().map(|qv| docs.iter().zip(&vecs).map(|(d, v)| (d.id.clone(), embed::dot(qv, v))).collect()).collect();
        let boost = SiteBoost { name: 0.01, any: 0.01, common: 0.25 };
        println!("bench начат");
        let t = std::time::Instant::now();
        for _ in 0..n {
            for (q, sc) in qs.iter().zip(&scored) {
                let _ = index.site(q, sc, 40, boost);
            }
        }
        println!("слова и триграммы: {:.0} мс на запрос ({} записей)", t.elapsed().as_secs_f64() * 1000.0 / (n * qs.len()) as f64, records.len());
        let tokens: usize = docs.iter().map(|d| d.body.split_whitespace().count() + d.name.split_whitespace().count()).sum();
        println!("слов во всех текстах: {tokens}");
        return;
    }

    // `--query "weapon"`: один запрос, весь список с оценкой гибрида и близостью по вектору.
    if let Some(q) = args.iter().position(|a| a == "--query").and_then(|i| args.get(i + 1)) {
        let qv = embed::embed_query(q).unwrap();
        let mut scores: Vec<(String, f32)> = docs.iter().zip(&vecs).map(|(d, v)| (d.id.clone(), embed::dot(&qv, v))).collect();
        let cos: HashMap<String, f32> = scores.iter().map(|(i, c)| (i.clone(), *c)).collect();
        let all_scores = scores.clone();
        // `--no-vector`: показать только то, что дают слова и триграммы.
        if args.iter().any(|a| a == "--no-vector") {
            scores.clear();
        }
        let boost = SiteBoost { name: 0.01, any: 0.01, common: 0.25 };
        let hits = index.site(q, &scores, 12, boost);
        println!("«{q}»: гибрид (порог сервера 0,02 отбрасывает всё, что ниже)");
        for h in &hits {
            let d = by_id[h.id.as_str()];
            let mark = if h.score >= 0.02 { "показано" } else { "скрыто  " };
            println!("  {mark} оценка {:.4}  близость {:.3}  [{}] {}", h.score, cos[&h.id], d.kind, d.label);
        }
        // `--find dbb7c363`: где стоит эта вещь по вектору, по словам и в итоге.
        if let Some(prefix) = args.iter().position(|a| a == "--find").and_then(|i| args.get(i + 1)) {
            let mut sorted: Vec<&(String, f32)> = all_scores.iter().collect();
            sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            let pos = sorted.iter().position(|(id, _)| id.starts_with(prefix.as_str()));
            let d = docs.iter().find(|d| d.id.starts_with(prefix.as_str())).unwrap();
            println!("--- {} [{}] ---", d.label, d.kind);
            println!("текст, по которому считается вектор: {}", d.passage.chars().take(400).collect::<String>());
            match pos {
                Some(p) => println!("по вектору: место {} из {}, близость {:.3} (лучшая {:.3})", p + 1, sorted.len(), sorted[p].1, sorted[0].1),
                None => println!("по вектору: нет"),
            }
            let lex = index.hybrid(q, &[], records.len());
            match lex.iter().position(|h| h.id.starts_with(prefix.as_str())) {
                Some(p) => println!("по словам и триграммам: место {}", p + 1),
                None => println!("по словам и триграммам: НЕТ В СПИСКЕ (в тексте нет ни слова запроса, ни похожего)"),
            }
            let full = index.hybrid(q, &all_scores, records.len());
            match full.iter().position(|h| h.id.starts_with(prefix.as_str())) {
                Some(p) => println!("итог гибрида: место {}, оценка {:.4} (порог показа 0.02)", p + 1, full[p].score),
                None => println!("итог гибрида: нет"),
            }
            println!("слова запроса в тексте вещи: {}", q.split_whitespace().map(|w| format!("«{w}» — {}", if d.passage.to_lowercase().contains(&w.to_lowercase()) || d.name.to_lowercase().contains(&w.to_lowercase()) || d.body.to_lowercase().contains(&w.to_lowercase()) {"есть"} else {"НЕТ"})).collect::<Vec<_>>().join(", "));
            return;
        }
        let mut by_cos: Vec<&(String, f32)> = all_scores.iter().collect();
        by_cos.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        println!("лучшие по одной близости:");
        for (id, c) in by_cos.iter().take(8) {
            println!("  близость {c:.3}  [{}] {}", by_id[id.as_str()].kind, by_id[id.as_str()].label);
        }
        return;
    }

    // Векторы фраз считаются один раз: перебор настроек их не меняет.
    struct Prepared {
        q: String,
        tag: String,
        expect: Vec<String>,
        scores: Vec<(String, f32)>,
    }
    let prepared: Vec<Prepared> = cases
        .iter()
        .filter(|c| only_tag.as_ref().is_none_or(|t| *t == s(&c["tag"])))
        .map(|c| {
            let q = s(&c["q"]);
            // `--no-vector`: сервер без весов модели — оценку дают только слова и триграммы.
            let mut scores: Vec<(String, f32)> = if args.iter().any(|a| a == "--no-vector") {
                Vec::new()
            } else {
                let qv = embed::embed_query(&q).unwrap();
                docs.iter().zip(&vecs).map(|(d, v)| (d.id.clone(), embed::dot(&qv, v))).collect()
            };
            scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            Prepared {
                q,
                tag: s(&c["tag"]),
                expect: c["expect"].as_array().unwrap().iter().map(|x| s(x)).collect(),
                scores,
            }
        })
        .collect();

    /// Прогон одной настройки: (подбор: чётные фразы, проверка: нечётные, по видам).
    fn run(
        prepared: &[Prepared],
        index: &SearchIndex,
        boost: SiteBoost,
        floor: f32,
        vonly: f32,
    ) -> (Tally, Tally, HashMap<String, Tally>, usize) {
        let (mut tune, mut eval) = (Tally::new(), Tally::new());
        let mut by_tag: HashMap<String, Tally> = HashMap::new();
        let mut shown = 0usize;
        for (i, p) in prepared.iter().enumerate() {
            if p.expect.is_empty() {
                continue;
            }
            let hits = index.site(&p.q, &p.scores, 40, boost);
            // Порог на каждую вещь: ниже него вещь подтверждена одним списком из трёх.
            let hits = keep_hits(hits, &p.scores, floor, vonly);
            shown += hits.len();
            let order: Vec<String> = hits.iter().map(|h| h.id.clone()).collect();
            let r = rank_of(&order, &p.expect);
            if i % 2 == 0 { tune.add(r) } else { eval.add(r) }
            by_tag.entry(p.tag.clone()).or_insert_with(Tally::new).add(r);
        }
        (tune, eval, by_tag, shown)
    }

    // Только вектор — для сравнения.
    let mut dense = Tally::new();
    for p in &prepared {
        if p.expect.is_empty() {
            continue;
        }
        let order: Vec<String> = p.scores.iter().map(|(id, _)| id.clone()).collect();
        dense.add(rank_of(&order, &p.expect));
    }
    println!("{}", dense.line("только вектор"));

    let sweep = args.iter().any(|a| a == "--sweep");
    if sweep {
        let mut rows: Vec<(f32, SiteBoost, Tally, Tally)> = Vec::new();
        for name in [0.0f32, 0.01, 0.02, 0.03, 0.05] {
            for any in [0.0f32, 0.01] {
                for common in [0.25f32, 0.4] {
                    let b = SiteBoost { name, any, common };
                    let (tune, eval, _, _) = run(&prepared, &index, b, 0.0, 0.0);
                    let mrr = tune.rr / tune.n.max(1) as f32;
                    rows.push((mrr, b, tune, eval));
                }
            }
        }
        rows.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        println!("\n=== лучшие по подбору (чётные фразы); проверка — нечётные, настройки её не видели ===");
        for (_, b, tune, eval) in rows.iter().take(8) {
            println!("name {:.3} any {:.3} common {:.2}\n    подбор  {}\n    проверка {}", b.name, b.any, b.common, tune.line(""), eval.line(""));
        }
        let (tune, eval, _, _) = run(&prepared, &index, SiteBoost::NONE, 0.0, 0.0);
        println!("\nбез бонуса:\n    подбор  {}\n    проверка {}", tune.line(""), eval.line(""));
        return;
    }

    let boost = SiteBoost {
        name: arg_f32(&args, "--name").unwrap_or(0.0),
        any: arg_f32(&args, "--any").unwrap_or(0.0),
        common: arg_f32(&args, "--common").unwrap_or(1.0),
    };
    let floor = arg_f32(&args, "--floor").unwrap_or(0.0);
    let vonly = arg_f32(&args, "--vonly").unwrap_or(0.0);
    let started = std::time::Instant::now();
    let (tune, eval, by_tag, shown) = run(&prepared, &index, boost, floor, vonly);
    println!("время поиска без вектора: {:.0} мс на фразу", started.elapsed().as_secs_f64() * 1000.0 / (tune.n + eval.n).max(1) as f64);
    println!("\nнастройка: {boost:?}, порог на вещь {floor}; в среднем вещей в ответе {:.1}", shown as f32 / (tune.n + eval.n).max(1) as f32);
    println!("{}", tune.line("подбор"));
    println!("{}", eval.line("проверка"));
    let mut tags: Vec<_> = by_tag.keys().cloned().collect();
    tags.sort();
    for t in tags {
        println!("{}", by_tag[&t].line(&t));
    }

    if verbose {
        println!();
        for p in &prepared {
            if p.expect.is_empty() {
                continue;
            }
            let hits = index.site(&p.q, &p.scores, 40, boost);
            let order: Vec<String> = hits.iter().map(|h| h.id.clone()).collect();
            let r = rank_of(&order, &p.expect);
            if r.is_none_or(|r| r >= 1) {
                println!("— {:?}: верная на месте {:?}", p.q, r.map(|r| r + 1));
                for h in hits.iter().take(4) {
                    let d = by_id[h.id.as_str()];
                    let mark = if p.expect.iter().any(|e| h.id.starts_with(e.as_str())) { "✓" } else { " " };
                    println!("    {mark} {:.4} [{}] {}", h.score, d.kind, d.label);
                }
            }
        }
    }

    // Мусорные запросы не должны ничего показывать; «оружие» и подобные — должны.
    let mut shown_for = |qs: &[&str]| -> Vec<(String, usize)> {
        qs.iter()
            .map(|q| {
                let qv = embed::embed_query(q).unwrap();
                let scores: Vec<(String, f32)> = docs.iter().zip(&vecs).map(|(d, v)| (d.id.clone(), embed::dot(&qv, v))).collect();
                let hits = index.site(q, &scores, 40, boost);
                (q.to_string(), keep_hits(hits, &scores, floor, vonly).len())
            })
            .collect()
    };
    let junk = ["pizza recipe", "bitcoin price", "football score", "dragon", "weather tomorrow", "how to fix a car", "asdf qwer zxcv", "курс доллара", "ракетный двигатель", "президент франции", "футбол", "квантовая физика формулы", "best laptop 2026", "space rocket", "stock market"];
    let junk_shown = shown_for(&junk);
    println!("\n=== мусорные запросы: сколько вещей показано (нужно 0) ===");
    println!("{}", junk_shown.iter().map(|(q, n)| format!("{q}: {n}")).collect::<Vec<_>>().join(" · "));
    let wanted = shown_for(&["weapon", "weapons", "оружие", "sword", "axe", "меч", "топор"]);
    println!("=== запросы про оружие: сколько показано (нужно > 0) ===");
    println!("{}", wanted.iter().map(|(q, n)| format!("{q}: {n}")).collect::<Vec<_>>().join(" · "));

    println!("\n=== фразы без верного ответа: оценка лучшей вещи ===");
    for p in &prepared {
        if !p.expect.is_empty() {
            continue;
        }
        let hits = index.site(&p.q, &p.scores, 40, boost);
        let top = hits.first();
        let name = top.and_then(|h| by_id.get(h.id.as_str())).map(|d| d.label.clone()).unwrap_or_default();
        println!("{:<20} оценка {:.4}  лучшая: {name}", p.q, top.map(|h| h.score).unwrap_or(0.0));
    }
    println!("\n=== верные фразы: оценка лучшей вещи (наименьшие) ===");
    let mut lows: Vec<(f32, String)> = prepared
        .iter()
        .filter(|p| !p.expect.is_empty())
        .map(|p| (index.site(&p.q, &p.scores, 1, boost).first().map(|h| h.score).unwrap_or(0.0), p.q.clone()))
        .collect();
    lows.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    for (sc, q) in lows.iter().take(8) {
        println!("{q:<40} {sc:.4}");
    }
}

fn arg_f32(args: &[String], name: &str) -> Option<f32> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok())
}
