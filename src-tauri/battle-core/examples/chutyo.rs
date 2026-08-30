//! Чутьё против досмотра: можно ли выбирать ход, не зная ни одного глагола.
//!
//! Жадная рука (`bot::choose`) — лестница из пяти ступеней, и каждая названа
//! глаголом: добить, залечить, выставить, ранить, подойти. Пока глаголов три,
//! это работает. С двадцатью двумя лестница либо вырастет до двадцати двух
//! ступеней, каждая из которых — вкусовщина, не подтверждённая замером, либо
//! промолчит про новые глаголы вовсе: проклятие, щит и вызов не попадут ни на
//! одну ступень, и хранитель ими просто не сыграет.
//!
//! Здесь меряется третья возможность: рука, которая **не знает глаголов вовсе**.
//! Каждое законное действие применяется, партия досматривается до конца обеими
//! жадными руками, и берётся то действие, после которого партия кончилась лучше.
//! Такая рука играет любым глаголом, который умеет движок, и ни строчки в ней
//! не надо править, когда глаголов станет больше.
//!
//! Вопрос замера ровно один: не слабее ли она нынешней на тех глаголах, что уже
//! есть. Если не слабее — лестницу можно снять до того, как её придётся
//! достраивать.
//!
//!     cargo run --release --example chutyo

use battle_core::*;
use std::time::Instant;

struct Roll(u32);

impl Roll {
    fn next(&mut self, upto: u32) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 16) % upto
    }
}

