//! Штурм: строения не держат сторону, а лимит кругов в штурме — за обороной.
//!
//! Оба правила заведены по замеру штурма (`BATTLE-GENRE-RESEARCH.md` § 7.5):
//! без первого город «стоял» одной стеной, когда в нём никого не было; без
//! второго нападающий, не взявший город, побеждал подсчётом здоровья.

use battle_core::*;

fn cell(x: u8, y: u8) -> Cell {
    Cell::new(x, y).unwrap()
}

fn wall() -> CardSnapshot {
    let mut w = CardSnapshot::new("стена", 0, 8, 0).with_armor(3).with_step(0);
    w.strikes = false;
    w.structure = true;
    w
}

fn id_of(st: &MatchState, name: &str) -> UnitId {
    st.units.iter().find(|u| u.card.name == name).expect("тело на поле").id
}

/// Город: стена в углу и один защитник у шва; у нападающего — боец вплотную.
fn town(wall: CardSnapshot) -> Setup {
    Setup {
        player_board: vec![(CardSnapshot::new("боец", 1, 10, 5), cell(1, 3))],
        keeper_board: vec![(wall, cell(0, 0)), (CardSnapshot::new("защитник", 1, 1, 1), cell(1, 2))],
        ..Default::default()
    }
}

fn rules() -> Rules {
    Rules { idle_toll: 0, ..Rules::default() }
}

#[test]
fn a_side_left_with_only_structures_is_beaten() {
    let st = MatchState::begin_with(town(wall()), rules());
    let (attacker, target) = (id_of(&st, "боец"), id_of(&st, "защитник"));
    let (st, events) = reduce(&st, &Action::Attack { attacker, target }).expect("удар законен");
    assert_eq!(st.outcome, Some(Outcome::Player), "стена стоит, но защищать некому");
    assert!(events.iter().any(|e| matches!(e, Event::Finished { outcome: Outcome::Player })));
    // Стена при этом цела: её не надо было ломать.
    let w = st.unit(id_of(&st, "стена")).unwrap();
    assert!(!w.health.is_dead());
}

#[test]
fn a_still_body_that_is_not_a_structure_still_holds_the_side() {
    // Котёл: не ходит и не бьёт, но строением не назван — сторона жива.
    let mut cauldron = wall();
    cauldron.structure = false;
    let st = MatchState::begin_with(town(cauldron), rules());
    let (attacker, target) = (id_of(&st, "боец"), id_of(&st, "защитник"));
    let (st, _) = reduce(&st, &Action::Attack { attacker, target }).expect("удар законен");
    assert_eq!(st.outcome, None);
}

/// Доиграть до лимита, никого не трогая.
fn run_out(rules: Rules) -> MatchState {
    // Защитник вне досягаемости бойца, чтобы никто никого не убил.
    let setup = Setup {
        player_board: vec![(CardSnapshot::new("боец", 1, 20, 0).with_step(0), cell(0, 5))],
        keeper_board: vec![(wall(), cell(2, 0)), (CardSnapshot::new("защитник", 1, 3, 0).with_step(0), cell(1, 0))],
        ..Default::default()
    };
    let mut st = MatchState::begin_with(setup, rules);
    let mut guard = 0;
    while st.outcome.is_none() && guard < 200 {
        st = reduce(&st, &Action::EndTurn).expect("конец хода законен").0;
        guard += 1;
    }
    st
}

#[test]
fn by_default_the_round_limit_goes_to_the_healthier_side() {
    let st = run_out(Rules { max_rounds: 3, ..rules() });
    assert_eq!(st.outcome, Some(Outcome::Player), "20 здоровья против 11");
}

#[test]
fn in_a_siege_the_round_limit_goes_to_the_defence() {
    let st = run_out(Rules { max_rounds: 3, keeper_holds_on_time: true, ..rules() });
    assert_eq!(st.outcome, Some(Outcome::Keeper), "город не взят — город устоял");
}

#[test]
fn old_records_read_without_the_new_fields() {
    // Записанное до этих полей читается как прежде: не строение, лимит — по здоровью.
    let card: CardSnapshot = serde_json::from_str(
        r#"{"name":"тело","cost":1,"health":3,"power":1,"armor":0,"ward":0,"reach":1,"step":1,"mend":0,"channel":"physical"}"#,
    )
    .unwrap();
    assert!(!card.structure);
    let written = serde_json::to_string(&Rules::default()).unwrap();
    assert!(!written.contains("keeperHoldsOnTime"), "выключенное не пишется");
    let read: Rules = serde_json::from_str(&written).unwrap();
    assert!(!read.keeper_holds_on_time);
}

// ── Через стену бьют только стрелки и маги ───────────────────────────────────

