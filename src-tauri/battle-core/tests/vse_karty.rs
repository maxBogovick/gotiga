//! Все карты архива на всех землях и под чужими чарами.
//!
//! Остальные батареи берут выдуманные тела («Стрелок», «Бегун»): они доказывают
//! правило, но не доказывают, что правило верно для карты, которую хранитель
//! положил на полку. Эта берёт настоящие — `tests/fixtures/cards.json`, выгрузка
//! `GET /battles/cards` — и собирает из них тела тем же способом, что
//! `server::battles::to_snapshot` (те же зажимы дальности и шага).
//!
//! Ожидаемое число считает НЕЗАВИСИМАЯ формула `expected` — записанная здесь
//! заново из описания правил, а не вызовом движка: тест, сверяющий движок с
//! самим собой, не проверяет ничего.
//!
//! Выгрузку обновляют, когда на полке появляется карта:
//! `curl localhost:3000/api/v1/battles/cards`, оставить поля из фикстуры.

use battle_core::*;
use serde_json::Value;

// ── Карты ───────────────────────────────────────────────────────────────────

fn channel_of(raw: &str) -> Channel {
    match raw {
        "magic" => Channel::Magic,
        "pure" => Channel::Pure,
        _ => Channel::Physical,
    }
}

fn stat_of(raw: &str) -> Option<Stat> {
    match raw.trim() {
        "power" => Some(Stat::Power),
        "armor" => Some(Stat::Armor),
        "ward" => Some(Stat::Ward),
        "vulnerable" => Some(Stat::Vulnerable),
        _ => None,
    }
}

fn i(v: &Value, key: &str) -> i64 {
    v[key].as_i64().unwrap_or(0)
}

fn s<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key].as_str().unwrap_or("")
}

/// То же, что `to_snapshot` на сервере.
fn shelf() -> Vec<CardSnapshot> {
    let raw: Vec<Value> = serde_json::from_str(include_str!("fixtures/cards.json")).unwrap();
    raw.iter()
        .map(|c| CardSnapshot {
            name: s(c, "slug").to_string(),
            cost: i(c, "cost") as i32,
            health: i(c, "health") as i32,
            power: i(c, "power") as i32,
            armor: i(c, "armor") as i32,
            ward: i(c, "ward") as i32,
            reach: i(c, "reach").clamp(0, 5) as u8,
            step: i(c, "step").clamp(0, 3) as u8,
            mend: i(c, "mend") as i32,
            channel: channel_of(s(c, "attackChannel")),
            strikes: s(c, "attackChannel") != "none",
            abilities: c["abilities"]
                .as_array()
                .unwrap()
                .iter()
                .map(|a| AbilitySnapshot {
                    id: s(a, "id").to_string(),
                    verb: s(a, "verb").to_string(),
                    amount: i(a, "amount") as i32,
                    shape: s(a, "shape").to_string(),
                    range: i(a, "range").clamp(0, 5) as u8,
                    mana_cost: i(a, "manaCost") as i32,
                    cooldown: i(a, "cooldown").clamp(0, 5) as u8,
                    trigger: s(a, "trigger").to_string(),
                    channel: channel_of(s(a, "channel")),
                    duration: i(a, "duration").clamp(0, 5) as u8,
                    radius: i(a, "radius").clamp(0, 3) as u8,
                    stat: stat_of(s(a, "stat")),
                    body: None,
                })
                .collect(),
        })
        .collect()
}

fn card(slug: &str) -> CardSnapshot {
    shelf().into_iter().find(|c| c.name == slug).unwrap_or_else(|| panic!("нет карты {slug}"))
}

// ── Поле ────────────────────────────────────────────────────────────────────

fn cell(x: u8, y: u8) -> Cell {
    Cell::new(x, y).unwrap()
}

fn tile(x: u8, y: u8, ground: Ground) -> Tile {
    Tile { cell: cell(x, y), ground }
}

fn rules() -> Rules {
    Rules { idle_toll: 0, ..Rules::default() }
}

/// 3 × 10: половины по пять рядов, самая длинная дорога из возможных — дальность
/// 5 с холма это 6, и ей нужно куда лететь.
fn long() -> Field {
    Field { width: 3, depth: 5 }
}

fn act(st: &MatchState, a: Action) -> (MatchState, Vec<Event>) {
    reduce(st, &a).unwrap_or_else(|e| panic!("{a:?} должно быть законным: {e:?}"))
}

fn id_at(st: &MatchState, c: Cell) -> UnitId {
    st.board.at(c).unwrap_or_else(|| panic!("на {c:?} никого нет"))
}

fn hp(st: &MatchState, id: UnitId) -> i32 {
    st.units[id as usize].health.current
}

fn offers(st: &MatchState, a: &Action) -> bool {
    legal_actions(st).contains(a)
}

/// Ни одной карты под одной крышей с одной землёй: все земли, на которых можно
/// СТОЯТЬ. Стена и овраг — отдельный разговор (`walls_and_ravines_*`).
const STANDABLE: [Option<Ground>; 6] = [
    None,
    Some(Ground::Hill),
    Some(Ground::Cover),
    Some(Ground::Spring),
    Some(Ground::Mire),
    Some(Ground::Pit),
];

