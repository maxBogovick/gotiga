//! Куда противник достаёт ударом за свой ход (`MatchState::strike_zones`).
//!
//! Это то, что гость видит при расстановке перед встречей, поэтому зоны
//! обязаны совпадать с тем, что тело действительно сможет сделать в бою.

use battle_core::*;

fn cell(x: u8, y: u8) -> Cell {
    Cell::new(x, y).unwrap()
}

fn tile(x: u8, y: u8, ground: Ground) -> Tile {
    Tile { cell: cell(x, y), ground }
}

fn keeper(card: CardSnapshot, at: Cell, terrain: Vec<Tile>, rules: Rules) -> MatchState {
    let setup = Setup {
        player_board: vec![],
        player_hand: vec![],
        keeper_board: vec![(card, at)],
        keeper_hand: vec![],
        terrain,
        field: Default::default(),
    };
    MatchState::begin_with(setup, rules)
}

fn zone(state: &MatchState) -> Vec<Cell> {
    let mut all: Vec<Cell> = state
        .strike_zones(Side::Keeper)
        .into_iter()
        .flat_map(|(_, cells)| cells)
        .collect();
    all.sort();
    all
}

fn rows(cells: &[Cell]) -> Vec<u8> {
    let mut r: Vec<u8> = cells.iter().map(|c| c.y).collect();
    r.dedup();
    r
}

#[test]
fn a_swordsman_reaches_one_row_past_the_step() {
    let st = keeper(
        CardSnapshot::new("Мечник", 1, 8, 3).with_step(1),
        cell(1, 2),
        vec![],
        Rules::default(),
    );
    let z = zone(&st);
    // Шаг к шву, удар через него: ряды 3 и 4, но не задний.
    assert_eq!(rows(&z), vec![3, 4]);
    assert!(z.iter().all(|c| c.y >= 3), "своя половина в зону не входит");
}

#[test]
fn when_walking_spends_the_turn_it_strikes_only_from_where_it_stands() {
    let st = keeper(
        CardSnapshot::new("Мечник", 1, 8, 3).with_step(1),
        cell(1, 2),
        vec![],
        Rules { walk_spends_turn: true, ..Rules::default() },
    );
    assert_eq!(rows(&zone(&st)), vec![3]);
}

#[test]
fn walls_stop_the_step_and_are_not_marked() {
    let st = keeper(
        CardSnapshot::new("Мечник", 1, 8, 3).with_step(1),
        cell(1, 1),
        vec![
            tile(0, 2, Ground::Wall),
            tile(1, 2, Ground::Wall),
            tile(2, 2, Ground::Wall),
            tile(1, 3, Ground::Wall),
        ],
        Rules::default(),
    );
    // За стеной не пройти, а с места до шва не дотянуться.
    assert!(zone(&st).is_empty());
}

#[test]
fn an_archer_with_the_long_shot_covers_the_whole_half() {
    let st = keeper(
        CardSnapshot::new("Лучник", 1, 6, 2).with_reach(2).with_step(0),
        cell(1, 0),
        vec![],
        Rules::default(),
    );
    assert_eq!(zone(&st).len(), 9);

    let close = keeper(
        CardSnapshot::new("Лучник", 1, 6, 2).with_reach(2).with_step(0),
        cell(1, 0),
        vec![],
        Rules { long_shot_power: 0, ..Rules::default() },
    );
    assert!(zone(&close).is_empty(), "без дальнего выстрела дальность 2 до шва не достаёт");
}

#[test]
fn a_hill_under_the_new_place_counts() {
    // Стрелок с дальностью 2 без дальнего выстрела: с (1,1) достаёт ряд 3,
    // а встав на холм в (1,2) — и ряд 5 (2 + 1 за холм).
    let st = keeper(
        CardSnapshot::new("Лучник", 1, 6, 2).with_reach(2).with_step(1),
        cell(1, 1),
        vec![tile(1, 2, Ground::Hill)],
        Rules { long_shot_power: 0, ..Rules::default() },
    );
    assert!(zone(&st).contains(&cell(1, 5)));
}
