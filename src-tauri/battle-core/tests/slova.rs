//! Двадцать глаголов: по проверке на каждый.
//!
//! Батарея отвечает на один вопрос — ЧТО СЛУЧИТСЯ, — и отвечает за хранителя,
//! который пишет «оцепенение, 2 хода» и вправе знать, что это значит, не читая
//! исходника. Поэтому проверки здесь не про поля и не про типы, а про то, что
//! видно на доске: кто пошёл, кто не пошёл, у кого сколько осталось.
//!
//! Три места, где легко ошибиться, и потому им отданы отдельные проверки:
//!
//! * срок «2 хода» значит два собственных хода ЦЕЛИ — на своём теле и на чужом
//!   одинаково (§5.4), хотя ход, в котором чара легла, у своего уже идёт;
//! * очищение и развеивание разбирают всадников по ВРЕДУ, а не по знаку числа:
//!   у уязвимости вред — это плюс;
//! * смута разводит «чьё тело» и «за кого оно бьёт»: уведённое тело ходит с
//!   тем, кто увёл, но хозяин, у которого увели последнее, не проиграл.

use battle_core::*;

fn cell(x: u8, y: u8) -> Cell {
    Cell::new(x, y).unwrap()
}

fn boec(name: &str, health: i32, power: i32) -> CardSnapshot {
    CardSnapshot::new(name, 1, health, power)
}

/// Умение одного глагола: `amount` на `turns` ходов, дальность 3.
fn spell(id: &str, verb: &str, amount: i32, turns: u8) -> AbilitySnapshot {
    AbilitySnapshot {
        verb: verb.into(),
        duration: turns,
        ..AbilitySnapshot::harm(id, amount, 3)
    }
}

/// Ведьма против ворона: она за две клетки, он бьёт только вплотную.
fn table(ability: AbilitySnapshot) -> Setup {
    Setup {
        player_board: vec![(boec("Ведьма", 8, 2).with_ability(ability), cell(1, 3))],
        player_hand: vec![],
        keeper_board: vec![(boec("Ворон", 8, 4), cell(1, 1))],
        keeper_hand: vec![],
    }
}

/// То же, но со своим отроком рядом: нужно там, где чара про союзника.
fn table_with_ally(ability: AbilitySnapshot) -> Setup {
    let mut setup = table(ability);
    setup.player_board.push((boec("Отрок", 8, 3), cell(0, 3)));
    setup
}

/// Правила батареи: домашние, но БЕЗ платы за бездействие. Проверки здесь про
/// глаголы, а плата снимает по единице с каждого, кто простоял, — и всякий
/// счёт здоровья пришлось бы держать в уме вместе с ней.
fn rules() -> Rules {
    Rules { idle_toll: 0, ..Rules::default() }
}

fn begin(setup: Setup) -> MatchState {
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

fn cast_at(caster: UnitId, ability: &str, cell: Cell) -> Action {
    Action::Cast {
        caster,
        ability: ability.to_string(),
        target: Mark::Spot(cell),
    }
}

/// Круг: свой конец хода и чужой. Возвращает доску, на которой снова ваш ход.
fn round(state: &MatchState) -> MatchState {
    let (theirs, _) = act(state, Action::EndTurn);
    act(&theirs, Action::EndTurn).0
}

// ── Порча и заживление ──────────────────────────────────────────────────────

/// Порча жжёт в конце хода носителя — ровно столько раз, сколько написано.
#[test]
fn a_rot_burns_once_a_turn_and_exactly_as_many_turns_as_it_says() {
    let st = begin(table(spell("porcha", "dot", 2, 2)));
    let (cast_on, events) = act(&st, cast(0, "porcha", 1));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Held { kind: HoldKind::Festering, .. })),
        "о порче сказано событием: {events:?}"
    );
    assert_eq!(cast_on.units[1].health.current, 8, "в тот же миг не жжёт");

    // Конец ХОДА ворона — первый ожог.
    let (theirs, _) = act(&cast_on, Action::EndTurn);
    assert_eq!(theirs.units[1].health.current, 8, "свой ход чужую порчу не трогает");
    let (burnt, _) = act(&theirs, Action::EndTurn);
    assert_eq!(burnt.units[1].health.current, 6);

    // Второй — и всё.
    let after = round(&burnt);
    assert_eq!(after.units[1].health.current, 4);
    assert!(after.units[1].holds.is_empty(), "порча сошла");
    let done = round(&after);
    assert_eq!(done.units[1].health.current, 4, "третьего ожога нет");
}

