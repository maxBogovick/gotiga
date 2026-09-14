//! Поводы: от чего чара случается, когда её никто не просил.
//!
//! Ось третья из §4 — и единственная, где чару НЕ выбирает человек. Отсюда все
//! трудные места батареи, и все они об одном: кто решает за него.
//!
//! * реакция выбирает цель САМА — того, о ком повод, а если повод ни о ком, то
//!   ближайшего из тех, кого этот глагол берёт;
//! * реакция НЕ вызывает реакций: цепь «ранили — отвечаю — ранил — отвечают»
//!   не имеет дна, и одна карта уронила бы доску;
//! * предсмертный дар идёт с ДОСКИ — тело ещё стоит на ней в этот миг, иначе
//!   чаре неоткуда взяться;
//! * аура держится не сроком, а тем, кто ею дышит: пал — и её нет в тот же миг.

use battle_core::*;

fn cell(x: u8, y: u8) -> Cell {
    Cell::new(x, y).unwrap()
}

fn boec(name: &str, health: i32, power: i32) -> CardSnapshot {
    CardSnapshot::new(name, 1, health, power)
}

/// Умение на поводе: глагол, число, повод, форма.
fn on(id: &str, verb: &str, amount: i32, trigger: &str, shape: &str) -> AbilitySnapshot {
    AbilitySnapshot {
        verb: verb.into(),
        trigger: trigger.into(),
        shape: shape.into(),
        duration: 2,
        radius: 1,
        ..AbilitySnapshot::harm(id, amount, 5)
    }
}

fn rules() -> Rules {
    Rules { idle_toll: 0, opening_attacks: 255, ..Rules::default() }
}

fn act(state: &MatchState, action: Action) -> (MatchState, Vec<Event>) {
    reduce(state, &action).expect("действие должно быть законным")
}

fn cast(caster: UnitId, ability: &str, target: UnitId) -> Action {
    Action::Cast {
        caster,
        ability: ability.to_string(),
        target: Mark::Unit(target),
    }
}

/// Двое своих против двоих чужих, вплотную через черту.
fn face_off(mine: CardSnapshot, theirs: CardSnapshot) -> MatchState {
    let setup = Setup {
        player_board: vec![(mine, cell(1, 3)), (boec("Отрок", 9, 3), cell(0, 3))],
        player_hand: vec![],
        keeper_board: vec![(theirs, cell(1, 2)), (boec("Тень", 9, 2), cell(0, 2))],
        keeper_hand: vec![],
    };
    MatchState::begin_with(setup, rules())
}

fn health(st: &MatchState, id: UnitId) -> i32 {
    st.units[id as usize].health.current
}

// ── Когда я бью ─────────────────────────────────────────────────────────────

/// «Когда бью» срабатывает по тому, кого ударили, — и не тратит ни хода, ни маны.
#[test]
fn on_hit_answers_the_one_who_was_struck() {
    let witch = boec("Ведьма", 9, 3).with_ability(on("яд", "dot", 2, "onHit", "one"));
    let st = face_off(witch, boec("Ворон", 9, 2));

    let (after, events) = act(&st, Action::Attack { attacker: 0, target: 2 });
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Held { target: 2, kind: HoldKind::Festering, .. })),
        "порча легла на того, кого ударили: {events:?}"
    );
    assert!(after.units[3].holds.is_empty(), "и только на него");
    assert_eq!(after.player.mana, st.player.mana, "реакция маны не стоит");
}

/// Она же слушается отката: «раз в два хода» значит раз в два хода, кем бы
/// повод ни был подан.
#[test]
fn a_reaction_keeps_its_cooldown() {
    let mut spell = on("яд", "dot", 2, "onHit", "one");
    spell.cooldown = 3;
    let witch = boec("Ведьма", 9, 3).with_ability(spell);
    let st = face_off(witch, boec("Ворон", 20, 2));

    let (once, _) = act(&st, Action::Attack { attacker: 0, target: 2 });
    assert_eq!(once.units[0].ability_cd("яд"), 3);
    let mine = act(&act(&once, Action::EndTurn).0, Action::EndTurn).0;
    let (twice, events) = act(&mine, Action::Attack { attacker: 0, target: 2 });
    assert!(
        !events.iter().any(|e| matches!(e, Event::Held { .. })),
        "ещё не вернулась"
    );
    assert_eq!(twice.units[0].ability_cd("яд"), 2);
}

