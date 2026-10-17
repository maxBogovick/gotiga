//! Проверка встреч: решает ли выбор карт гостя.
//!
//! Встреча — это расставленная половина хранителя, а карты гость приводит
//! свои. Хорошая встреча награждает выбор: команда, собранная ПРОТИВ неё, должна
//! выигрывать заметно чаще случайной, а собранная НАОБОРОТ — проигрывать.
//!
//!     cargo run --release --example meeting_check -- cards.json meetings.json [игр]
//!
//! `meetings.json` — список `{ "slug", "botDepth", "setup", "counter": [раса EN…], "wrong": [раса EN…] }`.
//! `setup` несёт ТОЛЬКО половину хранителя, местность, поле и правила.

use battle_core::*;
use gotiga_server::battles::{check_field, check_terrain, rules_of, to_snapshot_with};
use gotiga_server::models::{BattleCardDto, ChallengeSetup};
use std::collections::BTreeMap;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Meeting {
    slug: String,
    #[serde(default = "one")]
    bot_depth: u8,
    setup: ChallengeSetup,
    #[serde(default)]
    counter: Vec<String>,
    #[serde(default)]
    wrong: Vec<String>,
}
fn one() -> u8 { 1 }

struct Roll(u64);
impl Roll {
    fn next(&mut self, upto: usize) -> usize {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % upto.max(1)
    }
}

fn team(pool: &[&BattleCardDto], favour: &[&BattleCardDto], from_favoured: usize, r: &mut Roll) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut fav: Vec<&BattleCardDto> = favour.to_vec();
    while out.len() < from_favoured && !fav.is_empty() {
        let c = fav.swap_remove(r.next(fav.len()));
        out.push(c.slug.clone());
    }
    let mut rest: Vec<&BattleCardDto> = pool.iter().copied().filter(|c| !out.contains(&c.slug)).collect();
    while out.len() < 6 && !rest.is_empty() {
        out.push(rest.swap_remove(r.next(rest.len())).slug.clone());
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dtos: Vec<BattleCardDto> = {
        let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
        serde_json::from_value(v.get("cards").cloned().unwrap_or(v)).unwrap()
    };
    let games: usize = args.get(3).and_then(|g| g.parse().ok()).unwrap_or(400);
    let all: BTreeMap<String, BattleCardDto> = dtos.into_iter().map(|c| (c.slug.clone(), c)).collect();
    let pool: Vec<&BattleCardDto> = all.values().filter(|c| c.status == "published" && c.kind != "structure" && c.tier <= 3).collect();
    let meetings: Vec<Meeting> = serde_json::from_str(&std::fs::read_to_string(&args[2]).unwrap()).unwrap();
    println!("{:26} {:>10} {:>10} {:>10}   (доля побед гостя с думающим ботом, {games} игр)", "встреча", "случайная", "против", "наоборот");
    for m in &meetings {
        let field = check_field(m.setup.field).unwrap();
        let terrain = check_terrain(&m.setup).unwrap();
        let rules = rules_of(&m.setup);
        let snap = |slug: &str| to_snapshot_with(&all[slug], |w| all.get(w));
        let keeper_board: Vec<_> = m.setup.keeper_board.iter().map(|p| (snap(&p.card), field.cell(p.x, p.y).unwrap())).collect();
        let keeper_hand: Vec<_> = m.setup.keeper_hand.iter().map(|s| snap(s)).collect();
        // клетки гостя: вторая половина поля, без местности, на которую не встают
        let width = field.width as usize;
        let depth = field.depth as usize;
        let blocked = |x: usize, y: usize| terrain.iter().any(|t| t.cell.x as usize == x && t.cell.y as usize == y && matches!(t.ground, Ground::Wall | Ground::Ravine));
        let mut cells: Vec<(usize, usize)> = Vec::new();
        for y in depth..2 * depth { for x in 0..width { if !blocked(x, y) { cells.push((x, y)); } } }
        let favoured = |names: &Vec<String>| -> Vec<&BattleCardDto> { pool.iter().copied().filter(|c| names.contains(&c.race_name_en.clone().unwrap_or_default())).collect() };
        let (counter, wrong) = (favoured(&m.counter), favoured(&m.wrong));
        // MEET_RACES=1: вместо «против/наоборот» — доля побед команды каждой расы
        let by_race = std::env::var("MEET_RACES").is_ok();
        let race_names = ["People", "Gnomes", "Witches", "Mages", "Undead", "Water Folk"];
        let race_pools: Vec<Vec<&BattleCardDto>> = race_names.iter().map(|n| favoured(&vec![n.to_string()])).collect();
        let kinds = if by_race { 7 } else { 3 };
        let mut rates = Vec::new();
        for kind in 0..kinds {
            let mut r = Roll(77 + kind as u64);
            let mut won = 0usize;
            for _ in 0..games {
                let names = if by_race {
                    if kind == 0 { team(&pool, &[], 0, &mut r) } else { team(&pool, &race_pools[kind - 1], 4, &mut r) }
                } else {
                    match kind {
                        0 => team(&pool, &[], 0, &mut r),
                        1 => team(&pool, &counter, 4, &mut r),
                        _ => team(&pool, &wrong, 4, &mut r),
                    }
                };
                let mut free = cells.clone();
                let mut board = Vec::new();
                for name in names.iter().take(3) {
                    if free.is_empty() { break; }
                    let (x, y) = free.swap_remove(r.next(free.len()));
                    board.push((snap(name), field.cell(x as u8, y as u8).unwrap()));
                }
                let hand: Vec<_> = names.iter().skip(3).map(|s| snap(s)).collect();
                let setup = Setup { player_board: board, player_hand: hand, keeper_board: keeper_board.clone(), keeper_hand: keeper_hand.clone(), terrain: terrain.clone(), field };
                let mut st = MatchState::begin_with(setup, rules);
                let mut guard = 0;
                while st.outcome.is_none() && guard < 4000 {
                    let d = if st.active == Side::Player { 2 } else { m.bot_depth };
                    let a = bot::choose_at(&st, d);
                    st = match reduce(&st, &a) { Ok((n, _)) => n, Err(_) => match reduce(&st, &Action::EndTurn) { Ok((n, _)) => n, Err(_) => break } };
                    guard += 1;
                }
                if st.outcome == Some(Outcome::Player) { won += 1; }
            }
            rates.push(100.0 * won as f64 / games as f64);
        }
        if by_race {
            println!("{:26} случ {:>3.0}% | люди {:>3.0}% гномы {:>3.0}% ведьмы {:>3.0}% маги {:>3.0}% нежить {:>3.0}% водяные {:>3.0}%", m.slug, rates[0], rates[1], rates[2], rates[3], rates[4], rates[5], rates[6]);
        } else {
            println!("{:26} {:>9.0}% {:>9.0}% {:>9.0}%", m.slug, rates[0], rates[1], rates[2]);
        }
    }
}