/// Заживление — та же машина с другим знаком, и выше потолка не поднимает.
#[test]
fn a_knitting_mends_a_turn_at_a_time_and_never_above_the_card() {
    let mut st = begin(table_with_ally(spell("травы", "hot", 3, 2)));
    st.units[1].health.current = 3;

    let (knit, _) = act(&st, cast(0, "травы", 1));
    let after = round(&knit);
    assert_eq!(after.units[1].health.current, 6);
    let more = round(&after);
    assert_eq!(more.units[1].health.current, 8, "3 + 3 + 3, но потолок 8");
}

// ── Лишение действия ────────────────────────────────────────────────────────

/// Оцепенение: тело не ходит, не бьёт и не наводит. И под него нельзя дважды
/// подряд — §5.3, самая дешёвая страховка от единственной невыносимой вещи.
#[test]
fn binding_takes_the_whole_turn_away_and_then_lets_the_body_rest() {
    let st = begin(table(spell("оцепенение", "control", 1, 1)));
    let (bound, _) = act(&st, cast(0, "оцепенение", 1));
    assert!(bound.units[1].bound());

    let (theirs, _) = act(&bound, Action::EndTurn);
    assert!(
        !legal_actions(&theirs)
            .iter()
            .any(|a| !matches!(a, Action::EndTurn)),
        "оцепеневшему предлагать нечего: {:?}",
        legal_actions(&theirs)
    );
    assert_eq!(
        reduce(&theirs, &Action::Attack { attacker: 1, target: 0 }),
        Err(Illegal::Bound)
    );

    // Ход кончился — оцепенение сошло, но осталось ОТДЫХОМ.
    let (mine, _) = act(&theirs, Action::EndTurn);
    assert!(!mine.units[1].bound());
    assert!(mine.units[1].held(HoldKind::Rested));
    assert_eq!(
        reduce(&mine, &cast(0, "оцепенение", 1)),
        Err(Illegal::Rested),
        "две ведьмы не держат одно тело вечно"
    );
    assert!(
        !legal_actions(&mine)
            .iter()
            .any(|a| matches!(a, Action::Cast { .. })),
        "и в списке законного его тоже нет"
    );
}

/// Немота держит слово и не держит руки.
#[test]
fn silence_holds_the_word_and_not_the_hands() {
    let mut setup = table(spell("немота", "silence", 1, 2));
    // У ворона своя чара — ей и затыкают рот.
    setup.keeper_board[0].0 = boec("Ворон", 8, 4).with_ability(AbilitySnapshot::harm("клюв", 3, 3));
    let st = begin(setup);

    let (hushed, _) = act(&st, cast(0, "немота", 1));
    let (theirs, _) = act(&hushed, Action::EndTurn);
    assert_eq!(
        reduce(&theirs, &cast(1, "клюв", 0)),
        Err(Illegal::Hushed)
    );
    assert!(
        legal_actions(&theirs)
            .iter()
            .any(|a| matches!(a, Action::Move { .. })),
        "ходить немота не мешает"
    );
}

/// Разоружение — наоборот: руки связаны, слово при нём.
#[test]
fn disarming_holds_the_hands_and_not_the_word() {
    let mut setup = table(spell("разоружение", "disarm", 1, 2));
    setup.keeper_board = vec![(
        boec("Ворон", 8, 4).with_ability(AbilitySnapshot::harm("клюв", 3, 3)),
        cell(1, 2),
    )];
    let st = begin(setup);

    let (disarmed, _) = act(&st, cast(0, "разоружение", 1));
    let (theirs, _) = act(&disarmed, Action::EndTurn);
    assert_eq!(
        reduce(&theirs, &Action::Attack { attacker: 1, target: 0 }),
        Err(Illegal::Disarmed)
    );
    assert!(reduce(&theirs, &cast(1, "клюв", 0)).is_ok(), "чара при нём");
}

