//! Волны: подкрепление по кругам и условия победы.
//!
//! Движок не знает слова «забег»: он принимает расстановку, расписание выхода
//! и условие победы. Всё остальное считает сервер (`TASKS-BATTLE-RUN.md` § 3.1).

use battle_core::*;

fn cell(x: u8, y: u8) -> Cell {
    Cell::new(x, y).unwrap()
}

fn rules() -> Rules {
    Rules { idle_toll: 0, ..Rules::default() }
}

fn still(name: &str, health: i32, power: i32) -> CardSnapshot {
    CardSnapshot::new(name, 0, health, power).with_step(0)
}

fn wall(name: &str, health: i32) -> CardSnapshot {
    let mut w = still(name, health, 0);
    w.strikes = false;
    w.structure = true;
    w
}

fn end(st: &MatchState) -> (MatchState, Vec<Event>) {
    reduce(st, &Action::EndTurn).expect("конец хода законен")
}

fn id_of(st: &MatchState, name: &str) -> UnitId {
    st.units.iter().find(|u| u.card.name == name).expect("тело есть").id
}

fn names(st: &MatchState, side: Side) -> Vec<String> {
    let mut all: Vec<String> =
        st.standing(side).iter().map(|id| st.units[*id as usize].card.name.clone()).collect();
    all.sort();
    all
}

/// Оборона без подкрепления: поле, на котором враг стоит с начала.
fn plain() -> Setup {
    Setup {
        player_board: vec![(still("страж", 5, 1), cell(1, 5))],
        keeper_board: vec![(still("враг", 5, 1), cell(1, 0))],
        ..Default::default()
    }
}

fn with_reinforcements(list: Vec<(u8, Cell, &str)>) -> Setup {
    Setup {
        reinforcements: list
            .into_iter()
            .map(|(round, cell, name)| Reinforcement { round, cell, card: still(name, 3, 1) })
            .collect(),
        ..plain()
    }
}

#[test]
fn a_match_without_waves_is_written_exactly_as_before() {
    // Замороженные партии не меняются ни на байт: новых ключей нет.
    let setup = serde_json::to_string(&plain()).unwrap();
    let state = serde_json::to_string(&MatchState::begin_with(plain(), rules())).unwrap();
    for key in ["reinforcements", "goal", "playerHealth", "pending"] {
        assert!(!setup.contains(key), "{key} в расстановке");
        assert!(!state.contains(key), "{key} в партии");
    }
    // И старая запись читается: ключей нет, поля по умолчанию.
    let read: Setup = serde_json::from_str(&setup).unwrap();
    assert!(read.reinforcements.is_empty() && read.goal.is_none() && read.player_health.is_empty());
}

#[test]
fn a_reinforcement_comes_out_at_its_round_and_not_before() {
    let st = MatchState::begin_with(with_reinforcements(vec![(2, cell(0, 0), "второй")]), rules());
    assert_eq!(names(&st, Side::Keeper), ["враг"]);
    let (st, _) = end(&st); // конец хода игрока
    let (st, _) = end(&st); // конец хода хранителя: начался круг 2
    assert_eq!(st.round, 2);
    assert_eq!(names(&st, Side::Keeper), ["враг", "второй"]);
    assert!(st.pending.is_empty());
}

#[test]
fn the_entrance_is_announced_like_a_played_card() {
    let st = MatchState::begin_with(with_reinforcements(vec![(2, cell(0, 0), "второй")]), rules());
    let (st, _) = end(&st);
    let (_, events) = end(&st);
    assert!(events.iter().any(|e| matches!(
        e,
        Event::Played { side: Side::Keeper, cell: c, cost: 0, .. } if *c == cell(0, 0)
    )));
}

#[test]
fn a_round_one_reinforcement_stands_from_the_start() {
    let st = MatchState::begin_with(with_reinforcements(vec![(1, cell(0, 0), "первый")]), rules());
    assert_eq!(names(&st, Side::Keeper), ["враг", "первый"]);
    assert!(!st.units[id_of(&st, "первый") as usize].acted);
}

