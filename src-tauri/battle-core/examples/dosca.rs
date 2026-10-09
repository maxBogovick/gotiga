//! Форма доски: что она делает с ИГРОЙ, а не с картинкой.
//!
//! Вопрос пришёл как вопрос о вёрстке — «враг сверху или слева». Но у доски
//! `3 × 6` враг не может оказаться сверху, пока она остаётся шестирядной:
//! глубина, положенная вниз, делает поле узким. Значит, выбор картинки есть
//! выбор формы, а форма — это правила.
//!
//! Здесь меряется то же, чем мерили все прочие умолчания (`revizia`, `svodka`):
//!   1. Решает партию карта или очередь? (50 % — карта, 100 % — очередь.)
//!   2. Значат ли что-нибудь решения? (Жадный против случайного.)
//!   3. Из чего состоит партия: круги, удары, шаги, лимит.
//!   4. Сколько решений на ходу.
//!   5. Стоит ли чего-нибудь ШАГ.
//!   6. Жив ли стрелок как порода.
//!   7. Есть ли на доске ТЫЛ — клетка, до которой врагу за ход не дотянуться.
//!
//! Числа §1–§6 сняты прогоном, §7 — счётом по клеткам: это геометрия, и
//! statistics ей не нужна.
//!
//! Прогоняется на СОБРАННОЙ форме: `WIDTH`/`DEPTH` — константы крейта, и чтобы
//! сравнить две доски, надо собрать крейт дважды. Ничего в этом файле формы не
//! знает — все расстановки считаются из `WIDTH`/`DEPTH`.
//!
//!     cargo run --release --example dosca

// Замер формы доски 3 × 6 — поля по умолчанию.
const WIDTH: u8 = 3;
const DEPTH: u8 = 6;
use battle_core::*;

/// Сколько рядов у одной стороны.
const HALF: u8 = DEPTH / 2;

struct Roll(u32);
impl Roll {
    fn next(&mut self, upto: u32) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 16) % upto.max(1)
    }
}

/// Та же клетка на другой стороне стола.
fn face(cell: Cell) -> Cell {
    Cell::new(cell.x, DEPTH - 1 - cell.y).unwrap()
}

fn keeper_cells() -> Vec<Cell> {
    (0..HALF).flat_map(|y| (0..WIDTH).map(move |x| Cell::new(x, y).unwrap())).collect()
}

fn player_cells() -> Vec<Cell> {
    (HALF..DEPTH).flat_map(|y| (0..WIDTH).map(move |x| Cell::new(x, y).unwrap())).collect()
}

/// Зеркальная расстановка: всё, что отличает стороны, — право хода.
/// Тот же генератор, что у `svodka`, но клетки берутся из формы доски.
fn mirrored(seed: u32, hand: u32) -> Setup {
    let mut r = Roll(seed);
    let mut setup = Setup::default();
    for _ in 0..(1 + r.next(3)) {
        let card = CardSnapshot::new("тело", 1, 3 + r.next(6) as i32, 1 + r.next(4) as i32)
            .with_reach(1 + r.next(3) as u8)
            .with_step(1);
        let x = r.next(WIDTH as u32) as u8;
        let y = r.next(HALF as u32) as u8;
        let keeper = Cell::new(x, y).unwrap();
        if setup.keeper_board.iter().any(|(_, c)| *c == keeper) {
            continue;
        }
        setup.keeper_board.push((card.clone(), keeper));
        setup.player_board.push((card, face(keeper)));
    }
    for _ in 0..hand {
        let held = CardSnapshot::new("рука", 1 + r.next(4) as i32, 3 + r.next(5) as i32, 1 + r.next(4) as i32);
        setup.keeper_hand.push(held.clone());
        setup.player_hand.push(held);
    }
    if setup.keeper_board.is_empty() {
        return mirrored(seed + 7_919, hand);
    }
    setup
}