/// Стена во всю ширину поля 3 × 3 на ряду 2; за ней — защитник с дальностью 2,
/// перед ней, на ряду 3, — нападающий.
fn walled_town(defender: CardSnapshot) -> (MatchState, UnitId, UnitId) {
    let setup = Setup {
        player_board: vec![(CardSnapshot::new("нападающий", 1, 20, 1), cell(1, 3))],
        keeper_board: vec![
            (wall(), cell(0, 2)),
            (wall(), cell(1, 2)),
            (wall(), cell(2, 2)),
            (defender, cell(1, 1)),
        ],
        ..Default::default()
    };
    // Ходит хранитель: монета первого хода отдана игроку, поэтому сразу конец хода.
    let st = MatchState::begin_with(setup, rules());
    let st = reduce(&st, &Action::EndTurn).expect("конец хода").0;
    let (d, a) = (id_of(&st, "защитник"), id_of(&st, "нападающий"));
    (st, d, a)
}

fn hits(st: &MatchState, attacker: UnitId, target: UnitId) -> bool {
    legal_actions(st)
        .iter()
        .any(|a| matches!(a, Action::Attack { attacker: x, target: y } if *x == attacker && *y == target))
}

#[test]
fn a_long_weapon_does_not_strike_over_a_wall() {
    let (st, d, a) = walled_town(CardSnapshot::new("защитник", 1, 5, 3).with_reach(2));
    assert!(!hits(&st, d, a), "копьём через стену не бьют");
    // И присланный руками удар движок не примет — договор держится с обеих сторон.
    assert_eq!(
        reduce(&st, &Action::Attack { attacker: d, target: a }).unwrap_err(),
        Illegal::WallInTheWay
    );
}

#[test]
fn an_archer_shoots_over_a_wall() {
    let mut archer = CardSnapshot::new("защитник", 1, 5, 3).with_reach(2);
    archer.shoots = true;
    let (st, d, a) = walled_town(archer);
    assert!(hits(&st, d, a));
}

#[test]
fn a_mage_casts_over_a_wall() {
    let mage = CardSnapshot::new("защитник", 1, 5, 3)
        .with_reach(2)
        .with_channel(damage::Channel::Magic);
    let (st, d, a) = walled_town(mage);
    assert!(hits(&st, d, a));
}

#[test]
fn a_breach_opens_the_way_for_a_long_weapon() {
    // Ломаем среднюю стену: прямая от (1,1) к (1,3) идёт ровно через пролом.
    let (mut st, d, a) = walled_town(CardSnapshot::new("защитник", 1, 5, 3).with_reach(2));
    let middle = st.board.at(cell(1, 2)).unwrap();
    st.units[middle as usize].health.current = 0;
    st.board.clear(cell(1, 2));
    assert!(hits(&st, d, a));
}

#[test]
fn a_wall_on_the_diagonal_closes_the_diagonal() {
    // Прямая от (1,1) к (3,3) на поле 4 × 3 идёт через (2,2): стена там —
    // удар закрыт. Уберём её — открыт.
    let setup = Setup {
        field: Field { width: 4, depth: 3 },
        player_board: vec![(CardSnapshot::new("нападающий", 1, 20, 1), cell(3, 3))],
        keeper_board: vec![
            (wall(), cell(2, 2)),
            (CardSnapshot::new("защитник", 1, 5, 3).with_reach(2), cell(1, 1)),
        ],
        ..Default::default()
    };
    let st = MatchState::begin_with(setup.clone(), rules());
    let st = reduce(&st, &Action::EndTurn).unwrap().0;
    let (d, a) = (id_of(&st, "защитник"), id_of(&st, "нападающий"));
    assert!(!hits(&st, d, a));

    let mut open = setup;
    open.keeper_board.remove(0);
    let st = MatchState::begin_with(open, rules());
    let st = reduce(&st, &Action::EndTurn).unwrap().0;
    let (d, a) = (id_of(&st, "защитник"), id_of(&st, "нападающий"));
    assert!(hits(&st, d, a));
}

#[test]
fn a_wall_is_struck_point_blank_by_anyone() {
    // Нападающий стоит вплотную к стене и бьёт её: соседней клетке путь не нужен.
    let (st, _, _) = walled_town(CardSnapshot::new("защитник", 1, 5, 3).with_reach(2));
    let st = reduce(&st, &Action::EndTurn).unwrap().0;
    let a = id_of(&st, "нападающий");
    let w = st.board.at(cell(1, 2)).unwrap();
    assert!(hits(&st, a, w));
}