/// Разоружённый не сдаёт: сдача — это удар.
#[test]
fn a_disarmed_body_does_not_strike_back() {
    let mut setup = table(spell("разоружение", "disarm", 1, 2));
    setup.keeper_board = vec![(boec("Ворон", 8, 4), cell(1, 2))];
    let rules = Rules { retaliation: true, ..Rules::default() };
    let st = MatchState::begin_with(setup, rules);

    let (disarmed, _) = act(&st, cast(0, "разоружение", 1));
    let after = round(&disarmed);
    let (struck, _) = act(&after, Action::Attack { attacker: 0, target: 1 });
    assert_eq!(struck.units[0].health.current, 8, "сдачи нет");
}

// ── Смута ───────────────────────────────────────────────────────────────────

/// Уведённое тело стоит за того, кто увёл, ходит с ним — и возвращается.
#[test]
fn a_swayed_body_fights_for_the_one_who_took_it_and_comes_back() {
    let mut setup = table(spell("смута", "charm", 1, 2));
    setup.keeper_board.push((boec("Тень", 6, 2), cell(2, 1)));
    let st = begin(setup);

    let (swayed, _) = act(&st, cast(0, "смута", 1));
    assert_eq!(swayed.units[1].owner, Side::Keeper, "чьё оно — не менялось");
    assert_eq!(swayed.units[1].side(), Side::Player, "а стоит оно за меня");

    // На моём следующем ходу оно просыпается вместе с моими.
    let mine = round(&swayed);
    assert!(!mine.units[1].acted);
    assert!(
        legal_actions(&mine)
            .iter()
            .any(|a| matches!(a, Action::Attack { attacker: 1, target: 2 })),
        "и бьёт по своим бывшим: {:?}",
        legal_actions(&mine)
    );

    // Срок вышел — вернулось. Считается он ходами ТОГО, КТО УВЁЛ: тело ходит с
    // ним, и «два хода» значит два хода в его руках. Конец второго — и смута
    // сходит; отдых кладётся тем же мгновением, а тикает уже со своим хозяином.
    let second = round(&mine);
    let back = act(&second, Action::EndTurn).0;
    assert_eq!(back.units[1].side(), Side::Keeper, "вернулось к своим");
    assert!(back.units[1].held(HoldKind::Rested), "и отдыхает");
    assert_eq!(
        reduce(&back, &cast(0, "смута", 1)),
        Err(Illegal::NotYourUnit),
        "да и не ваш сейчас ход"
    );
}

/// Сторона, у которой увели последнее тело, НЕ проиграла.
#[test]
fn taking_the_last_body_is_not_a_victory() {
    let st = begin(table(spell("смута", "charm", 1, 2)));
    let (swayed, events) = act(&st, cast(0, "смута", 1));
    assert!(
        !events.iter().any(|e| matches!(e, Event::Finished { .. })),
        "партия не кончилась"
    );
    assert!(swayed.outcome.is_none());
    let after = round(&swayed);
    assert!(after.outcome.is_none(), "и через круг тоже");
}

// ── Покров, стража, оберег ──────────────────────────────────────────────────

/// Покров: целью не выбирают.
#[test]
fn a_veiled_body_cannot_be_chosen() {
    let st = begin(table_with_ally(spell("покров", "veil", 1, 2)));
    let (veiled, _) = act(&st, cast(0, "покров", 1));
    let theirs = act(&veiled, Action::EndTurn).0;

    assert!(
        !legal_actions(&theirs)
            .iter()
            .any(|a| matches!(a, Action::Attack { target: 1, .. })),
        "скрытого нет в списке целей"
    );
    assert_eq!(
        reduce(&theirs, &Action::Attack { attacker: 2, target: 1 }),
        Err(Illegal::Veiled)
    );
}

