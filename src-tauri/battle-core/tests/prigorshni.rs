//! Пригоршни: сколько тел берёт чара, когда цель уже выбрана.
//!
//! Ось вторая из §4, и батарея отвечает на её единственный трудный вопрос:
//! КОГО ИМЕННО зацепит. Три правила, которые легко записать неверно и нельзя
//! проверить глазом:
//!
//! * пригоршня расширяет ЧИСЛО целей, а не круг тех, кого чара берёт: сглаз,
//!   брошенный в чужого, не перекидывается на своих, сколько бы их ни стояло
//!   рядом;
//! * ПОКРОВ пригоршней достаётся — «нельзя выбрать целью, но по площади
//!   достаёт» (§4), и это ровно то, ради чего покров заведён;
//! * запрещённые §4 сочетания — массовые оцепенение, смута и покров — не
//!   играются вовсе: «✗ — запрещённые, а не дорогие».

use battle_core::*;

fn cell(x: u8, y: u8) -> Cell {
    Cell::new(x, y).unwrap()
}

fn boec(name: &str, health: i32, power: i32) -> CardSnapshot {
    CardSnapshot::new(name, 1, health, power)
}

/// Умение с пригоршней: глагол, число, форма и ширина.
fn spell(id: &str, verb: &str, amount: i32, shape: &str, radius: u8) -> AbilitySnapshot {
    AbilitySnapshot {
        verb: verb.into(),
        shape: shape.into(),
        radius,
        duration: 2,
        ..AbilitySnapshot::harm(id, amount, 5)
    }
}

fn rules() -> Rules {
    Rules { idle_toll: 0, opening_attacks: 255, ..Rules::default() }
}

/// Ведьма внизу, а перед ней ТРИ чужих тела в ряд и одно поодаль. Своих двое:
/// есть на ком проверить, что чужая чара своих не берёт.
fn crowd(ability: AbilitySnapshot) -> MatchState {
    let setup = Setup {
        player_board: vec![
            (boec("Ведьма", 10, 2).with_ability(ability), cell(1, 4)),
            (boec("Отрок", 8, 3), cell(0, 4)),
        ],
        player_hand: vec![],
        keeper_board: vec![
            (boec("Ворон", 9, 4), cell(0, 2)),
            (boec("Тень", 9, 2), cell(1, 2)),
            (boec("Сыч", 9, 2), cell(2, 2)),
            (boec("Волк", 9, 3), cell(1, 0)),
        ],
        keeper_hand: vec![],
        terrain: Vec::new(),
        field: Default::default(),
    };
    MatchState::begin_with(setup, rules())
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

/// Кого зацепило — по событиям, а не по доске: событие и есть то, что увидит
/// комната, и если она его не получила, для неё этого не случилось.
fn hurt(events: &[Event]) -> Vec<UnitId> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Damaged { target, .. } => Some(*target),
            _ => None,
        })
        .collect()
}

// ── Сколько берёт каждая ────────────────────────────────────────────────────

/// Одно тело — по-прежнему одно.
#[test]
fn one_is_still_one() {
    let st = crowd(spell("огонь", "damage", 3, "one", 0));
    let (_, events) = act(&st, cast(0, "огонь", 2));
    assert_eq!(hurt(&events), vec![2]);
}

/// Соседние: цель и всё, что стоит с ней рядом.
#[test]
fn adjacent_takes_the_target_and_what_stands_by_it() {
    let st = crowd(spell("цепь", "damage", 3, "adjacent", 0));
    // Тень (3) стоит между вороном (2) и сычом (4).
    let (_, events) = act(&st, cast(0, "цепь", 3));
    let mut got = hurt(&events);
    got.sort();
    assert_eq!(got, vec![2, 3, 4], "трое в ряд");
    assert_eq!(hurt(&events)[0], 3, "первым — тот, в кого ткнули");
}

/// Круг мерится шагами короля: радиус 2 достаёт до стоящего через ряд.
#[test]
fn a_circle_is_measured_in_kings_steps() {
    let near = crowd(spell("круг", "damage", 3, "radius", 1));
    let (_, events) = act(&near, cast(0, "круг", 3));
    let mut got = hurt(&events);
    got.sort();
    assert_eq!(got, vec![2, 3, 4], "волк за два ряда не достался");

    let wide = crowd(spell("круг", "damage", 3, "radius", 2));
    let (_, events) = act(&wide, cast(0, "круг", 3));
    let mut got = hurt(&events);
    got.sort();
    assert_eq!(got, vec![2, 3, 4, 5], "а с радиусом в два — достался");
}