// ── Когда меня ранят ────────────────────────────────────────────────────────

/// «Когда меня ранят» отвечает тому, кто ранил.
#[test]
fn on_damaged_answers_the_one_who_struck() {
    let witch = boec("Ведьма", 12, 1).with_ability(on("сглаз", "curse", 2, "onDamaged", "one"));
    let st = face_off(witch, boec("Ворон", 9, 4));

    let (theirs, _) = act(&st, Action::EndTurn);
    let (after, events) = act(&theirs, Action::Attack { attacker: 2, target: 0 });
    assert!(
        events.iter().any(|e| matches!(e, Event::Rider { target: 2, .. })),
        "проклятие легло на ударившего: {events:?}"
    );
    assert_eq!(after.units[2].printed_power(), 2, "4 − 2");
}

/// Реакция НЕ вызывает реакций: два тела, отвечающие друг другу, отвечают по
/// разу, а не до конца партии.
#[test]
fn a_reaction_never_sets_off_another_reaction() {
    let spell = || on("ответ", "damage", 2, "onDamaged", "one");
    let witch = boec("Ведьма", 20, 3).with_ability(spell());
    let raven = boec("Ворон", 20, 2).with_ability(spell());
    let st = face_off(witch, raven);

    let (after, events) = act(&st, Action::Attack { attacker: 0, target: 2 });
    let hits = events
        .iter()
        .filter(|e| matches!(e, Event::Damaged { .. }))
        .count();
    // Удар, ответ ворона — и всё. Ответ на ответ не случается.
    assert_eq!(hits, 2, "{events:?}");
    assert_eq!(health(&after, 0), 18, "ведьме прилетело один раз");
}

// ── Предсмертный ────────────────────────────────────────────────────────────

/// Предсмертный дар срабатывает С ДОСКИ — и по тому, кто свалил.
#[test]
fn a_dying_gift_goes_off_from_the_board_it_still_stands_on() {
    let raven = boec("Ворон", 3, 2).with_ability(on("проклятие", "damage", 4, "onDeath", "one"));
    let st = face_off(boec("Ведьма", 9, 5), raven);

    let (after, events) = act(&st, Action::Attack { attacker: 0, target: 2 });
    let order: Vec<&str> = events
        .iter()
        .map(|e| match e {
            Event::Damaged { .. } => "урон",
            Event::Died { .. } => "гибель",
            _ => "иное",
        })
        .collect();
    assert_eq!(order, vec!["урон", "гибель", "урон"], "дар после гибели");
    assert_eq!(health(&after, 0), 5, "9 − 4: свалившему прилетело");
    assert!(after.board.cell_of(2).is_none(), "и тело всё-таки убрано");
}

/// Дар, который свалил ещё кого-то, второй цепи не запускает.
#[test]
fn a_dying_gift_that_kills_does_not_start_a_second_chain() {
    let first = boec("Первый", 2, 1).with_ability(on("дар", "damage", 9, "onDeath", "one"));
    let second = boec("Второй", 2, 1).with_ability(on("дар2", "damage", 9, "onDeath", "one"));
    let setup = Setup {
        player_board: vec![(boec("Ведьма", 20, 5), cell(1, 3))],
        player_hand: vec![],
        keeper_board: vec![(first, cell(1, 2)), (second, cell(0, 2))],
        keeper_hand: vec![],
    };
    let st = MatchState::begin_with(setup, rules());

    let (after, events) = act(&st, Action::Attack { attacker: 0, target: 1 });
    let deaths = events.iter().filter(|e| matches!(e, Event::Died { .. })).count();
    assert_eq!(deaths, 1, "второй не падал: дар бьёт ведьму, а не его");
    assert_eq!(health(&after, 0), 11, "20 − 9, и только один раз");
}

// ── Когда выхожу и когда начинается мой ход ─────────────────────────────────

/// «Когда выхожу» случается в тот же миг, как тело встало на поле.
#[test]
fn on_play_goes_off_the_moment_the_body_stands() {
    let guest = boec("Гость", 6, 2).with_ability(on("явление", "damage", 3, "onPlay", "one"));
    let mut st = face_off(boec("Ведьма", 9, 2), boec("Ворон", 9, 2));
    st.player.hand = vec![guest];
    st.player.mana = 5;

    let (after, events) = act(&st, Action::Play { hand_index: 0, cell: cell(2, 3) });
    assert!(
        events.iter().any(|e| matches!(e, Event::Damaged { .. })),
        "вышел и ударил: {events:?}"
    );
    // Ближайший из чужих — тень в (0,2)? Нет: ворон в (1,2) ближе к (2,3).
    assert_eq!(health(&after, 2), 6, "9 − 3 у ближайшего");
    assert_eq!(health(&after, 3), 9, "дальний цел");
}

