//! Перепроверка: те же вопросы, обе руки бота рядом.
//!
//! Все числа §14–§16 сняты ПЕРВОЙ рукой — жадной, до того как появилась вторая.
//! Вторая обыгрывает первую 100 из 100, то есть играет иначе; значит выбранные
//! числа надо перепроверить ею, а не считать проверенными.
//!
//! Партий здесь меньше, чем в `svodka`, и намеренно: ход второй рукой стоит
//! 4.3 мс против 0.01. Считать те же две тысячи партий значило бы ждать час
//! ради третьего знака.
//!
//!     cargo run --release --example pereproverka

use battle_core::*;

struct Roll(u32);
impl Roll {
    fn next(&mut self, upto: u32) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 16) % upto
    }
}

/// Партия, где ОБЕ стороны играют одной и той же рукой.
fn play(setup: Setup, rules: Rules, depth: u8) -> MatchState {
    let mut st = MatchState::begin_with(setup, rules);
    let mut guard = 0;
    while st.outcome.is_none() && guard < 2000 {
        let (next, _) = reduce(&st, &bot::choose_at(&st, depth)).unwrap();
        st = next;
        guard += 1;
    }
    st
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
    first: f64,
    rounds: f64,
    draws: f64,
    by_limit: f64,
}

fn shape(runs: u32, hand: u32, depth: u8) -> Shape {
    let (mut first, mut draws, mut by_limit, mut rounds) = (0u32, 0u32, 0u32, 0u32);
    for seed in 0..runs {
        let st = play(mirrored(seed, hand), Rules::default(), depth);
        match st.outcome {
            Some(Outcome::Player) => first += 1,
            Some(Outcome::Draw) => draws += 1,
            _ => {}
        }
        if st.round > battle_core::state::MAX_ROUNDS {
            by_limit += 1;
        }
        rounds += st.round as u32;
    }
    let n = runs as f64;
    Shape {
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
fn duel(a: &CardSnapshot, b: &CardSnapshot, seeds: u32, depth: u8) -> (f64, f64) {
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
            let st = play(Setup { player_board, keeper_board, ..Default::default() }, Rules::default(), depth);
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
fn melee_versus_ranged(depth: u8) -> (u32, u32) {
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
            let st = play(Setup { player_board, keeper_board, ..Default::default() }, Rules::default(), depth);
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

    println!("══ 1. Форма партии и честность первого хода (§14.3) ══\n");
    println!("{:<14} {:>10} {:>9} {:>9} {:>10}", "рука", "1-й ход", "кругов", "ничьих", "лимитом");
    for (depth, name) in hands {
        let s = shape(250, 3, depth);
        println!("{:<14} {:>9.1}% {:>9.1} {:>8.1}% {:>9.1}%", name, s.first, s.rounds, s.draws, s.by_limit);
    }

    println!("\n══ 2. Держится ли правило открытия при разной руке (§13.3, §14.2) ══\n");
    print!("{:<14}", "рука");
    for h in 1..=4 {
        print!(" {:>9}", format!("рука {h}"));
    }
    println!(" {:>10}", "разброс");
    for (depth, name) in hands {
        print!("{:<14}", name);
        let mut all = Vec::new();
        for h in 1..=4 {
            let s = shape(150, h, depth);
            all.push(s.first);
            print!(" {:>8.1}%", s.first);
        }
        let spread = all.iter().cloned().fold(f64::MIN, f64::max) - all.iter().cloned().fold(f64::MAX, f64::min);
        println!(" {:>7.1} п.п.", spread);
    }

    println!("\n══ 3. Кто решает у равных карт — карта или очередь (§14.1) ══\n");
    let a = body(10, 5, 1, 1, 0);
    let b = body(8, 4, 1, 1, 2);
    println!("{:<14} {:>18}", "рука", "решила очередь");
    for (depth, name) in hands {
        let (_, queue) = duel(&a, &b, 60, depth);
        println!("{:<14} {:>17.1}%", name, queue);
    }

    println!("\n══ 4. Стрелки против ближнего боя, 18 партий (§15.5) ══\n");
    println!("{:<14} {:>14} {:>10}", "рука", "ближний бой", "стрелки");
    for (depth, name) in hands {
        let (m, r) = melee_versus_ranged(depth);
        println!("{:<14} {:>14} {:>10}", name, m, r);
    }

    println!("\n══ 5. Один бюджет, потраченный по-разному (§15.1) ══\n");
    let pairs: [(&str, CardSnapshot, CardSnapshot); 4] = [
        ("крепкий / неподвижный", body(14, 5, 1, 1, 0), body(14, 4, 3, 0, 0)),
        ("крепкий / подвижный", body(14, 5, 1, 1, 0), body(14, 4, 3, 1, 0)),
        ("сильный / живучий", body(10, 7, 1, 1, 0), body(16, 4, 3, 1, 0)),
        ("живучий / сильный", body(18, 3, 1, 1, 0), body(10, 6, 3, 1, 0)),
    ];
    print!("{:<14}", "рука");
    for (name, ..) in &pairs {
        print!(" {:>23}", *name);
    }
    println!();
    for (depth, name) in hands {
        print!("{:<14}", name);
        for (_, near, far) in &pairs {
            let (share, _) = duel(near, far, 40, depth);
            print!(" {:>22.0}%", share);
        }
        println!();
    }

    println!("\n══ 6. Цена шага (§16.4) ══\n");
    println!("{:<14} {:>20} {:>20}", "рука", "шаг 2 против шага 1", "шаг 3 против шага 1");
    for (depth, name) in hands {
        let (two, _) = duel(&body(10, 5, 1, 2, 0), &body(10, 5, 1, 1, 0), 40, depth);
        let (three, _) = duel(&body(10, 5, 1, 3, 0), &body(10, 5, 1, 1, 0), 40, depth);
        println!("{:<14} {:>19.0}% {:>19.0}%", name, two, three);
    }
}
