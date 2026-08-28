//! The frame: field, turn, action economy, ending.
//!
//! The two tests that matter most are at the bottom — a whole match played out
//! by two bots, and the same match played twice giving the same journal. Every
//! later stage has to keep both of them passing.

use battle_core::*;

fn cell(x: u8, y: u8) -> Cell {
    Cell::new(x, y).unwrap()
}

fn boec(name: &str, cost: i32, health: i32, power: i32) -> CardSnapshot {
    CardSnapshot::new(name, cost, health, power)
}

/// Player at the front of their half, keeper at the front of theirs, one step
/// apart: y 2 and y 3 are neighbouring rows across the middle.
fn face_off() -> Setup {
    Setup {
        player_board: vec![(boec("Боец", 1, 6, 3), cell(1, 3))],
        player_hand: vec![],
        keeper_board: vec![(boec("Ворон", 1, 6, 3), cell(1, 2))],
        keeper_hand: vec![],
    }
}

fn act(state: &MatchState, action: Action) -> (MatchState, Vec<Event>) {
    reduce(state, &action).expect("действие должно быть законным")
}

// ── поле ────────────────────────────────────────────────────────────────────

#[test]
fn a_diagonal_costs_the_same_as_a_straight_step() {
    assert_eq!(cell(0, 0).distance(cell(1, 1)), 1);
    assert_eq!(cell(0, 0).distance(cell(0, 1)), 1);
}

#[test]
fn range_five_is_exactly_the_whole_field() {
    // The far rank of one side to the far rank of the other, corner to corner.
    assert_eq!(cell(0, 0).distance(cell(2, 5)), 5);
    assert!(Cell::new(0, 6).is_none(), "поля глубже шести рядов нет");
}

#[test]
fn the_half_a_cell_belongs_to_is_read_off_its_row() {
    assert_eq!(cell(0, 2).side(), Side::Keeper);
    assert_eq!(cell(0, 3).side(), Side::Player);
}

// ── розыгрыш карты ──────────────────────────────────────────────────────────

#[test]
fn playing_a_card_costs_mana_and_puts_a_body_on_the_field() {
    let mut setup = face_off();
    setup.player_hand = vec![boec("Стрелок", 1, 4, 2)];
    let st = MatchState::begin(setup);
    assert_eq!(st.player.mana, 1);

    let (st, events) = act(&st, Action::Play { hand_index: 0, cell: cell(0, 5) });
    assert_eq!(st.player.mana, 0);
    assert!(st.player.hand.is_empty());
    assert_eq!(st.standing(Side::Player).len(), 2);
    assert!(matches!(events[0], Event::Played { cost: 1, .. }));
}

#[test]
fn a_card_cannot_be_played_onto_the_other_half() {
    let mut setup = face_off();
    setup.player_hand = vec![boec("Стрелок", 1, 4, 2)];
    let st = MatchState::begin(setup);
    let r = reduce(&st, &Action::Play { hand_index: 0, cell: cell(0, 1) });
    assert_eq!(r.unwrap_err(), Illegal::NotYourHalf);
}

#[test]
fn a_taken_cell_refuses_a_second_body() {
    let mut setup = face_off();
    setup.player_hand = vec![boec("Стрелок", 1, 4, 2)];
    let st = MatchState::begin(setup);
    let r = reduce(&st, &Action::Play { hand_index: 0, cell: cell(1, 3) });
    assert_eq!(r.unwrap_err(), Illegal::CellTaken);
}

#[test]
fn a_card_beyond_the_mana_is_refused() {
    let mut setup = face_off();
    setup.player_hand = vec![boec("Маг", 5, 4, 8)];
    let st = MatchState::begin(setup);
    let r = reduce(&st, &Action::Play { hand_index: 0, cell: cell(0, 5) });
    assert_eq!(r.unwrap_err(), Illegal::NotEnoughMana);
}

#[test]
fn a_body_played_this_turn_does_not_swing_this_turn() {
    let mut setup = face_off();
    setup.player_hand = vec![boec("Стрелок", 1, 4, 2)];
    let st = MatchState::begin(setup);
    let (st, _) = act(&st, Action::Play { hand_index: 0, cell: cell(1, 4) });
    let newcomer = st.standing(Side::Player)[1];
    let r = reduce(&st, &Action::Attack { attacker: newcomer, target: 1 });
    assert_eq!(r.unwrap_err(), Illegal::AlreadyActed);
}

