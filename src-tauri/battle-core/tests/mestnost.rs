//! Местность: стена, укрытие, топь.
//!
//! Клетки с правилом задаёт этюд. Батарея проверяет смысл каждого слова и то,
//! что поле без местности играется ровно как прежде.

use battle_core::*;

fn cell(x: u8, y: u8) -> Cell {
    Cell::new(x, y).unwrap()
}

fn body(name: &str, health: i32, power: i32) -> CardSnapshot {
    CardSnapshot::new(name, 1, health, power)
}

fn tile(x: u8, y: u8, ground: Ground) -> Tile {
    Tile { cell: cell(x, y), ground }
}

fn rules() -> Rules {
    Rules { idle_toll: 0, ..Rules::default() }
}

fn act(state: &MatchState, action: Action) -> (MatchState, Vec<Event>) {
    reduce(state, &action).expect("действие должно быть законным")
}

fn moves_of(state: &MatchState, unit: UnitId) -> Vec<Cell> {
    legal_actions(state)
        .into_iter()
        .filter_map(|a| match a {
            Action::Move { unit: u, to } if u == unit => Some(to),
            _ => None,
        })
        .collect()
}

/// Бегун посреди своего переднего ряда, хранитель далеко.
fn runner_at(x: u8, y: u8, step: u8, terrain: Vec<Tile>) -> MatchState {
    let setup = Setup {
        player_board: vec![(body("Бегун", 8, 1).with_step(step), cell(x, y))],
        player_hand: vec![body("Запас", 8, 1)],
        keeper_board: vec![(body("Страж", 8, 1), cell(2, 0))],
        keeper_hand: vec![],
        terrain,
        field: Default::default(),
    };
    MatchState::begin_with(setup, rules())
}

// ── Стена ───────────────────────────────────────────────────────────────────

/// На стену не встают: ни шагом, ни из руки.
#[test]
fn a_wall_is_neither_walked_onto_nor_played_onto() {
    let st = runner_at(1, 4, 1, vec![tile(1, 3, Ground::Wall), tile(0, 5, Ground::Wall)]);
    assert!(!moves_of(&st, 0).contains(&cell(1, 3)));
    assert_eq!(
        reduce(&st, &Action::Move { unit: 0, to: cell(1, 3) }),
        Err(Illegal::NoWayThere)
    );
    assert!(!legal_actions(&st)
        .iter()
        .any(|a| matches!(a, Action::Play { cell: c, .. } if *c == cell(0, 5))));
    assert_eq!(
        reduce(&st, &Action::Play { hand_index: 0, cell: cell(0, 5) }),
        Err(Illegal::CellTaken),
        "стена для выставления — занятая клетка"
    );
}

/// Через стену не проходят: стена во весь ряд закрывает путь вперёд.
#[test]
fn a_row_of_walls_closes_the_way() {
    let walls = vec![tile(0, 3, Ground::Wall), tile(1, 3, Ground::Wall), tile(2, 3, Ground::Wall)];
    let st = runner_at(1, 4, 3, walls);
    assert!(moves_of(&st, 0).iter().all(|c| c.y >= 4), "{:?}", moves_of(&st, 0));
}

/// Стена под стоящим телом не ложится: расстановку ломать нечем.
#[test]
fn a_wall_under_a_standing_body_is_dropped() {
    let st = runner_at(1, 4, 1, vec![tile(1, 4, Ground::Wall)]);
    assert_eq!(st.ground(cell(1, 4)), None);
}

// ── Топь ────────────────────────────────────────────────────────────────────

/// В топь войти можно, пройти её насквозь — нет.
#[test]
fn a_mire_is_entered_but_not_crossed() {
    let lane = vec![tile(0, 3, Ground::Wall), tile(1, 3, Ground::Mire), tile(2, 3, Ground::Wall)];
    let st = runner_at(1, 4, 3, lane);
    let moves = moves_of(&st, 0);
    assert!(moves.contains(&cell(1, 3)), "в топь войти можно");
    assert!(moves.iter().all(|c| c.y >= 3), "дальше топи в этот ход не уйти: {moves:?}");
}

/// Стоящий в топи выходит на одну клетку, какой бы ни был шаг.
#[test]
fn from_a_mire_only_one_step() {
    let st = runner_at(1, 4, 3, vec![tile(1, 4, Ground::Mire)]);
    assert!(moves_of(&st, 0).iter().all(|c| c.distance(cell(1, 4)) == 1));
}

// ── Укрытие ─────────────────────────────────────────────────────────────────