fn name_of(g: Option<Ground>) -> String {
    g.map_or("ровно".to_string(), |g| format!("{g:?}"))
}

// ── Независимая формула урона ───────────────────────────────────────────────

// Числа земли записаны здесь словами правил, а НЕ взяты из движка: тест, читающий
// `COVER_KEPT` у самого движка, выдержал бы и укрытие в 70 %, и родник в +2.
// Если число правил решено поменять, меняют его и тут — осознанно.
/// «Удар издали по укрытому доходит вполсилы».
const COVER_PERCENT: i32 = 50;
/// «Родник возвращает одно здоровье».
const SPRING_POINTS: i32 = 1;
/// «Яма ранит на два».
const PIT_POINTS: i32 = 2;

/// Что должен потерять `target` от обычного удара `attacker` с расстояния `d`.
///
/// Записано заново по описанию правил, а не вызовом движка:
/// 1. помехи бьющему множатся в таком порядке: дальний выстрел, упор, укрытие;
/// 2. прибавка к силе от всадников на бьющем;
/// 3. уязвимость цели прибавляется;
/// 4. защита канала вычитается (не ниже нуля);
/// 5. удар, дошедший, забирает не меньше одной единицы.
struct Blow {
    d: u8,
    attacker_on_hill: bool,
    target_in_cover: bool,
    /// Сумма всадников силы на бьющем (положительные — благословение).
    attacker_power: i32,
    /// Сумма всадников на цели.
    target_vulnerable: i32,
    target_armor: i32,
    target_ward: i32,
}

fn expected(a: &CardSnapshot, t: &CardSnapshot, b: &Blow, rules: &Rules) -> i32 {
    let reach = if b.attacker_on_hill && a.reach > 1 { a.reach + 1 } else { a.reach };
    let mut kept = a.power;
    if a.reach > 1 && b.d > reach {
        kept = kept * rules.long_shot_power as i32 / 100;
    }
    if a.reach > 1 && b.d == 1 {
        kept = kept * rules.point_blank_power as i32 / 100;
    }
    if b.target_in_cover && b.d > 1 {
        kept = kept * COVER_PERCENT / 100;
    }
    let defence = match a.channel {
        Channel::Physical => t.armor + b.target_armor,
        Channel::Magic => t.ward + b.target_ward,
        Channel::Pure => 0,
    };
    (kept + b.attacker_power + b.target_vulnerable - defence.max(0)).max(1)
}

fn plain_blow(d: u8) -> Blow {
    Blow {
        d,
        attacker_on_hill: false,
        target_in_cover: false,
        attacker_power: 0,
        target_vulnerable: 0,
        target_armor: 0,
        target_ward: 0,
    }
}

/// Цель на (1,0), бьющий на (1,d); при `with` — ещё один союзник бьющего.
fn duel(a: &CardSnapshot, ga: Option<Ground>, t: &CardSnapshot, gt: Option<Ground>, d: u8) -> MatchState {
    let mut terrain = Vec::new();
    if let Some(g) = ga {
        terrain.push(tile(1, d, g));
    }
    if let Some(g) = gt {
        terrain.push(tile(1, 0, g));
    }
    MatchState::begin_with(
        Setup {
            player_board: vec![(a.clone(), cell(1, d))],
            player_hand: vec![],
            keeper_board: vec![(t.clone(), cell(1, 0))],
            keeper_hand: vec![],
            terrain,
            field: long(),
        },
        rules(),
    )
}

/// То же и ещё один союзник бьющего: id 0 — бьющий, 1 — союзник, 2 — цель.
fn duel_with(
    a: &CardSnapshot,
    ga: Option<Ground>,
    t: &CardSnapshot,
    gt: Option<Ground>,
    d: u8,
    ally: &CardSnapshot,
    ally_at: Cell,
) -> MatchState {
    let mut terrain = Vec::new();
    if let Some(g) = ga {
        terrain.push(tile(1, d, g));
    }
    if let Some(g) = gt {
        terrain.push(tile(1, 0, g));
    }
    // Номера раздаются сперва игроку, затем хранителю: бьющий — 0, союзник — 1,
    // цель — 2.
    MatchState::begin_with(
        Setup {
            player_board: vec![(a.clone(), cell(1, d)), (ally.clone(), ally_at)],
            player_hand: vec![],
            keeper_board: vec![(t.clone(), cell(1, 0))],
            keeper_hand: vec![],
            terrain,
            field: long(),
        },
        rules(),
    )
}

// ════════════════════════════════════════════════════════════════════════════
// 1. Холм
// ════════════════════════════════════════════════════════════════════════════