// ── удар ────────────────────────────────────────────────────────────────────

#[test]
fn a_blow_lands_only_within_reach() {
    let setup = Setup {
        player_board: vec![(boec("Боец", 1, 6, 3), cell(1, 5))],
        keeper_board: vec![(boec("Ворон", 1, 6, 3), cell(1, 0))],
        ..Default::default()
    };
    let st = MatchState::begin(setup);
    assert_eq!(
        reduce(&st, &Action::Attack { attacker: 0, target: 1 }).unwrap_err(),
        Illegal::OutOfReach
    );
}

#[test]
fn a_longer_reach_crosses_the_field() {
    let setup = Setup {
        player_board: vec![(boec("Стрелок", 1, 6, 3).with_reach(5), cell(1, 5))],
        keeper_board: vec![(boec("Ворон", 1, 6, 3), cell(1, 0))],
        ..Default::default()
    };
    let st = MatchState::begin(setup);
    let (st, _) = act(&st, Action::Attack { attacker: 0, target: 1 });
    assert_eq!(st.unit(1).unwrap().health.current, 3);
}

#[test]
fn nobody_strikes_their_own() {
    let setup = Setup {
        player_board: vec![
            (boec("Боец", 1, 6, 3), cell(0, 3)),
            (boec("Второй", 1, 6, 3), cell(1, 3)),
        ],
        ..Default::default()
    };
    let st = MatchState::begin(setup);
    assert_eq!(
        reduce(&st, &Action::Attack { attacker: 0, target: 1 }).unwrap_err(),
        Illegal::TargetIsAlly
    );
}

#[test]
fn one_action_per_body_per_turn() {
    let st = MatchState::begin(face_off());
    let (st, _) = act(&st, Action::Attack { attacker: 0, target: 1 });
    assert_eq!(
        reduce(&st, &Action::Attack { attacker: 0, target: 1 }).unwrap_err(),
        Illegal::AlreadyActed
    );
}

#[test]
fn a_fallen_body_leaves_the_field_but_keeps_its_name() {
    let setup = Setup {
        player_board: vec![(boec("Боец", 1, 6, 9), cell(1, 3))],
        keeper_board: vec![(boec("Ворон", 1, 3, 1), cell(1, 2))],
        keeper_hand: vec![boec("Второй ворон", 1, 3, 1)],
        ..Default::default()
    };
    let st = MatchState::begin(setup);
    let (st, events) = act(&st, Action::Attack { attacker: 0, target: 1 });

    assert!(events.contains(&Event::Died { target: 1 }));
    assert!(st.standing(Side::Keeper).is_empty());
    assert!(st.unit(1).is_some(), "личность остаётся в списке — журнал её назовёт");
}

// ── ход ─────────────────────────────────────────────────────────────────────

#[test]
fn mana_rises_by_one_each_of_your_own_turns() {
    // Второй стороне к этому прибавлена монета за право второго хода —
    // см. `Rules::default`, где размер выбран замером.
    let st = MatchState::begin(face_off());
    assert_eq!(st.player.mana, 1);
    let (st, _) = act(&st, Action::EndTurn);
    assert_eq!(st.active, Side::Keeper);
    assert_eq!(st.keeper.mana, 3, "монета в два плюс собственный ход");
    let (st, _) = act(&st, Action::EndTurn);
    assert_eq!(st.active, Side::Player);
    assert_eq!(st.round, 2);
    assert_eq!(st.player.mana, 2, "у первой стороны рост обычный");
    let (st, _) = act(&st, Action::EndTurn);
    assert_eq!(st.keeper.mana, 4, "и дальше растёт от монеты");
}

#[test]
fn bodies_are_ready_again_at_the_start_of_your_turn() {
    let st = MatchState::begin(face_off());
    let (st, _) = act(&st, Action::Attack { attacker: 0, target: 1 });
    let (st, _) = act(&st, Action::EndTurn);
    let (st, _) = act(&st, Action::EndTurn);
    assert!(!st.unit(0).unwrap().acted);
}

// ── конец партии ────────────────────────────────────────────────────────────

#[test]
fn a_side_with_nothing_standing_and_nothing_held_has_lost() {
    let setup = Setup {
        player_board: vec![(boec("Боец", 1, 6, 9), cell(1, 3))],
        keeper_board: vec![(boec("Ворон", 1, 3, 1), cell(1, 2))],
        ..Default::default()
    };
    let st = MatchState::begin(setup);
    let (st, events) = act(&st, Action::Attack { attacker: 0, target: 1 });
    assert_eq!(st.outcome, Some(Outcome::Player));
    assert!(events.contains(&Event::Finished { outcome: Outcome::Player }));
}

