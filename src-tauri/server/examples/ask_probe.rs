//! Замер: хватает ли поиска, чтобы узнавать вопрос посетителя среди записей
//! каталога «Спросить сайт» (`tools/ask/catalog*.json`).
//!
//! Два режима на одних и тех же вопросах:
//!   * `dense`  — только вектор: лучшая из формулировок записи, порог и разрыв;
//!   * `hybrid` — `search::hybrid_search` (вектор + BM25 + триграммы через RRF):
//!                оценка RRF — это ранги, поэтому порог берётся по ней же.
//! Вопрос с `expect: null` не должен находить ничего (поиск по теме, вопрос не
//! про сайт, вопрос, которым владеют правила). Верный ответ +1, уверенный
//! неверный −3: молчание лучше уверенной ошибки.
//!
//!   EMBED_WEIGHTS_DIR=/путь/к/весам \
//!     cargo run --release --example ask_probe -- <catalog.json> <cases.json> [-v]
//!
//! Вопрос и формулировка кладутся с одним префиксом `query:` (сравнение вопроса с
//! вопросом), поэтому используется только `embed_query`.
use gotiga_server::embed;
use gotiga_server::search::{hybrid_search, SearchRecord};
use serde::Deserialize;

#[derive(Deserialize)]
struct Entry {
    id: String,
    phrases: Vec<String>,
}

#[derive(Deserialize)]
struct Case {
    q: String,
    expect: Option<String>,
}

/// Что вернул режим на одном вопросе: запись и две меры уверенности.
struct Pick {
    id: String,
    /// Уровень: сходство (dense) или оценка RRF (hybrid).
    level: f32,
    /// Разрыв с второй записью в тех же единицах.
    gap: f32,
    /// Первая запись гибридного поиска совпала с первой по вектору.
    agree: bool,
    /// Вторая запись по вектору и её сходство: из них строится переспрос.
    id2: String,
    /// Место верной записи в порядке по вектору (0 — первая); `usize::MAX`, если молчать.
    rank: usize,
}

#[derive(Clone, Copy)]
struct Tally {
    answered: usize,
    right: usize,
    wrong: usize,
    wrong_out: usize,
}

/// `rescue`: на сколько ниже порога вектор всё ещё отвечает, если с ним согласен
/// гибридный поиск (BM25 и триграммы подтверждают пограничный случай).
fn tally(picks: &[Pick], cases: &[Case], level: f32, gap: f32, rescue: Option<f32>) -> Tally {
    let mut t = Tally { answered: 0, right: 0, wrong: 0, wrong_out: 0 };
    for (p, c) in picks.iter().zip(cases) {
        let strong = p.level >= level && p.gap >= gap;
        let rescued = rescue.is_some_and(|d| p.agree && p.level >= level - d);
        if !strong && !rescued {
            continue;
        }
        t.answered += 1;
        match c.expect.as_deref() {
            Some(e) if e == p.id => t.right += 1,
            Some(_) => t.wrong += 1,
            None => {
                t.wrong += 1;
                t.wrong_out += 1;
            }
        }
    }
    t
}