/// Насквозь: цель и всё, что стоит ЗА ней, если смотреть от наводящего.
#[test]
fn a_line_carries_through_the_target_and_out_the_far_side() {
    let st = crowd(spell("луч", "damage", 3, "line", 0));
    // Ведьма в (1,4), тень в (1,2), волк в (1,0) — одна колонна.
    let (_, events) = act(&st, cast(0, "луч", 3));
    assert_eq!(hurt(&events), vec![3, 5], "тень, а за ней волк");

    // По ворону слева луч уходит вбок и никого больше не задевает.
    let (_, events) = act(&st, cast(0, "луч", 2));
    assert_eq!(hurt(&events), vec![2]);
}

/// Цепь прыгает от цели к ближайшему — столько раз, сколько звеньев.
#[test]
fn a_chain_jumps_to_the_nearest_and_counts_its_links() {
    let st = crowd(spell("молния", "damage", 3, "chain", 2));
    let (_, events) = act(&st, cast(0, "молния", 2));
    assert_eq!(hurt(&events).len(), 2, "два звена — два тела");
    assert_eq!(hurt(&events)[0], 2, "первым — тот, в кого ткнули");

    let long = crowd(spell("молния", "damage", 3, "chain", 3));
    let (_, events) = act(&long, cast(0, "молния", 2));
    assert_eq!(hurt(&events).len(), 3);
}

/// Сторона — вся сторона, и только она.
#[test]
fn a_side_spell_takes_the_whole_side_and_no_one_else() {
    let st = crowd(spell("кара", "damage", 2, "side", 0));
    let (after, events) = act(&st, cast(0, "кара", 2));
    let mut got = hurt(&events);
    got.sort();
    assert_eq!(got, vec![2, 3, 4, 5], "все четверо чужих");
    assert_eq!(after.units[1].health.current, 8, "свой отрок цел");
    assert_eq!(after.units[0].health.current, 10, "и сама ведьма тоже");
}

// ── Кого пригоршня НЕ берёт ─────────────────────────────────────────────────

/// Чужая чара своих не берёт, как бы близко они ни стояли.
#[test]
fn a_spell_thrown_at_a_foe_never_spills_onto_your_own() {
    let mut setup = Setup {
        player_board: vec![
            (
                boec("Ведьма", 10, 2)
                    .with_ability(spell("круг", "damage", 3, "radius", 2)),
                cell(1, 4),
            ),
            // Отрок стоит ВПЛОТНУЮ к ворону — в самой середине круга.
            (boec("Отрок", 8, 3), cell(0, 3)),
        ],
        player_hand: vec![],
        keeper_board: vec![(boec("Ворон", 9, 4), cell(0, 2))],
        keeper_hand: vec![],
        terrain: Vec::new(),
        field: Default::default(),
    };
    setup.keeper_board.push((boec("Тень", 9, 2), cell(1, 2)));
    let st = MatchState::begin_with(setup, rules());

    let (after, events) = act(&st, cast(0, "круг", 2));
    let mut got = hurt(&events);
    got.sort();
    assert_eq!(got, vec![2, 3], "только чужие");
    assert_eq!(after.units[1].health.current, 8, "отрок в кругу цел");
}

/// Своя чара своих и берёт: тот же круг, но благословением.
#[test]
fn a_spell_thrown_at_your_own_gathers_your_own() {
    let mut st = crowd(AbilitySnapshot {
        stat: Some(Stat::Armor),
        ..spell("знамя", "bless", 2, "radius", 2)
    });
    st.units[1].health.current = 8;

    let (blessed, events) = act(&st, cast(0, "знамя", 1));
    let laid: Vec<UnitId> = events
        .iter()
        .filter_map(|e| match e {
            Event::Rider { target, .. } => Some(*target),
            _ => None,
        })
        .collect();
    let mut got = laid.clone();
    got.sort();
    assert_eq!(got, vec![0, 1], "ведьма и отрок");
    assert!(blessed.units[2].statuses.is_empty(), "ворону ничего не досталось");
}