#[derive(Default)]
struct Tally {
    runs: u32,
    first: u32,
    draws: u32,
    by_limit: u32,
    rounds: u32,
    blows: u32,
    walks: u32,
    plays: u32,
    acts: u32,
    choices: Vec<usize>,
    idle_turns: u32,
    turns: u32,
    first_blow: u32,
    saw_blow: u32,
}

fn sweep(runs: u32) -> Tally {
    let mut t = Tally::default();
    for seed in 0..runs {
        let mut st = MatchState::begin(mirrored(seed, 3));
        let mut opened: Option<u8> = None;
        let mut acted_this_turn = false;
        while st.outcome.is_none() {
            let acts = legal_actions(&st);
            t.choices.push(acts.len());
            let action = bot::choose(&st);
            match action {
                Action::EndTurn => {
                    t.turns += 1;
                    if !acted_this_turn {
                        t.idle_turns += 1;
                    }
                    acted_this_turn = false;
                }
                _ => {
                    acted_this_turn = true;
                    t.acts += 1;
                    if matches!(action, Action::Move { .. }) {
                        t.walks += 1;
                    }
                    if matches!(action, Action::Play { .. }) {
                        t.plays += 1;
                    }
                }
            }
            let (next, events) = reduce(&st, &action).unwrap();
            for e in &events {
                if matches!(e, Event::Damaged { .. }) {
                    t.blows += 1;
                    if opened.is_none() {
                        opened = Some(st.round);
                    }
                }
            }
            st = next;
        }
        t.runs += 1;
        match st.outcome.unwrap() {
            Outcome::Player => t.first += 1,
            Outcome::Draw => t.draws += 1,
            Outcome::Keeper => {}
        }
        if st.round > battle_core::state::MAX_ROUNDS {
            t.by_limit += 1;
        }
        t.rounds += st.round as u32;
        if let Some(round) = opened {
            t.saw_blow += 1;
            t.first_blow += round as u32;
        }
    }
    t
}

fn choose_random(st: &MatchState, r: &mut Roll) -> Action {
    let acts = legal_actions(st);
    acts[r.next(acts.len() as u32) as usize].clone()
}

/// Жадный против случайного, обе расстановки. Доля побед думающего.
fn thinker_versus_dice(seeds: u32) -> f64 {
    let (mut thinker, mut total) = (0u32, 0u32);
    for seed in 0..seeds {
        for greedy_is_player in [true, false] {
            let mut r = Roll(seed * 2_654_435_761 + 11);
            let mut st = MatchState::begin(mirrored(seed, 3));
            while st.outcome.is_none() {
                let mine = st.active == Side::Player;
                let action = if mine == greedy_is_player {
                    bot::choose(&st)
                } else {
                    choose_random(&st, &mut r)
                };
                let (next, _) = reduce(&st, &action).unwrap();
                st = next;
            }
            total += 1;
            let greedy = if greedy_is_player { Side::Player } else { Side::Keeper };
            match st.outcome.unwrap() {
                Outcome::Player if greedy == Side::Player => thinker += 1,
                Outcome::Keeper if greedy == Side::Keeper => thinker += 1,
                _ => {}
            }
        }
    }
    100.0 * thinker as f64 / total as f64
}