/// Порог подбирается на `tune` (верный +1, уверенно неверный −3) и затем
/// применяется без изменений к `eval`, которого подбор не видел.
fn report(
    name: &str,
    tune: (&[Pick], &[Case]),
    eval: (&[Pick], &[Case]),
    levels: &[f32],
    gaps: &[f32],
    rescue: Option<f32>,
    verbose: bool,
) {
    let mut best = (f32::MIN, 0.0f32, 0.0f32);
    for &l in levels {
        for &g in gaps {
            let t = tally(tune.0, tune.1, l, g, rescue);
            let score = t.right as f32 - 3.0 * t.wrong as f32;
            if score > best.0 {
                best = (score, l, g);
            }
        }
    }
    let show = |label: &str, picks: &[Pick], cases: &[Case]| {
        let in_scope = cases.iter().filter(|c| c.expect.is_some()).count();
        let top1 = picks
            .iter()
            .zip(cases)
            .filter(|(p, c)| c.expect.as_deref() == Some(p.id.as_str()))
            .count();
        let t = tally(picks, cases, best.1, best.2, rescue);
        println!(
            "  {name:<7} {label:<6} первая верная {top1:>3}/{in_scope} | отвечено {:>3}, верно {:>3} ({:>2.0}% охвата), уверенно-неверно {:>2} (мимо каталога {})",
            t.answered,
            t.right,
            100.0 * t.right as f32 / in_scope as f32,
            t.wrong,
            t.wrong_out
        );
    };
    println!("{name}: порог {:.3}, разрыв {:.3}", best.1, best.2);
    show("подбор", tune.0, tune.1);
    show("проверка", eval.0, eval.1);
    if verbose {
        for (p, c) in eval.0.iter().zip(eval.1) {
            let strong = p.level >= best.1 && p.gap >= best.2;
            let rescued = rescue.is_some_and(|d| p.agree && p.level >= best.1 - d);
            let got = if strong || rescued { Some(p.id.as_str()) } else { None };
            if got != c.expect.as_deref() {
                println!(
                    "    «{}» → ждали {:?}, вышло {:?} (лучшая {} = {:.3}, разрыв {:.3})",
                    c.q, c.expect, got, p.id, p.level, p.gap
                );
            }
        }
    }
}


// ── Серая зона: две близкие записи → переспрос ────────────────────────────

/// Пороги трёх исходов: ответить, переспросить, промолчать.
#[derive(Clone, Copy)]
struct Zones {
    /// Ответ без вопросов: сходство и разрыв (как в `dense`).
    answer_level: f32,
    answer_gap: f32,
    /// Подтверждение гибридным поиском снижает порог ответа на столько.
    rescue: f32,
    /// Переспрос: лучшая запись не ниже этого сходства…
    ask_level: f32,
    /// …и вторая не дальше этого от лучшей («две близкие»).
    ask_gap: f32,
}

enum Outcome<'a> {
    Answer(&'a str),
    Ask(&'a str, &'a str),
    Silent,
}

fn decide<'a>(p: &'a Pick, z: &Zones, ask_on: bool) -> Outcome<'a> {
    let strong = p.level >= z.answer_level && p.gap >= z.answer_gap;
    let rescued = p.agree && p.level >= z.answer_level - z.rescue;
    if strong || rescued {
        return Outcome::Answer(&p.id);
    }
    if ask_on && p.level >= z.ask_level && p.gap <= z.ask_gap {
        return Outcome::Ask(&p.id, &p.id2);
    }
    Outcome::Silent
}

#[derive(Default, Clone, Copy)]
struct Counts {
    right: usize,
    wrong: usize,
    /// Переспрос, в паре которого есть верная запись.
    ask_hit: usize,
    /// Переспрос, в паре которого верной записи нет.
    ask_miss: usize,
    /// Переспрос там, где каталог должен был молчать.
    ask_false: usize,
    /// Промолчали, хотя ответ в каталоге был.
    silent_missed: usize,
    /// Промолчали там, где и надо.
    silent_ok: usize,
}

fn count(picks: &[Pick], cases: &[Case], z: &Zones, ask_on: bool) -> Counts {
    let mut c = Counts::default();
    for (p, case) in picks.iter().zip(cases) {
        let want = case.expect.as_deref();
        match (decide(p, z, ask_on), want) {
            (Outcome::Answer(a), Some(w)) if a == w => c.right += 1,
            (Outcome::Answer(_), _) => c.wrong += 1,
            (Outcome::Ask(a, b), Some(w)) if a == w || b == w => c.ask_hit += 1,
            (Outcome::Ask(..), Some(_)) => c.ask_miss += 1,
            (Outcome::Ask(..), None) => c.ask_false += 1,
            (Outcome::Silent, Some(_)) => c.silent_missed += 1,
            (Outcome::Silent, None) => c.silent_ok += 1,
        }
    }
    c
}