fn archer_and_target(cover: bool) -> MatchState {
    let setup = Setup {
        player_board: vec![(body("Стрелок", 8, 4).with_reach(3), cell(1, 3))],
        player_hand: vec![],
        // Цель далеко: стрелка никто не достаёт вплотную, и штраф в упор не
        // примешивается к укрытию.
        keeper_board: vec![(body("Цель", 10, 1), cell(1, 0))],
        keeper_hand: vec![],
        terrain: if cover { vec![tile(1, 0, Ground::Cover)] } else { vec![] },
        field: Default::default(),
    };
    MatchState::begin_with(setup, rules())
}

/// Издали по укрытому — вполсилы, и разбор называет укрытие.
#[test]
fn a_shot_into_cover_lands_at_half() {
    let st = archer_and_target(true);
    let (st, events) = act(&st, Action::Attack { attacker: 0, target: 1 });
    assert_eq!(st.units[1].health.current, 8, "4 вполсилы — 2");
    let trail = events.iter().find_map(|e| match e {
        Event::Damaged { trail, .. } => Some(trail.clone()),
        _ => None,
    });
    assert!(trail.unwrap().iter().any(|b| b.step == StepId::Cover));
}

/// Без укрытия тот же выстрел — в полную силу.
#[test]
fn the_same_shot_without_cover_lands_in_full() {
    let st = archer_and_target(false);
    let (st, _) = act(&st, Action::Attack { attacker: 0, target: 1 });
    assert_eq!(st.units[1].health.current, 6);
}

/// Вплотную укрытие не спасает: оно от тех, кто бьёт издали.
#[test]
fn cover_does_not_help_against_a_blow_from_the_next_cell() {
    let setup = Setup {
        player_board: vec![(body("Боец", 8, 4), cell(1, 3))],
        player_hand: vec![],
        keeper_board: vec![(body("Цель", 10, 1), cell(1, 2))],
        keeper_hand: vec![],
        terrain: vec![tile(1, 2, Ground::Cover)],
        field: Default::default(),
    };
    let st = MatchState::begin_with(setup, rules());
    let (st, _) = act(&st, Action::Attack { attacker: 0, target: 1 });
    assert_eq!(st.units[1].health.current, 6, "вплотную — в полную силу");
}

// ── Старые записи ───────────────────────────────────────────────────────────

/// Расстановка без местности пишется так же, как писалась, и читается
/// обратно: замороженные партии не меняются ни на байт.
#[test]
fn a_plain_setup_is_written_as_before() {
    let plain = Setup { player_hand: vec![body("Запас", 8, 1)], ..Setup::default() };
    let written = serde_json::to_value(&plain).unwrap();
    assert!(written.get("terrain").is_none());
    let read: Setup = serde_json::from_value(written).unwrap();
    assert!(read.terrain.is_empty());
}

// ── Бот видит землю ─────────────────────────────────────────────────────────

/// Тупик за стеной: по прямой ближе всего клетка, из которой дальше хода нет.
fn dead_end() -> MatchState {
    let setup = Setup {
        player_board: vec![(body("Путник", 8, 2), cell(0, 5))],
        player_hand: vec![],
        keeper_board: vec![(body("Страж", 8, 1), cell(0, 0))],
        keeper_hand: vec![],
        terrain: vec![tile(0, 3, Ground::Wall), tile(1, 3, Ground::Wall)],
        field: Default::default(),
    };
    MatchState::begin_with(setup, rules())
}

/// Слепая рука идёт в тупик, видящая — в обход.
#[test]
fn the_hand_walks_around_a_wall_instead_of_into_a_dead_end() {
    let st = dead_end();
    assert_eq!(bot::choose_blind(&st), Action::Move { unit: 0, to: cell(0, 4) }, "иначе испытание проверяет не то");
    assert_eq!(bot::choose(&st), Action::Move { unit: 0, to: cell(1, 4) });
}

/// Из тупика слепая рука не выходит вовсе, видящая — выходит.
#[test]
fn the_hand_does_not_stand_in_a_dead_end() {
    let st = dead_end();
    let (st, _) = act(&st, Action::Move { unit: 0, to: cell(0, 4) });
    let (st, _) = act(&st, Action::EndTurn);
    let (st, _) = act(&st, Action::EndTurn);
    assert_eq!(bot::choose_blind(&st), Action::EndTurn, "слепая стоит");
    assert_eq!(bot::choose(&st), Action::Move { unit: 0, to: cell(1, 4) });
}