#[test]
fn a_blow_along_a_seam_is_closed_only_by_two_walls() {
    // От (1,1) к (2,3): на середине пути прямая идёт по стыку (1,2) и (2,2).
    let run = |walls: &[Cell]| {
        let mut keeper: Vec<(CardSnapshot, Cell)> = walls.iter().map(|c| (wall(), *c)).collect();
        keeper.push((CardSnapshot::new("защитник", 1, 5, 3).with_reach(2), cell(1, 1)));
        let setup = Setup {
            player_board: vec![(CardSnapshot::new("нападающий", 1, 20, 1), cell(2, 3))],
            keeper_board: keeper,
            ..Default::default()
        };
        let st = MatchState::begin_with(setup, rules());
        let st = reduce(&st, &Action::EndTurn).unwrap().0;
        hits(&st, id_of(&st, "защитник"), id_of(&st, "нападающий"))
    };
    assert!(run(&[cell(1, 2)]), "одна стена у стыка — щель остаётся");
    assert!(run(&[cell(2, 2)]), "и с другой стороны тоже");
    assert!(!run(&[cell(1, 2), cell(2, 2)]), "две стены — щели нет");
}

// ── Подкрепление выходит из города, а не появляется перед стеной ─────────────

/// Город на поле 3 × 3: стена во всю ширину на ряду 1, за ней ряд 0, перед
/// ней ряд 2 (тоже половина хранителя). В руке хранителя — подкрепление.
fn town_with_reinforcement(walls: &[u8]) -> MatchState {
    let mut keeper: Vec<(CardSnapshot, Cell)> = walls.iter().map(|&x| (wall(), cell(x, 1))).collect();
    keeper.push((CardSnapshot::new("защитник", 1, 5, 1), cell(0, 0)));
    let setup = Setup {
        player_board: vec![(CardSnapshot::new("нападающий", 1, 20, 1), cell(1, 5))],
        keeper_board: keeper,
        keeper_hand: vec![CardSnapshot::new("подкрепление", 0, 3, 1)],
        ..Default::default()
    };
    let st = MatchState::begin_with(setup, rules());
    reduce(&st, &Action::EndTurn).expect("конец хода").0
}