/// Цена исхода при подборе. Уверенно неверный ответ дороже всего; переспрос с
/// верной парой стоит почти как ответ (один щелчок), без верной пары — хуже
/// молчания: посетителю показали два чужих варианта.
fn cost(c: &Counts) -> f32 {
    c.right as f32 + 0.6 * c.ask_hit as f32 - 3.0 * c.wrong as f32 - 1.0 * (c.ask_miss + c.ask_false) as f32
}

fn ask_report(tune: (&[Pick], &[Case]), eval: (&[Pick], &[Case]), rescue: f32, verbose: bool) {
    let mut grid = Vec::new();
    for ai in 0..=14 {
        let al = 0.80 + 0.01 * ai as f32;
        for ag in [0.0, 0.01, 0.02, 0.03, 0.04] {
            for ki in 0..=12 {
                let kl = 0.78 + 0.01 * ki as f32;
                if kl > al {
                    continue;
                }
                for kg in [0.01, 0.02, 0.03, 0.05, 0.08] {
                    grid.push(Zones { answer_level: al, answer_gap: ag, rescue, ask_level: kl, ask_gap: kg });
                }
            }
        }
    }
    let best = |ask_on: bool| {
        grid.iter()
            .copied()
            .max_by(|a, b| {
                cost(&count(tune.0, tune.1, a, ask_on))
                    .partial_cmp(&cost(&count(tune.0, tune.1, b, ask_on)))
                    .unwrap()
            })
            .unwrap()
    };
    // Потолок серой зоны: как часто верная запись стоит в первой двойке и тройке.
    for (name, picks, cases) in [("подбор", tune.0, tune.1), ("проверка", eval.0, eval.1)] {
        let n = cases.iter().filter(|c| c.expect.is_some()).count() as f32;
        let at = |k: usize| picks.iter().filter(|p| p.rank < k).count() as f32;
        println!(
            "Верная запись среди лучших по вектору ({name}): 1 — {:.0}%, 2 — {:.0}%, 3 — {:.0}%, 5 — {:.0}%",
            100.0 * at(1) / n,
            100.0 * at(2) / n,
            100.0 * at(3) / n,
            100.0 * at(5) / n
        );
    }
    println!("\nСерая зона (цена: верно +1, переспрос с верной парой +0,6, уверенно неверно −3, плохой переспрос −1):");
    for (label, on) in [("без переспроса", false), ("с переспросом", true)] {
        let z = best(on);
        println!(
            "{label}: ответ при сходстве ≥{:.2} и разрыве ≥{:.2}{}",
            z.answer_level,
            z.answer_gap,
            if on {
                format!("; переспрос при сходстве ≥{:.2}, если вторая запись не дальше {:.2}", z.ask_level, z.ask_gap)
            } else {
                String::new()
            }
        );
        for (name, picks, cases) in [("подбор", tune.0, tune.1), ("проверка", eval.0, eval.1)] {
            let c = count(picks, cases, &z, on);
            let in_scope = cases.iter().filter(|x| x.expect.is_some()).count();
            let silent = cases.len() - in_scope;
            let helped = c.right + c.ask_hit;
            println!(
                "  {name:<8} верный ответ {:>2}, ошибка {:>2} | переспрос: верная пара {:>2}, без верной {:>2}, лишний {:>2} | промолчали: зря {:>2}, как надо {:>2}/{silent} | помогли {:>2}/{in_scope} ({:.0}%), навредили {:>2}",
                c.right,
                c.wrong,
                c.ask_hit,
                c.ask_miss,
                c.ask_false,
                c.silent_missed,
                c.silent_ok,
                helped,
                100.0 * helped as f32 / in_scope as f32,
                c.wrong + c.ask_miss + c.ask_false
            );
        }
        if !on {
            // Если не отвечать, а всегда показывать лучшие записи кнопками.
            for (name, picks, cases) in [("подбор", tune.0, tune.1), ("проверка", eval.0, eval.1)] {
                let silent: Vec<&Pick> = picks
                    .iter()
                    .zip(cases)
                    .filter(|(p, c)| c.expect.is_some() && matches!(decide(p, &z, false), Outcome::Silent))
                    .map(|(p, _)| p)
                    .collect();
                let at = |k: usize| silent.iter().filter(|p| p.rank < k).count();
                println!(
                    "  {name:<8} из {} вопросов, на которые не ответили: верная запись среди 1 кнопки — {}, 2 — {}, 3 — {}",
                    silent.len(),
                    at(1),
                    at(2),
                    at(3)
                );
            }
        }
        if on && verbose {
            for (p, case) in eval.0.iter().zip(eval.1) {
                if let Outcome::Ask(a, b) = decide(p, &z, true) {
                    let w = case.expect.as_deref();
                    let tag = match w {
                        Some(w) if w == a || w == b => "верная пара",
                        Some(_) => "ПАРА БЕЗ ВЕРНОЙ",
                        None => "лишний",
                    };
                    println!("    «{}» → переспрос {a} / {b} (ждали {:?}) — {tag}", case.q, w);
                }
            }
        }
    }
}

