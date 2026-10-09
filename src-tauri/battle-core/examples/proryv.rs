//! Прорыв: что вторая цель делает с партией.
//!
//! Те же вопросы, что у `pereproverka`, и те же расстановки — меняется одна
//! ручка, `Rules::breakthrough`. Калитка (`BATTLE-MECHANICS-RESEARCH.md`, Д1):
//! у руки с перебором счётчиком решено меньше 5 %, ничьих меньше 2 %, первый
//! ход в коридоре 45–55 %.
//!
//!     cargo run --release --example proryv

use battle_core::*;

struct Roll(u32);
impl Roll {
    fn next(&mut self, upto: u32) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 16) % upto
    }
}

/// Партия, где ОБЕ стороны играют одной и той же рукой. Второе число —
/// решил ли её прорыв.
fn play_full(setup: Setup, rules: Rules, depth: u8) -> (MatchState, bool) {
    let mut st = MatchState::begin_with(setup, rules);
    let mut guard = 0;
    let mut breached = false;
    while st.outcome.is_none() && guard < 5000 {
        let (next, events) = reduce(&st, &bot::choose_at(&st, depth)).unwrap();
        breached |= events.iter().any(|e| matches!(e, Event::Breached { .. }));
        st = next;
        guard += 1;
    }
    (st, breached)
}

fn play(setup: Setup, rules: Rules, depth: u8) -> MatchState {
    play_full(setup, rules, depth).0
}

fn mirrored(seed: u32, hand: u32) -> Setup {
    let mut r = Roll(seed);
    let mut setup = Setup::default();
    for _ in 0..(1 + r.next(3)) {
        let card = CardSnapshot::new("тело", 1, 3 + r.next(6) as i32, 1 + r.next(4) as i32)
            .with_reach(1 + r.next(3) as u8)
            .with_step(1);
        let (x, y) = (r.next(3) as u8, r.next(3) as u8);
        let keeper = Cell::new(x, y).unwrap();
        if setup.keeper_board.iter().any(|(_, c)| *c == keeper) {
            continue;
        }
        setup.keeper_board.push((card.clone(), keeper));
        setup.player_board.push((card, Cell::new(x, 5 - y).unwrap()));
    }
    for _ in 0..hand {
        let held =
            CardSnapshot::new("рука", 1 + r.next(4) as i32, 3 + r.next(5) as i32, 1 + r.next(4) as i32);
        setup.keeper_hand.push(held.clone());
        setup.player_hand.push(held);
    }
    if setup.keeper_board.is_empty() {
        return mirrored(seed + 7_919, hand);
    }
    setup
}

struct Shape {
    breached: f64,
    first: f64,
    rounds: f64,
    draws: f64,
    by_limit: f64,
}

fn shape(runs: u32, hand: u32, depth: u8, rules: Rules) -> Shape {
    let (mut first, mut draws, mut by_limit, mut rounds, mut breached) = (0u32, 0u32, 0u32, 0u32, 0u32);
    for seed in 0..runs {
        let (st, b) = play_full(mirrored(seed, hand), rules, depth);
        if b {
            breached += 1;
        }
        match st.outcome {
            Some(Outcome::Player) => first += 1,
            Some(Outcome::Draw) => draws += 1,
            _ => {}
        }
        if st.round > rules.max_rounds && !b {
            by_limit += 1;
        }
        rounds += st.round as u32;
    }
    let n = runs as f64;
    Shape {
        breached: 100.0 * breached as f64 / n,
        first: 100.0 * first as f64 / n,
        rounds: rounds as f64 / n,
        draws: 100.0 * draws as f64 / n,
        by_limit: 100.0 * by_limit as f64 / n,
    }
}

fn body(h: i32, p: i32, reach: u8, step: u8, armor: i32) -> CardSnapshot {
    CardSnapshot::new("тело", 2, h, p).with_reach(reach).with_step(step).with_armor(armor)
}