/// Покров пригоршнёй ДОСТАЁТСЯ — ровно то, ради чего он и заведён.
#[test]
fn a_veiled_body_is_reached_by_a_circle_though_it_cannot_be_chosen() {
    let mut setup = Setup {
        player_board: vec![(
            boec("Ведьма", 10, 2).with_ability(spell("круг", "damage", 3, "radius", 1)),
            cell(1, 4),
        )],
        player_hand: vec![],
        keeper_board: vec![(boec("Ворон", 9, 4), cell(0, 2)), (boec("Тень", 9, 2), cell(1, 2))],
        keeper_hand: vec![],
        terrain: Vec::new(),
        field: Default::default(),
    };
    setup.keeper_board[0]
        .0
        .abilities
        .push(AbilitySnapshot { shape: "self".into(), ..spell("покров", "veil", 1, "self", 0) });
    let st = MatchState::begin_with(setup, rules());

    // Ворон укрывается сам.
    let (theirs, _) = act(&st, Action::EndTurn);
    let (veiled, _) = act(&theirs, cast(1, "покров", 1));
    let mine = act(&veiled, Action::EndTurn).0;

    assert!(
        !legal_actions(&mine)
            .iter()
            .any(|a| matches!(a, Action::Cast { target: Mark::Unit(1), .. })),
        "скрытого нельзя ВЫБРАТЬ"
    );
    // А круг, брошенный в его соседа, его достаёт.
    let (_, events) = act(&mine, cast(0, "круг", 2));
    let mut got = hurt(&events);
    got.sort();
    assert_eq!(got, vec![1, 2], "по площади достался");
}

// ── Что пригоршня меняет в счёте ────────────────────────────────────────────

/// Задетые кругом чувствуют ПЛЕСК, а не чару: шипы отвечают только тому, в кого
/// целились, — иначе круг по пятерым в шипах убивал бы ведьму об её же чару.
#[test]
fn those_caught_by_the_edge_feel_a_splash_and_thorns_do_not_answer_it() {
    let thorns = AbilitySnapshot { shape: "self".into(), ..spell("шипы", "thorns", 3, "self", 0) };
    let setup = Setup {
        player_board: vec![(
            boec("Ведьма", 10, 2).with_ability(spell("круг", "damage", 3, "radius", 1)),
            cell(1, 4),
        )],
        player_hand: vec![],
        keeper_board: vec![
            (boec("Ворон", 9, 4).with_ability(thorns.clone()), cell(0, 2)),
            (boec("Тень", 9, 2).with_ability(thorns), cell(1, 2)),
        ],
        keeper_hand: vec![],
        terrain: Vec::new(),
        field: Default::default(),
    };
    let st = MatchState::begin_with(setup, rules());

    let (theirs, _) = act(&st, Action::EndTurn);
    let (one, _) = act(&theirs, cast(1, "шипы", 1));
    let (two, _) = act(&one, cast(2, "шипы", 2));
    let mine = act(&two, Action::EndTurn).0;

    let before = mine.units[0].health.current;
    let (after, events) = act(&mine, cast(0, "круг", 1));
    let pricks = events
        .iter()
        .filter(|e| matches!(e, Event::Damaged { source: Source::Thorns, .. }))
        .count();
    assert_eq!(pricks, 1, "отвечает только тот, в кого целились");
    assert_eq!(after.units[0].health.current, before - 3);
    assert!(
        events.iter().any(|e| matches!(e, Event::Damaged { source: Source::Splash, .. })),
        "задетому кругом — плеск"
    );
}

/// Стража принимает НАЦЕЛЕННОЕ, а не всё, что сыплется кругом.
#[test]
fn a_guard_takes_the_aimed_blow_and_not_the_whole_circle() {
    let guard = AbilitySnapshot { shape: "self".into(), ..spell("стража", "guard", 1, "self", 0) };
    let setup = Setup {
        player_board: vec![(
            boec("Ведьма", 10, 2).with_ability(spell("круг", "damage", 3, "radius", 1)),
            cell(1, 4),
        )],
        player_hand: vec![],
        keeper_board: vec![
            (boec("Ворон", 9, 4), cell(0, 2)),
            (boec("Страж", 9, 2).with_ability(guard), cell(1, 2)),
        ],
        keeper_hand: vec![],
        terrain: Vec::new(),
        field: Default::default(),
    };
    let st = MatchState::begin_with(setup, rules());

    let (theirs, _) = act(&st, Action::EndTurn);
    let (up, _) = act(&theirs, cast(2, "стража", 2));
    let mine = act(&up, Action::EndTurn).0;

    let (_, events) = act(&mine, cast(0, "круг", 1));
    let got = hurt(&events);
    assert_eq!(got[0], 2, "нацеленное в ворона принял страж");
    assert!(got.contains(&2), "и он же задет кругом как сосед");
    assert_eq!(got.len(), 2, "двое, а не трое: тел на поле всего два");
}

