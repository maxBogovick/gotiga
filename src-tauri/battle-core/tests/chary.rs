//! Чары: урон, проклятие, благословение, щит.
//!
//! Батарея стоит на одном вопросе: ЧТО значит число, написанное на умении. «На
//! два, на два хода» — это два чего и два когда? Ответ на оба даётся здесь, а
//! не в исходнике, потому что оба уже однажды были выбраны неверно:
//!
//! * проклятие уязвимости считается ПЛЮСОМ, потому что уязвимость прибавляется
//!   к входящему, — и знак, выведенный из слова «вред», проклял бы вверх;
//! * всадник сходит в конце хода НОСИТЕЛЯ, а не в начале, — иначе проклятие на
//!   один ход снималось бы у противника прежде, чем он что-нибудь сделает.
//!
//! И одно правило сверху: предложенное обязано быть применимо. `legal_actions`
//! и `reduce` отвечают на «кого можно взять в цель» порознь, и разойдись они на
//! полслова — клиент получит законное действие, которое сервер отвергнет.

use battle_core::*;

fn cell(x: u8, y: u8) -> Cell {
    Cell::new(x, y).unwrap()
}

fn boec(name: &str, health: i32, power: i32) -> CardSnapshot {
    CardSnapshot::new(name, 1, health, power)
}

/// Ведьма против ворона, через черту поля: дальность чары — две клетки.
fn witchcraft(ability: AbilitySnapshot) -> Setup {
    Setup {
        player_board: vec![(
            boec("Ведьма", 6, 1).with_ability(ability),
            cell(1, 3),
        )],
        player_hand: vec![],
        keeper_board: vec![(boec("Ворон", 8, 4), cell(1, 1))],
        keeper_hand: vec![],
    }
}

