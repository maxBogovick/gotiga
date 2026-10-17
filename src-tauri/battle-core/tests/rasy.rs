//! Урон по расам: прибавка бьющего против расы цели и защита цели от расы
//! бьющего. Шаги стоят в конвейере урона, поэтому их видят и удар, и чара,
//! и предпросмотр, и бот — отдельной проверки на каждого не нужно.

use battle_core::*;

fn body(id: UnitId, race: &str, damage: RaceDamage) -> Unit {
    let card = CardSnapshot::new(race, 1, 20, 4).with_race(race, damage);
    Unit::from_card(id, &card, Side::Player)
}

fn plain(id: UnitId, race: &str) -> Unit {
    body(id, race, RaceDamage::default())
}

fn witches() -> RaceDamage {
    RaceDamage { attack: 1, against: vec!["people".into()], ..Default::default() }
}

#[test]
fn a_race_strikes_harder_at_the_races_it_is_set_against() {
    let witch = body(1, "witches", witches());
    let granny = plain(2, "people");
    let gnome = plain(3, "gnomes");

    let on_granny = strike(&witch, &granny);
    assert_eq!(on_granny.to_health, 5);
    assert!(on_granny.trail.iter().any(|b| b.step == StepId::RaceAttack && b.from == 4 && b.to == 5));

    let on_gnome = strike(&witch, &gnome);
    assert_eq!(on_gnome.to_health, 4, "против не отмеченной расы прибавки нет");
    assert!(on_gnome.trail.is_empty());
}

#[test]
fn a_race_takes_less_from_the_races_it_is_guarded_from() {
    let gnome = body(
        1,
        "gnomes",
        RaceDamage { defence: 2, from: vec!["witches".into()], ..Default::default() },
    );
    let witch = body(2, "witches", witches());
    let granny = plain(3, "people");

    let res = strike(&witch, &gnome);
    assert_eq!(res.to_health, 2);
    assert!(res.trail.iter().any(|b| b.step == StepId::RaceDefence && b.from == 4 && b.to == 2));
    assert_eq!(strike(&granny, &gnome).to_health, 4, "от другой расы защиты нет");
}

#[test]
fn race_defence_keeps_the_floor_of_one() {
    let gnome = body(
        1,
        "gnomes",
        RaceDamage { defence: 3, from: vec!["people".into()], ..Default::default() },
    );
    let mut weak = CardSnapshot::new("granny", 1, 20, 1);
    weak.race = Some("people".into());
    let granny = Unit::from_card(2, &weak, Side::Player);
    assert_eq!(strike(&granny, &gnome).to_health, 1);
}

#[test]
fn a_card_without_a_race_is_untouched() {
    let witch = body(1, "witches", witches());
    let nobody = Unit::from_card(2, &CardSnapshot::new("x", 1, 20, 4), Side::Player);
    assert_eq!(strike(&witch, &nobody).to_health, 4);
    assert_eq!(strike(&nobody, &witch).to_health, 4);
}

#[test]
fn a_zone_has_no_race() {
    let granny = plain(1, "people");
    let res = resolve(None, &granny, DamagePacket::new(3, Channel::Physical, Source::Ability));
    assert_eq!(res.to_health, 3);
}

#[test]
fn an_old_snapshot_without_races_still_reads() {
    let raw = r#"{"name":"x","cost":1,"health":5,"power":2,"armor":0,"ward":0,
                 "reach":1,"step":1,"mend":0,"channel":"physical"}"#;
    let card: CardSnapshot = serde_json::from_str(raw).expect("старый снимок читается");
    assert!(card.race.is_none());
    assert!(card.race_damage.is_empty());
}

// ── Через партию целиком ────────────────────────────────────────────────────

fn duel(player: RaceDamage, keeper: RaceDamage) -> MatchState {
    duel_by(player, keeper, Rules::default())
}

fn duel_by(player: RaceDamage, keeper: RaceDamage, rules: Rules) -> MatchState {
    let setup = Setup {
        player_board: vec![(
            CardSnapshot::new("Ведьма", 1, 9, 3).with_race("witches", player),
            Cell::new(1, 3).unwrap(),
        )],
        keeper_board: vec![(
            CardSnapshot::new("Бабушка", 1, 9, 3).with_race("people", keeper),
            Cell::new(1, 2).unwrap(),
        )],
        ..Default::default()
    };
    MatchState::begin_with(setup, rules)
}

fn damaged(events: &[Event], target: UnitId) -> Option<(i32, Vec<StepId>)> {
    events.iter().find_map(|e| match e {
        Event::Damaged { target: t, to_health, trail, .. } if *t == target => {
            Some((*to_health, trail.iter().map(|b| b.step).collect()))
        }
        _ => None,
    })
}

/// Удар через `reduce` — тот же путь, что у игрока, предпросмотра хода и бота.
#[test]
fn a_blow_in_a_match_carries_the_race_bonus() {
    let st = duel(
        RaceDamage { attack: 1, against: vec!["people".into()], ..Default::default() },
        RaceDamage::default(),
    );
    let (_, events) = reduce(&st, &Action::Attack { attacker: 0, target: 1 }).unwrap();
    let (hurt, steps) = damaged(&events, 1).expect("удар дошёл");
    assert_eq!(hurt, 4);
    assert!(steps.contains(&StepId::RaceAttack));
}

/// Ответный удар считается тем же конвейером: защита ведьмы от людей и
/// прибавка людей против ведьм в нём видны.
#[test]
fn the_answer_blow_reads_the_table_too() {
    let st = duel_by(
        RaceDamage { defence: 1, from: vec!["people".into()], ..Default::default() },
        RaceDamage { attack: 1, against: vec!["witches".into()], ..Default::default() },
        Rules { retaliation: true, ..Rules::default() },
    );
    let (_, events) = reduce(&st, &Action::Attack { attacker: 0, target: 1 }).unwrap();
    let (hurt, steps) = damaged(&events, 0).expect("ответный удар был");
    // 3 силы + 1 против ведьм − 1 защиты ведьмы от людей.
    assert_eq!(hurt, 3);
    assert!(steps.contains(&StepId::RaceAttack) && steps.contains(&StepId::RaceDefence));
}