#[test]
fn an_occupied_entrance_delays_the_entry_and_loses_nothing() {
    // Клетка входа (1, 0) занята «врагом»: подкрепление ждёт.
    let st = MatchState::begin_with(with_reinforcements(vec![(2, cell(1, 0), "второй")]), rules());
    let (st, _) = end(&st);
    let (st, _) = end(&st);
    assert_eq!(st.round, 2);
    assert_eq!(names(&st, Side::Keeper), ["враг"]);
    assert_eq!(st.pending.len(), 1, "карта осталась в расписании");
}

#[test]
fn a_delayed_entry_comes_out_once_the_cell_is_free() {
    let mut st = MatchState::begin_with(with_reinforcements(vec![(2, cell(1, 0), "второй")]), rules());
    // «Враг» убирается с клетки входа до круга 3 — как если бы его убили.
    let (s, _) = end(&st);
    let (s, _) = end(&s);
    st = s;
    let blocker = id_of(&st, "враг");
    st.units[blocker as usize].health.current = 0;
    st.board.clear(cell(1, 0));
    let (st, _) = end(&st);
    let (st, _) = end(&st);
    assert_eq!(st.round, 3);
    assert_eq!(names(&st, Side::Keeper), ["второй"]);
    assert!(st.pending.is_empty());
}

#[test]
fn an_empty_field_with_enemies_still_to_come_is_not_a_win() {
    let setup = Setup {
        player_board: vec![(CardSnapshot::new("страж", 0, 5, 9).with_reach(6), cell(1, 5))],
        keeper_board: vec![(still("враг", 1, 0), cell(1, 0))],
        reinforcements: vec![Reinforcement { round: 3, cell: cell(0, 0), card: still("позже", 3, 1) }],
        ..Default::default()
    };
    let st = MatchState::begin_with(setup, rules());
    let (attacker, target) = (id_of(&st, "страж"), id_of(&st, "враг"));
    let (st, _) = reduce(&st, &Action::Attack { attacker, target }).expect("удар законен");
    assert!(st.standing(Side::Keeper).is_empty());
    assert_eq!(st.outcome, None, "второй группе ещё выходить");
}

#[test]
fn once_the_schedule_is_spent_an_empty_field_is_a_win() {
    let setup = Setup {
        player_board: vec![(CardSnapshot::new("страж", 0, 5, 9).with_reach(6), cell(1, 5))],
        keeper_board: vec![(still("враг", 1, 0), cell(1, 0))],
        ..Default::default()
    };
    let st = MatchState::begin_with(setup, rules());
    let (attacker, target) = (id_of(&st, "страж"), id_of(&st, "враг"));
    let (st, _) = reduce(&st, &Action::Attack { attacker, target }).unwrap();
    assert_eq!(st.outcome, Some(Outcome::Player));
}

#[test]
fn a_card_enters_the_wave_with_the_health_it_has_left() {
    let setup = Setup { player_health: vec![2], ..plain() };
    let st = MatchState::begin_with(setup, rules());
    let u = st.unit(id_of(&st, "страж")).unwrap();
    assert_eq!((u.health.current, u.health.max), (2, 5));
}

#[test]
fn carried_health_is_clamped_to_the_card() {
    let setup = Setup { player_health: vec![99], ..plain() };
    let st = MatchState::begin_with(setup, rules());
    assert_eq!(st.unit(id_of(&st, "страж")).unwrap().health.current, 5);
    let setup = Setup { player_health: vec![0], ..plain() };
    let st = MatchState::begin_with(setup, rules());
    assert_eq!(st.unit(id_of(&st, "страж")).unwrap().health.current, 1);
}

// ── Условия победы ────────────────────────────────────────────────────────

/// Круги идут, никто никого не трогает.
fn idle_until_over(mut st: MatchState) -> MatchState {
    let mut guard = 0;
    while st.outcome.is_none() && guard < 100 {
        st = end(&st).0;
        guard += 1;
    }
    st
}

#[test]
fn hold_rounds_goes_to_the_player_when_the_rounds_run_out() {
    let setup = Setup { goal: Some(WaveGoal::HoldRounds { rounds: 3 }), ..plain() };
    let st = idle_until_over(MatchState::begin_with(setup, rules()));
    assert_eq!(st.outcome, Some(Outcome::Player));
    assert_eq!(st.round, 4, "три круга сыграны целиком");
}