fn plays_at(st: &MatchState) -> Vec<(u8, u8)> {
    legal_actions(st)
        .iter()
        .filter_map(|a| match a {
            Action::Play { cell, .. } => Some((cell.x, cell.y)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_reinforcement_does_not_appear_beyond_a_whole_wall() {
    let st = town_with_reinforcement(&[0, 1, 2]);
    let at = plays_at(&st);
    assert!(at.iter().all(|&(_, y)| y == 0), "только за стеной: {at:?}");
    let refused = reduce(&st, &Action::Play { hand_index: 0, cell: cell(1, 2) });
    assert_eq!(refused.unwrap_err(), Illegal::BeyondTheWalls);
}

#[test]
fn a_breach_lets_the_reinforcement_out() {
    // Средней стены нет: через пролом ряд 2 соединён с тылом.
    let st = town_with_reinforcement(&[0, 2]);
    assert!(plays_at(&st).contains(&(1, 2)));
}

#[test]
fn without_structures_the_whole_half_is_open_as_before() {
    let st = town_with_reinforcement(&[]);
    let at = plays_at(&st);
    for y in 0..3 {
        assert!(at.iter().any(|&(_, yy)| yy == y), "ряд {y} открыт: {at:?}");
    }
}

// ── Умения тоже не проходят над стеной, если это не стрелок и не магия ───────

/// Поле 3 × 3: стена во всю ширину на ряду 2 (передний ряд хранителя), за ней
/// на (1,0) ворон. Перед стеной на (1,3) — тот, кто наводит умение на ворона:
/// до него три клетки, и на прямой стоит стена (1,2).
fn spell_over_the_wall(ability: AbilitySnapshot, shoots: bool) -> (MatchState, UnitId, UnitId) {
    let mut caster = CardSnapshot::new("наводящий", 1, 8, 1).with_ability(ability);
    caster.shoots = shoots;
    let setup = Setup {
        player_board: vec![(caster, cell(1, 3))],
        keeper_board: vec![
            (wall(), cell(0, 2)),
            (wall(), cell(1, 2)),
            (wall(), cell(2, 2)),
            (CardSnapshot::new("ворон", 1, 10, 1), cell(1, 0)),
        ],
        ..Default::default()
    };
    let st = MatchState::begin_with(setup, Rules { opening_attacks: 255, ..rules() });
    (st.clone(), id_of(&st, "наводящий"), id_of(&st, "ворон"))
}

fn bodily(ability: AbilitySnapshot) -> AbilitySnapshot {
    AbilitySnapshot { channel: damage::Channel::Physical, ..ability }
}

fn offered(st: &MatchState, caster: UnitId, target: Mark) -> bool {
    legal_actions(st)
        .iter()
        .any(|a| matches!(a, Action::Cast { caster: c, target: t, .. } if *c == caster && *t == target))
}

#[test]
fn a_bodily_spell_does_not_pass_over_a_wall() {
    let (st, me, raven) = spell_over_the_wall(bodily(AbilitySnapshot::harm("копьё", 3, 3)), false);
    assert!(!offered(&st, me, Mark::Unit(raven)), "в веере такой цели нет");
    let sent = Action::Cast { caster: me, ability: "копьё".into(), target: Mark::Unit(raven) };
    assert_eq!(reduce(&st, &sent).unwrap_err(), Illegal::WallInTheWay);
}

#[test]
fn a_magic_spell_passes_over_a_wall() {
    let (st, me, raven) = spell_over_the_wall(AbilitySnapshot::harm("огонь", 3, 3), false);
    assert!(offered(&st, me, Mark::Unit(raven)));
    let sent = Action::Cast { caster: me, ability: "огонь".into(), target: Mark::Unit(raven) };
    let (after, _) = reduce(&st, &sent).expect("магия идёт над стеной");
    assert_eq!(after.unit(raven).unwrap().health.current, 7);
}

#[test]
fn an_archer_s_bodily_spell_passes_over_a_wall() {
    let (st, me, raven) = spell_over_the_wall(bodily(AbilitySnapshot::harm("залп", 3, 3)), true);
    assert!(offered(&st, me, Mark::Unit(raven)));
}

#[test]
fn a_bodily_spell_goes_through_a_breach() {
    // Правило про стену, а не про расстояние: убрали стену с прямой — достаёт.
    let (mut st, me, raven) = spell_over_the_wall(bodily(AbilitySnapshot::harm("копьё", 3, 3)), false);
    let middle = st.board.at(cell(1, 2)).unwrap();
    st.units[middle as usize].health.current = 0;
    st.board.clear(cell(1, 2));
    assert!(offered(&st, me, Mark::Unit(raven)));
}

#[test]
fn a_bodily_reaction_does_not_reach_over_a_wall_but_magic_does() {
    // «В начале хода» реакция сама берёт ближайшего чужого. Стена здесь СВОЯ
    // (ряд 4 у игрока), чтобы единственным чужим был ворон за ней: чужая стена
    // сама была бы ближайшей целью, и проверка спрашивала бы не то.
    let turn_start = |channel: damage::Channel| AbilitySnapshot {
        trigger: "turnStart".into(),
        channel,
        ..AbilitySnapshot::harm("тлен", 2, 5)
    };
    let raven_after_my_next_turn_start = |channel| {
        let setup = Setup {
            player_board: vec![
                (wall(), cell(0, 4)),
                (wall(), cell(1, 4)),
                (wall(), cell(2, 4)),
                (CardSnapshot::new("наводящий", 1, 8, 1).with_ability(turn_start(channel)), cell(1, 5)),
            ],
            keeper_board: vec![(CardSnapshot::new("ворон", 1, 10, 0).with_step(0), cell(1, 2))],
            ..Default::default()
        };
        let st = MatchState::begin_with(setup, Rules { opening_attacks: 255, ..rules() });
        let raven = id_of(&st, "ворон");
        let st = reduce(&st, &Action::EndTurn).unwrap().0;
        let st = reduce(&st, &Action::EndTurn).unwrap().0;
        st.unit(raven).unwrap().health.current
    };
    assert_eq!(raven_after_my_next_turn_start(damage::Channel::Physical), 10, "своя стена закрывает");
    assert_eq!(raven_after_my_next_turn_start(damage::Channel::Magic), 8, "магия — над ней");
}

#[test]
fn a_summoned_body_rises_inside_its_own_walls_only() {
    // Свои стены у игрока: ряд 4 во всю ширину, призыватель на (1,5) за ними.
    // Клетка (1,3) — перед своей стеной, (0,5) — внутри.
    let mut summon = AbilitySnapshot { verb: "summon".into(), ..AbilitySnapshot::harm("призыв", 1, 3) };
    summon.body = Some(Box::new(CardSnapshot::new("тень", 0, 3, 1)));
    let setup = Setup {
        player_board: vec![
            (wall(), cell(0, 4)),
            (wall(), cell(1, 4)),
            (wall(), cell(2, 4)),
            (CardSnapshot::new("призыватель", 1, 8, 1).with_ability(summon), cell(1, 5)),
        ],
        keeper_board: vec![(CardSnapshot::new("ворон", 1, 10, 1), cell(1, 0))],
        ..Default::default()
    };
    let st = MatchState::begin_with(setup, rules());
    let me = id_of(&st, "призыватель");
    assert!(!offered(&st, me, Mark::Spot(cell(1, 3))), "перед своей стеной не встаёт");
    assert!(offered(&st, me, Mark::Spot(cell(0, 5))), "внутри — встаёт");
    let sent = Action::Cast { caster: me, ability: "призыв".into(), target: Mark::Spot(cell(1, 3)) };
    assert_eq!(reduce(&st, &sent).unwrap_err(), Illegal::BeyondTheWalls);
}
