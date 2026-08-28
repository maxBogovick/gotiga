//! Прогонщик в самой ранней форме: много партий, немного чисел.
//!
//! Это заготовка этапа 7. Пока она не правит баланс, а отвечает на три вопроса,
//! на которые нельзя ответить, глядя на одну партию: все ли партии доигрываются,
//! решает ли их бой или лимит кругов, и сколько стоит право первого хода.
//!
//!     cargo run --release --example svodka

use battle_core::*;

struct Roll(u32);

impl Roll {
    fn next(&mut self, upto: u32) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 16) % upto
    }
}

fn play(setup: Setup) -> (MatchState, Vec<Event>) {
    play_by(setup, Rules::default())
}

fn play_by(setup: Setup, rules: Rules) -> (MatchState, Vec<Event>) {
    let mut st = MatchState::begin_with(setup, rules);
    let mut journal = Vec::new();
    while st.outcome.is_none() {
        let action = bot::choose(&st);
        let (next, events) = reduce(&st, &action).unwrap();
        st = next;
        journal.extend(events);
    }
    (st, journal)
}

/// Одинаковые силы по обе стороны: всё, что отличает стороны, — право хода.
/// `vigour` умножает здоровье — рычаг, которым проверяется гипотеза «партии
/// слишком коротки, поэтому первый удар и решает».
fn mirrored_with(seed: u32, vigour: i32) -> Setup {
    mirrored_full(seed, vigour, false)
}

fn mirrored_full(seed: u32, vigour: i32, menders: bool) -> Setup {
    mirrored_hand(seed, vigour, menders, 3)
}

fn mirrored_hand(seed: u32, vigour: i32, menders: bool, hand: u32) -> Setup {
    let mut r = Roll(seed);
    let mut setup = Setup::default();
    for _ in 0..(1 + r.next(3)) {
        let mut card = CardSnapshot::new("тело", 1, (3 + r.next(6) as i32) * vigour, 1 + r.next(4) as i32)
            .with_reach(1 + r.next(3) as u8)
            .with_step(1);
        if menders && r.next(3) == 0 {
            card = card.with_mend(2 + r.next(3) as i32);
        }
        let x = r.next(3) as u8;
        let y = r.next(3) as u8;
        let keeper = Cell::new(x, y).unwrap();
        let player = Cell::new(x, 5 - y).unwrap();
        if setup.keeper_board.iter().any(|(_, c)| *c == keeper) {
            continue;
        }
        setup.keeper_board.push((card.clone(), keeper));
        setup.player_board.push((card, player));
    }
    // Три карты в руке каждой стороне — примерно то, что остаётся от колоды
    // из шести после расстановки. С одной картой монета насыщалась на двойке,
    // и сетка мерила глубину руки, а не правило.
    for _ in 0..hand {
        let held = CardSnapshot::new("рука", 1 + r.next(4) as i32, 3 + r.next(5) as i32, 1 + r.next(4) as i32);
        setup.keeper_hand.push(held.clone());
        setup.player_hand.push(held);
    }

    if setup.keeper_board.is_empty() {
        return mirrored_hand(seed + 7_919, vigour, menders, hand);
    }
    setup
}

/// Прогон одного набора условий. Возвращает долю побед первого хода, среднее
/// число кругов и долю партий, решённых лимитом.
fn sweep_full(runs: u32, vigour: i32, menders: bool) -> (f64, f64, f64) {
    sweep_rules(runs, vigour, menders, Rules::default())
}

fn sweep_rules(runs: u32, vigour: i32, menders: bool, rules: Rules) -> (f64, f64, f64) {
    sweep_hand(runs, vigour, menders, rules, 3)
}

fn sweep_hand(runs: u32, vigour: i32, menders: bool, rules: Rules, hand: u32) -> (f64, f64, f64) {
    let mut first = 0u32;
    let mut by_limit = 0u32;
    let mut rounds = 0u32;
    for seed in 0..runs {
        let (st, _) = play_by(mirrored_hand(seed, vigour, menders, hand), rules);
        if st.outcome == Some(Outcome::Player) {
            first += 1;
        }
        if st.round > battle_core::state::MAX_ROUNDS {
            by_limit += 1;
        }
        rounds += st.round as u32;
    }
    (
        100.0 * first as f64 / runs as f64,
        rounds as f64 / runs as f64,
        100.0 * by_limit as f64 / runs as f64,
    )
}

fn body(reach: u8, step: u8) -> CardSnapshot {
    CardSnapshot::new("тело", 1, 6, 3).with_reach(reach).with_step(step)
}

/// Одна сторона ближнего боя против одной стороны стрелков, в обе расстановки —
/// чтобы преимущество первого хода вычиталось само.
fn melee_versus_ranged_at(melee_step: u8) -> (u32, u32, u32) {
    let mut melee = 0;
    let mut ranged = 0;
    let mut draws = 0;

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
                far.iter().map(|(c, cell)| (c.clone(), Cell::new(cell.x, 5 - cell.y).unwrap())).collect(),
                near.iter().map(|(c, cell)| (c.clone(), Cell::new(cell.x, 5 - cell.y).unwrap())).collect(),
            )
        } else {
            (near.clone(), far.clone())
        };
        let setup = Setup { player_board, keeper_board, ..Default::default() };
        let (st, _) = play(setup);
        let melee_side = if swap { Side::Keeper } else { Side::Player };
        match st.outcome.unwrap() {
            Outcome::Draw => draws += 1,
            Outcome::Player if melee_side == Side::Player => melee += 1,
            Outcome::Keeper if melee_side == Side::Keeper => melee += 1,
            _ => ranged += 1,
        }
    }
    (melee, ranged, draws)
}