#[test]
fn without_a_goal_the_round_limit_still_counts_health() {
    // Прежнее правило: у врага здоровья столько же — ничья, не победа игрока.
    let st = idle_until_over(MatchState::begin_with(plain(), Rules { max_rounds: 3, ..rules() }));
    assert_eq!(st.outcome, Some(Outcome::Draw));
}

#[test]
fn defend_structure_is_lost_when_the_structure_falls() {
    let setup = Setup {
        player_board: vec![(wall("башня", 2), cell(1, 3)), (still("страж", 9, 1), cell(0, 5))],
        keeper_board: vec![(still("враг", 5, 5), cell(1, 2))],
        goal: Some(WaveGoal::DefendStructure),
        ..Default::default()
    };
    let st = MatchState::begin_with(setup, rules());
    let (st, _) = end(&st);
    let (attacker, target) = (id_of(&st, "враг"), id_of(&st, "башня"));
    let (st, _) = reduce(&st, &Action::Attack { attacker, target }).expect("удар законен");
    assert_eq!(st.outcome, Some(Outcome::Keeper), "страж жив, но защищать было строение");
}

#[test]
fn defend_structure_is_won_when_it_stands_to_the_limit() {
    let setup = Setup {
        player_board: vec![(wall("башня", 9), cell(1, 5)), (still("страж", 9, 1), cell(0, 5))],
        keeper_board: vec![(still("враг", 5, 5), cell(1, 0))],
        goal: Some(WaveGoal::DefendStructure),
        ..Default::default()
    };
    let st = idle_until_over(MatchState::begin_with(setup, Rules { max_rounds: 4, ..rules() }));
    assert_eq!(st.outcome, Some(Outcome::Player));
}

#[test]
fn kill_target_is_won_when_the_boss_falls_though_others_stand() {
    let setup = Setup {
        player_board: vec![(CardSnapshot::new("страж", 0, 5, 9).with_reach(6), cell(1, 5))],
        keeper_board: vec![(still("босс", 1, 0), cell(1, 0)), (still("прислужник", 9, 0), cell(0, 0))],
        goal: Some(WaveGoal::KillTarget { name: "босс".into() }),
        ..Default::default()
    };
    let st = MatchState::begin_with(setup, rules());
    let (attacker, target) = (id_of(&st, "страж"), id_of(&st, "босс"));
    let (st, _) = reduce(&st, &Action::Attack { attacker, target }).unwrap();
    assert_eq!(st.outcome, Some(Outcome::Player));
}

#[test]
fn kill_target_waits_for_a_boss_that_has_not_come_out_yet() {
    let setup = Setup {
        player_board: vec![(CardSnapshot::new("страж", 0, 5, 9).with_reach(6), cell(1, 5))],
        keeper_board: vec![(still("прислужник", 1, 0), cell(1, 0))],
        reinforcements: vec![Reinforcement { round: 3, cell: cell(0, 0), card: still("босс", 3, 0) }],
        goal: Some(WaveGoal::KillTarget { name: "босс".into() }),
        ..Default::default()
    };
    let st = MatchState::begin_with(setup, rules());
    assert_eq!(st.outcome, None, "босса ещё нет, победить рано");
}

#[test]
fn kill_target_goes_to_the_enemy_when_the_boss_outlasts_the_rounds() {
    let setup = Setup {
        goal: Some(WaveGoal::KillTarget { name: "враг".into() }),
        ..plain()
    };
    let st = idle_until_over(MatchState::begin_with(setup, Rules { max_rounds: 3, ..rules() }));
    assert_eq!(st.outcome, Some(Outcome::Keeper));
}

#[test]
fn a_wave_state_survives_the_journal() {
    let setup = Setup {
        goal: Some(WaveGoal::HoldRounds { rounds: 4 }),
        player_health: vec![3],
        ..with_reinforcements(vec![(2, cell(0, 0), "второй")])
    };
    let st = MatchState::begin_with(setup, rules());
    let back: MatchState = serde_json::from_str(&serde_json::to_string(&st).unwrap()).unwrap();
    assert_eq!(back, st);
}
