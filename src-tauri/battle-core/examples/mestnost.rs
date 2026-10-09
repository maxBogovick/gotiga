//! Местность и рука бота: видит ли бот землю — и что это меняет в партии.
//!
//! Расстановки те же, что у `pereproverka`, плюс случайная местность,
//! зеркальная по двум половинам: стороны получают одно и то же поле, и разница
//! в исходе — разница рук, а не земли.
//!
//!     cargo run --release --example mestnost

use battle_core::*;

struct Roll(u32);
impl Roll {
    fn next(&mut self, upto: u32) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 16) % upto
    }
}

fn mirrored(seed: u32, hand: u32, ground: bool) -> Setup {
    let mut r = Roll(seed);
    let mut setup = Setup::default();
    for _ in 0..(1 + r.next(3)) {
        let card = CardSnapshot::new("тело", 1, 3 + r.next(6) as i32, 1 + r.next(4) as i32)
            .with_reach(1 + r.next(3) as u8)
            .with_step(1 + r.next(2) as u8);
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
        return mirrored(seed + 7_919, hand, ground);
    }
    if ground {
        // Две-три клетки на половину, зеркально. Стена не встаёт под тело —
        // так её не поставит и стол.
        for _ in 0..(2 + r.next(2)) {
            let (x, y) = (r.next(3) as u8, r.next(3) as u8);
            let g = match r.next(3) {
                0 => Ground::Wall,
                1 => Ground::Cover,
                _ => Ground::Mire,
            };
            for cell in [Cell::new(x, y).unwrap(), Cell::new(x, 5 - y).unwrap()] {
                let stood = setup.player_board.iter().chain(&setup.keeper_board).any(|(_, c)| *c == cell);
                let taken = setup.terrain.iter().any(|t| t.cell == cell);
                if !taken && !(stood && g == Ground::Wall) {
                    setup.terrain.push(Tile { cell, ground: g });
                }
            }
        }
    }
    setup
}

#[derive(Clone, Copy)]
enum Hand {
    Blind,
    Sees,
    Search,
}

fn pick(st: &MatchState, hand: Hand) -> Action {
    match hand {
        Hand::Blind => bot::choose_blind(st),
        Hand::Sees => bot::choose(st),
        Hand::Search => bot::choose_at(st, 2),
    }
}

fn play(setup: Setup, player: Hand, keeper: Hand) -> MatchState {
    play_counting(setup, player, keeper).0
}

/// Партия и число ударов в ней. Удар — урон от тела по телу; плата за простой
/// и опасные клетки ударами не считаются: застрявшие тела гибнут и от них, и
/// такая партия кончается, хотя боя в ней не было.
fn play_counting(setup: Setup, player: Hand, keeper: Hand) -> (MatchState, u32) {
    let mut st = MatchState::begin_with(setup, Rules::default());
    let mut guard = 0;
    let mut blows = 0;
    while st.outcome.is_none() && guard < 5000 {
        let hand = if st.active == Side::Player { player } else { keeper };
        let (next, events) = reduce(&st, &pick(&st, hand)).unwrap();
        blows += events
            .iter()
            .filter(|e| matches!(e, Event::Damaged { by: Some(_), .. }))
            .count() as u32;
        st = next;
        guard += 1;
    }
    (st, blows)
}

/// Доля партий, решённых лимитом кругов, и средняя длина.
fn shape(runs: u32, ground: bool, hand: Hand) -> (f64, f64) {
    let (mut limit, mut rounds) = (0u32, 0u32);
    for seed in 0..runs {
        let st = play(mirrored(seed, 3, ground), hand, hand);
        if st.round > MAX_ROUNDS_HOUSE {
            limit += 1;
        }
        rounds += st.round as u32;
    }
    (100.0 * limit as f64 / runs as f64, rounds as f64 / runs as f64)
}

const MAX_ROUNDS_HOUSE: u8 = battle_core::state::MAX_ROUNDS;