/// Дальность с холма: стрелку (дальность больше одной клетки) — на клетку
/// больше, ближнему бою — ничего. Для КАЖДОЙ карты и с обеих сторон поля.
#[test]
fn hill_adds_one_cell_only_to_those_who_shoot() {
    for c in shelf() {
        for ground in [None, Some(Ground::Hill)] {
            for side in [Side::Player, Side::Keeper] {
                let at = cell(1, if side == Side::Player { 6 } else { 3 });
                let mut setup = Setup { field: long(), ..Setup::default() };
                match side {
                    Side::Player => setup.player_board.push((c.clone(), at)),
                    Side::Keeper => setup.keeper_board.push((c.clone(), at)),
                }
                if let Some(g) = ground {
                    setup.terrain.push(Tile { cell: at, ground: g });
                }
                let st = MatchState::begin_with(setup, rules());
                let u = &st.units[0];
                let want = if ground == Some(Ground::Hill) && c.reach > 1 { c.reach + 1 } else { c.reach };
                assert_eq!(st.reach_of(u), want, "{} ({side:?}) на {}", c.name, name_of(ground));
            }
        }
    }
}

/// Холм меняет, до кого стрелок достаёт в ПОЛНУЮ силу, и ничего больше: дальше —
/// по-прежнему дальний выстрел, а ближнему бою дальше соседней клетки нельзя.
/// Для каждой карты, на каждом расстоянии, с холма и без.
#[test]
fn hill_moves_the_edge_of_full_power_by_exactly_one() {
    let cards = shelf();
    let target = CardSnapshot::new("мишень", 1, 99, 1);
    for a in &cards {
        for ground in [None, Some(Ground::Hill)] {
            let hill = ground == Some(Ground::Hill);
            for d in 1..=8u8 {
                let st = duel(a, ground, &target, None, d);
                let reach = if hill && a.reach > 1 { a.reach + 1 } else { a.reach };
                let may = d <= reach || (a.reach > 1 && st.rules.long_shot_power > 0);
                let act_ = Action::Attack { attacker: 0, target: 1 };
                assert_eq!(
                    offers(&st, &act_),
                    may,
                    "{} на {} бьёт с {d}: законность удара",
                    a.name,
                    name_of(ground)
                );
                if !may {
                    continue;
                }
                let (after, events) = act(&st, act_);
                let lost = 99 - hp(&after, 1);
                let blow = Blow { attacker_on_hill: hill, ..plain_blow(d) };
                assert_eq!(lost, expected(a, &target, &blow, &st.rules), "{} на {} с {d}", a.name, name_of(ground));
                // След называет дальний выстрел ровно тогда, когда он был.
                let trail = events
                    .iter()
                    .find_map(|e| match e {
                        Event::Damaged { trail, .. } => Some(trail.clone()),
                        _ => None,
                    })
                    .unwrap();
                let far = a.reach > 1 && d > reach;
                assert_eq!(
                    trail.iter().any(|b| b.step == StepId::LongShot),
                    far,
                    "{} на {} с {d}: дальний выстрел в следе",
                    a.name,
                    name_of(ground)
                );
            }
        }
    }
}

/// Холм ничего не даёт тому, кого бьют: цель на холме теряет ровно столько,
/// сколько на ровном месте.
#[test]
fn hill_gives_no_defence() {
    let cards = shelf();
    for a in &cards {
        for t in &cards {
            for d in 1..=3u8 {
                let lost = |g| {
                    let st = duel(a, None, t, g, d);
                    if !offers(&st, &Action::Attack { attacker: 0, target: 1 }) {
                        return None;
                    }
                    let (after, _) = act(&st, Action::Attack { attacker: 0, target: 1 });
                    Some(t.health - hp(&after, 1))
                };
                assert_eq!(lost(Some(Ground::Hill)), lost(None), "{} → {} с {d}", a.name, t.name);
            }
        }
    }
}

/// Сдача достаёт ровно настолько, насколько достаёт тело С ХОЛМА: стрелок на
/// холме отвечает на клетку дальше, ближний бой — как прежде.
#[test]
fn retaliation_reaches_as_far_as_the_hill_lets_it() {
    let cards = shelf();
    let shooter = CardSnapshot::new("бьющий", 1, 99, 1).with_reach(5);
    for c in &cards {
        for ground in [None, Some(Ground::Hill)] {
            let hill = ground == Some(Ground::Hill);
            for d in 1..=7u8 {
                let mut terrain = vec![];
                if let Some(g) = ground {
                    terrain.push(tile(1, 0, g));
                }
                let st = MatchState::begin_with(
                    Setup {
                        player_board: vec![(shooter.clone(), cell(1, d))],
                        player_hand: vec![],
                        keeper_board: vec![(c.clone(), cell(1, 0))],
                        keeper_hand: vec![],
                        terrain,
                        field: long(),
                    },
                    Rules { retaliation: true, ..rules() },
                );
                let (after, _) = act(&st, Action::Attack { attacker: 0, target: 1 });
                assert!(hp(&after, 1) > 0, "{} пережил слабый удар", c.name);
                let reach = if hill && c.reach > 1 { c.reach + 1 } else { c.reach };
                let answers = c.strikes && d <= reach;
                let lost = 99 - hp(&after, 0);
                assert_eq!(
                    lost,
                    if answers { c.power.max(1) } else { 0 },
                    "{} на {} отвечает на {d}",
                    c.name,
                    name_of(ground)
                );
            }
        }
    }
}