fn picks_for(
    cases: &[Case],
    catalog: &[Entry],
    vectors: &[(usize, Vec<f32>)],
    records: &[SearchRecord],
) -> anyhow::Result<(Vec<Pick>, Vec<Pick>)> {
    let (mut dense, mut hybrid) = (Vec::new(), Vec::new());
    for c in cases {
        let qv = embed::embed_query(&c.q)?;
        let mut best = vec![f32::NEG_INFINITY; catalog.len()];
        for (i, v) in vectors {
            let s = embed::dot(&qv, v);
            if s > best[*i] {
                best[*i] = s;
            }
        }
        let mut order: Vec<usize> = (0..catalog.len()).collect();
        order.sort_by(|a, b| best[*b].partial_cmp(&best[*a]).unwrap());
        let top_id = catalog[order[0]].id.clone();

        let scores: Vec<(String, f32)> = catalog.iter().zip(&best).map(|(e, s)| (e.id.clone(), *s)).collect();
        let hits = hybrid_search(&gotiga_server::ask::content_words(&c.q), records, &scores, catalog.len());
        let (h1, h2) = (hits.first(), hits.get(1));
        let agree = h1.is_some_and(|h| h.id == top_id);
        dense.push(Pick {
            id: top_id,
            level: best[order[0]],
            gap: best[order[0]] - best[order[1]],
            agree,
            id2: catalog[order[1]].id.clone(),
            rank: c
                .expect
                .as_deref()
                .and_then(|w| order.iter().position(|&i| catalog[i].id == w))
                .unwrap_or(usize::MAX),
        });
        hybrid.push(match h1 {
            Some(h) => Pick {
                id: h.id.clone(),
                level: h.score,
                gap: h.score - h2.map(|x| x.score).unwrap_or(0.0),
                agree,
                id2: String::new(),
                rank: usize::MAX,
            },
            None => Pick { id: String::new(), level: 0.0, gap: 0.0, agree: false, id2: String::new(), rank: usize::MAX },
        });
    }
    Ok((dense, hybrid))
}


/// Сходство с каждой записью и первая запись поиска по словам — для каждого
/// вопроса. Считается один раз, пороги перебираются поверх.
fn signals(
    cases: &[Case],
    catalog: &[Entry],
    vectors: &[(usize, Vec<f32>)],
    records: &[SearchRecord],
) -> anyhow::Result<Vec<(Vec<f32>, Option<usize>)>> {
    let mut out = Vec::new();
    for c in cases {
        let qv = embed::embed_query(&c.q)?;
        let mut levels = vec![f32::NEG_INFINITY; catalog.len()];
        for (i, v) in vectors {
            let s = embed::dot(&qv, v);
            if s > levels[*i] {
                levels[*i] = s;
            }
        }
        let scores: Vec<(String, f32)> = catalog.iter().zip(&levels).map(|(e, s)| (e.id.clone(), *s)).collect();
        let hits = hybrid_search(&gotiga_server::ask::content_words(&c.q), records, &scores, catalog.len());
        let lexical_top = hits.first().and_then(|h| catalog.iter().position(|e| e.id == h.id));
        out.push((levels, lexical_top));
    }
    Ok(out)
}