/// Две стороны РАЗНЫХ тел, обе расстановки — право первого хода вычитается.
/// Возвращает долю побед первой стороны.
fn duel(a: &CardSnapshot, a_back: bool, b: &CardSnapshot, b_back: bool) -> f64 {
    let (mut wins, mut total) = (0u32, 0u32);
    let keeper = keeper_cells();
    let player = player_cells();
    // Передняя линия — ближние к середине клетки, тыл — дальние.
    let front_k: Vec<Cell> = keeper.iter().copied().filter(|c| c.y == HALF - 1).collect();
    let back_k: Vec<Cell> = keeper.iter().copied().filter(|c| c.y == 0).collect();
    let front_p: Vec<Cell> = player.iter().copied().filter(|c| c.y == HALF).collect();
    let back_p: Vec<Cell> = player.iter().copied().filter(|c| c.y == DEPTH - 1).collect();

    // Одной пары мало: две расстановки дают долю с шагом в полсотни процентов.
    // Меняем ещё и число тел, и запас здоровья — отличие, которое стоит чего-то
    // только при одном размере отряда, не стоит ничего.
    for swap in [false, true] {
    for bodies in 1..=2usize {
    for vigour in [5i32, 6, 7, 8] {
        let (mut a, mut b) = (a.clone(), b.clone());
        a.health = vigour;
        b.health = vigour;
        let (a, b) = (&a, &b);
        let take = |from: &Vec<Cell>| -> Vec<Cell> { from.iter().copied().take(bodies).collect() };
        let a_cells = if swap {
            take(if a_back { &back_k } else { &front_k })
        } else {
            take(if a_back { &back_p } else { &front_p })
        };
        let b_cells = if swap {
            take(if b_back { &back_p } else { &front_p })
        } else {
            take(if b_back { &back_k } else { &front_k })
        };
        if a_cells.len() < bodies || b_cells.len() < bodies {
            continue;
        }
        let a_board: Vec<_> = a_cells.into_iter().map(|c| (a.clone(), c)).collect();
        let b_board: Vec<_> = b_cells.into_iter().map(|c| (b.clone(), c)).collect();
        let setup = if swap {
            Setup { keeper_board: a_board, player_board: b_board, ..Default::default() }
        } else {
            Setup { player_board: a_board, keeper_board: b_board, ..Default::default() }
        };
        let a_side = if swap { Side::Keeper } else { Side::Player };
        let mut st = MatchState::begin(setup);
        while st.outcome.is_none() {
            let (next, _) = reduce(&st, &bot::choose(&st)).unwrap();
            st = next;
        }
        total += 1;
        match st.outcome.unwrap() {
            Outcome::Player if a_side == Side::Player => wins += 1,
            Outcome::Keeper if a_side == Side::Keeper => wins += 1,
            _ => {}
        }
    }}}
    100.0 * wins as f64 / total.max(1) as f64
}

/// ТЫЛ: доля своих клеток, до которых враг с таким шагом и такой дланью за один
/// ход не дотягивается. Считается по клеткам, а не прогоном: это геометрия.
///
/// Угроза — шаг плюс длань: шаг у нас хода не тратит (`walk_spends_turn` ложь),
/// значит тело подходит и бьёт в один ход.
fn tail(step: u8, reach: u8) -> f64 {
    let keeper = keeper_cells();
    let threat = (step + reach) as u8;
    let safe = player_cells()
        .into_iter()
        .filter(|p| keeper.iter().all(|k| k.distance(*p) > threat))
        .count();
    100.0 * safe as f64 / (WIDTH * HALF) as f64
}