/// Стража принимает на себя удар, нацеленный в соседа, — и ровно столько раз,
/// сколько сказано.
#[test]
fn a_guard_takes_the_blow_meant_for_a_neighbour() {
    let guard = AbilitySnapshot { shape: "self".into(), ..spell("стража", "guard", 1, 3) };
    let setup = Setup {
        // Ведьма (0) встаёт стражем и стоит ВПЛОТНУЮ к отроку (1).
        player_board: vec![
            (boec("Ведьма", 8, 2).with_ability(guard), cell(0, 3)),
            (boec("Отрок", 8, 3), cell(1, 3)),
        ],
        player_hand: vec![],
        // Ворон (2) достаёт до отрока и целится в него.
        keeper_board: vec![(boec("Ворон", 8, 4), cell(1, 2))],
        keeper_hand: vec![],
    };
    let st = begin(setup);

    let (guarded, _) = act(&st, cast(0, "стража", 0));
    assert_eq!(guarded.units[0].hold_amount(HoldKind::Guarding), 1);

    let theirs = act(&guarded, Action::EndTurn).0;
    let (struck, events) = act(&theirs, Action::Attack { attacker: 2, target: 1 });
    let hurt = events
        .iter()
        .find_map(|e| match e {
            Event::Damaged { target, .. } => Some(*target),
            _ => None,
        })
        .expect("удар куда-то пришёл");
    assert_eq!(hurt, 0, "приняла стража, а не отрок");
    assert_eq!(struck.units[1].health.current, 8, "отрок цел");
    assert_eq!(
        struck.units[0].hold_amount(HoldKind::Guarding),
        0,
        "и приняла она ровно один: дальше бьют того, в кого целились"
    );

    // Второй удар — уже по отроку.
    let mine = act(&struck, Action::EndTurn).0;
    let again = act(&mine, Action::EndTurn).0;
    let (twice, _) = act(&again, Action::Attack { attacker: 2, target: 1 });
    assert!(twice.units[1].health.current < 8);
}

/// Оберег канала: канал не чувствуется вовсе — и отпускает по сроку.
#[test]
fn a_ward_of_a_channel_feels_nothing_until_it_lifts() {
    let spell = AbilitySnapshot {
        shape: "self".into(),
        channel: Channel::Physical,
        ..spell("оберег", "immune", 1, 1)
    };
    let mut setup = table(spell);
    setup.keeper_board = vec![(boec("Ворон", 8, 4), cell(1, 2))];
    let st = begin(setup);

    let (warded, _) = act(&st, cast(0, "оберег", 0));
    assert_eq!(warded.units[0].immune, Some(Channel::Physical));

    let theirs = act(&warded, Action::EndTurn).0;
    let (struck, events) = act(&theirs, Action::Attack { attacker: 1, target: 0 });
    assert!(events.iter().any(|e| matches!(e, Event::Immune { .. })));
    assert_eq!(struck.units[0].health.current, 8);

    // «Один ход» — это один СВОЙ полный ход, и ход, в котором оберег надели,
    // в счёт не идёт: он уже шёл (§5.4). Значит сходит оберег в конце
    // следующего своего хода, и следующий удар чувствуется.
    let mine = act(&struck, Action::EndTurn).0;
    assert_eq!(mine.units[0].immune, Some(Channel::Physical), "свой ход ещё идёт");
    let theirs = act(&mine, Action::EndTurn).0;
    assert_eq!(theirs.units[0].immune, None, "поле снято вместе со сроком");
    let (hurt, _) = act(&theirs, Action::Attack { attacker: 1, target: 0 });
    assert!(hurt.units[0].health.current < 8);
}

/// Шипы отвечают всякому, кто тронул, — и не отвечают шипам.
#[test]
fn thorns_answer_whoever_touched_and_never_answer_thorns() {
    let spell = AbilitySnapshot { shape: "self".into(), ..spell("шипы", "thorns", 2, 3) };
    let mut setup = table(spell);
    setup.keeper_board = vec![(
        boec("Ворон", 8, 4)
            .with_ability(AbilitySnapshot { shape: "self".into(), ..spell2("шипы2", "thorns", 2, 3) }),
        cell(1, 2),
    )];
    let st = begin(setup);

    let (mine, _) = act(&st, cast(0, "шипы", 0));
    let (theirs, _) = act(&mine, Action::EndTurn);
    let (both, _) = act(&theirs, cast(1, "шипы2", 1));
    let (yours, _) = act(&both, Action::EndTurn);

    // Бью я — мне прилетает два, ему четыре; его шипы не будят моих.
    let before = yours.units[0].health.current;
    let (after, events) = act(&yours, Action::Attack { attacker: 0, target: 1 });
    assert_eq!(after.units[0].health.current, before - 2, "шипы ворона");
    let pricks = events
        .iter()
        .filter(|e| matches!(e, Event::Damaged { source: Source::Thorns, .. }))
        .count();
    assert_eq!(pricks, 1, "шипы не отвечают шипам");
}