/// Шагнув на холм, тело сразу бьёт дальше; шагнув с него — сразу как прежде.
#[test]
fn stepping_on_and_off_a_hill_changes_reach_at_once() {
    for c in shelf().into_iter().filter(|c| c.step >= 1) {
        let st = MatchState::begin_with(
            Setup {
                player_board: vec![(c.clone(), cell(1, 5))],
                player_hand: vec![],
                keeper_board: vec![(CardSnapshot::new("страж", 1, 9, 1).with_step(0), cell(2, 0))],
                keeper_hand: vec![],
                terrain: vec![tile(1, 4, Ground::Hill)],
                field: long(),
            },
            rules(),
        );
        assert_eq!(st.reach_of(&st.units[0]), c.reach);
        let (st, _) = act(&st, Action::Move { unit: 0, to: cell(1, 4) });
        let up = if c.reach > 1 { c.reach + 1 } else { c.reach };
        assert_eq!(st.reach_of(&st.units[0]), up, "{} взошёл на холм", c.name);
        let (st, _) = act(&st, Action::EndTurn);
        let (st, _) = act(&st, Action::EndTurn);
        let (st, _) = act(&st, Action::Move { unit: 0, to: cell(1, 5) });
        assert_eq!(st.reach_of(&st.units[0]), c.reach, "{} сошёл с холма", c.name);
    }
}