/// То же, но ворон стоит вплотную: нужно там, где проверяется его УДАР, а он
/// достаёт на одну клетку.
fn witchcraft_face_to_face(ability: AbilitySnapshot) -> Setup {
    Setup {
        keeper_board: vec![(boec("Ворон", 8, 4), cell(1, 2))],
        ..witchcraft(ability)
    }
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

/// То же, но чара наведена на КЛЕТКУ: опасная клетка, призыв, свой шаг.
fn cast_at(caster: UnitId, ability: &str, cell: Cell) -> Action {
    Action::Cast {
        caster,
        ability: ability.to_string(),
        target: Mark::Spot(cell),
    }
}

fn rider_on(state: &MatchState, unit: UnitId) -> Option<Status> {
    state.units[unit as usize].statuses.first().cloned()
}

// ── что чара вообще такое ───────────────────────────────────────────────────

/// Чара бьёт на свою дальность, а не на дальность тела.
#[test]
fn a_spell_carries_as_far_as_it_says_and_the_body_does_not() {
    let st = MatchState::begin(witchcraft(AbilitySnapshot::harm("ogon", 3, 2)));
    // Тело ведьмы достаёт на одну клетку, ворон стоит за две.
    assert_eq!(st.units[0].reach, 1);
    assert!(
        !legal_actions(&st)
            .iter()
            .any(|a| matches!(a, Action::Attack { .. })),
        "кулаком до ворона не достать"
    );

    let (next, events) = act(&st, cast(0, "ogon", 1));
    let burnt = events
        .iter()
        .find_map(|e| match e {
            Event::Damaged { target, source, .. } => Some((*target, *source)),
            _ => None,
        })
        .expect("чара обязана оставить след урона");
    assert_eq!(burnt, (1, Source::Ability), "урон чарой, а не ударом");
    assert_eq!(next.units[1].health.current, 5);
}

/// Чара — не удар, и сдачей на неё не отвечают.
#[test]
fn a_spell_is_not_a_blow_and_draws_no_answer() {
    let mut setup = witchcraft(AbilitySnapshot::harm("ogon", 3, 2));
    // Ворона придвигаем вплотную: достать он может, значит и сдать мог бы.
    setup.keeper_board = vec![(boec("Ворон", 8, 4), cell(1, 2))];
    let rules = Rules {
        retaliation: true,
        ..Rules::default()
    };
    let st = MatchState::begin_with(setup, rules);

    let (next, _) = act(&st, cast(0, "ogon", 1));
    assert_eq!(
        next.units[0].health.current, 6,
        "ведьма цела: мечом на проклятие с расстояния не отвечают"
    );

    // А на УДАР тот же ворон отвечает — иначе проверка ничего не значит.
    let (after, _) = act(&st, Action::Attack { attacker: 0, target: 1 });
    assert!(after.units[0].health.current < 6, "сдача на удар работает");
}

/// Запрет первого круга держит и дальнобойную чару. Иначе в правиле дыра ровно
/// того размера, ради которого чару и берут.
#[test]
fn the_opening_rule_holds_spells_too() {
    let rules = Rules {
        opening_attacks: 0,
        ..Rules::default()
    };
    let st = MatchState::begin_with(witchcraft(AbilitySnapshot::harm("ogon", 3, 2)), rules);
    assert!(
        !legal_actions(&st).iter().any(|a| matches!(a, Action::Cast { .. })),
        "в первом круге урона нет ни кулаком, ни чарой"
    );
    assert_eq!(
        reduce(&st, &cast(0, "ogon", 1)),
        Err(Illegal::HeldAtTheOpening)
    );
}

/// А проклятие запрет не держит: оно не урон.
#[test]
fn the_opening_rule_does_not_hold_a_curse() {
    let rules = Rules {
        opening_attacks: 0,
        ..Rules::default()
    };
    let spell = AbilitySnapshot::curse("porcha", 2, Stat::Power, 2, 2);
    let st = MatchState::begin_with(witchcraft(spell), rules);
    assert!(
        legal_actions(&st).iter().any(|a| matches!(a, Action::Cast { .. })),
        "проклясть в первом круге можно: это не удар"
    );
}

// ── всадник: знак ───────────────────────────────────────────────────────────

/// Проклятие силы ослабляет удар — тем же конвейером, что уже считал всадников.
#[test]
fn a_curse_of_power_takes_the_number_off_the_blow() {
    let spell = AbilitySnapshot::curse("porcha", 2, Stat::Power, 2, 2);
    let st = MatchState::begin(witchcraft_face_to_face(spell));

    let (cursed, events) = act(&st, cast(0, "porcha", 1));
    let laid = events
        .iter()
        .find_map(|e| match e {
            Event::Rider { status, ill, .. } => Some((status.clone(), *ill)),
            _ => None,
        })
        .expect("всадник обязан сказать о себе событием");
    assert_eq!(laid.0.amount, -2, "на силу вред — это минус");
    assert!(laid.1, "проклятие названо проклятием, а не угадано по знаку");
    assert_eq!(cursed.units[1].printed_power(), 2, "4 − 2");

    // И это не украшение: удар проклятого правда слабее.
    let (ended, _) = act(&cursed, Action::EndTurn);
    let (struck, _) = act(&ended, Action::Attack { attacker: 1, target: 0 });
    assert_eq!(struck.units[0].health.current, 4, "6 − (4 − 2)");
}

/// Уязвимость считается наоборот, и ровно затем это поле и заведено.
#[test]
fn a_curse_of_vulnerability_counts_the_other_way() {
    let spell = AbilitySnapshot::curse("sglaz", 2, Stat::Vulnerable, 2, 2);
    let st = MatchState::begin(witchcraft(spell));
    let (cursed, _) = act(&st, cast(0, "sglaz", 1));
    assert_eq!(
        rider_on(&cursed, 1).unwrap().amount,
        2,
        "уязвимость прибавляется к входящему, поэтому вред — это плюс"
    );

    // Благословение на том же показателе — минус, и это тот же расчёт наоборот.
    let blessing = AbilitySnapshot::curse("obereg", 2, Stat::Vulnerable, 2, 2).with_verb("bless");
    let mut setup = witchcraft(blessing);
    setup.player_board.push((boec("Отрок", 5, 2), cell(0, 3)));
    let st = MatchState::begin(setup);
    let (blessed, _) = act(&st, cast(0, "obereg", 1));
    assert_eq!(rider_on(&blessed, 1).unwrap().amount, -2);
}

/// Одноимённый всадник освежает срок, а не удваивает число. Имя всадника — ключ
/// умения, и два наведения одной чары это один всадник.
#[test]
fn casting_the_same_curse_twice_refreshes_it_instead_of_stacking() {
    let spell = AbilitySnapshot::curse("porcha", 2, Stat::Power, 3, 2);
    let st = MatchState::begin(witchcraft(spell));
    let (once, _) = act(&st, cast(0, "porcha", 1));
    // Через круг — ещё раз тем же.
    let (mine, _) = act(&once, Action::EndTurn);
    let (theirs, _) = act(&mine, Action::EndTurn);
    let (twice, _) = act(&theirs, cast(0, "porcha", 1));
    assert_eq!(twice.units[1].statuses.len(), 1);
    assert_eq!(twice.units[1].printed_power(), 2, "не 0: число то же");
}

// ── всадник: срок ───────────────────────────────────────────────────────────

/// Всадник сходит в конце хода НОСИТЕЛЯ, и проклятие на один ход успевает
/// значить ровно один чужой ход.
#[test]
fn a_rider_of_one_turn_lasts_exactly_one_turn_of_its_bearer() {
    let spell = AbilitySnapshot::curse("porcha", 2, Stat::Power, 1, 2);
    let st = MatchState::begin(witchcraft(spell));
    let (cursed, _) = act(&st, cast(0, "porcha", 1));
    assert_eq!(cursed.units[1].statuses.len(), 1);

    // Свой конец хода чужого всадника не трогает.
    let (theirs, _) = act(&cursed, Action::EndTurn);
    assert_eq!(
        theirs.units[1].statuses.len(),
        1,
        "ворон ходит проклятым — иначе проклятие не значило бы ничего ни разу"
    );
    assert_eq!(theirs.units[1].printed_power(), 2);

    // А свой — снимает.
    let (back, _) = act(&theirs, Action::EndTurn);
    assert!(back.units[1].statuses.is_empty());
    assert_eq!(back.units[1].printed_power(), 4);
}

/// Срок ноль — это один ход, а не всадник, сошедший прежде, чем его заметили.
#[test]
fn a_rider_without_a_term_still_lasts_a_turn() {
    let spell = AbilitySnapshot::curse("porcha", 2, Stat::Power, 0, 2);
    let st = MatchState::begin(witchcraft(spell));
    let (cursed, _) = act(&st, cast(0, "porcha", 1));
    assert_eq!(rider_on(&cursed, 1).unwrap().turns, 1);
}

// ── щит ─────────────────────────────────────────────────────────────────────

/// Щит складывается с уже стоящим и тает, принимая: это запас, а не свойство.
#[test]
fn a_shield_adds_up_and_melts_as_it_takes() {
    let spell = AbilitySnapshot {
        shape: "self".into(),
        ..AbilitySnapshot::harm("zaslon", 3, 0)
    }
    .with_verb("shield");
    let st = MatchState::begin(witchcraft_face_to_face(spell));

    let (held, events) = act(&st, cast(0, "zaslon", 0));
    assert!(events.iter().any(|e| matches!(e, Event::Shielded { amount: 3, .. })));
    assert_eq!(held.units[0].shield, 3);

    let (theirs, _) = act(&held, Action::EndTurn);
    let (struck, _) = act(&theirs, Action::Attack { attacker: 1, target: 0 });
    assert_eq!(struck.units[0].shield, 0, "щит принял три и растаял");
    assert_eq!(
        struck.units[0].health.current, 5,
        "удар был в четыре: щит взял три, одно прошло"
    );
}

// ── на кого наводится ───────────────────────────────────────────────────────

/// Проклятие на своего и благословение на чужого — не выбор, а опечатка.
#[test]
fn a_spell_knows_whose_body_it_is_for() {
    let mut setup = witchcraft(AbilitySnapshot::curse("porcha", 2, Stat::Power, 2, 5));
    setup.player_board.push((boec("Отрок", 5, 2), cell(0, 3)));
    let st = MatchState::begin(setup);
    assert_eq!(reduce(&st, &cast(0, "porcha", 1)), Err(Illegal::TargetIsAlly));
    assert!(reduce(&st, &cast(0, "porcha", 2)).is_ok(), "чужому — можно");

    let blessing = AbilitySnapshot::curse("dar", 2, Stat::Power, 2, 5).with_verb("bless");
    let mut setup = witchcraft(blessing);
    setup.player_board.push((boec("Отрок", 5, 2), cell(0, 3)));
    let st = MatchState::begin(setup);
    assert_eq!(reduce(&st, &cast(0, "dar", 2)), Err(Illegal::TargetIsEnemy));
    assert!(reduce(&st, &cast(0, "dar", 1)).is_ok(), "своему — можно");
}

/// Форма `self` — только о носителе, и ни о ком больше.
#[test]
fn a_spell_shaped_self_is_only_about_its_bearer() {
    let spell = AbilitySnapshot {
        shape: "self".into(),
        ..AbilitySnapshot::curse("dar", 2, Stat::Power, 2, 5)
    }
    .with_verb("bless");
    let mut setup = witchcraft(spell);
    setup.player_board.push((boec("Отрок", 5, 2), cell(0, 3)));
    let st = MatchState::begin(setup);
    assert_eq!(reduce(&st, &cast(0, "dar", 1)), Err(Illegal::NotThatAim));
    assert!(reduce(&st, &cast(0, "dar", 0)).is_ok());
}

// ── мана, откат, дело ───────────────────────────────────────────────────────

/// Чара тратит ману, дело тела и уходит в откат. Всё три — одним действием.
#[test]
fn a_spell_spends_mana_the_turn_and_goes_to_sleep() {
    let spell = AbilitySnapshot::harm("ogon", 3, 2)
        .with_mana(1)
        .with_cooldown(2);
    let st = MatchState::begin(witchcraft(spell));
    assert_eq!(st.player.mana, 1);

    let (after, _) = act(&st, cast(0, "ogon", 1));
    assert_eq!(after.player.mana, 0);
    assert!(after.units[0].acted, "чара — дело тела, как удар и лечение");
    assert_eq!(after.units[0].ability_cd("ogon"), 2);
    assert_eq!(
        reduce(&after, &cast(0, "ogon", 1)),
        Err(Illegal::AlreadyActed)
    );
}

/// Спящая чара не предлагается и не применяется.
#[test]
fn a_sleeping_spell_is_neither_offered_nor_taken() {
    let spell = AbilitySnapshot::harm("ogon", 3, 2).with_cooldown(2);
    let st = MatchState::begin(witchcraft(spell));
    let (spent, _) = act(&st, cast(0, "ogon", 1));
    let (mine, _) = act(&spent, Action::EndTurn);
    let (theirs, _) = act(&mine, Action::EndTurn);

    assert_eq!(theirs.units[0].ability_cd("ogon"), 1);
    assert!(
        !legal_actions(&theirs)
            .iter()
            .any(|a| matches!(a, Action::Cast { .. })),
        "спящая чара в списке не стоит"
    );
    assert_eq!(
        reduce(&theirs, &cast(0, "ogon", 1)),
        Err(Illegal::AbilityAsleep)
    );
}

/// Маны не хватает — и это видно по тому же списку.
#[test]
fn a_spell_beyond_the_mana_is_not_offered() {
    let spell = AbilitySnapshot::harm("ogon", 3, 2).with_mana(5);
    let st = MatchState::begin(witchcraft(spell));
    assert!(
        !legal_actions(&st)
            .iter()
            .any(|a| matches!(a, Action::Cast { .. }))
    );
    assert_eq!(
        reduce(&st, &cast(0, "ogon", 1)),
        Err(Illegal::NotEnoughMana)
    );
}

/// Умения, которого на теле нет, — и сочетания, которое запрещено.
#[test]
fn an_unknown_spell_is_refused_by_one_word() {
    // Круг уже считается: пригоршни движок знает все восемь.
    let krug = AbilitySnapshot {
        shape: "radius".into(),
        radius: 1,
        ..AbilitySnapshot::harm("krug", 3, 2)
    };
    let st = MatchState::begin(witchcraft(krug));
    assert!(reduce(&st, &cast(0, "krug", 1)).is_ok(), "круг — обычная чара");

    // А вот сочетания, запрещённые §4, не играются вовсе: «✗ — запрещённые, а
    // не дорогие». Сторона, которой не дали шевельнуться, не играет, а смотрит.
    let vsem = AbilitySnapshot {
        shape: "side".into(),
        ..AbilitySnapshot::harm("оцепенение", 1, 5)
    }
    .with_verb("control");
    let st = MatchState::begin(witchcraft(vsem));
    assert_eq!(
        reduce(&st, &cast(0, "оцепенение", 1)),
        Err(Illegal::NoSuchAbility),
        "массовое оцепенение — конец партии одной картой"
    );
    assert!(
        !legal_actions(&st).iter().any(|a| matches!(a, Action::Cast { .. })),
        "и в списке законного его нет"
    );

    let st = MatchState::begin(witchcraft(AbilitySnapshot::harm("ogon", 3, 2)));
    assert_eq!(
        reduce(&st, &cast(0, "net-takoy", 1)),
        Err(Illegal::NoSuchAbility)
    );
}

/// Безымянное умение всё равно своё: ключ берётся из места на карте.
#[test]
fn a_nameless_spell_still_has_a_key_of_its_own() {
    let mut spell = AbilitySnapshot::harm("", 3, 2);
    spell.cooldown = 2;
    let st = MatchState::begin(witchcraft(spell));
    assert!(legal_actions(&st).contains(&cast(0, "#0", 1)));
    let (after, _) = act(&st, cast(0, "#0", 1));
    assert_eq!(after.units[0].ability_cd("#0"), 2);
}

// ── договор ─────────────────────────────────────────────────────────────────

/// Всё предложенное применимо. Клиент выбирает из списка и не знает ни одного
/// правила — значит список обязан быть честен до последней строки.
#[test]
fn everything_offered_can_be_done() {
    let witch = boec("Ведьма", 6, 1)
        .with_ability(AbilitySnapshot::harm("ogon", 3, 4))
        .with_ability(AbilitySnapshot::curse("porcha", 2, Stat::Power, 2, 4))
        .with_ability(
            AbilitySnapshot::curse("dar", 1, Stat::Armor, 2, 4).with_verb("bless"),
        )
        .with_ability(
            AbilitySnapshot {
                shape: "self".into(),
                ..AbilitySnapshot::harm("zaslon", 2, 0)
            }
            .with_verb("shield"),
        );
    let setup = Setup {
        player_board: vec![(witch, cell(1, 3)), (boec("Отрок", 5, 2), cell(0, 4))],
        player_hand: vec![],
        keeper_board: vec![(boec("Ворон", 8, 4), cell(1, 1)), (boec("Тень", 6, 2), cell(2, 0))],
        keeper_hand: vec![],
    };
    let st = MatchState::begin(setup);

    let offered = legal_actions(&st);
    let casts: Vec<&Action> = offered
        .iter()
        .filter(|a| matches!(a, Action::Cast { .. }))
        .collect();
    // Огонь и порча — по двум чужим, дар — по одному своему, заслон — на себя.
    assert_eq!(casts.len(), 6, "{casts:?}");
    for action in &offered {
        assert!(
            reduce(&st, action).is_ok(),
            "предложенное обязано быть применимо: {action:?}"
        );
    }
}

/// Ту же партию можно пересчитать по журналу и получить ту же доску — включая
/// всадников и их сроки.
#[test]
fn a_journal_of_spells_folds_back_into_the_same_board() {
    let spell = AbilitySnapshot::curse("porcha", 2, Stat::Power, 3, 2);
    let setup = witchcraft_face_to_face(spell);
    let journal = vec![
        cast(0, "porcha", 1),
        Action::EndTurn,
        Action::EndTurn,
        Action::Attack { attacker: 0, target: 1 },
        Action::EndTurn,
    ];

    let fold = || {
        let mut st = MatchState::begin(setup.clone());
        for action in &journal {
            st = reduce(&st, action).expect("журнал обязан переигрываться").0;
        }
        st
    };
    assert_eq!(fold(), fold());
}

/// Хранитель чарами пользуется. Не украшение: рука, которая не умеет того, что
/// умеет карта, мерит баланс карты неверно.
#[test]
fn the_keeper_reaches_for_a_spell_when_a_spell_is_the_best_thing_to_do() {
    let setup = Setup {
        player_board: vec![(boec("Отрок", 2, 2), cell(1, 3))],
        player_hand: vec![],
        // Дальность чары — две клетки, кулака — одна. Добить можно только чарой.
        keeper_board: vec![(
            boec("Ведьма", 6, 1).with_ability(AbilitySnapshot::harm("ogon", 3, 2)),
            cell(1, 1),
        )],
        keeper_hand: vec![],
    };
    let st = MatchState::begin(setup);
    let (theirs, _) = act(&st, Action::EndTurn);
    assert_eq!(theirs.active, Side::Keeper);
    assert_eq!(bot::choose(&theirs), cast(1, "ogon", 0));
}
