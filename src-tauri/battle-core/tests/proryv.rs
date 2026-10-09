//! Прорыв: тело, простоявшее ход противника на его краю, выигрывает партию.
//!
//! Правило заведено ради партий, которые не кончались: «уничтожить всех»
//! обороняющийся вправе не преследовать. Батарея проверяет не баланс, а смысл
//! слов: «дошло» — ещё не победа, «простояло» — победа, павший и уведённый края
//! не держат, а партия, записанная без правила, играется без него.

use battle_core::*;

fn cell(x: u8, y: u8) -> Cell {
    Cell::new(x, y).unwrap()
}

fn body(name: &str, health: i32, power: i32) -> CardSnapshot {
    CardSnapshot::new(name, 1, health, power)
}

/// Гонец с шагом 3: от переднего ряда до чужого края ровно три клетки.
fn runner() -> CardSnapshot {
    body("Гонец", 8, 1).with_step(3)
}

/// Правила батареи: прорыв включён, платы за бездействие нет — она снимает по
/// единице с простоявших, и счёт здоровья пришлось бы держать в уме.
fn rules() -> Rules {
    Rules { breakthrough: true, idle_toll: 0, ..Rules::default() }
}

fn act(state: &MatchState, action: Action) -> (MatchState, Vec<Event>) {
    reduce(state, &action).expect("действие должно быть законным")
}

/// Гонец у левого края, хранитель далеко справа и ему не мешает.
fn open_lane() -> Setup {
    Setup {
        player_board: vec![(runner(), cell(0, 3))],
        player_hand: vec![],
        keeper_board: vec![(body("Страж", 8, 1), cell(2, 2))],
        keeper_hand: vec![],
        terrain: Vec::new(),
        field: Default::default(),
    }
}

fn to_the_edge() -> Action {
    Action::Move { unit: 0, to: cell(0, 0) }
}

/// Дошёл, простоял ход хранителя — выиграл. Причина названа событием прямо
/// перед исходом: сцене не нужно угадывать её по доске.
#[test]
fn standing_through_the_keepers_turn_wins() {
    let st = MatchState::begin_with(open_lane(), rules());
    let (st, _) = act(&st, to_the_edge());
    let (st, _) = act(&st, Action::EndTurn);
    let (st, events) = act(&st, Action::EndTurn);

    assert_eq!(st.outcome, Some(Outcome::Player));
    let n = events.len();
    assert_eq!(events[n - 2], Event::Breached { unit: 0, side: Side::Player });
    assert_eq!(events[n - 1], Event::Finished { outcome: Outcome::Player });
}

/// Дойти — ещё не выиграть: у хранителя есть ход, чтобы ответить.
#[test]
fn arriving_is_not_yet_winning() {
    let st = MatchState::begin_with(open_lane(), rules());
    let (st, _) = act(&st, to_the_edge());
    assert_eq!(st.outcome, None);
    let (st, _) = act(&st, Action::EndTurn);
    assert_eq!(st.outcome, None, "ход хранителя ещё впереди");
    assert_eq!(st.poised, vec![0], "но гонец запомнен");
}

/// Павший края не держит.
#[test]
fn a_runner_killed_on_the_edge_wins_nothing() {
    let setup = Setup {
        player_board: vec![
            (body("Гонец", 2, 1).with_step(3), cell(0, 3)),
            // Второе тело — чтобы гибель гонца не кончала партию сама.
            (body("Тыл", 8, 1), cell(2, 5)),
        ],
        player_hand: vec![],
        keeper_board: vec![(body("Страж", 8, 5), cell(1, 1))],
        keeper_hand: vec![],
        terrain: Vec::new(),
        field: Default::default(),
    };
    let st = MatchState::begin_with(setup, rules());
    let (st, _) = act(&st, to_the_edge());
    let (st, _) = act(&st, Action::EndTurn);
    let (st, _) = act(&st, Action::Attack { attacker: 2, target: 0 });
    assert!(st.units[0].health.is_dead(), "иначе испытание проверяет не то");
    let (st, events) = act(&st, Action::EndTurn);

    assert_eq!(st.outcome, None);
    assert!(!events.iter().any(|e| matches!(e, Event::Breached { .. })));
}

