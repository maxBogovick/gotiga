//! Величина поля: 3–4 клетки поперёк и 3–5 вглубь у каждой половины.
//!
//! Батарея проверяет, что шов, края, выставление и прорыв считаются от поля
//! этюда, а не от прежних 3 × 6, — и что поле, не названное вовсе, осталось тем,
//! на котором игралось всё до этой ручки.

use battle_core::*;

fn cell(x: u8, y: u8) -> Cell {
    Cell::new(x, y).unwrap()
}

fn body(name: &str, health: i32, power: i32) -> CardSnapshot {
    CardSnapshot::new(name, 1, health, power)
}

fn big() -> Field {
    Field { width: 4, depth: 5 }
}

/// Поле 4 × 5: хранитель на рядах 0–4, гость на 5–9.
fn on(field: Field) -> MatchState {
    let setup = Setup {
        player_board: vec![(body("Гость", 8, 1), cell(0, field.rows() - 1))],
        player_hand: vec![body("Запас", 8, 1)],
        keeper_board: vec![(body("Страж", 8, 1), cell(0, 0))],
        keeper_hand: vec![],
        terrain: vec![],
        field,
    };
    MatchState::begin_with(setup, Rules { idle_toll: 0, ..Rules::default() })
}

#[test]
fn the_seam_moves_with_the_depth() {
    let f = big();
    assert_eq!(f.rows(), 10);
    assert_eq!(f.side_of(cell(3, 4)), Side::Keeper);
    assert_eq!(f.side_of(cell(3, 5)), Side::Player);
    assert_eq!(f.goal_row(Side::Keeper), 9);
    assert_eq!(f.goal_row(Side::Player), 0);
    assert_eq!(f.mirror(cell(1, 2)), cell(1, 7));
    assert_eq!(f.cells().count(), 40);
}

/// Выставить можно на любую свободную клетку своей половины — и на четвёртый
/// ряд поперёк, которого на поле 3 × 3 нет.
#[test]
fn a_card_is_played_anywhere_on_its_own_half() {
    let st = on(big());
    let cells = st.free_cells(Side::Player);
    assert_eq!(cells.len(), 4 * 5 - 1);
    assert!(cells.contains(&cell(3, 5)));
    assert!(cells.iter().all(|c| c.y >= 5));
    let (st, _) = reduce(&st, &Action::Play { hand_index: 0, cell: cell(3, 5) }).unwrap();
    assert!(st.board.at(cell(3, 5)).is_some());
}

/// На поле 3 × 3 той же клетки нет: выставить туда нельзя.
#[test]
fn a_cell_off_this_field_is_refused() {
    let st = on(Field::default());
    assert_eq!(
        reduce(&st, &Action::Play { hand_index: 0, cell: cell(3, 5) }),
        Err(Illegal::CellTaken)
    );
    assert_eq!(
        reduce(&st, &Action::Play { hand_index: 0, cell: cell(0, 2) }),
        Err(Illegal::NotYourHalf),
        "шов на 3 × 3 — между вторым и третьим рядом"
    );
}

/// Шаг ходит по полю этюда: до четвёртого столбца — на широком, не дальше
/// третьего — на обычном.
#[test]
fn walking_stays_on_the_field() {
    let wide = on(big());
    assert!(wide.walkable(cell(0, 9), 9).iter().any(|c| c.x == 3));
    let plain = on(Field::default());
    assert!(plain.walkable(cell(0, 5), 9).iter().all(|c| c.x < 3 && c.y < 6));
}

/// Прорыв на глубоком поле — дальний ряд ЭТОГО поля.
#[test]
fn breakthrough_counts_the_far_row_of_this_field() {
    let setup = Setup {
        player_board: vec![(body("Гонец", 8, 1).with_step(3), cell(0, 2))],
        player_hand: vec![],
        keeper_board: vec![(body("Страж", 8, 1), cell(3, 3))],
        keeper_hand: vec![],
        terrain: vec![],
        field: big(),
    };
    let rules = Rules { breakthrough: true, idle_toll: 0, ..Rules::default() };
    let st = MatchState::begin_with(setup, rules);
    let (st, _) = reduce(&st, &Action::Move { unit: 0, to: cell(0, 0) }).unwrap();
    let (st, _) = reduce(&st, &Action::EndTurn).unwrap();
    let (st, _) = reduce(&st, &Action::EndTurn).unwrap();
    assert_eq!(st.outcome, Some(Outcome::Player));
}

/// Бот доигрывает партию на самом большом поле.
#[test]
fn the_hand_plays_a_big_field_to_the_end() {
    let mut st = on(big());
    let mut guard = 0;
    while st.outcome.is_none() && guard < 2000 {
        st = reduce(&st, &bot::choose_at(&st, 2)).unwrap().0;
        guard += 1;
    }
    assert!(st.outcome.is_some());
}

/// Величина, которой нет, зажимается, а не роняет партию.
#[test]
fn an_impossible_size_is_clamped() {
    assert_eq!(Field { width: 9, depth: 1 }.normalized(), Field { width: 4, depth: 3 });
}

/// Не названное поле — 3 × 3 и не пишется вовсе: замороженные партии те же.
#[test]
fn a_plain_field_is_not_written() {
    let plain = Setup { player_hand: vec![body("Запас", 8, 1)], ..Setup::default() };
    let written = serde_json::to_value(&plain).unwrap();
    assert!(written.get("field").is_none());
    let read: Setup = serde_json::from_value(written).unwrap();
    assert_eq!(read.field, Field::default());

    let sized = Setup { field: big(), ..Setup::default() };
    let back: Setup = serde_json::from_value(serde_json::to_value(&sized).unwrap()).unwrap();
    assert_eq!(back.field, big());
}