#[test]
fn a_side_still_holding_a_card_has_not_lost_yet() {
    let setup = Setup {
        player_board: vec![(boec("Боец", 1, 6, 9), cell(1, 3))],
        keeper_board: vec![(boec("Ворон", 1, 3, 1), cell(1, 2))],
        keeper_hand: vec![boec("Второй ворон", 1, 3, 1)],
        ..Default::default()
    };
    let st = MatchState::begin(setup);
    let (st, _) = act(&st, Action::Attack { attacker: 0, target: 1 });
    assert_eq!(st.outcome, None);
}

#[test]
fn a_card_nobody_can_ever_afford_is_the_same_as_an_empty_hand() {
    // Пустая доска и карта дороже потолка маны. Раньше партия досиживала до
    // двенадцатого круга, пока обе стороны молча передавали ход.
    let setup = Setup {
        player_board: vec![(boec("Боец", 1, 6, 9), cell(1, 3))],
        keeper_board: vec![(boec("Ворон", 1, 3, 1), cell(1, 2))],
        keeper_hand: vec![boec("Неподъёмная", 99, 5, 5)],
        ..Default::default()
    };
    let st = MatchState::begin(setup);
    let (st, events) = act(&st, Action::Attack { attacker: 0, target: 1 });
    assert_eq!(st.outcome, Some(Outcome::Player), "выставить нечего — партия окончена");
    assert_eq!(st.round, 1, "и окончена сразу, а не по времени");
    assert!(events.contains(&Event::Finished { outcome: Outcome::Player }));
}

#[test]
fn a_card_that_becomes_affordable_later_keeps_the_match_alive() {
    // Дорогая, но в пределах потолка: сторона ещё вернётся, и обрывать нельзя.
    let setup = Setup {
        player_board: vec![(boec("Боец", 1, 6, 9), cell(1, 3))],
        keeper_board: vec![(boec("Ворон", 1, 3, 1), cell(1, 2))],
        keeper_hand: vec![boec("Тяжёлая", 9, 5, 5)],
        ..Default::default()
    };
    let st = MatchState::begin(setup);
    let (st, _) = act(&st, Action::Attack { attacker: 0, target: 1 });
    assert_eq!(st.outcome, None, "мана дорастёт до девяти — это не поражение");
}

#[test]
fn a_card_with_nowhere_to_stand_is_the_same_as_no_card() {
    // Половина хранителя занята чужими телами целиком, своё — одно, и стоит
    // оно на чужой половине. Снять его — и выставить купленное будет некуда.
    let mut player_board = Vec::new();
    for y in 0..3u8 {
        for x in 0..3u8 {
            player_board.push((boec("Осада", 1, 4, 1), cell(x, y)));
        }
    }
    let setup = Setup {
        player_board,
        keeper_board: vec![(boec("Ворон", 1, 3, 1), cell(1, 3))],
        keeper_hand: vec![boec("Дешёвая", 1, 5, 5)],
        ..Default::default()
    };
    let st = MatchState::begin(setup);
    let raven = st.standing(Side::Keeper)[0];
    // Бьёт сосед сверху: (1,2) и (1,3) — соседние ряды через середину.
    let neighbour = st
        .standing(Side::Player)
        .into_iter()
        .find(|id| st.board.cell_of(*id) == Some(cell(1, 2)))
        .expect("тело на (1,2)");
    let mut st = st;
    // Ворон живуч ровно настолько, чтобы его добили за два удара.
    while st.outcome.is_none() && st.standing(Side::Keeper).len() == 1 {
        let before = st.units[raven as usize].health.current;
        st = act(&st, Action::Attack { attacker: neighbour, target: raven }).0;
        if st.units[raven as usize].health.current == before {
            break;
        }
        if st.standing(Side::Keeper).is_empty() {
            break;
        }
        st = act(&st, Action::EndTurn).0;
        st = act(&st, Action::EndTurn).0;
    }
    assert!(st.standing(Side::Keeper).is_empty(), "ворона сняли");
    assert_eq!(
        st.outcome,
        Some(Outcome::Player),
        "карта в руке есть, а класть её некуда — это поражение, а не отсрочка",
    );
}

