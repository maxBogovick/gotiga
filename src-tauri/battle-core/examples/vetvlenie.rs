//! Кончится ли партия сама, если счётчик отодвинуть.
//!
//! Вопрос решает, что чинить: сам счётчик — или то, ради чего он поставлен.
//! Если при сорока кругах партии доигрываются боем, значит лимит просто тесен.
//! Если и там стоят — счётчик подпирает игру, у которой нет развязки, и убирать
//! его нельзя, пока развязка не появится.
//!
//!     cargo run --release --example vetvlenie

use battle_core::*;

struct Roll(u32);
impl Roll {
    fn next(&mut self, upto: u32) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 16) % upto
    }
}

fn mirrored(seed: u32) -> Setup {
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
    for _ in 0..3 {
        let held =
            CardSnapshot::new("рука", 1 + r.next(4) as i32, 3 + r.next(5) as i32, 1 + r.next(4) as i32);
        setup.keeper_hand.push(held.clone());
        setup.player_hand.push(held);
    }
    if setup.keeper_board.is_empty() {
        return mirrored(seed + 7_919);
    }
    setup
}

/// (доля решённых счётчиком, средняя длина, доля партий, где за круг никто
/// никого не тронул).
fn run(runs: u32, rules: Rules, depth: u8) -> (f64, f64, f64) {
    let (mut by_limit, mut rounds, mut quiet_total, mut quiet_rounds) = (0u32, 0u32, 0u32, 0u32);
    for seed in 0..runs {
        let mut st = MatchState::begin_with(mirrored(seed), rules);
        let mut guard = 0;
        let mut hits_this_round = 0u32;
        let mut round_now = st.round;
        while st.outcome.is_none() && guard < 20_000 {
            let (next, events) = reduce(&st, &bot::choose_at(&st, depth)).unwrap();
            hits_this_round += events.iter().filter(|e| matches!(e, Event::Damaged { .. })).count() as u32;
            if next.round != round_now {
                quiet_total += 1;
                if hits_this_round == 0 {
                    quiet_rounds += 1;
                }
                hits_this_round = 0;
                round_now = next.round;
            }
            st = next;
            guard += 1;
        }
        if st.round > rules.max_rounds {
            by_limit += 1;
        }
        rounds += st.round as u32;
    }
    (
        100.0 * by_limit as f64 / runs as f64,
        rounds as f64 / runs as f64,
        100.0 * quiet_rounds as f64 / quiet_total.max(1) as f64,
    )
}