/// «В начале хода» случается у каждого своего тела — и в свой ход, а не в чужой.
#[test]
fn turn_start_goes_off_for_your_own_bodies_on_your_own_turn() {
    // Чужой ОДИН: у повода «в начале хода» цели нет, и реакция берёт
    // ближайшего — а двое на равном расстоянии разрешались бы обходом доски,
    // то есть проверка спрашивала бы не то, что проверяет.
    let witch = boec("Ведьма", 9, 2).with_ability(on("тлен", "damage", 2, "turnStart", "one"));
    let setup = Setup {
        player_board: vec![(witch, cell(1, 3))],
        player_hand: vec![],
        keeper_board: vec![(boec("Ворон", 20, 2), cell(1, 2))],
        keeper_hand: vec![],
    };
    let st = MatchState::begin_with(setup, rules());

    // Свой ход только начался с расстановки — повод ещё не звучал.
    assert_eq!(health(&st, 1), 20);
    let (theirs, events) = act(&st, Action::EndTurn);
    assert!(
        !events.iter().any(|e| matches!(e, Event::Damaged { .. })),
        "в ЧУЖОЙ ход своё тело не тлеет"
    );
    let (mine, events) = act(&theirs, Action::EndTurn);
    assert!(events.iter().any(|e| matches!(e, Event::Damaged { .. })));
    assert_eq!(health(&mine, 1), 18);
}

// ── Однажды ─────────────────────────────────────────────────────────────────

/// «Однажды» просится рукой — и уходит навсегда.
#[test]
fn a_once_spell_is_asked_by_hand_and_never_returns() {
    let witch = boec("Ведьма", 9, 2).with_ability(on("клятва", "damage", 3, "once", "one"));
    let st = face_off(witch, boec("Ворон", 20, 2));

    assert!(
        legal_actions(&st)
            .iter()
            .any(|a| matches!(a, Action::Cast { .. })),
        "«однажды» стоит в веере, как и приказ"
    );
    let (spent, _) = act(&st, cast(0, "клятва", 2));
    assert_eq!(health(&spent, 2), 17);

    // Сколько бы ходов ни прошло — не вернётся.
    let mut st = spent;
    for _ in 0..6 {
        st = act(&st, Action::EndTurn).0;
    }
    assert!(
        !legal_actions(&st)
            .iter()
            .any(|a| matches!(a, Action::Cast { .. })),
        "и через три круга её нет"
    );
    assert_eq!(reduce(&st, &cast(0, "клятва", 2)), Err(Illegal::AbilityAsleep));
}

// ── Аура ────────────────────────────────────────────────────────────────────

/// Аура держится не сроком, а тем, кто ею дышит: пал — и её нет в тот же миг.
#[test]
fn an_aura_holds_by_the_body_that_breathes_it_and_not_by_a_term() {
    let banner = boec("Знамя", 4, 0)
        .without_a_blow()
        .with_ability(on("знамя", "bless", 2, "aura", "radius"));
    let setup = Setup {
        player_board: vec![(banner, cell(1, 3)), (boec("Отрок", 9, 3), cell(0, 3))],
        player_hand: vec![],
        keeper_board: vec![(boec("Ворон", 9, 9), cell(1, 2))],
        keeper_hand: vec![],
    };
    let st = MatchState::begin_with(setup, rules());

    assert_eq!(st.units[1].printed_power(), 5, "3 + 2 от знамени");
    assert!(st.units[1].statuses.is_empty(), "и это не наложенный всадник");
    assert_eq!(st.units[2].printed_power(), 9, "чужому не досталось");

    // Знамя валят — сила возвращается к своей в тот же миг.
    let (theirs, _) = act(&st, Action::EndTurn);
    let (fallen, _) = act(&theirs, Action::Attack { attacker: 2, target: 0 });
    assert!(fallen.units[0].health.is_dead());
    assert_eq!(fallen.units[1].printed_power(), 3, "знамени больше нет");
}