#[test]
fn a_match_that_runs_out_of_rounds_is_decided_on_health_left_standing() {
    // Two bodies that cannot reach each other: nothing can ever happen.
    let setup = Setup {
        player_board: vec![(boec("Боец", 1, 9, 3), cell(0, 5))],
        keeper_board: vec![(boec("Ворон", 1, 4, 3), cell(2, 0))],
        ..Default::default()
    };
    let mut st = MatchState::begin(setup);
    while st.outcome.is_none() {
        st = act(&st, Action::EndTurn).0;
    }
    assert_eq!(st.outcome, Some(Outcome::Player), "9 здоровья против 4");
}

#[test]
fn a_finished_match_refuses_everything() {
    let setup = Setup {
        player_board: vec![(boec("Боец", 1, 6, 9), cell(1, 3))],
        keeper_board: vec![(boec("Ворон", 1, 3, 1), cell(1, 2))],
        ..Default::default()
    };
    let st = MatchState::begin(setup);
    let (st, _) = act(&st, Action::Attack { attacker: 0, target: 1 });
    assert_eq!(reduce(&st, &Action::EndTurn).unwrap_err(), Illegal::MatchOver);
    assert!(legal_actions(&st).is_empty());
}

// ── законные действия ───────────────────────────────────────────────────────

#[test]
fn the_legal_list_is_what_the_client_draws_from() {
    let mut setup = face_off();
    setup.player_hand = vec![boec("Стрелок", 1, 4, 2)];
    let st = MatchState::begin(setup);
    let actions = legal_actions(&st);

    // Восемь свободных клеток своей половины, один достижимый удар, пас.
    assert_eq!(actions.iter().filter(|a| matches!(a, Action::Play { .. })).count(), 8);
    assert_eq!(actions.iter().filter(|a| matches!(a, Action::Attack { .. })).count(), 1);
    assert!(actions.contains(&Action::EndTurn));
}

#[test]
fn everything_the_list_offers_is_actually_allowed() {
    let mut setup = face_off();
    setup.player_hand = vec![boec("Стрелок", 1, 4, 2)];
    let st = MatchState::begin(setup);
    for action in legal_actions(&st) {
        assert!(reduce(&st, &action).is_ok(), "предложено, но отклонено: {action:?}");
    }
}

// ── бот и целая партия ──────────────────────────────────────────────────────

/// Play a match out with the bot on both sides. This is also the shape the
/// service will use for the keeper's half of a real turn.
fn play_out(setup: Setup) -> (MatchState, Vec<Event>) {
    let mut st = MatchState::begin(setup);
    let mut journal = Vec::new();
    let mut guard = 0;
    while st.outcome.is_none() {
        let action = bot::choose(&st);
        let (next, events) = reduce(&st, &action).expect("бот выбирает только из законного");
        st = next;
        journal.extend(events);
        guard += 1;
        assert!(guard < 2000, "партия не завершилась — цикл в правилах");
    }
    (st, journal)
}

fn skirmish() -> Setup {
    Setup {
        player_board: vec![
            (boec("Боец", 1, 6, 3), cell(1, 3)),
            (boec("Стрелок", 2, 4, 2).with_reach(3), cell(0, 4)),
        ],
        player_hand: vec![boec("Швея", 2, 5, 2), boec("Пекарь", 3, 7, 3)],
        keeper_board: vec![
            (boec("Ворон", 1, 6, 3), cell(1, 2)),
            (boec("Тень", 2, 4, 2).with_reach(3), cell(2, 1)),
        ],
        keeper_hand: vec![boec("Кот", 2, 5, 2), boec("Котёл", 3, 7, 3)],
    }
}

#[test]
fn a_whole_match_plays_itself_to_an_ending() {
    let (st, journal) = play_out(skirmish());
    assert!(st.outcome.is_some());
    assert!(matches!(journal.last(), Some(Event::Finished { .. })));
    assert!(journal.iter().any(|e| matches!(e, Event::Damaged { .. })));
    assert!(journal.iter().any(|e| matches!(e, Event::Played { .. })));
    assert!(st.round <= battle_core::state::MAX_ROUNDS + 1);
}

#[test]
fn the_same_match_played_twice_gives_the_same_journal() {
    // The property everything else rests on. If this ever fails, something has
    // reached for a clock, a hash map or a random number.
    let (first_state, first_journal) = play_out(skirmish());
    for _ in 0..20 {
        let (state, journal) = play_out(skirmish());
        assert_eq!(journal, first_journal);
        assert_eq!(state, first_state);
    }
}