/// Уведённое смутой тело не делает прорыва никому: хозяин им не ходит, а
/// уведшему засчитать его значило бы выигрывать смутой у собственного края.
#[test]
fn a_swayed_runner_wins_nothing() {
    let charm = AbilitySnapshot {
        verb: "charm".into(),
        duration: 2,
        ..AbilitySnapshot::harm("смута", 1, 3)
    };
    let setup = Setup {
        player_board: vec![(runner(), cell(0, 3)), (body("Тыл", 8, 1), cell(2, 5))],
        player_hand: vec![],
        keeper_board: vec![(body("Ведьма", 8, 1).with_ability(charm), cell(2, 0))],
        keeper_hand: vec![],
        terrain: Vec::new(),
        field: Default::default(),
    };
    let st = MatchState::begin_with(setup, rules());
    let (st, _) = act(&st, to_the_edge());
    let (st, _) = act(&st, Action::EndTurn);
    let witch = 2;
    let (st, _) = act(
        &st,
        Action::Cast { caster: witch, ability: "смута".into(), target: Mark::Unit(0) },
    );
    assert_eq!(st.units[0].side(), Side::Keeper, "иначе испытание проверяет не то");
    let (st, _) = act(&st, Action::EndTurn);

    assert_eq!(st.outcome, None);
}

/// Хранитель прорывается так же: его край — шестой ряд.
#[test]
fn the_keeper_breaks_through_the_same_way() {
    let setup = Setup {
        player_board: vec![(body("Страж", 8, 1), cell(2, 3))],
        player_hand: vec![],
        keeper_board: vec![(runner(), cell(0, 2))],
        keeper_hand: vec![],
        terrain: Vec::new(),
        field: Default::default(),
    };
    let st = MatchState::begin_with(setup, rules());
    let (st, _) = act(&st, Action::EndTurn);
    let (st, _) = act(&st, Action::Move { unit: 1, to: cell(0, 5) });
    let (st, _) = act(&st, Action::EndTurn);
    let (st, events) = act(&st, Action::EndTurn);

    assert_eq!(st.outcome, Some(Outcome::Keeper));
    assert!(events.contains(&Event::Breached { unit: 1, side: Side::Keeper }));
}

/// Простоявший ход заработал победу, и истёкший лимит кругов её не отнимает.
#[test]
fn a_breakthrough_is_decided_before_the_clock() {
    let rules = Rules { max_rounds: 1, ..rules() };
    let st = MatchState::begin_with(open_lane(), rules);
    let (st, _) = act(&st, to_the_edge());
    let (st, _) = act(&st, Action::EndTurn);
    let (st, events) = act(&st, Action::EndTurn);

    assert_eq!(st.outcome, Some(Outcome::Player));
    assert!(events.contains(&Event::Breached { unit: 0, side: Side::Player }));
}

/// Без правила — ничего не меняется: ни победы, ни памяти о крае.
#[test]
fn without_the_rule_the_edge_is_just_a_row() {
    let off = Rules { breakthrough: false, ..rules() };
    let st = MatchState::begin_with(open_lane(), off);
    let (st, _) = act(&st, to_the_edge());
    let (st, _) = act(&st, Action::EndTurn);
    let (st, _) = act(&st, Action::EndTurn);

    assert_eq!(st.outcome, None);
    assert!(st.poised.is_empty());
}

/// Записанное до ручки читается как «без прорыва»: старые этюды и партии
/// играются тем, чем игрались.
#[test]
fn rules_written_before_the_rule_read_as_without_it() {
    let mut json = serde_json::to_value(Rules::default()).unwrap();
    json.as_object_mut().unwrap().remove("breakthrough");
    let read: Rules = serde_json::from_value(json).unwrap();
    assert!(!read.breakthrough);

    let mut state = serde_json::to_value(MatchState::begin(open_lane())).unwrap();
    state.as_object_mut().unwrap().remove("poised");
    let read: MatchState = serde_json::from_value(state).unwrap();
    assert!(read.poised.is_empty());
}

/// Жадная рука знает цель: встаёт на край, когда может.
#[test]
fn the_greedy_hand_steps_onto_the_edge() {
    let st = MatchState::begin_with(open_lane(), rules());
    assert!(matches!(bot::choose(&st), Action::Move { unit: 0, to } if to.y == 0));
}

/// И бережёт свой край: чужое тело на нём бьётся прежде всего.
#[test]
fn the_greedy_hand_strikes_whoever_stands_on_its_edge() {
    let setup = Setup {
        player_board: vec![(body("Гонец", 8, 1), cell(0, 0)), (body("Тыл", 2, 1), cell(1, 3))],
        player_hand: vec![],
        // Страж может добить слабого «Тыла» — и всё же бьёт гонца.
        keeper_board: vec![(body("Страж", 8, 2).with_reach(3), cell(1, 1))],
        keeper_hand: vec![],
        terrain: Vec::new(),
        field: Default::default(),
    };
    let st = MatchState::begin_with(setup, rules());
    let (st, _) = act(&st, Action::EndTurn);
    assert_eq!(st.active, Side::Keeper);
    assert_eq!(bot::choose(&st), Action::Attack { attacker: 2, target: 0 });
}
