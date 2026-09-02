//! Every rule of the first slice, one test each.
//!
//! These are cheap on purpose: no database, no server, no clock. A rule that
//! cannot be checked in microseconds will not be checked often enough.

use battle_core::*;

fn attacker(power: i32) -> Unit {
    Unit::new(1, 20, power)
}

fn target() -> Unit {
    Unit::new(2, 20, 0)
}

fn hit(a: &Unit, t: &Unit, amount: i32, channel: Channel) -> Resolution {
    resolve(Some(a), t, DamagePacket::new(amount, channel, Source::Ability))
}

#[test]
fn armor_is_subtracted_from_bodily_damage() {
    let t = target().with_armor(3);
    assert_eq!(hit(&attacker(0), &t, 8, Channel::Physical).to_health, 5);
}

#[test]
fn armor_does_not_answer_charmed_damage() {
    let t = target().with_armor(3);
    assert_eq!(hit(&attacker(0), &t, 8, Channel::Magic).to_health, 8);
}

#[test]
fn ward_answers_charmed_damage_only() {
    let t = target().with_ward(3);
    assert_eq!(hit(&attacker(0), &t, 8, Channel::Magic).to_health, 5);
    assert_eq!(hit(&attacker(0), &t, 8, Channel::Physical).to_health, 8);
}

#[test]
fn the_evil_eye_ignores_both_defences() {
    let t = target().with_armor(6).with_ward(6);
    assert_eq!(hit(&attacker(0), &t, 7, Channel::Pure).to_health, 7);
}

#[test]
fn a_blow_that_lands_always_takes_at_least_one() {
    let t = target().with_armor(9);
    assert_eq!(hit(&attacker(0), &t, 2, Channel::Physical).to_health, 1);
}

#[test]
fn immunity_is_not_zero_damage_but_no_damage_at_all() {
    let mut t = target();
    t.immune = Some(Channel::Magic);
    let res = hit(&attacker(0), &t, 9, Channel::Magic);
    assert_eq!(res.total(), 0);
    assert_eq!(apply(&mut t, &res), vec![Event::Immune { target: 2, by: Some(1), channel: Channel::Magic }]);
    assert_eq!(t.health.current, 20);
}

#[test]
fn a_shield_catches_the_guaranteed_point_too() {
    // Armour 9 against 2 would leak exactly one point through the floor.
    // A shield of the same size must catch it, or no one would take a shield.
    let t = target().with_armor(9).with_shield(4);
    let res = hit(&attacker(0), &t, 2, Channel::Physical);
    assert_eq!(res.to_shield, 1);
    assert_eq!(res.to_health, 0);
}

#[test]
fn a_shield_melts_by_what_it_caught() {
    let mut t = target().with_shield(3);
    let res = hit(&attacker(0), &t, 5, Channel::Physical);
    apply(&mut t, &res);
    assert_eq!(t.shield, 0);
    assert_eq!(t.health.current, 18);
}

#[test]
fn blessings_and_curses_of_the_striker_shift_the_number() {
    let mut a = attacker(0);
    a.apply_status(Status::new("Бабушка знает лучше", Stat::Power, 2, 2));
    assert_eq!(hit(&a, &target(), 4, Channel::Physical).to_health, 6);

    a.apply_status(Status::new("Дым из печи", Stat::Power, -3, 2));
    assert_eq!(hit(&a, &target(), 4, Channel::Physical).to_health, 3);
}

#[test]
fn vulnerability_adds_to_everything_incoming() {
    let mut t = target().with_armor(2);
    t.apply_status(Status::new("Сглаз", Stat::Vulnerable, 3, 2));
    assert_eq!(hit(&attacker(0), &t, 5, Channel::Physical).to_health, 6);
}

#[test]
fn a_rider_of_the_same_name_refreshes_the_term_and_not_the_magnitude() {
    let mut t = target();
    t.apply_status(Status::new("Дым из печи", Stat::Armor, -2, 1));
    t.apply_status(Status::new("Дым из печи", Stat::Armor, -2, 3));
    assert_eq!(t.statuses.len(), 1);
    assert_eq!(t.status_sum(Stat::Armor), -2, "величина не удваивается");
    assert_eq!(t.statuses[0].turns, 3, "срок обновлён");
}

#[test]
fn riders_of_different_names_add_up() {
    let mut t = target().with_armor(1);
    t.apply_status(Status::new("Подогнать по фигуре", Stat::Armor, 2, 3));
    t.apply_status(Status::new("Стадный инстинкт", Stat::Armor, 2, 2));
    assert_eq!(t.status_sum(Stat::Armor), 4);
    assert_eq!(hit(&attacker(0), &t, 9, Channel::Physical).to_health, 4);
}

#[test]
fn the_sixth_rider_displaces_the_oldest() {
    let mut t = target();
    for i in 0..6 {
        t.apply_status(Status::new(&format!("рядовой {i}"), Stat::Power, 1, 2));
    }
    assert_eq!(t.statuses.len(), 5);
    assert_eq!(t.statuses[0].name, "рядовой 1", "первый вытеснен");
}

#[test]
fn lifting_a_rider_is_one_call_and_not_an_unwrapping() {
    // The operation a chain of wrappers cannot perform without being rebuilt.
    let mut t = target();
    t.apply_status(Status::new("Сглаз", Stat::Vulnerable, 3, 2));
    t.apply_status(Status::new("Наставник", Stat::Power, 2, 2));
    assert_eq!(t.clear_status("Сглаз"), 1);
    assert_eq!(t.status_sum(Stat::Vulnerable), 0);
    assert_eq!(t.status_sum(Stat::Power), 2, "остальное не тронуто");
}