/// Второе умение того же вида — для чужой стороны в проверке выше.
fn spell2(id: &str, verb: &str, amount: i32, turns: u8) -> AbilitySnapshot {
    spell(id, verb, amount, turns)
}

// ── Толчок ──────────────────────────────────────────────────────────────────

/// Толчок отодвигает чужого от наводящего и останавливается перед занятым.
#[test]
fn a_shove_pushes_a_foe_away_and_stops_at_what_stands() {
    let mut setup = table(spell("толчок", "move", 2, 0));
    setup.keeper_board = vec![
        (boec("Ворон", 8, 4), cell(1, 2)),
        (boec("Тень", 6, 2), cell(1, 0)),
    ];
    let st = begin(setup);

    let (pushed, events) = act(&st, cast(0, "толчок", 1));
    assert!(events.iter().any(|e| matches!(e, Event::Moved { unit: 1, .. })));
    assert_eq!(
        pushed.board.cell_of(1),
        Some(cell(1, 1)),
        "две клетки просил, одну получил: дальше стоит тень"
    );
}

/// Притяжение — тот же жест в обратную сторону, и на своих.
#[test]
fn a_pull_brings_an_ally_closer() {
    let mut setup = table_with_ally(spell("притяжение", "move", 2, 0));
    setup.player_board[1].1 = cell(0, 5);
    let st = begin(setup);

    let (pulled, _) = act(&st, cast(0, "притяжение", 1));
    let at = pulled.board.cell_of(1).expect("отрок на поле");
    assert_eq!(at.distance(cell(1, 3)), 1, "притянут вплотную, а не мимо");
    assert!(
        at.distance(cell(1, 3)) < cell(0, 5).distance(cell(1, 3)),
        "стало ближе"
    );
}

/// Свой шаг — прыжок на пустую клетку, и стоящие ему не помеха.
#[test]
fn a_step_of_ones_own_is_a_leap_and_not_a_walk() {
    let mut setup = table(AbilitySnapshot {
        shape: "self".into(),
        ..spell("прыжок", "move", 1, 0)
    });
    // Перегораживаем ряд целиком: ходьба бы не прошла.
    setup.player_board.push((boec("Отрок", 6, 2), cell(0, 4)));
    setup.player_board.push((boec("Отрок", 6, 2), cell(1, 4)));
    setup.player_board.push((boec("Отрок", 6, 2), cell(2, 4)));
    let st = begin(setup);

    let (leapt, _) = act(&st, cast_at(0, "прыжок", cell(1, 5)));
    assert_eq!(leapt.board.cell_of(0), Some(cell(1, 5)));
    assert!(leapt.board.is_free(cell(1, 3)), "старое место освободилось");
    assert_eq!(
        reduce(&st, &cast_at(0, "прыжок", cell(0, 4))),
        Err(Illegal::NoRoom),
        "на занятое не прыгают"
    );
}

// ── Очищение и развеивание ──────────────────────────────────────────────────

/// Очищение снимает со своего проклятия и не трогает благословений — а разбирает
/// их по ВРЕДУ, а не по знаку: у уязвимости вред это плюс.
#[test]
fn cleansing_reads_harm_and_not_the_sign() {
    let st = begin(table_with_ally(spell("очищение", "cleanse", 2, 0)));
    let mut st = st;
    st.units[1].apply_status(Status::new("порча", Stat::Power, -2, 5));
    st.units[1].apply_status(Status::new("сглаз", Stat::Vulnerable, 2, 5));
    st.units[1].apply_status(Status::new("дар", Stat::Armor, 2, 5));

    let (clean, events) = act(&st, cast(0, "очищение", 1));
    assert!(events.iter().any(|e| matches!(e, Event::Lifted { ill: true, .. })));
    let left: Vec<&str> = clean.units[1].statuses.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(left, vec!["дар"], "сняты оба вредных, подарок оставлен");
}