/// Доля побед видящей руки против слепой, обе расстановки.
fn sees_against_blind(runs: u32) -> (f64, f64) {
    let (mut won, mut draws, mut total) = (0u32, 0u32, 0u32);
    for seed in 0..runs {
        for sees_first in [true, false] {
            let (p, k) = if sees_first { (Hand::Sees, Hand::Blind) } else { (Hand::Blind, Hand::Sees) };
            let st = play(mirrored(seed, 3, true), p, k);
            total += 1;
            match (st.outcome.unwrap(), sees_first) {
                (Outcome::Player, true) | (Outcome::Keeper, false) => won += 1,
                (Outcome::Draw, _) => draws += 1,
                _ => {}
            }
        }
    }
    let n = total as f64;
    (100.0 * won as f64 / n, 100.0 * draws as f64 / n)
}

/// Стена поперёк поля в два ряда и проход у ПРАВОГО края: путь к врагу есть,
/// но по прямой он упирается в камень. Ровно тот этюд, ради которого
/// местность и заводилась.
///
/// Именно у правого: при равных шагах слепая рука берёт первую клетку в
/// порядке обхода, то есть левую, и проход слева она находила случайно —
/// первый вариант замера так и не отличил одну руку от другой.
fn corridor(seed: u32) -> Setup {
    let mut r = Roll(seed + 101);
    let mut setup = Setup::default();
    let mut used: Vec<u8> = Vec::new();
    for _ in 0..(1 + r.next(2)) {
        let x = r.next(3) as u8;
        if used.contains(&x) {
            continue;
        }
        used.push(x);
        let card = CardSnapshot::new("тело", 1, 4 + r.next(5) as i32, 1 + r.next(4) as i32)
            .with_step(1 + r.next(2) as u8);
        setup.keeper_board.push((card.clone(), Cell::new(x, 0).unwrap()));
        setup.player_board.push((card, Cell::new(x, 5).unwrap()));
    }
    for (x, y) in [(0u8, 2u8), (1, 2), (0, 3), (1, 3)] {
        setup.terrain.push(Tile { cell: Cell::new(x, y).unwrap(), ground: Ground::Wall });
    }
    setup
}

fn corridor_shape(runs: u32, player: Hand, keeper: Hand) -> (f64, f64, f64) {
    let (mut quiet, mut rounds, mut player_won) = (0u32, 0u32, 0u32);
    for seed in 0..runs {
        let (st, blows) = play_counting(corridor(seed), player, keeper);
        if blows == 0 {
            quiet += 1;
        }
        if st.outcome == Some(Outcome::Player) {
            player_won += 1;
        }
        rounds += st.round as u32;
    }
    let n = runs as f64;
    (100.0 * quiet as f64 / n, rounds as f64 / n, 100.0 * player_won as f64 / n)
}

fn main() {
    println!("══ 0. Стена поперёк поля, проход у края, 200 партий ══\n");
    println!("{:<26} {:>14} {:>8} {:>16}", "игрок / хранитель", "ни удара", "кругов", "побед игрока");
    for (name, p, k) in [
        ("слепая / слепая", Hand::Blind, Hand::Blind),
        ("видящая / видящая", Hand::Sees, Hand::Sees),
        ("видящая / слепая", Hand::Sees, Hand::Blind),
        ("слепая / видящая", Hand::Blind, Hand::Sees),
    ] {
        let (quiet, rounds, won) = corridor_shape(200, p, k);
        println!("{:<26} {:>13.1}% {:>8.1} {:>15.1}%", name, quiet, rounds, won);
    }
    println!();

    println!("══ 1. Видящая рука против слепой на случайной местности, 2 × 400 партий ══\n");
    let (won, draws) = sees_against_blind(400);
    println!("видящая выигрывает {won:.1} %, ничьих {draws:.1} %");

    println!("\n══ 2. Форма партии: лимитом решено и кругов, 400 партий ══\n");
    println!("{:<26} {:>10} {:>8}", "", "лимитом", "кругов");
    for (name, ground, hand) in [
        ("ровное поле, жадная", false, Hand::Sees),
        ("местность, слепая", true, Hand::Blind),
        ("местность, видящая", true, Hand::Sees),
    ] {
        let (limit, rounds) = shape(400, ground, hand);
        println!("{:<26} {:>9.1}% {:>8.1}", name, limit, rounds);
    }

    println!("\n══ 3. Рука с перебором, 150 партий ══\n");
    for (name, ground) in [("ровное поле", false), ("местность", true)] {
        let (limit, rounds) = shape(150, ground, Hand::Search);
        println!("{:<26} {:>9.1}% {:>8.1}", name, limit, rounds);
    }
}