fn main() {
    const RUNS: u32 = 2_000;

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  ДОСКА {WIDTH} в ширину × {DEPTH} в глубину  —  {:>2} клеток, по {:>2} на сторону  ║",
        WIDTH * DEPTH, WIDTH * HALF);
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let t = sweep(RUNS);
    let mean = t.choices.iter().sum::<usize>() as f64 / t.choices.len() as f64;
    let mut sorted = t.choices.clone();
    sorted.sort_unstable();
    let median = sorted[sorted.len() / 2];
    let dumb = sorted.iter().filter(|n| **n <= 1).count();

    println!("── 1. Решает карта или очередь? ──");
    println!("Зеркальные расстановки, {RUNS} партий. 50 % — решает карта.");
    println!("  побед у первого хода        {:>8.1}%", 100.0 * t.first as f64 / t.runs as f64);
    println!("  ничьих                      {:>8.1}%", 100.0 * t.draws as f64 / t.runs as f64);

    println!("\n── 2. Значат ли решения? ──");
    println!("Жадный бот против случайного, обе расстановки. 50 % — не значат.");
    println!("  побед у думающего           {:>8.1}%", thinker_versus_dice(400));

    println!("\n── 3. Форма партии ──");
    println!("  кругов в партии, среднее    {:>8.1}", t.rounds as f64 / t.runs as f64);
    println!("  решено лимитом кругов       {:>8.1}%", 100.0 * t.by_limit as f64 / t.runs as f64);
    println!("  первый удар на круге        {:>8.1}", t.first_blow as f64 / t.saw_blow.max(1) as f64);
    println!("  ходов, простоявших впустую  {:>8.1}%", 100.0 * t.idle_turns as f64 / t.turns.max(1) as f64);

    println!("\n── 4. Из чего состоит партия ──");
    let acts = t.acts.max(1) as f64;
    println!("  действий за партию          {:>8.1}", acts / t.runs as f64);
    println!("  из них шагов                {:>8.1}%", 100.0 * t.walks as f64 / acts);
    println!("  из них выставлений          {:>8.1}%", 100.0 * t.plays as f64 / acts);
    println!("  из них ударов и лечений     {:>8.1}%",
        100.0 * (t.acts - t.walks - t.plays) as f64 / acts);

    println!("\n── 5. Сколько решений на ходу ──");
    println!("  законных действий, среднее  {:>8.1}", mean);
    println!("                     медиана  {:>8}", median);
    println!("  точек без выбора            {:>8.1}%", 100.0 * dumb as f64 / sorted.len() as f64);

    println!("\n── 6. Стоит ли чего-нибудь ШАГ и ДАЛЬНОСТЬ ──");
    println!("Одинаковые тела, отличается одно. 50 % — отличие не стоит ничего.");
    let slow = CardSnapshot::new("тело", 1, 6, 3).with_reach(1).with_step(1);
    let fast = CardSnapshot::new("тело", 1, 6, 3).with_reach(1).with_step(2);
    println!("  шаг 2 против шага 1         {:>8.1}%", duel(&fast, false, &slow, false));
    for reach in [2u8, 3, 4, 5] {
        // Шаг у стрелка ТОТ ЖЕ, что у ближнего: иначе мерилась бы неподвижность,
        // а не дальность.
        let shooter = CardSnapshot::new("стрелок", 1, 6, 3).with_reach(reach).with_step(1);
        let melee = CardSnapshot::new("ближний", 1, 6, 3).with_reach(1).with_step(1);
        println!("  стрелок длани {reach} (в тылу) против ближнего {:>5.1}%",
            duel(&shooter, true, &melee, false));
    }

    println!("\n── 7. Есть ли ТЫЛ ──");
    println!("Доля своих клеток, до которых враг НЕ дотягивается за один ход.");
    println!("Ноль — прятать стрелка негде, и порода «стрелок» стоит в упор.");
    for (step, reach) in [(1u8, 1u8), (1, 2), (2, 1), (2, 2)] {
        println!("  против шага {step} и длани {reach}     {:>8.1}%", tail(step, reach));
    }
    // Точное число, ради которого всё это: сколько ШАГОВ от самой дальней своей
    // клетки до ближайшей чужой. Пока оно больше, чем «шаг + длань» ближнего
    // тела, у стрелка есть откуда стрелять; как только сравнялось — тыла нет.
    {
        let (k, p) = (keeper_cells(), player_cells());
        let deepest = p.iter().copied().max_by_key(|c| c.y).unwrap();
        let gap = k.iter().map(|a| a.distance(deepest)).min().unwrap();
        println!("\n  от своей ДАЛЬНЕЙ клетки до ближайшей чужой: {gap} шаг(а/ов)");
        println!("  ближний с шагом 1 и дланью 1 достаёт за: 2");
    }
    println!("\n  расстояний между сторонами: от {} до {}",
        {
            let (k, p) = (keeper_cells(), player_cells());
            k.iter().flat_map(|a| p.iter().map(move |b| a.distance(*b))).min().unwrap()
        },
        {
            let (k, p) = (keeper_cells(), player_cells());
            k.iter().flat_map(|a| p.iter().map(move |b| a.distance(*b))).max().unwrap()
        });
}