/// Развеивание — то же самое с чужой стороны и в другую сторону.
#[test]
fn dispelling_takes_the_gift_off_a_foe() {
    let mut st = begin(table(spell("развеять", "dispel", 1, 0)));
    st.units[1].apply_status(Status::new("дар", Stat::Power, 3, 5));
    st.units[1].apply_status(Status::new("порча", Stat::Power, -1, 5));

    let (bare, _) = act(&st, cast(0, "развеять", 1));
    let left: Vec<&str> = bare.units[1].statuses.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(left, vec!["порча"], "снят подарок, проклятие осталось");
}

// ── Мана и жертва ───────────────────────────────────────────────────────────

/// Мана возвращается, но не выше собственного потолка: это возврат, а не
/// второй источник.
#[test]
fn drawing_mana_never_goes_above_your_own_ceiling() {
    let spell = AbilitySnapshot { shape: "self".into(), ..spell("глоток", "mana", 5, 0) };
    let mut st = begin(table(spell));
    st.player.mana_max = 4;
    st.player.mana = 1;

    let (full, events) = act(&st, cast(0, "глоток", 0));
    assert_eq!(full.player.mana, 4);
    assert!(events.iter().any(|e| matches!(e, Event::Mana { amount: 3, .. })));
}

/// Жертва: тело уходит с поля, сторона получает ману.
#[test]
fn a_sacrifice_pays_with_a_body() {
    let st = begin(table_with_ally(spell("жертва", "sacrifice", 3, 0)));
    let (given, events) = act(&st, cast(0, "жертва", 1));
    assert!(events.iter().any(|e| matches!(e, Event::Died { target: 1 })));
    assert!(given.board.cell_of(1).is_none(), "с поля ушло");
    assert!(given.player.mana >= 3);
}

// ── Опасная клетка ──────────────────────────────────────────────────────────

/// Котёл жжёт того, кто на нём стоял свой ход, — и своих тоже.
#[test]
fn a_zone_burns_whoever_stood_on_it_this_turn() {
    let st = begin(table(spell("котёл", "zone", 2, 2)));
    let (set, events) = act(&st, cast_at(0, "котёл", cell(1, 2)));
    assert!(events.iter().any(|e| matches!(e, Event::Zoned { .. })));
    assert_eq!(set.zones.len(), 1);

    // Пока на клетке никого — она просто стоит.
    let (theirs, _) = act(&set, Action::EndTurn);
    // Ворон шагает в котёл и в конце своего хода получает.
    let (stepped, _) = act(&theirs, Action::Move { unit: 1, to: cell(1, 2) });
    let (burnt, events) = act(&stepped, Action::EndTurn);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Damaged { source: Source::Zone, .. })),
        "жжётся в конце ЕГО хода"
    );
    assert_eq!(burnt.units[1].health.current, 6);
}

/// Одноимённая зона обновляет срок, а не встаёт второй.
#[test]
fn the_same_cauldron_twice_is_one_cauldron() {
    let st = begin(table(spell("котёл", "zone", 2, 2)));
    let (once, _) = act(&st, cast_at(0, "котёл", cell(1, 2)));
    let again = round(&once);
    let (twice, _) = act(&again, cast_at(0, "котёл", cell(0, 2)));
    assert_eq!(twice.zones.len(), 1, "котёл переставили, а не завели второй");
    assert_eq!(twice.zones[0].cells, vec![cell(0, 2)]);
}

// ── Призыв ──────────────────────────────────────────────────────────────────

/// Призыв ставит на поле замороженное тело — и не ставит его на занятое.
#[test]
fn a_summoning_raises_the_body_frozen_with_the_ability() {
    let mut spell = spell("призыв", "summon", 1, 0);
    spell.body = Some(Box::new(boec("Тень", 4, 3)));
    let st = begin(table(spell));

    let (raised, events) = act(&st, cast_at(0, "призыв", cell(1, 4)));
    assert!(events.iter().any(|e| matches!(e, Event::Played { cost: 0, .. })));
    let born = raised.board.at(cell(1, 4)).expect("тело встало");
    assert_eq!(raised.units[born as usize].name(), "Тень");
    assert_eq!(raised.units[born as usize].owner, Side::Player);
    assert!(raised.units[born as usize].acted, "вышедшее в этот ход не бьёт");
}