#[test]
fn death_lifts_every_rider() {
    let mut t = Unit::new(2, 3, 0);
    t.apply_status(Status::new("Гипноз", Stat::Power, -1, 5));
    let res = hit(&attacker(0), &t, 9, Channel::Physical);
    let events = apply(&mut t, &res);
    assert!(events.contains(&Event::Died { target: 2 }));
    assert!(t.statuses.is_empty());
}

#[test]
fn the_trail_explains_the_number() {
    // Why 3 and not 8 — the thing a chain of decorators cannot tell you.
    let mut a = attacker(0);
    a.apply_status(Status::new("Проклятие", Stat::Power, -1, 2));
    let t = target().with_armor(4);
    let res = hit(&a, &t, 8, Channel::Physical);

    assert_eq!(res.to_health, 3);
    let steps: Vec<_> = res.trail.iter().map(|b| (b.step, b.from, b.to)).collect();
    assert_eq!(
        steps,
        vec![(StepId::AttackerCurse, 8, 7), (StepId::ChannelDefence, 7, 3)]
    );
}

#[test]
fn the_trail_always_ends_where_the_damage_did() {
    let t = target().with_armor(2).with_shield(1);
    let res = hit(&attacker(0), &t, 6, Channel::Physical);
    assert_eq!(res.trail.last().unwrap().to + res.to_shield, res.total());
}

#[test]
fn thorns_never_answer_thorns() {
    assert!(!Source::Thorns.provokes_thorns());
    assert!(!Source::Dot.provokes_thorns());
    assert!(!Source::Zone.provokes_thorns());
    assert!(Source::Attack.provokes_thorns());
    assert!(Source::Ability.provokes_thorns());
}

#[test]
fn poison_is_not_felt_as_a_blow() {
    assert!(!Source::Dot.is_felt());
    assert!(!Source::Recoil.is_felt());
    assert!(Source::Splash.is_felt());
}

#[test]
fn an_ordinary_strike_uses_the_strikers_strength() {
    let a = attacker(5);
    let t = target().with_armor(2);
    assert_eq!(strike(&a, &t).to_health, 3);
}

#[test]
fn resolving_changes_nobody() {
    let a = attacker(5);
    let before = target().with_armor(1);
    let t = before.clone();
    let _ = strike(&a, &t);
    assert_eq!(t, before, "счёт ничего не меняет — меняет только apply");
}

#[test]
fn the_same_blow_always_gives_the_same_answer() {
    let a = attacker(6);
    let t = target().with_armor(2).with_shield(2);
    let first = strike(&a, &t);
    for _ in 0..100 {
        assert_eq!(strike(&a, &t), first);
    }
}

#[test]
fn health_stops_at_nothing_rather_than_going_negative() {
    let mut t = Unit::new(2, 3, 0);
    let res = hit(&attacker(0), &t, 40, Channel::Physical);
    apply(&mut t, &res);
    assert_eq!(t.health.current, 0);
    assert!(t.health.is_dead());
}

// ── лечение ─────────────────────────────────────────────────────────────────

#[test]
fn mending_does_not_raise_a_body_above_what_its_card_says() {
    let mut t = Unit::new(2, 10, 0);
    t.health.current = 8;
    let mending = resolve_mend(&t, 5);
    assert_eq!(mending.offered, 5);
    assert_eq!(mending.restored, 2, "лишнее не переносится");
    apply_mend(None, &mut t, &mending);
    assert_eq!(t.health.current, 10);
}

#[test]
fn mending_an_unwounded_body_does_nothing_and_says_nothing() {
    let mut t = Unit::new(2, 10, 0);
    let mending = resolve_mend(&t, 5);
    assert_eq!(mending.restored, 0);
    assert!(apply_mend(None, &mut t, &mending).is_empty(), "события на пустом месте нет");
}

#[test]
fn resolving_a_mending_changes_nobody() {
    let before = Unit::new(2, 10, 0);
    let mut t = before.clone();
    t.health.current = 4;
    let untouched = t.clone();
    let _ = resolve_mend(&t, 5);
    assert_eq!(t, untouched);
}

// Две проверки, которых не было, пока мутационный прогон не показал, что
// сломать эти места можно и никто не заметит.

#[test]
fn a_striker_hits_with_its_own_channel_and_not_a_default_one() {
    // Тело, бьющее чарно, режется оберегом и не режется бронёй. Без этого
    // теста `strike` могла бы всегда брать телесный канал, и целая порода
    // карт — маги — считалась бы неверно молча.
    let mut a = attacker(6);
    a.card = a.card.clone().with_channel(Channel::Magic);
    a.channel = Channel::Magic;

    let warded = target().with_ward(4);
    let armored = target().with_armor(4);

    assert_eq!(strike(&a, &warded).to_health, 2, "оберег отвечает чарному удару");
    assert_eq!(strike(&a, &armored).to_health, 6, "броня чарному удару не отвечает");
}

#[test]
fn a_mending_of_a_negative_amount_puts_nothing_back() {
    // Лечение не должно уметь ранить. Отрицательная величина — это либо
    // проклятие, которое ещё не написано, либо ошибка в данных карты;
    // и то и другое обязано превратиться в ноль, а не в урон через лечение.
    let mut t = Unit::new(2, 10, 0);
    t.health.current = 4;
    let mending = resolve_mend(&t, -5);
    assert_eq!(mending.restored, 0);
    apply_mend(None, &mut t, &mending);
    assert_eq!(t.health.current, 4);
}
