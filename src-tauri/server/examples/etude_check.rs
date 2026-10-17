//! Проверка этюдов ботами: выигрывается ли этюд и чем.
//!
//! Этюд — расстановка с решением, а не лотерея: жадный бот (глубина 1) должен
//! его в основном проигрывать, а думающий (2–3) — находить решение. Движок без
//! случая, поэтому одна расстановка даёт одну партию на каждую глубину.
//!
//!     curl -s -H "Authorization: Bearer $ADMIN_KEY" localhost:3000/api/v1/admin/battles/cards > cards.json
//!     cargo run --release --example etude_check -- cards.json etudes.json
//!
//! `etudes.json` — список `{ "slug", "botDepth", "setup": {…ChallengeSetup…} }`.

use battle_core::*;
use gotiga_server::battles::{check_field, check_terrain, rules_of, to_snapshot_with};
use gotiga_server::models::{BattleCardDto, ChallengeSetup};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Etude {
    slug: String,
    #[serde(default = "one")]
    bot_depth: u8,
    setup: ChallengeSetup,
    /// Проверка «без механики»: частичные правки карт (`{слаг: {поле: значение}}`).
    #[serde(default)]
    card_patches: Option<serde_json::Value>,
    /// Убрать расовый урон у всех карт.
    #[serde(default)]
    no_race: bool,
}
fn one() -> u8 { 1 }

struct Played {
    outcome: Option<Outcome>,
    rounds: u8,
    acts: u32,
    /// чары и лечение игрока, и выигрыш прорывом: чем именно выиграно
    casts: u32,
    mends: u32,
    breached: bool,
}

fn raise(setup: &ChallengeSetup, cards: &BTreeMap<String, BattleCardDto>) -> Result<Setup, String> {
    let body = |slug: &str| -> Result<CardSnapshot, String> {
        cards.get(slug).map(|c| to_snapshot_with(c, |w| cards.get(w))).ok_or(format!("нет карты {slug}"))
    };
    let field = check_field(setup.field).map_err(|e| e.to_string())?;
    let place = |list: &[gotiga_server::models::ChallengePlacement]| -> Result<Vec<_>, String> {
        list.iter()
            .map(|p| Ok((body(&p.card)?, field.cell(p.x, p.y).ok_or(format!("нет клетки {},{}", p.x, p.y))?)))
            .collect()
    };
    let hand = |l: &[String]| -> Result<Vec<_>, String> { l.iter().map(|s| body(s)).collect() };
    Ok(Setup {
        player_board: place(&setup.player_board)?,
        player_hand: hand(&setup.player_hand)?,
        keeper_board: place(&setup.keeper_board)?,
        keeper_hand: hand(&setup.keeper_hand)?,
        terrain: check_terrain(setup).map_err(|e| e.to_string())?,
        field,
    })
}

fn play(setup: Setup, rules: Rules, player_depth: u8, keeper_depth: u8, budget: Duration) -> Played {
    let end = Instant::now() + budget;
    let mut st = MatchState::begin_with(setup, rules);
    let (mut guard, mut acts, mut casts, mut mends, mut breached) = (0, 0u32, 0u32, 0u32, false);
    while st.outcome.is_none() && guard < 4000 && Instant::now() < end {
        let depth = if st.active == Side::Player { player_depth } else { keeper_depth };
        let action = bot::choose_at(&st, depth);
        if st.active == Side::Player && !matches!(action, Action::EndTurn) {
            acts += 1;
            match action { Action::Cast { .. } => casts += 1, Action::Mend { .. } => mends += 1, _ => {} }
        }
        st = match reduce(&st, &action) {
            Ok((n, ev)) => {
                if ev.iter().any(|e| matches!(e, Event::Breached { side: Side::Player, .. })) { breached = true; }
                n
            }
            Err(_) => match reduce(&st, &Action::EndTurn) { Ok((n, _)) => n, Err(_) => break },
        };
        guard += 1;
    }
    Played { outcome: st.outcome, rounds: st.round, acts, casts, mends, breached }
}