#[derive(Default, Clone, Copy)]
struct Prod {
    right: usize,
    wrong: usize,
    hit: usize,
    miss: usize,
    noise: usize,
    silent_ok: usize,
    silent_missed: usize,
}

fn run_production(cases: &[Case], catalog: &[Entry], sig: &[(Vec<f32>, Option<usize>)], th: &gotiga_server::ask::Thresholds, mut stray: Option<&mut Vec<String>>) -> Prod {
    use gotiga_server::ask::{decide_with, Decision};
    let mut p = Prod::default();
    for (c, (levels, lexical_top)) in cases.iter().zip(sig) {
        let want = c.expect.as_deref();
        match decide_with(levels, *lexical_top, th) {
            Decision::Answer(i) => match want {
                Some(w) if w == catalog[i].id => p.right += 1,
                _ => {
                    p.wrong += 1;
                    if let Some(s) = stray.as_deref_mut() {
                        s.push(format!("«{}» → ответил {} (ждали {:?})", c.q, catalog[i].id, want));
                    }
                }
            },
            Decision::Suggest(v) => match want {
                Some(w) if v.iter().any(|&i| catalog[i].id == w) => p.hit += 1,
                Some(_) => {
                    p.miss += 1;
                    if let Some(s) = stray.as_deref_mut() {
                        s.push(format!("«{}» → кнопки {:?} без верной (ждали {:?})", c.q, v.iter().map(|&i| catalog[i].id.as_str()).collect::<Vec<_>>(), want));
                    }
                }
                None => p.noise += 1,
            },
            Decision::Nothing => match want {
                Some(_) => p.silent_missed += 1,
                None => p.silent_ok += 1,
            },
        }
    }
    p
}

/// Цена исхода: верный ответ +1, кнопки с верной +0,6, неверный ответ −3,
/// кнопки без верной или мимо каталога −0,3.
fn prod_cost(p: &Prod) -> f32 {
    p.right as f32 + 0.6 * p.hit as f32 - 3.0 * p.wrong as f32 - 0.3 * (p.miss + p.noise) as f32
}

fn show_prod(label: &str, name: &str, cases: &[Case], p: &Prod) {
    let in_scope = cases.iter().filter(|c| c.expect.is_some()).count();
    let silent = cases.len() - in_scope;
    println!(
        "  {label:<10} {name:<8} ответ верный {:>2}, неверный {:>2} | кнопки с верной {:>2}, без верной {:>2}, мимо каталога {:>2} | молчание: зря {:>2}, как надо {}/{silent} | помогли {:>2}/{in_scope} ({:.0}%)",
        p.right, p.wrong, p.hit, p.miss, p.noise, p.silent_missed, p.silent_ok, p.right + p.hit,
        100.0 * (p.right + p.hit) as f32 / in_scope as f32
    );
}