/// §12.2: ближний бой против неподвижных стрелков, обе расстановки.
fn melee_versus_ranged(rules: Rules) -> (u32, u32) {
    let body = |reach: u8, step: u8| CardSnapshot::new("тело", 1, 6, 3).with_reach(reach).with_step(step);
    let (mut melee, mut ranged) = (0, 0);
    for melee_step in [1u8, 2, 3] {
        for (swap, reach) in [(false, 2u8), (true, 2), (false, 3), (true, 3), (false, 4), (true, 4)] {
            let near = vec![
                (body(1, melee_step), Cell::new(0, 5).unwrap()),
                (body(1, melee_step), Cell::new(1, 5).unwrap()),
            ];
            let far = vec![
                (body(reach, 0), Cell::new(0, 0).unwrap()),
                (body(reach, 0), Cell::new(1, 0).unwrap()),
            ];
            let (player_board, keeper_board) = if swap {
                (
                    far.iter().map(|(c, l)| (c.clone(), Cell::new(l.x, 5 - l.y).unwrap())).collect::<Vec<_>>(),
                    near.iter().map(|(c, l)| (c.clone(), Cell::new(l.x, 5 - l.y).unwrap())).collect::<Vec<_>>(),
                )
            } else {
                (near, far)
            };
            let mut st = MatchState::begin_with(
                Setup { player_board, keeper_board, ..Default::default() }, rules);
            let mut guard = 0;
            while st.outcome.is_none() && guard < 2000 {
                let (next, _) = reduce(&st, &bot::choose_at(&st, 2)).unwrap();
                st = next;
                guard += 1;
            }
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

/// (доля побед первого хода, доля ничьих)
fn fairness(runs: u32, rules: Rules, depth: u8) -> (f64, f64) {
    let (mut first, mut draws) = (0u32, 0u32);
    for seed in 0..runs {
        let mut st = MatchState::begin_with(mirrored(seed), rules);
        let mut guard = 0;
        while st.outcome.is_none() && guard < 20_000 {
            let (next, _) = reduce(&st, &bot::choose_at(&st, depth)).unwrap();
            st = next;
            guard += 1;
        }
        match st.outcome {
            Some(Outcome::Player) => first += 1,
            Some(Outcome::Draw) => draws += 1,
            _ => {}
        }
    }
    let n = runs as f64;
    (100.0 * first as f64 / n, 100.0 * draws as f64 / n)
}

fn main() {
    const RUNS: u32 = 120;

    println!("══ Кончится ли партия сама, если счётчик отодвинуть ══\n");
    println!("Обе стороны — рука с перебором, плата за простой включена.\n");
    println!("{:>10} {:>16} {:>10} {:>22}", "лимит", "решено счётчиком", "кругов", "кругов без единого удара");
    for limit in [12u8, 20, 30, 60, 120] {
        let rules = Rules { max_rounds: limit, ..Default::default() };
        let (by_limit, rounds, quiet) = run(RUNS, rules, 2);
        println!("{:>10} {:>15.1}% {:>10.1} {:>21.1}%", limit, by_limit, rounds, quiet);
    }

    println!("\n══ Убрать безопасные клетки: стрелок бьёт дальше своей дальности ══\n");
    println!("Лимит 120 — чтобы увидеть тот класс партий, что не кончается НИКОГДА.\n");
    println!("{:>14} {:>16} {:>10} {:>22}", "сила за далью", "не кончилось", "кругов", "кругов без удара");
    for power in [0u8, 25, 50, 75, 100] {
        let rules = Rules { long_shot_power: power, max_rounds: 120, ..Default::default() };
        let (by_limit, rounds, quiet) = run(RUNS, rules, 2);
        println!(
            "{:>14} {:>15.1}% {:>10.1} {:>21.1}%",
            if power == 0 { "не достаёт".into() } else { format!("{power}%") },
            by_limit, rounds, quiet
        );
    }

    println!("\nПри нынешнем лимите в 12 кругов:\n");
    println!("{:>14} {:>16} {:>10} {:>22}", "сила за далью", "решено счётчиком", "кругов", "кругов без удара");
    for power in [0u8, 25, 50, 75] {
        let rules = Rules { long_shot_power: power, ..Default::default() };
        let (by_limit, rounds, quiet) = run(RUNS, rules, 2);
        println!(
            "{:>14} {:>15.1}% {:>10.1} {:>21.1}%",
            if power == 0 { "не достаёт".into() } else { format!("{power}%") },
            by_limit, rounds, quiet
        );
    }

    println!("\n══ Не сломало ли это ближний бой (§12.2, 18 партий) ══\n");
    println!("{:>14} {:>14} {:>10}", "сила за далью", "ближний бой", "стрелки");
    for power in [0u8, 25, 50, 75] {
        let rules = Rules { long_shot_power: power, ..Default::default() };
        let (melee, ranged) = melee_versus_ranged(rules);
        println!(
            "{:>14} {:>14} {:>10}",
            if power == 0 { "не достаёт".into() } else { format!("{power}%") },
            melee, ranged
        );
    }

    println!("\n══ Кандидат на большей выборке ══\n");
    println!("{:>22} {:>16} {:>10} {:>10} {:>9}", "правило", "не кончилось", "кругов", "1-й ход", "ничьих");
    for (name, power, limit) in [
        ("как сейчас", 0u8, 120u8),
        ("за далью 25 %", 25, 120),
        ("как сейчас, лимит 12", 0, 12),
        ("за далью 25 %, лимит 12", 25, 12),
    ] {
        let rules = Rules { long_shot_power: power, max_rounds: limit, ..Default::default() };
        let (by_limit, rounds, _) = run(300, rules, 2);
        let (first, draws) = fairness(300, rules, 2);
        println!("{:>22} {:>15.1}% {:>10.1} {:>9.1}% {:>8.1}%", name, by_limit, rounds, first, draws);
    }

    println!("\n══ То же без платы за простой ══\n");
    println!("{:>10} {:>16} {:>10} {:>22}", "лимит", "решено счётчиком", "кругов", "кругов без единого удара");
    for limit in [12u8, 30, 120] {
        let rules = Rules { max_rounds: limit, idle_toll: 0, ..Default::default() };
        let (by_limit, rounds, quiet) = run(RUNS, rules, 2);
        println!("{:>10} {:>15.1}% {:>10.1} {:>21.1}%", limit, by_limit, rounds, quiet);
    }
}