#[test]
fn reducing_leaves_the_state_it_was_given_untouched() {
    let st = MatchState::begin(skirmish());
    let before = st.clone();
    let _ = reduce(&st, &Action::EndTurn).unwrap();
    let _ = reduce(&st, &Action::Attack { attacker: 0, target: 2 });
    assert_eq!(st, before);
}

#[test]
fn the_bot_takes_the_killing_blow_when_there_is_one() {
    let setup = Setup {
        // Keeper moves second, so end the player's turn first.
        player_board: vec![
            (boec("Раненый", 1, 2, 1), cell(0, 3)),
            (boec("Целый", 1, 9, 1), cell(2, 3)),
        ],
        keeper_board: vec![(boec("Ворон", 1, 6, 3).with_reach(3), cell(1, 2))],
        ..Default::default()
    };
    let st = MatchState::begin(setup);
    let (st, _) = reduce(&st, &Action::EndTurn).unwrap();
    assert_eq!(bot::choose(&st), Action::Attack { attacker: 2, target: 0 });
}

#[test]
fn the_bot_places_towards_the_enemy_and_not_into_a_corner() {
    // Nothing moves in this slice, so a body put in the back rank can never
    // reach anything: the match then runs to the round limit with both sides
    // staring across the table. This test is the guard on that.
    let setup = Setup {
        player_board: vec![(boec("Боец", 1, 6, 3), cell(1, 3))],
        keeper_board: vec![(boec("Ворон", 1, 6, 3), cell(1, 2))],
        keeper_hand: vec![boec("Кот", 1, 5, 2)],
        ..Default::default()
    };
    let st = MatchState::begin(setup);
    let (st, _) = reduce(&st, &Action::EndTurn).unwrap();

    match bot::choose(&st) {
        Action::Play { cell: put, .. } => {
            assert_eq!(put.y, 2, "передний ряд своей половины, а не дальний");
        }
        other => panic!("ожидался выход карты, а не {other:?}"),
    }
}

#[test]
fn a_match_of_bodies_that_can_reach_each_other_ends_well_before_the_limit() {
    let (st, _) = play_out(skirmish());
    assert!(
        st.round < battle_core::state::MAX_ROUNDS,
        "партия упёрлась в лимит кругов — значит, стороны не достают друг до друга"
    );
}

// ── шаг ─────────────────────────────────────────────────────────────────────

#[test]
fn a_body_walks_to_a_free_cell_within_its_step() {
    let st = MatchState::begin(face_off());
    let (st, events) = act(&st, Action::Move { unit: 0, to: cell(0, 4) });
    assert_eq!(st.board.cell_of(0), Some(cell(0, 4)));
    assert!(st.board.is_free(cell(1, 3)), "прежняя клетка освободилась");
    assert_eq!(events[0], Event::Moved { unit: 0, from: cell(1, 3), to: cell(0, 4) });
}

#[test]
fn a_taken_cell_is_not_walked_into() {
    let setup = Setup {
        player_board: vec![
            (boec("Боец", 1, 6, 3), cell(1, 3)),
            (boec("Второй", 1, 6, 3), cell(0, 4)),
        ],
        ..Default::default()
    };
    let st = MatchState::begin(setup);
    assert_eq!(
        reduce(&st, &Action::Move { unit: 0, to: cell(0, 4) }).unwrap_err(),
        Illegal::NoWayThere
    );
}

#[test]
fn a_cell_beyond_the_step_is_not_walked_to() {
    let st = MatchState::begin(face_off());
    assert_eq!(
        reduce(&st, &Action::Move { unit: 0, to: cell(1, 5) }).unwrap_err(),
        Illegal::NoWayThere
    );
}

#[test]
fn a_rank_of_bodies_is_walked_around_and_not_through() {
    // The whole reason movement is a walk and not a distance check. Straight
    // line from (0,5) to (0,3) is two king's steps — but every cell of the row
    // between them is taken, so there is no way there at all.
    let setup = Setup {
        player_board: vec![
            (boec("Ходок", 1, 6, 3).with_step(2), cell(0, 5)),
            (boec("Стена", 1, 6, 1), cell(0, 4)),
            (boec("Стена", 1, 6, 1), cell(1, 4)),
            (boec("Стена", 1, 6, 1), cell(2, 4)),
        ],
        ..Default::default()
    };
    let st = MatchState::begin(setup);

    assert_eq!(cell(0, 5).distance(cell(0, 3)), 2, "по расстоянию — достижимо");
    assert_eq!(
        reduce(&st, &Action::Move { unit: 0, to: cell(0, 3) }).unwrap_err(),
        Illegal::NoWayThere,
        "но пройти негде"
    );
    assert!(!st.board.reachable(cell(0, 5), 2).contains(&cell(0, 3)));
}