fn line(setup: Setup, rules: Rules, keeper_depth: u8) {
    let mut st = MatchState::begin_with(setup, rules);
    let name = |st: &MatchState, id: UnitId| st.units[id as usize].name().to_string();
    let mut round = 0u8;
    let mut guard = 0;
    while st.outcome.is_none() && guard < 4000 {
        let depth = if st.active == Side::Player { 3 } else { keeper_depth };
        let action = bot::choose_at(&st, depth);
        if st.round != round { round = st.round; println!("  круг {round}:"); }
        let who = if st.active == Side::Player { "игрок" } else { "дом   " };
        let text = match &action {
            Action::Play { hand_index, cell } => format!("выставил карту из руки №{hand_index} на {},{}", cell.x, cell.y),
            Action::Move { unit, to } => format!("{} идёт на {},{}", name(&st, *unit), to.x, to.y),
            Action::Attack { attacker, target } => format!("{} бьёт {}", name(&st, *attacker), name(&st, *target)),
            Action::Mend { healer, target } => format!("{} лечит {}", name(&st, *healer), name(&st, *target)),
            Action::Cast { caster, ability, target } => {
                let t = match target { Mark::Unit(u) => name(&st, *u), Mark::Spot(c) => format!("клетку {},{}", c.x, c.y) };
                format!("{} чарой «{}» на {}", name(&st, *caster), ability, t)
            }
            Action::EndTurn => "конец хода".to_string(),
        };
        if !matches!(action, Action::EndTurn) { println!("    {who}: {text}"); }
        st = match reduce(&st, &action) { Ok((n, _)) => n, Err(_) => match reduce(&st, &Action::EndTurn) { Ok((n, _)) => n, Err(_) => break } };
        guard += 1;
    }
    println!("  итог: {:?}", st.outcome);
}

fn cell(p: &Played) -> String {
    match p.outcome {
        Some(Outcome::Player) => format!("ВЫИГРАЛ {:2} кр. {:3} дейст.", p.rounds, p.acts),
        Some(Outcome::Keeper) => format!("проиграл {:2} кр.          ", p.rounds),
        Some(Outcome::Draw) => format!("ничья    {:2} кр.          ", p.rounds),
        None => "не доиграл (время)    ".to_string(),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dtos: Vec<BattleCardDto> = {
        let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
        serde_json::from_value(v.get("cards").cloned().unwrap_or(v)).unwrap()
    };
    let cards: BTreeMap<String, BattleCardDto> = dtos.into_iter().map(|c| (c.slug.clone(), c)).collect();
    let etudes: Vec<Etude> = serde_json::from_str(&std::fs::read_to_string(&args[2]).unwrap()).unwrap();
    let only: Vec<&String> = args.iter().skip(3).collect();
    let json = std::env::var("ETUDE_JSON").is_ok();
    let budget = Duration::from_secs(std::env::var("ETUDE_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(90));
    if !json {
        println!("{:24} {:25} {:25} {:25}", "этюд", "жадный (1)", "думающий (2)", "думающий (3)");
    }
    for e in etudes.iter().filter(|e| only.is_empty() || only.contains(&&e.slug)) {
        let mut local = cards.clone();
        if let Some(p) = e.card_patches.as_ref().and_then(|p| p.as_object()) {
            for (slug, patch) in p {
                if let Some(c) = local.get(slug) {
                    let mut v = serde_json::to_value(c).unwrap();
                    for (k, val) in patch.as_object().unwrap() { v[k] = val.clone(); }
                    local.insert(slug.clone(), serde_json::from_value(v).unwrap());
                }
            }
        }
        if e.no_race {
            for c in local.values_mut() { c.race_damage = battle_core::RaceDamage::default(); }
        }
        let setup = match raise(&e.setup, &local) {
            Ok(s) => s,
            Err(err) => { println!("{:24} ОШИБКА: {err}", e.slug); continue; }
        };
        let rules = rules_of(&e.setup);
        if std::env::var("ETUDE_LINE").is_ok() {
            println!("== {}", e.slug);
            line(setup.clone(), rules, e.bot_depth);
            continue;
        }
        let games: Vec<Played> = [1u8, 2, 3]
            .iter()
            .map(|&d| play(setup.clone(), rules, d, e.bot_depth, budget))
            .collect();
        if json {
            let one = |p: &Played| serde_json::json!({
                "o": match p.outcome { Some(Outcome::Player) => "W", Some(Outcome::Keeper) => "L", Some(Outcome::Draw) => "D", None => "T" },
                "rounds": p.rounds, "acts": p.acts, "casts": p.casts, "mends": p.mends, "breached": p.breached });
            println!("{}", serde_json::json!({"slug": e.slug, "d1": one(&games[0]), "d2": one(&games[1]), "d3": one(&games[2])}));
        } else {
            let row: Vec<String> = games.iter().map(cell).collect();
            println!("{:24} {} {} {}", e.slug, row[0], row[1], row[2]);
        }
    }
}
