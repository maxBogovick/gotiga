//! Развязка: чем вернуть партии конец, когда обе стороны играют умело.
//!
//! §17.1: против руки с перебором половина партий доигрывалась до лимита кругов
//! и решалась остатком здоровья. Здесь меряется ручка `escalation_from` —
//! с какого круга удары растут по единице за круг.
//!
//!     cargo run --release --example razvyazka

use battle_core::*;

struct Roll(u32);
impl Roll {
    fn next(&mut self, upto: u32) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 16) % upto
    }
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

/// (доля побед 1-го хода, кругов, ничьих, решено лимитом)
fn shape(runs: u32, rules: Rules, depth: u8) -> (f64, f64, f64, f64) {
    let (mut first, mut draws, mut by_limit, mut rounds) = (0u32, 0u32, 0u32, 0u32);
    for seed in 0..runs {
        let st = play(mirrored(seed, 3), rules, depth);
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
    (
        100.0 * first as f64 / n,
        rounds as f64 / n,
        100.0 * draws as f64 / n,
        100.0 * by_limit as f64 / n,
    )
}

/// Не обесценил ли рост ударов сам выбор: думающий против случайного.
fn thinker_versus_dice(runs: u32, rules: Rules, depth: u8) -> f64 {
    let (mut wins, mut total) = (0u32, 0u32);
    for seed in 0..runs {
        for thinker_is_player in [true, false] {
            let mut r = Roll(seed * 2_654_435_761 + 11);
            let mut st = MatchState::begin_with(mirrored(seed, 3), rules);
            let mut guard = 0;
            while st.outcome.is_none() && guard < 2000 {
                let mine = st.active == Side::Player;
                let action = if mine == thinker_is_player {
                    bot::choose_at(&st, depth)
                } else {
                    let acts = legal_actions(&st);
                    acts[r.next(acts.len() as u32) as usize].clone()
                };
                let (next, _) = reduce(&st, &action).unwrap();
                st = next;
                guard += 1;
            }
            total += 1;
            let side = if thinker_is_player { Side::Player } else { Side::Keeper };
            match st.outcome.unwrap() {
                Outcome::Player if side == Side::Player => wins += 1,
                Outcome::Keeper if side == Side::Keeper => wins += 1,
                _ => {}
            }
        }
    }
    100.0 * wins as f64 / total.max(1) as f64
}

fn main() {
    const RUNS: u32 = 120;

    println!("══ С какого круга удары растут ══\n");
    println!("Обе стороны играют рукой с перебором. Ноль — как сейчас.\n");
    println!("{:>10} {:>10} {:>9} {:>9} {:>11}", "с круга", "1-й ход", "кругов", "ничьих", "лимитом");
    for from in [0u8, 8, 6, 5, 4, 3] {
        let rules = Rules { escalation_from: from, ..Default::default() };
        let (first, rounds, draws, limit) = shape(RUNS, rules, 2);
        let mark = if limit <= 10.0 && draws <= 5.0 { "  ◀ партия кончается боем" } else { "" };
        println!(
            "{:>10} {:>9.1}% {:>9.1} {:>8.1}% {:>10.1}%{}",
            if from == 0 { "не растут".into() } else { from.to_string() },
            first, rounds, draws, limit, mark
        );
    }

    println!("\n══ Плата за бездействие ══\n");
    println!("Тело, простоявшее свой ход без дела, теряет здоровье. Рукой с перебором.\n");
    println!("{:>10} {:>10} {:>10} {:>9} {:>9} {:>11}", "плата", "рост с", "1-й ход", "кругов", "ничьих", "лимитом");
    for (toll, from) in [(1i32, 0u8), (2, 0), (3, 0), (1, 6), (2, 6)] {
        let rules = Rules { idle_toll: toll, escalation_from: from, ..Default::default() };
        let (first, rounds, draws, limit) = shape(RUNS, rules, 2);
        let mark = if limit <= 10.0 && draws <= 5.0 { "  ◀ партия кончается боем" } else { "" };
        println!(
            "{:>10} {:>10} {:>9.1}% {:>9.1} {:>8.1}% {:>10.1}%{}",
            toll,
            if from == 0 { "нет".into() } else { from.to_string() },
            first, rounds, draws, limit, mark
        );
    }

    println!("\n══ То же жадной рукой: не сломалось ли то, что работало ══\n");
    println!("{:>10} {:>10} {:>9} {:>9} {:>11}", "с круга", "1-й ход", "кругов", "ничьих", "лимитом");
    println!("(плата за бездействие в скобках)");
    for (from, toll) in [(0u8, 0i32), (6, 0), (0, 1), (0, 2)] {
        let rules = Rules { escalation_from: from, idle_toll: toll, ..Default::default() };
        let (first, rounds, draws, limit) = shape(RUNS * 2, rules, 1);
        println!(
            "{:>10} {:>9.1}% {:>9.1} {:>8.1}% {:>10.1}%",
            format!("{}({})", if from == 0 { "нет".to_string() } else { from.to_string() }, toll),
            first, rounds, draws, limit
        );
    }

    println!("\n══ Кандидат на большей выборке ══\n");
    println!("{:>26} {:>10} {:>9} {:>9} {:>11}", "правило", "1-й ход", "кругов", "ничьих", "лимитом");
    for (name, rules, runs, depth) in [
        ("как сейчас, с перебором", Rules::default(), 400u32, 2u8),
        ("плата 1, с перебором", Rules { idle_toll: 1, ..Default::default() }, 400, 2),
        ("как сейчас, жадной", Rules::default(), 800, 1),
        ("плата 1, жадной", Rules { idle_toll: 1, ..Default::default() }, 800, 1),
    ] {
        let (first, rounds, draws, limit) = shape(runs, rules, depth);
        println!("{:>26} {:>9.1}% {:>9.1} {:>8.1}% {:>10.1}%", name, first, rounds, draws, limit);
    }

    println!("\n══ Значит ли что-нибудь выбор ══\n");
    println!("Думающий против случайного. 50 % — решения не значат ничего.\n");
    for (from, toll) in [(0u8, 0i32), (0, 1), (0, 2), (6, 1)] {
        let rules = Rules { escalation_from: from, idle_toll: toll, ..Default::default() };
        println!(
            "рост {:>4}, плата {}: жадной {:>5.1}%   с перебором {:>5.1}%",
            if from == 0 { "нет".into() } else { from.to_string() },
            toll,
            thinker_versus_dice(60, rules, 1),
            thinker_versus_dice(40, rules, 2)
        );
    }
}