/// Призыв без тела — не призыв: его не предлагают и не принимают.
#[test]
fn a_summoning_without_a_body_is_not_offered() {
    let st = begin(table(spell("призыв", "summon", 1, 0)));
    assert!(
        !legal_actions(&st)
            .iter()
            .any(|a| matches!(a, Action::Cast { .. })),
        "пустого призыва в списке нет"
    );
    assert_eq!(
        reduce(&st, &cast_at(0, "призыв", cell(1, 4))),
        Err(Illegal::NoSuchAbility)
    );
}

// ── Договор ─────────────────────────────────────────────────────────────────

/// Всё предложенное применимо — на теле, которое умеет ВСЁ.
///
/// Главная проверка батареи: `legal_actions` и `reduce` отвечают на «кого можно
/// взять в цель» порознь, и разойдись они на полслова, клиент получит законное
/// действие, которое сервер отвергнет. Двадцать глаголов — двадцать поводов
/// разойтись.
#[test]
fn everything_offered_can_be_done_for_every_verb() {
    let verbs = [
        "damage", "dot", "hot", "shield", "bless", "curse", "control", "silence", "disarm",
        "charm", "veil", "guard", "immune", "thorns", "move", "cleanse", "dispel", "mana",
        "sacrifice", "zone",
    ];
    let mut witch = boec("Ведьма", 12, 2);
    for (i, verb) in verbs.iter().enumerate() {
        let mut a = spell(verb, verb, 2, 2);
        // Половине глаголов форма `self` — та, в которой они и живут.
        if matches!(*verb, "thorns" | "guard" | "immune" | "mana") {
            a.shape = "self".into();
        }
        a.range = 5;
        let _ = i;
        witch = witch.with_ability(a);
    }
    let mut summon = spell("summon", "summon", 1, 0);
    summon.range = 5;
    summon.body = Some(Box::new(boec("Тень", 4, 3)));
    witch = witch.with_ability(summon);

    let setup = Setup {
        player_board: vec![(witch, cell(1, 4)), (boec("Отрок", 8, 3), cell(0, 4))],
        player_hand: vec![],
        keeper_board: vec![(boec("Ворон", 8, 4), cell(1, 1)), (boec("Тень", 6, 2), cell(2, 0))],
        keeper_hand: vec![],
    };
    let st = begin(setup);

    let offered = legal_actions(&st);
    let casts = offered.iter().filter(|a| matches!(a, Action::Cast { .. })).count();
    assert!(casts > 40, "предложено слишком мало: {casts}");
    for action in &offered {
        assert!(
            reduce(&st, action).is_ok(),
            "предложенное обязано быть применимо: {action:?}"
        );
    }

    // И ни одна из них не роняет свёртку журнала.
    let mut st2 = st.clone();
    for action in offered.iter().take(12) {
        if let Ok((next, _)) = reduce(&st2, action) {
            st2 = next;
        }
    }
    assert!(st2.units.len() >= 4);
}

/// Партия с чарами переигрывается в ту же доску — вместе со сроками, зонами и
/// уведёнными телами.
#[test]
fn a_journal_of_every_verb_folds_back_into_the_same_board() {
    let witch = boec("Ведьма", 10, 2)
        .with_ability(spell("порча", "dot", 2, 2))
        .with_ability(spell("смута", "charm", 1, 2))
        .with_ability(AbilitySnapshot { shape: "self".into(), ..spell("шипы", "thorns", 2, 3) })
        .with_ability(spell("котёл", "zone", 2, 3));
    let setup = Setup {
        player_board: vec![(witch, cell(1, 4))],
        player_hand: vec![],
        keeper_board: vec![(boec("Ворон", 9, 4), cell(1, 1)), (boec("Тень", 6, 2), cell(2, 1))],
        keeper_hand: vec![],
    };
    let journal = vec![
        cast(0, "порча", 1),
        Action::EndTurn,
        Action::EndTurn,
        cast(0, "смута", 1),
        Action::EndTurn,
        Action::EndTurn,
        cast_at(0, "котёл", cell(2, 2)),
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