#[test]
fn walking_is_what_this_body_does_this_turn() {
    let st = MatchState::begin(face_off());
    let (st, _) = act(&st, Action::Move { unit: 0, to: cell(0, 3) });
    assert_eq!(
        reduce(&st, &Action::Attack { attacker: 0, target: 1 }).unwrap_err(),
        Illegal::AlreadyActed
    );
}

#[test]
fn a_body_that_struck_does_not_also_walk() {
    let st = MatchState::begin(face_off());
    let (st, _) = act(&st, Action::Attack { attacker: 0, target: 1 });
    assert_eq!(
        reduce(&st, &Action::Move { unit: 0, to: cell(0, 4) }).unwrap_err(),
        Illegal::AlreadyActed
    );
}

#[test]
fn a_body_without_a_step_stands_where_it_was_put() {
    let setup = Setup {
        player_board: vec![(boec("Котёл", 1, 7, 3).with_step(0), cell(1, 3))],
        keeper_board: vec![(boec("Ворон", 1, 6, 3), cell(1, 2))],
        ..Default::default()
    };
    let st = MatchState::begin(setup);
    assert!(!legal_actions(&st).iter().any(|a| matches!(a, Action::Move { .. })));
}

#[test]
fn a_body_may_cross_into_the_other_half() {
    let setup = Setup {
        player_board: vec![(boec("Боец", 1, 6, 3), cell(0, 3))],
        keeper_board: vec![(boec("Ворон", 1, 6, 3), cell(2, 0))],
        ..Default::default()
    };
    let st = MatchState::begin(setup);
    let (st, _) = act(&st, Action::Move { unit: 0, to: cell(0, 2) });
    assert_eq!(st.board.cell_of(0).unwrap().side(), Side::Keeper);
}

#[test]
fn the_bot_walks_when_it_cannot_strike() {
    let setup = Setup {
        player_board: vec![(boec("Боец", 1, 6, 3), cell(1, 5))],
        keeper_board: vec![(boec("Ворон", 1, 6, 3), cell(1, 0))],
        ..Default::default()
    };
    let st = MatchState::begin(setup);
    let (st, _) = reduce(&st, &Action::EndTurn).unwrap();
    match bot::choose(&st) {
        Action::Move { unit: 1, to } => {
            assert_eq!(to.y, 1, "шаг навстречу, а не вбок");
        }
        other => panic!("ожидался шаг, а не {other:?}"),
    }
}

#[test]
fn two_lines_that_start_apart_now_meet_instead_of_staring() {
    // Before `move` existed this setup ran the full twelve rounds and was
    // decided on leftover health without a single blow struck.
    let setup = Setup {
        player_board: vec![
            (boec("Боец", 1, 6, 3), cell(0, 5)),
            (boec("Швея", 1, 5, 2), cell(2, 5)),
        ],
        keeper_board: vec![
            (boec("Ворон", 1, 6, 3), cell(0, 0)),
            (boec("Кот", 1, 5, 2), cell(2, 0)),
        ],
        ..Default::default()
    };
    let (st, journal) = play_out(setup);

    assert!(journal.iter().any(|e| matches!(e, Event::Moved { .. })));
    assert!(journal.iter().any(|e| matches!(e, Event::Damaged { .. })), "дошли и подрались");
    assert!(journal.iter().any(|e| matches!(e, Event::Died { .. })));
    assert!(st.round < battle_core::state::MAX_ROUNDS, "решено боем, а не лимитом");
}

#[test]
fn walking_did_not_disturb_the_journal() {
    let setup = Setup {
        player_board: vec![(boec("Боец", 1, 6, 3), cell(0, 5))],
        keeper_board: vec![(boec("Ворон", 1, 6, 3), cell(2, 0))],
        ..Default::default()
    };
    let (first_state, first_journal) = play_out(setup.clone());
    for _ in 0..20 {
        let (state, journal) = play_out(setup.clone());
        assert_eq!(journal, first_journal);
        assert_eq!(state, first_state);
    }
}