fn mirrored(seed: u32, bodies: u32, hand: u32) -> Setup {
    let mut r = Roll(seed);
    let mut setup = Setup::default();
    let mut guard = 0;
    while (setup.keeper_board.len() as u32) < bodies && guard < 200 {
        guard += 1;
        let mut card = CardSnapshot::new("тело", 1, 3 + r.next(6) as i32, 1 + r.next(4) as i32)
            .with_reach(1 + r.next(3) as u8)
            .with_step(1);
        if r.next(4) == 0 {
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
    for _ in 0..hand {
        let held = CardSnapshot::new("рука", 1 + r.next(4) as i32, 3 + r.next(5) as i32, 1 + r.next(4) as i32);
        setup.keeper_hand.push(held.clone());
        setup.player_hand.push(held);
    }
    setup
}

/// Грубая мерка «кто впереди» — та же, что внутри перебора: исход весит на
/// порядки больше материала.
fn evaluate(st: &MatchState, side: Side) -> i32 {
    let material = |s: Side| -> i32 {
        st.standing(s)
            .iter()
            .map(|id| {
                let u = &st.units[*id as usize];
                u.health.current * 2 + u.printed_power()
            })
            .sum()
    };
    let decided = match st.outcome {
        None | Some(Outcome::Draw) => 0,
        Some(Outcome::Player) => if side == Side::Player { 1_000_000 } else { -1_000_000 },
        Some(Outcome::Keeper) => if side == Side::Keeper { 1_000_000 } else { -1_000_000 },
    };
    decided + 100 * (material(side) - material(side.other()))
}

/// Рука на одну ступень вперёд, тоже без глаголов: применить каждое законное
/// действие и взять то, после которого позиция лучше **сразу**. Ни досмотра, ни
/// лестницы. Годится в подстилку для досмотра — там нужна быстрая и одинаковая
/// рука, а не мудрая.
fn choose_static(state: &MatchState) -> Action {
    let side = state.active;
    let mut best: Option<(Action, i32)> = None;
    for action in legal_actions(state) {
        let Ok((next, _)) = reduce(state, &action) else { continue };
        // Конец хода оценивать нечестно: он не портит позицию, но и не делает
        // ничего, а по мерке «материал» выглядит не хуже удара по броне.
        let score = evaluate(&next, side) - if matches!(action, Action::EndTurn) { 1 } else { 0 };
        if best.as_ref().is_none_or(|(_, seen)| score > *seen) {
            best = Some((action, score));
        }
    }
    best.map(|(a, _)| a).unwrap_or(Action::EndTurn)
}

/// Досмотреть партию до конца одной и той же рукой с обеих сторон.
fn playout_by(state: &MatchState, side: Side, rollout: fn(&MatchState) -> Action) -> i32 {
    let mut st = state.clone();
    let mut guard = 0;
    while st.outcome.is_none() && guard < 512 {
        let Ok((next, _)) = reduce(&st, &rollout(&st)) else { break };
        st = next;
        guard += 1;
    }
    evaluate(&st, side)
}

/// Рука без глаголов: применить каждое законное действие и досмотреть.
fn choose_blind_with(state: &MatchState, rollout: fn(&MatchState) -> Action) -> Action {
    let side = state.active;
    let mut best: Option<(Action, i32)> = None;
    for action in legal_actions(state) {
        let Ok((next, _)) = reduce(state, &action) else { continue };
        let score = playout_by(&next, side, rollout);
        if best.as_ref().is_none_or(|(_, seen)| score > *seen) {
            best = Some((action, score));
        }
    }
    best.map(|(a, _)| a).unwrap_or(Action::EndTurn)
}

#[derive(Clone, Copy, PartialEq)]
enum Hand {
    /// Нынешняя жадная: лестница из пяти ступеней, каждая названа глаголом.
    Ladder,
    /// Одна ступень вперёд по мерке позиции. Глаголов не знает.
    Static,
    /// Досмотр до конца, подстилка — лестница.
    Blind,
    /// Досмотр до конца, подстилка — тоже без глаголов.
    BlindPure,
    /// Нынешний перебор на две ступени.
    Deep,
}

fn act(hand: Hand, st: &MatchState) -> Action {
    match hand {
        Hand::Ladder => bot::choose(st),
        Hand::Static => choose_static(st),
        Hand::Blind => choose_blind_with(st, bot::choose),
        Hand::BlindPure => choose_blind_with(st, choose_static),
        Hand::Deep => bot::choose_at(st, 2),
    }
}

/// Партия двух рук. Возвращает исход и время, потраченное каждой стороной.
fn duel(setup: Setup, player: Hand, keeper: Hand) -> (Option<Outcome>, f64, f64, u32) {
    let mut st = MatchState::begin(setup);
    let (mut t_player, mut t_keeper) = (0f64, 0f64);
    let mut moves = 0;
    let mut guard = 0;
    while st.outcome.is_none() && guard < 512 {
        guard += 1;
        let side = st.active;
        let hand = if side == Side::Player { player } else { keeper };
        let t0 = Instant::now();
        let action = act(hand, &st);
        let took = t0.elapsed().as_secs_f64() * 1000.0;
        if side == Side::Player { t_player += took } else { t_keeper += took }
        moves += 1;
        let Ok((next, _)) = reduce(&st, &action) else { break };
        st = next;
    }
    (st.outcome, t_player, t_keeper, moves)
}

/// `a` против `b` в обе расстановки, чтобы право первого хода вычлось само.
fn tournament(name: &str, a: Hand, b: Hand, runs: u32, bodies: u32) {
    let (mut wins_a, mut wins_b, mut draws) = (0u32, 0u32, 0u32);
    let (mut ms_a, mut ms_b) = (0f64, 0f64);
    let mut moves_a = 0u32;
    let mut moves_b = 0u32;

    for seed in 0..runs {
        // a ходит первым
        let (out, ta, tb, m) = duel(mirrored(seed, bodies, 3), a, b);
        match out {
            Some(Outcome::Player) => wins_a += 1,
            Some(Outcome::Keeper) => wins_b += 1,
            _ => draws += 1,
        }
        ms_a += ta;
        ms_b += tb;
        moves_a += m / 2;
        moves_b += m / 2;

        // b ходит первым
        let (out, ta, tb, m) = duel(mirrored(seed, bodies, 3), b, a);
        match out {
            Some(Outcome::Player) => wins_b += 1,
            Some(Outcome::Keeper) => wins_a += 1,
            _ => draws += 1,
        }
        ms_a += tb;
        ms_b += ta;
        moves_a += m / 2;
        moves_b += m / 2;
    }

    let played = 2 * runs;
    println!(
        "{name}: {:.1} % против {:.1} %, ничьих {} из {}   |   ход: {:.2} мс и {:.2} мс",
        100.0 * wins_a as f64 / played as f64,
        100.0 * wins_b as f64 / played as f64,
        draws,
        played,
        ms_a / moves_a.max(1) as f64,
        ms_b / moves_b.max(1) as f64,
    );
}

fn main() {
    println!("Рука без глаголов против лестницы из пяти ступеней.");
    println!("В обе расстановки: право первого хода вычитается само.\n");

    for bodies in [2u32, 3, 4] {
        println!("── {bodies} тела на сторону ──");
        tournament("  досмотр(лестница) / лестница ", Hand::Blind, Hand::Ladder, 40, bodies);
        tournament("  досмотр(без глаг) / лестница ", Hand::BlindPure, Hand::Ladder, 40, bodies);
        tournament("  досмотр(без глаг) / досмотр  ", Hand::BlindPure, Hand::Blind, 40, bodies);
        tournament("  досмотр(лестница) / перебор  ", Hand::Blind, Hand::Deep, 20, bodies);
        tournament("  досмотр(без глаг) / перебор  ", Hand::BlindPure, Hand::Deep, 20, bodies);
        tournament("  одна ступень      / лестница ", Hand::Static, Hand::Ladder, 40, bodies);
        println!();
    }
}