/// Доля побед A и доля партий, решённых очередью. 50 % во второй колонке —
/// «решает карта», 100 % — «решает право хода».
fn duel(a: &CardSnapshot, b: &CardSnapshot, seeds: u32, depth: u8, rules: Rules) -> (f64, f64) {
    let (mut wins, mut first, mut total) = (0u32, 0u32, 0u32);
    for seed in 0..seeds {
        for swap in [false, true] {
            let mut r = Roll(seed * 31 + 7);
            let bodies = 2 + (seed % 2) as usize;
            let mut cells: Vec<(u8, u8)> = Vec::new();
            while cells.len() < bodies {
                let c = (r.next(3) as u8, r.next(3) as u8);
                if !cells.contains(&c) {
                    cells.push(c);
                }
            }
            let lay = |card: &CardSnapshot, mine: bool| -> Vec<(CardSnapshot, Cell)> {
                cells
                    .iter()
                    .map(|(x, y)| {
                        (card.clone(), Cell::new(*x, if mine { 5 - y } else { *y }).unwrap())
                    })
                    .collect()
            };
            let (player_board, keeper_board) = if swap {
                (lay(b, true), lay(a, false))
            } else {
                (lay(a, true), lay(b, false))
            };
            let st = play(Setup { player_board, keeper_board, ..Default::default() }, rules, depth);
            let a_side = if swap { Side::Keeper } else { Side::Player };
            total += 1;
            if st.outcome == Some(Outcome::Player) {
                first += 1;
            }
            match st.outcome.unwrap() {
                Outcome::Player if a_side == Side::Player => wins += 1,
                Outcome::Keeper if a_side == Side::Keeper => wins += 1,
                _ => {}
            }
        }
    }
    let n = total.max(1) as f64;
    (100.0 * wins as f64 / n, 100.0 * first as f64 / n)
}

/// §12.2 в точности: ближний бой против неподвижных стрелков, обе расстановки.
fn melee_versus_ranged(depth: u8, rules: Rules) -> (u32, u32) {
    let (mut melee, mut ranged) = (0, 0);
    for melee_step in [1u8, 2, 3] {
        for (swap, reach) in [(false, 2u8), (true, 2), (false, 3), (true, 3), (false, 4), (true, 4)] {
            let near = vec![
                (body(6, 3, 1, melee_step, 0), Cell::new(0, 5).unwrap()),
                (body(6, 3, 1, melee_step, 0), Cell::new(1, 5).unwrap()),
            ];
            let far = vec![
                (body(6, 3, reach, 0, 0), Cell::new(0, 0).unwrap()),
                (body(6, 3, reach, 0, 0), Cell::new(1, 0).unwrap()),
            ];
            let (player_board, keeper_board) = if swap {
                (
                    far.iter().map(|(c, l)| (c.clone(), Cell::new(l.x, 5 - l.y).unwrap())).collect::<Vec<_>>(),
                    near.iter().map(|(c, l)| (c.clone(), Cell::new(l.x, 5 - l.y).unwrap())).collect::<Vec<_>>(),
                )
            } else {
                (near, far)
            };
            let st = play(Setup { player_board, keeper_board, ..Default::default() }, rules, depth);
            let melee_side = if swap { Side::Keeper } else { Side::Player };
            match st.outcome.unwrap() {
                Outcome::Draw => {}
                Outcome::Player if melee_side == Side::Player => melee += 1,
                Outcome::Keeper if melee_side == Side::Keeper => melee += 1,
                _ => ranged += 1,
            }
        }
    }
    (melee, ranged)
}