/// Холм не удлиняет ни лечение, ни чары: он про удар, как и сказано на земле.
#[test]
fn hill_does_not_stretch_healing() {
    let healers = [card("woman-with-the-basket"), card("small-doll-baba-yaga")];
    for h in healers {
        let range = h.abilities.iter().find(|a| a.verb == "heal").map_or(h.reach, |a| a.range);
        for ground in [None, Some(Ground::Hill)] {
            for d in 1..=6u8 {
                let mut terrain = vec![];
                if let Some(g) = ground {
                    terrain.push(tile(1, 9, g));
                }
                let mut st = MatchState::begin_with(
                    Setup {
                        player_board: vec![
                            (h.clone(), cell(1, 9)),
                            (CardSnapshot::new("раненый", 1, 9, 1), cell(1, 9 - d)),
                        ],
                        player_hand: vec![],
                        keeper_board: vec![(CardSnapshot::new("страж", 1, 9, 1).with_step(0), cell(2, 0))],
                        keeper_hand: vec![],
                        terrain,
                        field: long(),
                    },
                    rules(),
                );
                st.units[1].health.current = 5;
                st.player.mana = 10;
                let may = offers(&st, &Action::Mend { healer: 0, target: 1 });
                assert_eq!(may, d <= range, "{} на {} лечит с {d} (дальность {range})", h.name, name_of(ground));
            }
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════
// 2. Каждая карта бьёт каждую: земля под обоими и чужое проклятие
// ════════════════════════════════════════════════════════════════════════════

/// Главная батарея. Для каждой пары карт (бьющий, цель), для каждой земли под
/// бьющим (ровно / холм) и под целью (все шесть, на которых стоят), для каждого
/// расстояния и с проклятием настоящего вампира на цели и без него:
/// законность удара и потерянное здоровье сходятся с независимой формулой.
///
/// Проклятие «Холодный взгляд» накладывается НАСТОЯЩИМ ходом — `Action::Cast`
/// той самой карты из архива, с её маной, дальностью и сроком, — а не вписано в
/// тело руками.
#[test]
fn every_card_strikes_every_card_on_every_ground_cursed_or_not() {
    let cards = shelf();
    let vampire = card("the-vampire");
    let gaze = vampire.abilities.iter().find(|a| a.id == "gaze").unwrap().clone();
    assert_eq!((gaze.verb.as_str(), gaze.stat, gaze.amount), ("curse", Some(Stat::Vulnerable), 2));

    let mut checked = 0u32;
    for a in &cards {
        for t in &cards {
            for ga in [None, Some(Ground::Hill)] {
                for gt in STANDABLE {
                    for d in 1..=7u8 {
                        for cursed in [false, true] {
                            let mut st = duel(a, ga, t, gt, d);
                            let mut vampire_id = None;
                            if cursed {
                                // Вампир стоит в двух клетках от цели: ровно на дальность взгляда.
                                st = duel_with(a, ga, t, gt, d, &vampire, cell(0, 2));
                                st.player.mana = 10;
                                vampire_id = Some(1 as UnitId);
                            }
                            let attacker: UnitId = 0;
                            let target: UnitId = if cursed { 2 } else { 1 };
                            let where_ = format!(
                                "{} ({}) → {} ({}) с {d}, {}",
                                a.name,
                                name_of(ga),
                                t.name,
                                name_of(gt),
                                if cursed { "проклятая" } else { "чистая" }
                            );

                            if let Some(v) = vampire_id {
                                let cast = Action::Cast {
                                    caster: v,
                                    ability: "gaze".into(),
                                    target: Mark::Unit(target),
                                };
                                assert!(offers(&st, &cast), "{where_}: взгляд предложен");
                                let (next, _) = act(&st, cast);
                                st = next;
                                assert_eq!(st.units[target as usize].status_sum(Stat::Vulnerable), 2, "{where_}");
                            }

                            let reach = if ga == Some(Ground::Hill) && a.reach > 1 { a.reach + 1 } else { a.reach };
                            let may = d <= reach || (a.reach > 1 && st.rules.long_shot_power > 0);
                            let blow = Action::Attack { attacker, target };
                            assert_eq!(offers(&st, &blow), may, "{where_}: законность удара");
                            if !may {
                                continue;
                            }
                            let before = hp(&st, target);
                            let (after, events) = act(&st, blow);
                            let want = expected(
                                a,
                                t,
                                &Blow {
                                    d,
                                    attacker_on_hill: ga == Some(Ground::Hill),
                                    target_in_cover: gt == Some(Ground::Cover),
                                    attacker_power: 0,
                                    target_vulnerable: if cursed { 2 } else { 0 },
                                    target_armor: 0,
                                    target_ward: 0,
                                },
                                &st.rules,
                            );
                            assert_eq!(before - hp(&after, target), want.min(before), "{where_}: потеряно");

                            // След называет укрытие ровно тогда, когда оно сработало.
                            let trail = events.iter().find_map(|e| match e {
                                Event::Damaged { trail, .. } => Some(trail.clone()),
                                _ => None,
                            });
                            let trail = trail.unwrap_or_default();
                            assert_eq!(
                                trail.iter().any(|b| b.step == StepId::Cover),
                                gt == Some(Ground::Cover) && d > 1,
                                "{where_}: укрытие в следе"
                            );
                            assert_eq!(
                                trail.iter().any(|b| b.step == StepId::TargetVulnerable),
                                cursed,
                                "{where_}: уязвимость в следе"
                            );
                            checked += 1;
                        }
                    }
                }
            }
        }
    }
    assert!(checked > 10_000, "проверено только {checked} ударов — батарея не дошла до дела");
    println!("ударов проверено: {checked}");
}

// ════════════════════════════════════════════════════════════════════════════
// 3. Всадники на теле и земля: все четыре показателя, оба знака
// ════════════════════════════════════════════════════════════════════════════

/// Благословение и проклятие любого показателя, лёгшие на любую карту, считаются
/// одинаково на любой земле: земля не съедает всадника и не удваивает его.
///
/// Всадник на ЦЕЛИ (армор, оберег, уязвимость) и на БЬЮЩЕМ (сила) кладётся тем
/// же `lay_rider`, которым его кладёт чара; число и знак берёт `spell::rider`,
/// то есть правило «уязвимость — наоборот» тоже проверяется, а не обходится.
#[test]
fn riders_of_every_stat_and_sign_add_up_on_every_ground() {
    let cards = shelf();
    let verbs = [("bless", Casting::Bless), ("curse", Casting::Curse)];
    let stats = [Stat::Power, Stat::Armor, Stat::Ward, Stat::Vulnerable];
    let mut checked = 0u32;

    for (verb, what) in verbs {
        for stat in stats {
            let ability = AbilitySnapshot {
                id: "x".into(),
                verb: verb.into(),
                amount: 2,
                shape: "one".into(),
                range: 3,
                mana_cost: 0,
                cooldown: 0,
                trigger: "active".into(),
                channel: Channel::Magic,
                duration: 3,
                radius: 0,
                stat: Some(stat),
                body: None,
            };
            let rid = rider(&ability, what, "x").expect("всадник");
            // Знак: проклятие бьёт, благословение помогает; уязвимость — наоборот.
            let helps = (what == Casting::Bless) != (stat == Stat::Vulnerable);
            assert_eq!(rid.amount > 0, helps, "{verb} {stat:?}: знак");

            for a in &cards {
                for t in &cards {
                    for ga in [None, Some(Ground::Hill)] {
                        for gt in STANDABLE {
                            for d in 1..=3u8 {
                                for on_attacker in [false, true] {
                                    // Всадник «сила» значит что-то только на бьющем,
                                    // остальные — только на цели.
                                    if on_attacker != (stat == Stat::Power) {
                                        continue;
                                    }
                                    let mut st = duel(a, ga, t, gt, d);
                                    let bearer = if on_attacker { 0 } else { 1 };
                                    lay_rider(None, &mut st.units[bearer], rid.clone(), what);
                                    let blow = Action::Attack { attacker: 0, target: 1 };
                                    let reach =
                                        if ga == Some(Ground::Hill) && a.reach > 1 { a.reach + 1 } else { a.reach };
                                    let may = d <= reach || (a.reach > 1 && st.rules.long_shot_power > 0);
                                    assert_eq!(offers(&st, &blow), may);
                                    if !may {
                                        continue;
                                    }
                                    let before = hp(&st, 1);
                                    let (after, _) = act(&st, blow);
                                    let b = Blow {
                                        d,
                                        attacker_on_hill: ga == Some(Ground::Hill),
                                        target_in_cover: gt == Some(Ground::Cover),
                                        attacker_power: if stat == Stat::Power { rid.amount } else { 0 },
                                        target_vulnerable: if stat == Stat::Vulnerable { rid.amount } else { 0 },
                                        target_armor: if stat == Stat::Armor { rid.amount } else { 0 },
                                        target_ward: if stat == Stat::Ward { rid.amount } else { 0 },
                                    };
                                    assert_eq!(
                                        before - hp(&after, 1),
                                        expected(a, t, &b, &st.rules).min(before),
                                        "{verb} {stat:?}: {} ({}) → {} ({}) с {d}",
                                        a.name,
                                        name_of(ga),
                                        t.name,
                                        name_of(gt)
                                    );
                                    checked += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(checked > 20_000, "проверено только {checked}");
    println!("ударов со всадниками проверено: {checked}");
}

// ════════════════════════════════════════════════════════════════════════════
// 4. Умения самих карт на земле: глоток вампира, лечение корзинщицы, мазь
// ════════════════════════════════════════════════════════════════════════════

/// «Глоток» вампира (лечит себя, когда он бьёт) работает с любой земли и по
/// любой цели: даже когда удар дошёл вполсилы через укрытие, он всё равно удар.
#[test]
fn the_vampires_sip_works_from_every_ground_at_every_target() {
    let cards = shelf();
    let v = card("the-vampire");
    let sip = v.abilities.iter().find(|a| a.id == "sip").unwrap();
    assert_eq!((sip.trigger.as_str(), sip.verb.as_str(), sip.amount), ("onHit", "heal", 2));
    for t in &cards {
        for gv in STANDABLE {
            for gt in STANDABLE {
                // Вампир бьёт только вплотную (дальность 1).
                for d in [1u8] {
                    let mut st = duel(&v, gv, t, gt, d);
                    st.units[0].health.current = v.health - 2;
                    let (after, events) = act(&st, Action::Attack { attacker: 0, target: 1 });
                    assert!(
                        events.iter().any(|e| matches!(e, Event::Healed { target: 0, amount: 2, .. })),
                        "глоток на {} → {} ({}) с {d}",
                        name_of(gv),
                        t.name,
                        name_of(gt)
                    );
                    assert_eq!(hp(&after, 0), v.health, "вернулись ровно два");
                }
            }
        }
    }
}

/// Корзинщица лечит на любую землю под раненым — столько, сколько написано, и не
/// выше полного здоровья; проклятие на раненом лечению не мешает.
#[test]
fn the_basket_woman_heals_whoever_stands_on_any_ground() {
    let cards = shelf();
    let w = card("woman-with-the-basket");
    let heal = w.abilities.iter().find(|a| a.verb == "heal").unwrap().clone();
    let vampire = card("the-vampire");
    for t in &cards {
        for gt in STANDABLE {
            for wound in [1, 2, t.health - 1] {
                for cursed in [false, true] {
                    let mut st = MatchState::begin_with(
                        Setup {
                            player_board: vec![
                                (w.clone(), cell(1, 7)),
                                (t.clone(), cell(1, 6)),
                                (vampire.clone(), cell(0, 9)),
                            ],
                            player_hand: vec![],
                            keeper_board: vec![(CardSnapshot::new("страж", 1, 9, 1).with_step(0), cell(2, 0))],
                            keeper_hand: vec![],
                            terrain: gt.map(|g| tile(1, 6, g)).into_iter().collect(),
                            field: long(),
                        },
                        rules(),
                    );
                    st.player.mana = 10;
                    st.units[1].health.current = t.health - wound;
                    if cursed {
                        let rid = Status::new("взгляд", Stat::Vulnerable, 2, 2);
                        lay_rider(Some(2), &mut st.units[1], rid, Casting::Curse);
                    }
                    let mend = Action::Mend { healer: 0, target: 1 };
                    assert!(offers(&st, &mend), "{} ({}): лечение предложено", t.name, name_of(gt));
                    let (after, _) = act(&st, mend);
                    let got = heal.amount.min(wound);
                    assert_eq!(hp(&after, 1), t.health - wound + got, "{} ({})", t.name, name_of(gt));
                    assert!(hp(&after, 1) <= t.health, "выше полного не лечат");
                }
            }
        }
    }
}

/// Мазь Бабы-Яги (`mend`, без умения): лечит соседа на землях всех родов.
#[test]
fn the_ointment_works_on_every_ground() {
    let baba = card("small-doll-baba-yaga");
    assert!(baba.mend > 0);
    for t in shelf() {
        for gt in STANDABLE {
            let mut st = MatchState::begin_with(
                Setup {
                    player_board: vec![(baba.clone(), cell(1, 7)), (t.clone(), cell(1, 6))],
                    player_hand: vec![],
                    keeper_board: vec![(CardSnapshot::new("страж", 1, 9, 1).with_step(0), cell(2, 0))],
                    keeper_hand: vec![],
                    terrain: gt.map(|g| tile(1, 6, g)).into_iter().collect(),
                    field: long(),
                },
                rules(),
            );
            st.units[1].health.current = t.health - 2;
            let (after, _) = act(&st, Action::Mend { healer: 0, target: 1 });
            assert_eq!(hp(&after, 1), t.health - 2 + baba.mend.min(2), "{} ({})", t.name, name_of(gt));
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════
// 5. Родник, яма, болото, стена, овраг — для каждой карты
// ════════════════════════════════════════════════════════════════════════════

/// Родник возвращает ровно `SPRING_MEND` и не выше полного здоровья; не родник —
/// ничего. Начало СВОЕГО хода, у обеих сторон, при любой карте и любом проклятии.
#[test]
fn a_spring_gives_back_exactly_one_to_every_card() {
    for c in shelf() {
        for side in [Side::Keeper, Side::Player] {
            for ground in [Some(Ground::Spring), None, Some(Ground::Hill)] {
                for wound in [0, 1, 2] {
                    let wound = wound.min(c.health - 1);
                    let at = cell(1, if side == Side::Player { 9 } else { 0 });
                    let far = cell(1, if side == Side::Player { 0 } else { 9 });
                    let ours = (c.clone(), at);
                    let theirs = (CardSnapshot::new("далёкий", 1, 9, 1).with_step(0), far);
                    let mut setup = Setup { field: long(), ..Setup::default() };
                    match side {
                        Side::Player => {
                            setup.player_board.push(ours);
                            setup.keeper_board.push(theirs);
                        }
                        Side::Keeper => {
                            setup.keeper_board.push(ours);
                            setup.player_board.push(theirs);
                        }
                    }
                    if let Some(g) = ground {
                        setup.terrain.push(Tile { cell: at, ground: g });
                    }
                    let mut st = MatchState::begin_with(setup, rules());
                    let mine = id_at(&st, at);
                    st.units[mine as usize].health.current = c.health - wound;
                    // Дойти до начала своего хода.
                    let turns = if side == Side::Keeper { 1 } else { 2 };
                    let mut healed = 0;
                    for _ in 0..turns {
                        let (next, events) = act(&st, Action::EndTurn);
                        st = next;
                        healed += events
                            .iter()
                            .filter_map(|e| match e {
                                Event::Healed { target, by: None, amount } if *target == mine => Some(*amount),
                                _ => None,
                            })
                            .sum::<i32>();
                    }
                    let want = if ground == Some(Ground::Spring) { SPRING_POINTS.min(wound) } else { 0 };
                    assert_eq!(healed, want, "{} ({side:?}, {}, рана {wound})", c.name, name_of(ground));
                    assert_eq!(hp(&st, mine), c.health - wound + want);
                }
            }
        }
    }
}

/// Яма: каждая карта, вышедшая из руки в яму, ранена землёй на `PIT_HARM` по
/// общему конвейеру — броня вычитается, но не ниже единицы; источник — земля.
#[test]
fn a_pit_wounds_every_card_played_into_it() {
    for c in shelf() {
        let mut st = MatchState::begin_with(
            Setup {
                player_board: vec![],
                player_hand: vec![c.clone()],
                keeper_board: vec![(CardSnapshot::new("страж", 1, 9, 1).with_step(0), cell(2, 0))],
                keeper_hand: vec![],
                terrain: vec![tile(1, 6, Ground::Pit)],
                field: long(),
            },
            rules(),
        );
        st.player.mana = 10;
        let (st, events) = act(&st, Action::Play { hand_index: 0, cell: cell(1, 6) });
        let id = id_at(&st, cell(1, 6));
        let harm = (PIT_POINTS - c.armor.max(0)).max(1);
        assert_eq!(c.health - hp(&st, id), harm, "{} в яме", c.name);
        assert!(
            events.iter().any(|e| matches!(e, Event::Damaged { source: Source::Zone, .. })),
            "{}: источник — земля",
            c.name
        );
    }
}

/// Шагнувшая в яму карта ранена тем же, и яма держит: дальше в этот ход нет.
/// То же — болото. Для каждой карты с шагом от двух; с шагом один «дальше»
/// не бывает.
#[test]
fn pit_and_mire_hold_every_card_that_could_run_through() {
    for c in shelf().into_iter().filter(|c| c.step >= 2) {
        for hold in [Ground::Pit, Ground::Mire] {
            // Узкий проход: стены по бокам, яма или болото посередине.
            let st = MatchState::begin_with(
                Setup {
                    player_board: vec![(c.clone(), cell(1, 7))],
                    player_hand: vec![],
                    keeper_board: vec![(CardSnapshot::new("страж", 1, 9, 1).with_step(0), cell(2, 0))],
                    keeper_hand: vec![],
                    terrain: vec![
                        tile(1, 6, hold),
                        tile(0, 6, Ground::Wall),
                        tile(2, 6, Ground::Wall),
                    ],
                    field: long(),
                },
                rules(),
            );
            let moves: Vec<Cell> = legal_actions(&st)
                .into_iter()
                .filter_map(|a| match a {
                    Action::Move { unit: 0, to } => Some(to),
                    _ => None,
                })
                .collect();
            assert!(moves.contains(&cell(1, 6)), "{}: в {hold:?} войти можно", c.name);
            assert!(moves.iter().all(|m| m.y >= 6), "{}: сквозь {hold:?} не пройти: {moves:?}", c.name);
        }
    }
}

/// Из болота выходят на одну клетку, какой бы ни был шаг у карты.
#[test]
fn from_a_mire_every_card_leaves_by_one_cell() {
    for c in shelf().into_iter().filter(|c| c.step >= 1) {
        let st = MatchState::begin_with(
            Setup {
                player_board: vec![(c.clone(), cell(1, 7))],
                player_hand: vec![],
                keeper_board: vec![(CardSnapshot::new("страж", 1, 9, 1).with_step(0), cell(2, 0))],
                keeper_hand: vec![],
                terrain: vec![tile(1, 7, Ground::Mire)],
                field: long(),
            },
            rules(),
        );
        for a in legal_actions(&st) {
            if let Action::Move { unit: 0, to } = a {
                assert!(to.distance(cell(1, 7)) <= 1, "{}: из болота на {to:?}", c.name);
            }
        }
    }
}

/// На стену и на овраг не встают: ни шагом, ни из руки; овраг перепрыгивают
/// только с шагом от двух.
#[test]
fn walls_and_ravines_are_never_stood_on_and_ravines_are_leapt_only_by_two() {
    for c in shelf() {
        let st = MatchState::begin_with(
            Setup {
                player_board: vec![(c.clone(), cell(1, 7))],
                player_hand: vec![c.clone()],
                keeper_board: vec![(CardSnapshot::new("страж", 1, 9, 1).with_step(0), cell(2, 0))],
                keeper_hand: vec![],
                terrain: vec![
                    tile(0, 7, Ground::Wall),
                    tile(1, 6, Ground::Ravine),
                    tile(0, 6, Ground::Wall),
                    tile(2, 6, Ground::Wall),
                    tile(0, 5, Ground::Ravine),
                ],
                field: long(),
            },
            rules(),
        );
        let mut st = st;
        st.player.mana = 10;
        let mut leapt = false;
        for a in legal_actions(&st) {
            match a {
                Action::Move { unit: 0, to } => {
                    assert!(!matches!(st.ground(to), Some(Ground::Wall | Ground::Ravine)), "{}: шаг на {to:?}", c.name);
                    leapt |= to == cell(1, 5);
                }
                Action::Play { cell: at, .. } => {
                    assert!(!matches!(st.ground(at), Some(Ground::Wall | Ground::Ravine)), "{}: выход на {at:?}", c.name);
                }
                _ => {}
            }
        }
        assert_eq!(leapt, c.step >= 2, "{}: прыжок через овраг (шаг {})", c.name, c.step);
    }
}

// ════════════════════════════════════════════════════════════════════════════
// 6. Укрытие
// ════════════════════════════════════════════════════════════════════════════

/// Укрытие не помогает от удара с соседней клетки и вполовину гасит издали — для
/// каждой пары карт; взятое число равно формуле, даже когда пол в 1 единицу
/// (`max(.., 1)`) перебивает половину.
#[test]
fn cover_halves_only_what_comes_from_afar() {
    let cards = shelf();
    for a in &cards {
        for t in &cards {
            for d in 1..=3u8 {
                let lost = |g: Option<Ground>| {
                    let st = duel(a, None, t, g, d);
                    if !offers(&st, &Action::Attack { attacker: 0, target: 1 }) {
                        return None;
                    }
                    let (after, _) = act(&st, Action::Attack { attacker: 0, target: 1 });
                    Some(t.health - hp(&after, 1))
                };
                let (open, covered) = (lost(None), lost(Some(Ground::Cover)));
                if d == 1 {
                    assert_eq!(open, covered, "{} → {} вплотную: укрытие не помогает", a.name, t.name);
                } else if let (Some(o), Some(c)) = (open, covered) {
                    assert!(c <= o, "{} → {} с {d}: укрытие не должно ухудшать защиту", a.name, t.name);
                }
            }
        }
    }
}

/// Глоток не лечит выше полного здоровья и не случается вхолостую: целый вампир
/// ударом не получает ничего, раненный на единицу — ровно единицу.
#[test]
fn the_sip_never_heals_above_full_health() {
    let v = card("the-vampire");
    let target = CardSnapshot::new("мишень", 1, 99, 1);
    for (wound, want) in [(0, 0), (1, 1), (2, 2), (3, 2)] {
        let mut st = duel(&v, None, &target, None, 1);
        st.units[0].health.current = v.health - wound;
        let (after, events) = act(&st, Action::Attack { attacker: 0, target: 1 });
        let got: i32 = events
            .iter()
            .filter_map(|e| match e {
                Event::Healed { target: 0, amount, .. } => Some(*amount),
                _ => None,
            })
            .sum();
        assert_eq!(got, want, "рана {wound}");
        assert_eq!(hp(&after, 0), v.health - wound + want);
    }
}