fn main() {
    const RUNS: u32 = 2_000;

    let mut first = 0;
    let mut second = 0;
    let mut draws = 0;
    let mut by_limit = 0;
    let mut rounds = 0u32;
    let mut blows = 0u32;

    for seed in 0..RUNS {
        let (st, journal) = play(mirrored_with(seed, 1));
        match st.outcome.unwrap() {
            Outcome::Player => first += 1,
            Outcome::Keeper => second += 1,
            Outcome::Draw => draws += 1,
        }
        if st.round > battle_core::state::MAX_ROUNDS {
            by_limit += 1;
        }
        rounds += st.round as u32;
        blows += journal.iter().filter(|e| matches!(e, Event::Damaged { .. })).count() as u32;
    }

    let pct = |n: u32| 100.0 * n as f64 / RUNS as f64;
    println!("── {RUNS} зеркальных партий ──");
    println!("доиграли до исхода   {:>6}", RUNS);
    println!("победа первого хода  {:>6}  {:>5.1}%", first, pct(first));
    println!("победа второго хода  {:>6}  {:>5.1}%", second, pct(second));
    println!("ничья                {:>6}  {:>5.1}%", draws, pct(draws));
    println!("решено лимитом       {:>6}  {:>5.1}%", by_limit, pct(by_limit));
    println!("кругов в среднем     {:>9.1}", rounds as f64 / RUNS as f64);
    println!("ударов в среднем     {:>9.1}", blows as f64 / RUNS as f64);

    println!("\n── что делает с этим запас здоровья ──");
    println!("{:>10} {:>10} {:>10} {:>10}", "здоровье", "первый ход", "кругов", "лимитом");
    for vigour in [1, 2, 3, 4] {
        let (share, avg, limit) = sweep_full(RUNS, vigour, false);
        println!("{:>9}× {:>9.1}% {:>10.1} {:>9.1}%", vigour, share, avg, limit);
    }

    println!("\n── что делает с этим лекарь (каждое третье тело) ──");
    println!("{:>10} {:>10} {:>10} {:>10}", "лекари", "первый ход", "кругов", "лимитом");
    for menders in [false, true] {
        let (share, avg, limit) = sweep_full(RUNS, 1, menders);
        println!("{:>10} {:>9.1}% {:>10.1} {:>9.1}%", if menders { "есть" } else { "нет" }, share, avg, limit);
    }

    // Устойчивость: правило, выбранное по одной глубине руки, подогнано под
    // генератор, а не под игру. Хорошее правило держится на всех.
    println!("\n── насколько кандидаты держатся при разной руке ──");
    print!("{:>34}", "правило");
    for hand in 1..=4 {
        print!(" {:>9}", format!("рука {hand}"));
    }
    println!("     разброс");
    for (name, rules) in [
        ("как сейчас", Rules::default()),
        ("один удар · монета 2", Rules { second_side_coin: 2, opening_attacks: 1 }),
        ("ни одного удара", Rules { second_side_coin: 0, opening_attacks: 0 }),
    ] {
        print!("{:>34}", name);
        let mut shares = Vec::new();
        for hand in 1..=4 {
            let (share, _, _) = sweep_hand(RUNS, 1, false, rules, hand);
            shares.push(share);
            print!(" {:>8.1}%", share);
        }
        let spread = shares.iter().cloned().fold(f64::MIN, f64::max)
            - shares.iter().cloned().fold(f64::MAX, f64::min);
        println!("   {:>6.1} п.п.", spread);
    }

    println!("\n── ближний бой против стрелков, дальность 2–4, обе расстановки ──");
    println!("{:>12} {:>14} {:>10}", "шаг ближних", "ближний бой", "стрелки");
    for step in [1u8, 2, 3] {
        let (melee, ranged, _) = melee_versus_ranged_at(step);
        println!("{:>12} {:>14} {:>10}", step, melee, ranged);
    }

    println!("\n── чем ответить на перевес первого хода ──");
    println!("{:>34} {:>12} {:>10} {:>10}", "правило", "первый ход", "кругов", "лимитом");
    let candidates: Vec<(String, Rules)> = {
        let mut v = vec![("как сейчас".to_string(), Rules::default())];
        for opening in [1u8, 0] {
            for coin in [0i32, 1, 2, 3] {
                v.push((
                    format!("ударов в первом круге {opening} · монета {coin}"),
                    Rules { second_side_coin: coin, opening_attacks: opening },
                ));
            }
        }
        v
    };
    for (name, rules) in &candidates {
        let (share, avg, limit) = sweep_rules(RUNS, 1, false, *rules);
        let off = (share - 50.0).abs();
        println!(
            "{:>34} {:>11.1}% {:>10.1} {:>9.1}%  {}",
            name, share, avg, limit,
            if off <= 5.0 { "◀ в коридоре" } else { "" }
        );
    }
}
