//! Когда партия кончается.
//!
//! Правило одно: НЕТ ТЕЛ НА ПОЛЕ — ПОРАЖЕНИЕ, рука не в счёт. Оно живёт в
//! `is_spent` и записано там словами, но до этой батареи его не проверяло
//! ничего, а первый же вопрос, который ему задают в комнате, — «у меня на поле
//! не осталось карт, а победы противнику не дали»; отвечать на такое чтением
//! исходника нельзя.
//!
//! Прежнее, мягкое правило считало ещё и руку: карта, до которой мана когда-
//! нибудь дорастёт, держала партию живой. Оно читалось как поломка — доска
//! пуста, партия очевидно проиграна, а комната молчит, — и заменено нарочно.

use battle_core::*;

fn body(cost: i32, health: i32, power: i32) -> CardSnapshot {
    CardSnapshot::new("тело", cost, health, power)
}

fn keeper_cell() -> Cell {
    Cell::new(1, 1).unwrap()
}

/// Голая доска — сторона исчерпана, и это видно сразу.
#[test]
fn a_bare_board_is_lost_at_once() {
    let setup = Setup {
        player_board: vec![],
        player_hand: vec![],
        keeper_board: vec![(body(1, 6, 2), keeper_cell())],
        keeper_hand: vec![],
    };
    let st = MatchState::begin(setup);
    let (st, events) = reduce(&st, &Action::EndTurn).expect("конец хода законен");
    assert_eq!(st.outcome, Some(Outcome::Keeper));
    assert!(events.iter().any(|e| matches!(e, Event::Finished { outcome: Outcome::Keeper })));
}

/// Голая доска и карта в руке, которая сейчас не по карману, — поражение.
/// Ровно тот случай, из-за которого правило и меняли: раньше здесь было `None`.
#[test]
fn a_card_too_dear_this_turn_does_not_save_a_bare_board() {
    let setup = Setup {
        player_board: vec![],
        player_hand: vec![body(4, 6, 2)],
        keeper_board: vec![(body(1, 6, 2), keeper_cell())],
        keeper_hand: vec![],
    };
    let st = MatchState::begin(setup);
    assert!(st.player.mana < 4, "иначе испытание проверяет не то");
    let (st, _) = reduce(&st, &Action::EndTurn).expect("конец хода законен");
    assert_eq!(st.outcome, Some(Outcome::Keeper));
}

/// И дешёвая не спасает: дело не в цене, а в том, что на поле пусто.
#[test]
fn even_an_affordable_card_does_not_save_a_bare_board() {
    let setup = Setup {
        player_board: vec![],
        player_hand: vec![body(1, 6, 2)],
        keeper_board: vec![(body(1, 6, 2), keeper_cell())],
        keeper_hand: vec![],
    };
    let st = MatchState::begin(setup);
    let (st, _) = reduce(&st, &Action::EndTurn).expect("конец хода законен");
    assert_eq!(st.outcome, Some(Outcome::Keeper), "рука не в счёт");
}

/// Но карту всё ещё можно ВЫЛОЖИТЬ и тем спастись: правило наказывает за то,
/// что тела не поставили, а не за то, что оно было в руке.
#[test]
fn playing_the_card_saves_the_match() {
    let setup = Setup {
        player_board: vec![],
        player_hand: vec![body(1, 6, 2)],
        keeper_board: vec![(body(1, 6, 2), keeper_cell())],
        keeper_hand: vec![],
    };
    let st = MatchState::begin(setup);
    let cell = st.board.free_cells(Side::Player).next().expect("своя клетка свободна");
    let (st, _) = reduce(&st, &Action::Play { hand_index: 0, cell }).expect("выкладка законна");
    assert_eq!(st.outcome, None);
    let (st, _) = reduce(&st, &Action::EndTurn).expect("конец хода законен");
    assert_eq!(st.outcome, None, "тело стоит — партия идёт");
}

/// Снял последнее тело противника — выиграл, даже если у него полна рука.
#[test]
fn taking_the_last_body_wins_however_full_the_other_hand() {
    let setup = Setup {
        player_board: vec![(body(1, 9, 9), Cell::new(1, 3).unwrap())],
        player_hand: vec![],
        keeper_board: vec![(body(1, 2, 0), Cell::new(1, 2).unwrap())],
        keeper_hand: vec![body(1, 9, 9), body(1, 9, 9)],
    };
    let st = MatchState::begin(setup);
    let (st, _) = reduce(&st, &Action::Attack { attacker: 0, target: 1 }).expect("удар законен");
    assert_eq!(st.outcome, Some(Outcome::Player));
}

/// Забор от вечной партии: как бы стороны ни пасовали, счётчик кругов
/// заканчивает её сам.
#[test]
fn passing_back_and_forth_still_ends() {
    let setup = Setup {
        player_board: vec![(body(1, 6, 0), Cell::new(1, 4).unwrap())],
        player_hand: vec![],
        keeper_board: vec![(body(1, 6, 0), keeper_cell())],
        keeper_hand: vec![],
    };
    let mut st = MatchState::begin(setup);
    let mut guard = 0;
    while st.outcome.is_none() {
        st = reduce(&st, &Action::EndTurn).expect("конец хода законен").0;
        guard += 1;
        assert!(guard < 512, "партия не кончилась за 512 концов хода");
    }
}