/// Топь обходится, когда рядом есть сухой шаг не хуже.
#[test]
fn the_hand_steps_around_a_mire() {
    let setup = Setup {
        player_board: vec![(body("Путник", 8, 2), cell(0, 5))],
        player_hand: vec![],
        keeper_board: vec![(body("Страж", 8, 1), cell(0, 0))],
        keeper_hand: vec![],
        terrain: vec![tile(0, 4, Ground::Mire)],
        field: Default::default(),
    };
    let st = MatchState::begin_with(setup, rules());
    assert_eq!(bot::choose_blind(&st), Action::Move { unit: 0, to: cell(0, 4) }, "иначе испытание проверяет не то");
    assert_eq!(bot::choose(&st), Action::Move { unit: 0, to: cell(1, 4) });
}

/// Из равных шагов — тот, что кончается в укрытии.
#[test]
fn among_equal_steps_the_hand_takes_cover() {
    let setup = Setup {
        player_board: vec![(body("Путник", 8, 2), cell(1, 5))],
        player_hand: vec![],
        keeper_board: vec![(body("Страж", 8, 1), cell(1, 0))],
        keeper_hand: vec![],
        terrain: vec![tile(2, 4, Ground::Cover)],
        field: Default::default(),
    };
    let st = MatchState::begin_with(setup, rules());
    assert_eq!(bot::choose(&st), Action::Move { unit: 0, to: cell(2, 4) });
}

// ── Овраг ───────────────────────────────────────────────────────────────────

/// Овраг во весь ряд: шаг 1 его не пересекает, шаг 2 перепрыгивает.
#[test]
fn a_ravine_is_crossed_only_by_a_leap() {
    let ravine = vec![tile(0, 3, Ground::Ravine), tile(1, 3, Ground::Ravine), tile(2, 3, Ground::Ravine)];
    let slow = runner_at(1, 4, 1, ravine.clone());
    assert!(moves_of(&slow, 0).iter().all(|c| c.y >= 4), "{:?}", moves_of(&slow, 0));

    let quick = runner_at(1, 4, 2, ravine);
    let moves = moves_of(&quick, 0);
    assert!(moves.contains(&cell(1, 2)), "перепрыгнул: {moves:?}");
    assert!(moves.iter().all(|c| c.y != 3), "на овраге не стоят: {moves:?}");
}

/// На овраг не выставляют.
#[test]
fn nobody_is_played_onto_a_ravine() {
    let st = runner_at(1, 4, 1, vec![tile(0, 5, Ground::Ravine)]);
    assert_eq!(
        reduce(&st, &Action::Play { hand_index: 0, cell: cell(0, 5) }),
        Err(Illegal::CellTaken)
    );
}

fn shove() -> AbilitySnapshot {
    AbilitySnapshot { verb: "move".into(), ..AbilitySnapshot::harm("толчок", 1, 3) }
}

/// Толчок в овраг — гибель при любом здоровье, и сцена узнаёт, куда упало.
#[test]
fn a_body_shoved_into_a_ravine_falls() {
    let setup = Setup {
        player_board: vec![(body("Толкач", 8, 1).with_ability(shove()), cell(1, 3))],
        player_hand: vec![],
        keeper_board: vec![(body("Великан", 40, 1), cell(1, 2)), (body("Тыл", 8, 1), cell(2, 0))],
        keeper_hand: vec![],
        terrain: vec![tile(1, 1, Ground::Ravine)],
        field: Default::default(),
    };
    let st = MatchState::begin_with(setup, rules());
    let (st, events) = act(&st, Action::Cast { caster: 0, ability: "толчок".into(), target: Mark::Unit(1) });
    assert!(st.units[1].health.is_dead());
    assert!(st.board.cell_of(1).is_none());
    let fell = events.iter().position(|e| matches!(e, Event::Fell { unit: 1, .. }));
    let died = events.iter().position(|e| matches!(e, Event::Died { target: 1 }));
    assert!(fell.is_some() && fell < died, "{events:?}");
}

/// Жадная рука толкает в овраг, а не бьёт кулаком.
#[test]
fn the_hand_shoves_into_a_ravine() {
    let setup = Setup {
        player_board: vec![(body("Толкач", 8, 3).with_ability(shove()), cell(1, 3))],
        player_hand: vec![],
        keeper_board: vec![(body("Великан", 40, 1), cell(1, 2)), (body("Тыл", 8, 1), cell(2, 0))],
        keeper_hand: vec![],
        terrain: vec![tile(1, 1, Ground::Ravine)],
        field: Default::default(),
    };
    let st = MatchState::begin_with(setup, rules());
    assert!(matches!(bot::choose(&st), Action::Cast { target: Mark::Unit(1), .. }), "{:?}", bot::choose(&st));
}