fn production(
    tune: (&[Case], &[(Vec<f32>, Option<usize>)]),
    eval: (&[Case], &[(Vec<f32>, Option<usize>)]),
    catalog: &[Entry],
    verbose: bool,
) {
    use gotiga_server::ask::Thresholds;
    println!("\nКак в работе (`ask::decide_with`): константы из ask.rs и пороги, подобранные на первом наборе.");
    let default = Thresholds::DEFAULT;
    for (name, set) in [("подбор", tune), ("проверка", eval)] {
        show_prod("константы", name, set.0, &run_production(set.0, catalog, set.1, &default, None));
    }
    let mut best: Option<(f32, Thresholds)> = None;
    for al in (84..=95).map(|x| x as f32 / 100.0) {
        for ag in [0.0, 0.01, 0.02, 0.03, 0.04] {
            for rs in [0.0, 0.01, 0.02, 0.03, 0.04] {
                for fl in [0.78, 0.80, 0.82, 0.83, 0.84, 0.86] {
                    let th = Thresholds { answer_level: al, answer_gap: ag, rescue: rs, candidate_floor: fl };
                    let cost = prod_cost(&run_production(tune.0, catalog, tune.1, &th, None));
                    if best.as_ref().is_none_or(|(c, _)| cost > *c) {
                        best = Some((cost, th));
                    }
                }
            }
        }
    }
    let (_, th) = best.unwrap();
    println!(
        "  подобрано на первом наборе: ответ при сходстве ≥{:.2} и разрыве ≥{:.2}, подтверждение словами снижает порог на {:.2}, кнопки при сходстве ≥{:.2}",
        th.answer_level, th.answer_gap, th.rescue, th.candidate_floor
    );
    for (name, set) in [("подбор", tune), ("проверка", eval)] {
        show_prod("подобрано", name, set.0, &run_production(set.0, catalog, set.1, &th, None));
    }
    if verbose {
        let mut stray = Vec::new();
        run_production(eval.0, catalog, eval.1, &th, Some(&mut stray));
        println!("  ошибки на проверке при подобранных порогах:");
        for line in stray {
            println!("    {line}");
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let verbose = args.iter().any(|a| a == "-v");
    let files: Vec<&String> = args.iter().filter(|a| *a != "-v").collect();
    let catalog_path = files.first().map(|s| s.as_str()).unwrap_or("../../tools/ask/catalog.json");
    let tune_path = files.get(1).map(|s| s.as_str()).unwrap_or("../../tools/ask/held_out.json");
    let eval_path = files.get(2).map(|s| s.as_str()).unwrap_or("../../tools/ask/held_out_2.json");
    let catalog: Vec<Entry> = serde_json::from_str(&std::fs::read_to_string(catalog_path)?)?;
    let tune: Vec<Case> = serde_json::from_str(&std::fs::read_to_string(tune_path)?)?;
    let eval: Vec<Case> = serde_json::from_str(&std::fs::read_to_string(eval_path)?)?;

    if !embed::is_available() {
        anyhow::bail!("нет весов: задайте EMBED_WEIGHTS_DIR (config.json, tokenizer.json, model.safetensors)");
    }

    let mut vectors: Vec<(usize, Vec<f32>)> = Vec::new();
    for (i, e) in catalog.iter().enumerate() {
        for p in &e.phrases {
            vectors.push((i, embed::embed_query(p)?));
        }
    }
    let records: Vec<SearchRecord> = catalog
        .iter()
        .map(|e| SearchRecord::from_text(&e.id, &e.phrases[0], &e.phrases[1..].join(" \n ")))
        .collect();

    let (td, th) = picks_for(&tune, &catalog, &vectors, &records)?;
    let (ed, eh) = picks_for(&eval, &catalog, &vectors, &records)?;
    let count = |c: &[Case]| (c.iter().filter(|x| x.expect.is_some()).count(), c.iter().filter(|x| x.expect.is_none()).count());
    let (ti, tn) = count(&tune);
    let (ei, en) = count(&eval);
    println!(
        "{catalog_path}: записей {}, формулировок {}\n  подбор {tune_path}: {ti} по каталогу + {tn} молчать; проверка {eval_path}: {ei} + {en}",
        catalog.len(),
        vectors.len()
    );
    let dl: Vec<f32> = (0..=20).map(|i| 0.74 + 0.01 * i as f32).collect();
    let dg = [0.0, 0.01, 0.02, 0.03, 0.04];
    let hl: Vec<f32> = (0..=24).map(|i| 0.016 + 0.001 * i as f32).collect();
    let hg = [0.0, 0.001, 0.002, 0.004, 0.006, 0.01];
    report("dense", (&td, &tune), (&ed, &eval), &dl, &dg, None, verbose);
    report("hybrid", (&th, &tune), (&eh, &eval), &hl, &hg, None, verbose);
    report("rescue", (&td, &tune), (&ed, &eval), &dl, &dg, Some(0.03), verbose);
    ask_report((&td, &tune), (&ed, &eval), 0.03, verbose);
    let (ts, es) = (signals(&tune, &catalog, &vectors, &records)?, signals(&eval, &catalog, &vectors, &records)?);
    production((&tune, &ts), (&eval, &es), &catalog, verbose);
    Ok(())
}