// ── Запрещённое ─────────────────────────────────────────────────────────────

/// Массовое подчинение и массовое оцепенение не играются вовсе.
#[test]
fn mass_control_and_mass_charm_are_refused_outright() {
    for (verb, shape, radius) in [
        ("charm", "adjacent", 0),
        ("charm", "side", 0),
        ("control", "side", 0),
        ("control", "radius", 2),
        ("veil", "side", 0),
    ] {
        let st = crowd(spell("чара", verb, 1, shape, radius));
        assert!(
            !legal_actions(&st).iter().any(|a| matches!(a, Action::Cast { .. })),
            "{verb} · {shape} обязано быть запрещено"
        );
        assert_eq!(
            reduce(&st, &cast(0, "чара", 2)),
            Err(Illegal::NoSuchAbility),
            "{verb} · {shape}"
        );
    }
}

/// А разрешённое массовое — играется.
#[test]
fn the_mass_shapes_that_are_allowed_do_play() {
    for (verb, shape, radius) in [
        ("control", "adjacent", 0),
        ("control", "radius", 1),
        ("veil", "adjacent", 0),
        ("dot", "side", 0),
        ("curse", "line", 0),
        ("silence", "chain", 2),
    ] {
        let st = crowd(spell("чара", verb, 1, shape, radius));
        assert!(
            legal_actions(&st).iter().any(|a| matches!(a, Action::Cast { .. })),
            "{verb} · {shape} обязано играться"
        );
    }
}

// ── Договор ─────────────────────────────────────────────────────────────────

/// Всё предложенное применимо — на каждой пригоршне каждого глагола.
#[test]
fn everything_offered_can_be_done_for_every_shape() {
    let shapes = [
        ("one", 0u8),
        ("self", 0),
        ("adjacent", 0),
        ("chain", 2),
        ("line", 0),
        ("radius", 1),
        ("side", 0),
        ("cell", 0),
    ];
    let verbs = [
        "damage", "dot", "hot", "shield", "bless", "curse", "control", "silence", "disarm",
        "charm", "veil", "guard", "immune", "thorns", "move", "cleanse", "dispel", "mana",
        "sacrifice", "zone",
    ];
    for (shape, radius) in shapes {
        let mut witch = boec("Ведьма", 20, 2);
        for verb in verbs {
            witch = witch.with_ability(spell(verb, verb, 2, shape, radius));
        }
        let setup = Setup {
            player_board: vec![(witch, cell(1, 4)), (boec("Отрок", 8, 3), cell(0, 4))],
            player_hand: vec![],
            keeper_board: vec![
                (boec("Ворон", 9, 4), cell(0, 2)),
                (boec("Тень", 9, 2), cell(1, 2)),
            ],
            keeper_hand: vec![],
            terrain: Vec::new(),
            field: Default::default(),
        };
        let st = MatchState::begin_with(setup, rules());
        for action in legal_actions(&st) {
            assert!(
                reduce(&st, &action).is_ok(),
                "предложенное обязано быть применимо ({shape}): {action:?}"
            );
        }
    }
}

/// Партия с массовыми чарами переигрывается в ту же доску.
#[test]
fn a_journal_of_mass_spells_folds_back_into_the_same_board() {
    let witch = boec("Ведьма", 12, 2)
        .with_ability(spell("кара", "damage", 2, "side", 0))
        .with_ability(spell("уныние", "curse", 1, "radius", 1))
        .with_ability(spell("молния", "damage", 2, "chain", 3));
    let setup = Setup {
        player_board: vec![(witch, cell(1, 4))],
        player_hand: vec![],
        keeper_board: vec![
            (boec("Ворон", 9, 4), cell(0, 2)),
            (boec("Тень", 9, 2), cell(1, 2)),
            (boec("Сыч", 9, 2), cell(2, 2)),
        ],
        keeper_hand: vec![],
        terrain: Vec::new(),
        field: Default::default(),
    };
    let journal = vec![
        cast(0, "уныние", 2),
        Action::EndTurn,
        Action::EndTurn,
        cast(0, "молния", 1),
        Action::EndTurn,
        Action::EndTurn,
        cast(0, "кара", 3),
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