// ── лечение ─────────────────────────────────────────────────────────────────

fn mender_setup() -> Setup {
    Setup {
        player_board: vec![
            (boec("Раненый", 1, 8, 3), cell(1, 3)),
            (boec("Швея", 2, 5, 1).with_mend(4), cell(1, 4)),
        ],
        keeper_board: vec![(boec("Ворон", 1, 6, 3), cell(1, 2))],
        ..Default::default()
    }
}

/// Дать «Раненому» рану, чтобы было что лечить: ход хранителя, потом обратно.
fn wounded_state() -> MatchState {
    let st = MatchState::begin(mender_setup());
    let (st, _) = act(&st, Action::EndTurn);
    let (st, _) = act(&st, Action::Attack { attacker: 2, target: 0 });
    let (st, _) = act(&st, Action::EndTurn);
    st
}

#[test]
fn a_mender_puts_back_what_was_taken() {
    let st = wounded_state();
    assert_eq!(st.unit(0).unwrap().health.current, 5);
    let (st, events) = act(&st, Action::Mend { healer: 1, target: 0 });
    assert_eq!(st.unit(0).unwrap().health.current, 8);
    assert_eq!(events, vec![Event::Healed { target: 0, amount: 3 }]);
}

#[test]
fn mending_is_what_this_body_does_this_turn() {
    let st = wounded_state();
    let (st, _) = act(&st, Action::Mend { healer: 1, target: 0 });
    assert_eq!(
        reduce(&st, &Action::Attack { attacker: 1, target: 2 }).unwrap_err(),
        Illegal::AlreadyActed
    );
}

#[test]
fn nobody_mends_the_other_side() {
    let st = wounded_state();
    assert_eq!(
        reduce(&st, &Action::Mend { healer: 1, target: 2 }).unwrap_err(),
        Illegal::TargetIsEnemy
    );
}

#[test]
fn a_mender_tends_others_and_not_itself() {
    let st = wounded_state();
    assert_eq!(
        reduce(&st, &Action::Mend { healer: 1, target: 1 }).unwrap_err(),
        Illegal::TargetIsAlly
    );
}

#[test]
fn mending_an_unwounded_ally_is_refused_rather_than_wasted() {
    let st = MatchState::begin(mender_setup());
    assert_eq!(
        reduce(&st, &Action::Mend { healer: 1, target: 0 }).unwrap_err(),
        Illegal::NothingToMend
    );
}

#[test]
fn a_body_that_does_not_mend_cannot_be_asked_to() {
    let st = wounded_state();
    assert_eq!(
        reduce(&st, &Action::Mend { healer: 0, target: 1 }).unwrap_err(),
        Illegal::DoesNotMend
    );
}

#[test]
fn mending_reaches_only_as_far_as_the_body_does() {
    let setup = Setup {
        player_board: vec![
            (boec("Раненый", 1, 8, 3), cell(0, 3)),
            (boec("Швея", 2, 5, 1).with_mend(4), cell(2, 5)),
        ],
        keeper_board: vec![(boec("Ворон", 1, 6, 3), cell(0, 2))],
        ..Default::default()
    };
    let st = MatchState::begin(setup);
    let (st, _) = act(&st, Action::EndTurn);
    let (st, _) = act(&st, Action::Attack { attacker: 2, target: 0 });
    let (st, _) = act(&st, Action::EndTurn);
    assert_eq!(
        reduce(&st, &Action::Mend { healer: 1, target: 0 }).unwrap_err(),
        Illegal::OutOfReach
    );
}

#[test]
fn the_offered_list_holds_a_mending_only_when_there_is_a_wound_to_mend() {
    let whole = MatchState::begin(mender_setup());
    assert!(!legal_actions(&whole).iter().any(|a| matches!(a, Action::Mend { .. })));

    let hurt = wounded_state();
    assert!(legal_actions(&hurt).iter().any(|a| matches!(a, Action::Mend { .. })));
}

#[test]
fn the_bot_mends_a_deep_wound_before_anything_else_but_killing() {
    let setup = Setup {
        player_board: vec![(boec("Боец", 1, 6, 2), cell(1, 3))],
        keeper_board: vec![
            (boec("Раненый", 1, 8, 3), cell(1, 2)),
            (boec("Кот", 2, 5, 1).with_mend(4), cell(1, 1)),
        ],
        ..Default::default()
    };
    let st = MatchState::begin(setup);
    let (st, _) = act(&st, Action::Attack { attacker: 0, target: 1 });
    let (st, _) = act(&st, Action::EndTurn);
    assert_eq!(bot::choose(&st), Action::Mend { healer: 2, target: 1 });
}