fn main() {
    let hands = [(1u8, "жадная"), (2u8, "с перебором")];
    let modes = [("без прорыва", false), ("прорыв", true)];
    let rules_of = |on: bool| Rules { breakthrough: on, ..Rules::default() };

    println!("══ 1. Форма партии, рука по три карты, 250 партий ══\n");
    println!(
        "{:<14} {:<12} {:>9} {:>8} {:>8} {:>9} {:>9}",
        "рука", "правило", "1-й ход", "кругов", "ничьих", "лимитом", "прорывом"
    );
    for (depth, name) in hands {
        for (mode, on) in modes {
            let s = shape(250, 3, depth, rules_of(on));
            println!(
                "{:<14} {:<12} {:>8.1}% {:>8.1} {:>7.1}% {:>8.1}% {:>8.1}%",
                name, mode, s.first, s.rounds, s.draws, s.by_limit, s.breached
            );
        }
    }

    println!("\n══ 2. Первый ход по глубине руки, 150 партий на клетку ══\n");
    for (depth, name) in hands {
        for (mode, on) in modes {
            print!("{:<14} {:<12}", name, mode);
            let mut all = Vec::new();
            for h in 1..=4 {
                let s = shape(150, h, depth, rules_of(on));
                all.push(s.first);
                print!(" {:>7.1}%", s.first);
            }
            let spread = all.iter().cloned().fold(f64::MIN, f64::max) - all.iter().cloned().fold(f64::MAX, f64::min);
            println!("   разброс {:>4.1} п.п.", spread);
        }
    }

    println!("\n══ 3. Кончается ли партия сама: лимит 60 кругов, рука с перебором, 120 партий ══\n");
    for (mode, on) in modes {
        let rules = Rules { max_rounds: 60, ..rules_of(on) };
        let s = shape(120, 3, 2, rules);
        println!("{:<12} не кончилось за 60 кругов {:>5.1}%   кругов {:>5.1}", mode, s.by_limit, s.rounds);
    }

    println!("\n══ 4. Равные карты: доля партий, решённых очередью, 120 партий ══\n");
    let a = body(10, 5, 1, 1, 0);
    let b = body(8, 4, 1, 1, 2);
    for (depth, name) in hands {
        for (mode, on) in modes {
            let (_, queue) = duel(&a, &b, 60, depth, rules_of(on));
            println!("{:<14} {:<12} {:>6.1}%", name, mode, queue);
        }
    }

    println!("\n══ 5. Ближний бой против стрелков, 18 партий ══\n");
    for (depth, name) in hands {
        for (mode, on) in modes {
            let (m, r) = melee_versus_ranged(depth, rules_of(on));
            println!("{:<14} {:<12} ближний {:>2}  стрелки {:>2}", name, mode, m, r);
        }
    }

    println!("\n══ 6. Один бюджет, потраченный по-разному: доля побед ближнего боя, 80 партий ══\n");
    let pairs: [(&str, CardSnapshot, CardSnapshot); 4] = [
        ("крепк/неподв", body(14, 5, 1, 1, 0), body(14, 4, 3, 0, 0)),
        ("крепк/подвиж", body(14, 5, 1, 1, 0), body(14, 4, 3, 1, 0)),
        ("сильн/живуч", body(10, 7, 1, 1, 0), body(16, 4, 3, 1, 0)),
        ("живуч/сильн", body(18, 3, 1, 1, 0), body(10, 6, 3, 1, 0)),
    ];
    print!("{:<27}", "");
    for (n, ..) in &pairs {
        print!(" {:>13}", *n);
    }
    println!();
    for (depth, name) in hands {
        for (mode, on) in modes {
            print!("{:<14} {:<12}", name, mode);
            for (_, near, far) in &pairs {
                let (share, _) = duel(near, far, 40, depth, rules_of(on));
                print!(" {:>12.0}%", share);
            }
            println!();
        }
    }

    println!("\n══ 7. Цена шага: доля побед более подвижного, 80 партий ══\n");
    for (depth, name) in hands {
        for (mode, on) in modes {
            let (two, _) = duel(&body(10, 5, 1, 2, 0), &body(10, 5, 1, 1, 0), 40, depth, rules_of(on));
            let (three, _) = duel(&body(10, 5, 1, 3, 0), &body(10, 5, 1, 1, 0), 40, depth, rules_of(on));
            println!("{:<14} {:<12} шаг 2 {:>4.0}%   шаг 3 {:>4.0}%", name, mode, two, three);
        }
    }
}
