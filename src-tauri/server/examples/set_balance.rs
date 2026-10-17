//! Баланс набора карт замером: много партий бот против бота, команды из шести
//! случайных карт набора, у каждой стороны три на доске и три в руке. Каждая
//! пара команд играется дважды с обменом сторон, чтобы право первого хода
//! вычиталось само.
//!
//! Показывает долю побед стороны, в которой стояла карта. Это не сила карты
//! самой по себе, а её вклад в команду: карта, у которой доля заметно выше
//! пятидесяти, — та, которую возьмут все.
//!
//!     curl -s localhost:3000/api/v1/battles/cards > cards.json
//!     cargo run --release --example set_balance -- cards.json 4000

use battle_core::*;
use gotiga_server::battles::to_snapshot;
use gotiga_server::models::BattleCardDto;
use std::collections::HashMap;

struct Roll(u64);
impl Roll {
    fn next(&mut self, upto: usize) -> usize {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % upto
    }
}

fn team(pool: &[usize], r: &mut Roll) -> Vec<usize> {
    let mut left: Vec<usize> = pool.to_vec();
    let mut out = Vec::new();
    while out.len() < 6 {
        out.push(left.swap_remove(r.next(left.len())));
    }
    out
}

fn setup(a: &[usize], b: &[usize], snaps: &[CardSnapshot], r: &mut Roll) -> Setup {
    let mut s = Setup::default();
    let mut cells: Vec<(u8, u8)> = Vec::new();
    while cells.len() < 3 {
        let c = (r.next(3) as u8, r.next(3) as u8);
        if !cells.contains(&c) {
            cells.push(c);
        }
    }
    for (i, &(x, y)) in cells.iter().enumerate() {
        s.keeper_board.push((snaps[b[i]].clone(), Cell::new(x, y).unwrap()));
        s.player_board.push((snaps[a[i]].clone(), Cell::new(x, 5 - y).unwrap()));
    }
    for i in 3..6 {
        s.keeper_hand.push(snaps[b[i]].clone());
        s.player_hand.push(snaps[a[i]].clone());
    }
    s
}

fn play(s: Setup) -> Option<Outcome> {
    let mut st = MatchState::begin_with(s, Rules::default());
    let mut guard = 0;
    while st.outcome.is_none() && guard < 3000 {
        let action = bot::choose(&st);
        match reduce(&st, &action) {
            Ok((next, _)) => st = next,
            Err(_) => break,
        }
        guard += 1;
    }
    st.outcome
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let raw = std::fs::read_to_string(&args[1]).unwrap();
    let games: usize = args.get(2).and_then(|g| g.parse().ok()).unwrap_or(4000);
    let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let list = value.get("cards").cloned().unwrap_or(value);
    let dtos: Vec<BattleCardDto> = serde_json::from_value(list).unwrap();
    let dtos: Vec<&BattleCardDto> = dtos.iter().filter(|c| c.status == "published" && c.kind != "structure").collect();
    let snaps: Vec<CardSnapshot> = dtos.iter().map(|c| to_snapshot(c)).collect();
    let pool: Vec<usize> = (0..snaps.len()).collect();
    let mut r = Roll(20_261_016);
    let mut seen = vec![0u32; snaps.len()];
    let mut won = vec![0f64; snaps.len()];
    let mut limit = 0u32;
    let mut total = 0u32;
    for _ in 0..games {
        let a = team(&pool, &mut r);
        let b = team(&pool, &mut r);
        let seed = r.0;
        for swap in [false, true] {
            let (p, k) = if swap { (&b, &a) } else { (&a, &b) };
            let mut rr = Roll(seed);
            let outcome = play(setup(p, k, &snaps, &mut rr));
            total += 1;
            for &i in p.iter().chain(k.iter()) { seen[i] += 1; }
            match outcome {
                Some(Outcome::Player) => for &i in p.iter() { won[i] += 1.0; },
                Some(Outcome::Keeper) => for &i in k.iter() { won[i] += 1.0; },
                _ => {
                    limit += 1;
                    for &i in p.iter().chain(k.iter()) { won[i] += 0.5; }
                }
            }
        }
    }
    let mut rows: Vec<(f64, usize)> = (0..snaps.len()).map(|i| (100.0 * won[i] / seen[i].max(1) as f64, i)).collect();
    rows.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
    println!("партий {total}, без победителя {:.1}%", 100.0 * limit as f64 / total as f64);
    let mut by_race: HashMap<String, (f64, u32)> = HashMap::new();
    for (pct, i) in &rows {
        let c = dtos[*i];
        println!("{:5.1}%  {:22} ранг {} цена {} очки {:.1}", pct, c.slug, c.tier, c.cost, c.budget_points.unwrap_or(0.0));
        let e = by_race.entry(c.race_name_ru.clone().unwrap_or_default()).or_insert((0.0, 0));
        e.0 += pct; e.1 += 1;
    }
    println!("-- по расам (среднее)");
    for (race, (sum, n)) in by_race { println!("{:5.1}%  {}", sum / n as f64, race); }
}