/// Рука с шагом 2 прыгает через овраг, а не ищет обход, которого нет.
#[test]
fn the_hand_leaps_a_ravine_when_it_can() {
    let setup = Setup {
        player_board: vec![(body("Прыгун", 8, 2).with_step(2), cell(1, 4))],
        player_hand: vec![],
        keeper_board: vec![(body("Страж", 8, 1), cell(1, 0))],
        keeper_hand: vec![],
        terrain: vec![tile(0, 3, Ground::Ravine), tile(1, 3, Ground::Ravine), tile(2, 3, Ground::Ravine)],
        field: Default::default(),
    };
    let st = MatchState::begin_with(setup, rules());
    assert!(matches!(bot::choose(&st), Action::Move { unit: 0, to } if to.y == 2), "{:?}", bot::choose(&st));
}

// ── Яма ─────────────────────────────────────────────────────────────────────

/// Вошедший в яму ранен и дальше в этот ход не идёт.
#[test]
fn a_pit_wounds_and_stops() {
    let st = runner_at(1, 4, 3, vec![tile(1, 3, Ground::Pit), tile(0, 3, Ground::Wall), tile(2, 3, Ground::Wall)]);
    let moves = moves_of(&st, 0);
    assert!(moves.contains(&cell(1, 3)));
    assert!(moves.iter().all(|c| c.y >= 3), "дальше ямы нет: {moves:?}");
    let (st, events) = act(&st, Action::Move { unit: 0, to: cell(1, 3) });
    assert_eq!(st.units[0].health.current, 8 - PIT_HARM);
    assert!(events.iter().any(|e| matches!(e, Event::Damaged { target: 0, source: Source::Zone, .. })));
}

/// Выставленный в яму ранен тоже: яма — про всякого, кто в ней оказался.
#[test]
fn a_body_played_into_a_pit_is_wounded() {
    let st = runner_at(1, 4, 1, vec![tile(0, 5, Ground::Pit)]);
    let (st, _) = act(&st, Action::Play { hand_index: 0, cell: cell(0, 5) });
    let played = st.board.at(cell(0, 5)).unwrap();
    assert_eq!(st.units[played as usize].health.current, 8 - PIT_HARM);
}

// ── Холм ────────────────────────────────────────────────────────────────────

/// Стрелок на холме бьёт на клетку дальше — в полную силу, а не за далью.
#[test]
fn an_archer_on_a_hill_reaches_one_cell_further() {
    let setup = |hill: bool| Setup {
        player_board: vec![(body("Стрелок", 8, 4).with_reach(2), cell(1, 3))],
        player_hand: vec![],
        keeper_board: vec![(body("Цель", 10, 1), cell(1, 0))],
        keeper_hand: vec![],
        terrain: if hill { vec![tile(1, 3, Ground::Hill)] } else { vec![] },
        field: Default::default(),
    };
    let low = MatchState::begin_with(setup(false), rules());
    let (low, _) = act(&low, Action::Attack { attacker: 0, target: 1 });
    assert_eq!(low.units[1].health.current, 9, "за далью — четверть");

    let high = MatchState::begin_with(setup(true), rules());
    let (high, _) = act(&high, Action::Attack { attacker: 0, target: 1 });
    assert_eq!(high.units[1].health.current, 6, "с холма — в полную силу");
}

/// Ближнему бою холм не даёт ничего.
#[test]
fn a_hill_gives_nothing_to_a_fist() {
    let st = MatchState::begin_with(
        Setup {
            player_board: vec![(body("Боец", 8, 4), cell(1, 3))],
            player_hand: vec![],
            keeper_board: vec![(body("Цель", 10, 1), cell(1, 1))],
            keeper_hand: vec![],
            terrain: vec![tile(1, 3, Ground::Hill)],
            field: Default::default(),
        },
        rules(),
    );
    assert_eq!(st.reach_of(&st.units[0]), 1);
}

// ── Родник ──────────────────────────────────────────────────────────────────

/// Стоящий на роднике в начале своего хода возвращает здоровье.
#[test]
fn a_spring_mends_at_the_start_of_the_turn() {
    let setup = Setup {
        player_board: vec![(body("Путник", 8, 1), cell(1, 4))],
        player_hand: vec![],
        keeper_board: vec![(body("Стрелок", 8, 3).with_reach(3), cell(1, 1))],
        keeper_hand: vec![],
        terrain: vec![tile(1, 4, Ground::Spring)],
        field: Default::default(),
    };
    let st = MatchState::begin_with(setup, rules());
    let (st, _) = act(&st, Action::EndTurn);
    let (st, _) = act(&st, Action::Attack { attacker: 1, target: 0 });
    let hurt = st.units[0].health.current;
    assert!(hurt < 8);
    let (st, events) = act(&st, Action::EndTurn);
    assert_eq!(st.units[0].health.current, hurt + SPRING_MEND);
    assert!(events.iter().any(|e| matches!(e, Event::Healed { target: 0, by: None, .. })));
}