/// Аура ходит вместе с носителем: отошёл — и накрывает уже других.
#[test]
fn an_aura_moves_with_the_body() {
    let banner = boec("Знамя", 9, 2).with_ability(on("уныние", "curse", 2, "aura", "adjacent"));
    let setup = Setup {
        player_board: vec![(banner, cell(2, 3))],
        player_hand: vec![],
        keeper_board: vec![(boec("Ворон", 9, 5), cell(1, 2)), (boec("Тень", 9, 5), cell(0, 0))],
        keeper_hand: vec![],
    };
    let st = MatchState::begin_with(setup, rules());
    assert_eq!(st.units[1].printed_power(), 3, "ворон рядом — 5 − 2");
    assert_eq!(st.units[2].printed_power(), 5, "тень далеко");

    let (away, _) = act(&st, Action::Move { unit: 0, to: cell(2, 4) });
    assert_eq!(away.units[1].printed_power(), 5, "отошли — и отпустило");
}

/// Аурой бывают только благословение и проклятие: остальные глаголы — события,
/// и «призвать, пока стою» не значит ничего.
#[test]
fn only_a_blessing_or_a_curse_can_be_an_aura() {
    let odd = boec("Ведьма", 9, 2).with_ability(on("призыв", "summon", 1, "aura", "one"));
    let st = face_off(odd, boec("Ворон", 9, 2));
    assert!(st.units[2].aura.is_empty());
    assert!(st.units[0].aura.is_empty());
    assert_eq!(st.units[1].printed_power(), 3, "ничего не изменилось");
}

// ── Договор ─────────────────────────────────────────────────────────────────

/// Повод в веер не выходит: его никто не просит, и обещать выбор там, где его
/// нет, — та же ложь, что показать чару, которой не сыграть.
#[test]
fn only_what_is_asked_by_hand_is_offered() {
    let witch = boec("Ведьма", 9, 2)
        .with_ability(on("приказ", "damage", 3, "active", "one"))
        .with_ability(on("клятва", "damage", 3, "once", "one"))
        .with_ability(on("ответ", "damage", 3, "onDamaged", "one"))
        .with_ability(on("дар", "damage", 3, "onDeath", "one"))
        .with_ability(on("знамя", "bless", 2, "aura", "radius"));
    let st = face_off(witch, boec("Ворон", 20, 2));

    let offered: Vec<String> = legal_actions(&st)
        .iter()
        .filter_map(|a| match a {
            Action::Cast { ability, .. } => Some(ability.clone()),
            _ => None,
        })
        .collect();
    assert!(offered.contains(&"приказ".to_string()));
    assert!(offered.contains(&"клятва".to_string()));
    assert!(!offered.contains(&"ответ".to_string()));
    assert!(!offered.contains(&"дар".to_string()));
    assert!(!offered.contains(&"знамя".to_string()));

    for action in legal_actions(&st) {
        assert!(reduce(&st, &action).is_ok(), "{action:?}");
    }
}

/// Партия с поводами переигрывается в ту же доску — вместе с аурами, которые
/// в ней нигде не записаны, а пересчитываются.
#[test]
fn a_journal_of_occasions_folds_back_into_the_same_board() {
    let witch = boec("Ведьма", 14, 3)
        .with_ability(on("яд", "dot", 2, "onHit", "one"))
        .with_ability(on("знамя", "bless", 1, "aura", "radius"));
    let raven = boec("Ворон", 14, 3)
        .with_ability(on("ответ", "curse", 1, "onDamaged", "one"))
        .with_ability(on("дар", "damage", 3, "onDeath", "one"));
    let setup = Setup {
        player_board: vec![(witch, cell(1, 3)), (boec("Отрок", 9, 3), cell(0, 3))],
        player_hand: vec![],
        keeper_board: vec![(raven, cell(1, 2)), (boec("Тень", 9, 2), cell(0, 2))],
        keeper_hand: vec![],
    };
    let journal = vec![
        Action::Attack { attacker: 0, target: 2 },
        Action::EndTurn,
        Action::EndTurn,
        Action::Attack { attacker: 1, target: 3 },
        Action::EndTurn,
        Action::EndTurn,
    ];

    let fold = || {
        let mut st = MatchState::begin_with(setup.clone(), rules());
        for action in &journal {
            st = reduce(&st, action).expect("журнал обязан переигрываться").0;
        }
        st
    };
    assert_eq!(fold(), fold());
}