// ── правила открытия ────────────────────────────────────────────────────────

#[test]
fn the_side_moving_first_lands_one_blow_in_the_opening_round() {
    let setup = Setup {
        player_board: vec![
            (boec("Первый", 1, 6, 3), cell(0, 3)),
            (boec("Второй", 1, 6, 3), cell(2, 3)),
        ],
        keeper_board: vec![
            (boec("Ворон", 1, 9, 1), cell(0, 2)),
            (boec("Тень", 1, 9, 1), cell(2, 2)),
        ],
        ..Default::default()
    };
    let st = MatchState::begin(setup);
    let (st, _) = act(&st, Action::Attack { attacker: 0, target: 2 });
    assert_eq!(
        reduce(&st, &Action::Attack { attacker: 1, target: 3 }).unwrap_err(),
        Illegal::HeldAtTheOpening,
        "второй удар в первом круге не проходит"
    );
    // И не предлагается: список законного и правила не должны спорить.
    assert!(!legal_actions(&st).iter().any(|a| matches!(a, Action::Attack { .. })));
}

#[test]
fn from_the_second_round_the_first_side_strikes_freely() {
    let setup = Setup {
        player_board: vec![
            (boec("Первый", 1, 6, 3), cell(0, 3)),
            (boec("Второй", 1, 6, 3), cell(2, 3)),
        ],
        keeper_board: vec![
            (boec("Ворон", 1, 9, 1), cell(0, 2)),
            (boec("Тень", 1, 9, 1), cell(2, 2)),
        ],
        ..Default::default()
    };
    let st = MatchState::begin(setup);
    let (st, _) = act(&st, Action::Attack { attacker: 0, target: 2 });
    let (st, _) = act(&st, Action::EndTurn);
    let (st, _) = act(&st, Action::EndTurn);
    assert_eq!(st.round, 2);
    let (st, _) = act(&st, Action::Attack { attacker: 0, target: 2 });
    assert!(reduce(&st, &Action::Attack { attacker: 1, target: 3 }).is_ok());
}

#[test]
fn the_side_moving_second_is_paid_for_moving_second() {
    let st = MatchState::begin(face_off());
    assert_eq!(st.player.mana, 1, "первый ход — обычная мана");
    let (st, _) = act(&st, Action::EndTurn);
    assert_eq!(st.keeper.mana, 3, "монета в два плюс собственный ход");
}

#[test]
fn the_keeper_is_never_held_at_the_opening() {
    // Держат только того, кто ходит первым. Второму держаться не за что.
    let st = MatchState::begin(face_off());
    let (st, _) = act(&st, Action::EndTurn);
    assert!(!st.holds_at_the_opening());
    assert!(reduce(&st, &Action::Attack { attacker: 1, target: 0 }).is_ok());
}

#[test]
fn a_match_survives_being_written_down_and_read_back() {
    // Ради этого свойства ядро и держали простым: журнал и состояние должны
    // ложиться в базу и подниматься из неё без единой правки в правилах.
    let (state, journal) = play_out(skirmish());

    let written = serde_json::to_string(&journal).expect("журнал пишется");
    let back: Vec<Event> = serde_json::from_str(&written).expect("журнал читается");
    assert_eq!(back, journal);

    let written = serde_json::to_string(&state).expect("состояние пишется");
    let back: MatchState = serde_json::from_str(&written).expect("состояние читается");
    assert_eq!(back, state);
}

#[test]
fn a_recorded_match_replays_to_the_same_ending() {
    // Что делает возможной и регрессию по правилам, и пересмотр партии игроком.
    let mut state = MatchState::begin(skirmish());
    let mut actions = Vec::new();
    while state.outcome.is_none() {
        let action = bot::choose(&state);
        actions.push(action.clone());
        state = reduce(&state, &action).unwrap().0;
    }

    let written = serde_json::to_string(&actions).unwrap();
    let read_back: Vec<Action> = serde_json::from_str(&written).unwrap();

    let replayed = read_back
        .iter()
        .fold(MatchState::begin(skirmish()), |s, a| reduce(&s, a).unwrap().0);
    assert_eq!(replayed, state, "переигрывание журнала даёт ту же партию");
}
