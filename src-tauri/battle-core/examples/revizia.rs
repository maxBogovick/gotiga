//! Ревизия: проверка утверждений о балансе замером, а не рассуждением.
//!
//! Пять вопросов, на которые нельзя ответить, читая код:
//!   1. Предсказывают ли весы исход? (Равные по очкам карты должны биться вничью.)
//!   2. Стоит ли шаг чего-нибудь на поле? (В весах он стоит нуль.)
//!   3. Имеет ли значение выбор? (Жадный против случайного.)
//!   4. Решена ли партия к третьему кругу?
//!   5. Сколько решений на ходу?
//!
//!     cargo run --release --example revizia

use battle_core::*;

struct Roll(u32);
impl Roll {
    fn next(&mut self, upto: u32) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 16) % upto
    }
}

// ── Весы, как их считает сервер (`battles.rs::body_points`) ────────────────
fn range_multiplier(reach: i32) -> f64 { 0.9 + 0.1 * reach.clamp(0, 5) as f64 }
fn body_points(c: &CardSnapshot, speed: i32) -> f64 {
    0.5 * c.health as f64
        + 1.2 * c.armor as f64
        + 1.2 * c.ward as f64
        + c.power as f64 * range_multiplier(c.reach as i32)
        + 2.0 * (speed - 1) as f64
        + 0.7 * c.mend as f64 * range_multiplier(c.reach as i32)
}

#[derive(Clone)]
struct Build { name: &'static str, card: CardSnapshot, speed: i32 }

fn build(name: &'static str, h: i32, p: i32, reach: u8, step: u8, armor: i32, mend: i32, speed: i32) -> Build {
    Build {
        name,
        card: CardSnapshot::new(name, 2, h, p)
            .with_reach(reach).with_step(step).with_armor(armor).with_mend(mend),
        speed,
    }
}

/// Партия двух сторон, каждая из трёх одинаковых тел, в обеих расстановках —
/// право первого хода вычитается само.
fn duel(a: &Build, b: &Build, seeds: u32) -> (u32, u32, u32, u32) {
    duel_under(a, b, seeds, Rules::default())
}

fn duel_under(a: &Build, b: &Build, seeds: u32, rules: Rules) -> (u32, u32, u32, u32) {
    let (mut wins_a, mut wins_b, mut draws, mut first_won) = (0, 0, 0, 0);
    for seed in 0..seeds {
        for swap in [false, true] {
            let mut r = Roll(seed * 31 + 7);
            let bodies = 2 + (seed % 2) as usize;
            let mut cells: Vec<(u8, u8)> = Vec::new();
            while cells.len() < bodies {
                let c = (r.next(3) as u8, r.next(3) as u8);
                if !cells.contains(&c) { cells.push(c); }
            }
            let near: Vec<(CardSnapshot, Cell)> = cells.iter()
                .map(|(x, y)| (a.card.clone(), Cell::new(*x, 5 - y).unwrap())).collect();
            let far: Vec<(CardSnapshot, Cell)> = cells.iter()
                .map(|(x, y)| (b.card.clone(), Cell::new(*x, *y).unwrap())).collect();
            let (player_board, keeper_board) = if swap {
                // A уезжает на половину хранителя, B — на половину гостя.
                (cells.iter().map(|(x, y)| (b.card.clone(), Cell::new(*x, 5 - y).unwrap())).collect::<Vec<_>>(),
                 cells.iter().map(|(x, y)| (a.card.clone(), Cell::new(*x, *y).unwrap())).collect::<Vec<_>>())
            } else { (near, far) };
            let setup = Setup { player_board, keeper_board, ..Default::default() };
            let mut st = MatchState::begin_with(setup, rules);
            while st.outcome.is_none() {
                let action = bot::choose(&st);
                let (next, _) = reduce(&st, &action).unwrap();
                st = next;
            }
            let a_side = if swap { Side::Keeper } else { Side::Player };
            if st.outcome == Some(Outcome::Player) { first_won += 1; }
            match st.outcome.unwrap() {
                Outcome::Draw => draws += 1,
                Outcome::Player if a_side == Side::Player => wins_a += 1,
                Outcome::Keeper if a_side == Side::Keeper => wins_a += 1,
                _ => wins_b += 1,
            }
        }
    }
    (wins_a, wins_b, draws, first_won)
}

/// Партия на расстоянии: A внизу, B наверху, в обе стороны и на разной
/// глубине. Шаг виден только там, где есть куда идти.
///
/// Возвращает долю побед A и долю партий, выигранных тем, кто ходил первым.
/// Второе — обязательная поправка: без неё ровно половина побед читается как
/// «карты равны», хотя означать может «карты ни при чём, решила очередь».
fn duel_far(a: &Build, b: &Build, bodies: usize, rules: Rules) -> (f64, f64) {
    let (mut wins_a, mut first, mut total) = (0u32, 0u32, 0u32);
    for swap in [false, true] {
        for shift in 0..3u8 {
            for (near_row, far_row) in [(5u8, 0u8), (5, 1), (4, 0), (3, 2)] {
                let cols: Vec<u8> = (0..bodies as u8).map(|i| (i + shift) % 3).collect();
                let lay = |card: &CardSnapshot, row: u8| -> Vec<(CardSnapshot, Cell)> {
                    cols.iter().map(|x| (card.clone(), Cell::new(*x, row).unwrap())).collect()
                };
                let (player_board, keeper_board) = if swap {
                    (lay(&b.card, near_row), lay(&a.card, far_row))
                } else {
                    (lay(&a.card, near_row), lay(&b.card, far_row))
                };
                let mut st = MatchState::begin_with(
                    Setup { player_board, keeper_board, ..Default::default() }, rules);
                while st.outcome.is_none() {
                    let (next, _) = reduce(&st, &bot::choose(&st)).unwrap();
                    st = next;
                }
                let a_side = if swap { Side::Keeper } else { Side::Player };
                total += 1;
                if st.outcome == Some(Outcome::Player) { first += 1; }
                match st.outcome.unwrap() {
                    Outcome::Player if a_side == Side::Player => wins_a += 1,
                    Outcome::Keeper if a_side == Side::Keeper => wins_a += 1,
                    _ => {}
                }
            }
        }
    }
    let n = total.max(1) as f64;
    (100.0 * wins_a as f64 / n, 100.0 * first as f64 / n)
}

/// Доля побед A, одним числом. Для калибровки.
fn share(a: &Build, b: &Build, seeds: u32) -> f64 {
    let (wa, wb, d, _) = duel(a, b, seeds);
    100.0 * wa as f64 / (wa + wb + d).max(1) as f64
}

/// Сколько здоровья заменяет силу: при какой стойкости тело с силой `p`
/// становится равным опорному h10 p5. Двоичный поиск по здоровью.
fn health_for_power(p: i32, seeds: u32) -> Option<i32> {
    let reference = build("опора", 10, 5, 1, 1, 0, 0, 1);
    let (mut lo, mut hi) = (4i32, 80i32);
    if share(&build("проба", hi, p, 1, 1, 0, 0, 1), &reference, seeds) < 50.0 { return None; }
    while lo < hi {
        let mid = (lo + hi) / 2;
        let s = share(&build("проба", mid, p, 1, 1, 0, 0, 1), &reference, seeds);
        if s >= 50.0 { hi = mid; } else { lo = mid + 1; }
    }
    Some(lo)
}

/// Случайный выбор из законных — «игрок, который не думает».
fn choose_random(st: &MatchState, r: &mut Roll) -> Action {
    let acts = legal_actions(st);
    // Пропуск хода последним в списке; берём из всего списка, включая его.
    acts[r.next(acts.len() as u32) as usize].clone()
}

/// Жадный против случайного, обе расстановки. Возвращает долю побед жадного.
fn thinker_versus_dice(seeds: u32) -> f64 {
    let (mut thinker, mut total) = (0u32, 0u32);
    for seed in 0..seeds {
        for greedy_is_player in [true, false] {
            let mut r = Roll(seed * 2_654_435_761 + 11);
            let setup = mirrored(seed);
            let mut st = MatchState::begin(setup);
            while st.outcome.is_none() {
                let mine = st.active == Side::Player;
                let action = if mine == greedy_is_player { bot::choose(&st) } else { choose_random(&st, &mut r) };
                let (next, _) = reduce(&st, &action).unwrap();
                st = next;
            }
            total += 1;
            let greedy_side = if greedy_is_player { Side::Player } else { Side::Keeper };
            match st.outcome.unwrap() {
                Outcome::Player if greedy_side == Side::Player => thinker += 1,
                Outcome::Keeper if greedy_side == Side::Keeper => thinker += 1,
                _ => {}
            }
        }
    }
    100.0 * thinker as f64 / total as f64
}

fn mirrored(seed: u32) -> Setup {
    mirrored_hand(seed, 3)
}

/// Та же зеркальная расстановка, что у `svodka`, с глубиной руки.
fn mirrored_hand(seed: u32, hand: u32) -> Setup {
    let mut r = Roll(seed);
    let mut setup = Setup::default();
    for _ in 0..(1 + r.next(3)) {
        let card = CardSnapshot::new("тело", 1, 3 + r.next(6) as i32, 1 + r.next(4) as i32)
            .with_reach(1 + r.next(3) as u8).with_step(1);
        let (x, y) = (r.next(3) as u8, r.next(3) as u8);
        let keeper = Cell::new(x, y).unwrap();
        if setup.keeper_board.iter().any(|(_, c)| *c == keeper) { continue; }
        setup.keeper_board.push((card.clone(), keeper));
        setup.player_board.push((card, Cell::new(x, 5 - y).unwrap()));
    }
    for _ in 0..hand {
        let held = CardSnapshot::new("рука", 1 + r.next(4) as i32, 3 + r.next(5) as i32, 1 + r.next(4) as i32);
        setup.keeper_hand.push(held.clone());
        setup.player_hand.push(held);
    }
    if setup.keeper_board.is_empty() { return mirrored_hand(seed + 7_919, hand); }
    setup
}

/// Решена ли партия рано, и сколько на ходу выбора.
fn shape_of_a_match(seeds: u32) -> (f64, f64, f64, f64) {
    let (mut led_and_won, mut led_total) = (0u32, 0u32);
    let mut choices: Vec<usize> = Vec::new();
    let mut one_choice = 0u32;
    let mut turns = 0u32;
    for seed in 0..seeds {
        let mut st = MatchState::begin(mirrored(seed));
        let mut lead_at_two: Option<Side> = None;
        while st.outcome.is_none() {
            let acts = legal_actions(&st);
            choices.push(acts.len());
            turns += 1;
            if acts.len() <= 1 { one_choice += 1; }
            if st.round == 3 && lead_at_two.is_none() {
                let p = st.standing_health(Side::Player);
                let k = st.standing_health(Side::Keeper);
                if p != k { lead_at_two = Some(if p > k { Side::Player } else { Side::Keeper }); }
            }
            let action = bot::choose(&st);
            let (next, _) = reduce(&st, &action).unwrap();
            st = next;
        }
        if let Some(leader) = lead_at_two {
            led_total += 1;
            let won = match st.outcome.unwrap() {
                Outcome::Player => Some(Side::Player),
                Outcome::Keeper => Some(Side::Keeper),
                Outcome::Draw => None,
            };
            if won == Some(leader) { led_and_won += 1; }
        }
    }
    choices.sort_unstable();
    let mean = choices.iter().sum::<usize>() as f64 / choices.len() as f64;
    let median = choices[choices.len() / 2] as f64;
    (
        100.0 * led_and_won as f64 / led_total.max(1) as f64,
        mean,
        median,
        100.0 * one_choice as f64 / turns as f64,
    )
}

// ── Экономика действий ────────────────────────────────────────────────────
//
// Пять правил, отличающихся только тем, что тело успевает за ход. Мерится то
// же, чем мерили правило первого хода (§13), плюс два числа, которых там не
// было: доля точек без выбора и то, кем решена партия равных карт.

/// Партия по заданным правилам. Возвращает исход и приборы по ходу дела.
fn watch(setup: Setup, rules: Rules) -> (MatchState, u32, u32, u32, u32) {
    let mut st = MatchState::begin_with(setup, rules);
    let (mut points, mut forced, mut acts, mut turns) = (0, 0, 0, 0);
    while st.outcome.is_none() {
        let offered = legal_actions(&st);
        points += 1;
        if offered.len() <= 1 { forced += 1; }
        let action = bot::choose(&st);
        if matches!(action, Action::EndTurn) { turns += 1; } else { acts += 1; }
        let (next, _) = reduce(&st, &action).unwrap();
        st = next;
    }
    (st, points, forced, acts, turns.max(1))
}

struct Shape {
    first: f64,
    rounds: f64,
    by_limit: f64,
    acts_per_turn: f64,
    forced: f64,
}

fn shape_under(runs: u32, rules: Rules) -> Shape {
    let (mut first, mut by_limit, mut rounds) = (0u32, 0u32, 0u32);
    let (mut points, mut forced, mut acts, mut turns) = (0u32, 0u32, 0u32, 0u32);
    for seed in 0..runs {
        let (st, p, f, a, t) = watch(mirrored(seed), rules);
        if st.outcome == Some(Outcome::Player) { first += 1; }
        if st.round > battle_core::state::MAX_ROUNDS { by_limit += 1; }
        rounds += st.round as u32;
        points += p; forced += f; acts += a; turns += t;
    }
    Shape {
        first: 100.0 * first as f64 / runs as f64,
        rounds: rounds as f64 / runs as f64,
        by_limit: 100.0 * by_limit as f64 / runs as f64,
        acts_per_turn: acts as f64 / turns as f64,
        forced: 100.0 * forced as f64 / points as f64,
    }
}

/// §12.2 в точности: ближний бой против неподвижных стрелков, обе расстановки.
fn melee_versus_ranged(rules: Rules) -> (u32, u32) {
    let body = |reach: u8, step: u8| CardSnapshot::new("тело", 1, 6, 3).with_reach(reach).with_step(step);
    let (mut melee, mut ranged) = (0, 0);
    for melee_step in [1u8, 2, 3] {
        for (swap, reach) in [(false, 2u8), (true, 2), (false, 3), (true, 3), (false, 4), (true, 4)] {
            let near = vec![
                (body(1, melee_step), Cell::new(0, 5).unwrap()),
                (body(1, melee_step), Cell::new(1, 5).unwrap()),
            ];
            let far = vec![
                (body(reach, 0), Cell::new(0, 0).unwrap()),
                (body(reach, 0), Cell::new(1, 0).unwrap()),
            ];
            let (player_board, keeper_board) = if swap {
                (far.iter().map(|(c, l)| (c.clone(), Cell::new(l.x, 5 - l.y).unwrap())).collect::<Vec<_>>(),
                 near.iter().map(|(c, l)| (c.clone(), Cell::new(l.x, 5 - l.y).unwrap())).collect::<Vec<_>>())
            } else { (near, far) };
            let (st, ..) = watch(Setup { player_board, keeper_board, ..Default::default() }, rules);
            let melee_side = if swap { Side::Keeper } else { Side::Player };
            match st.outcome.unwrap() {
                Outcome::Draw => {}
                Outcome::Player if melee_side == Side::Player => melee += 1,
                Outcome::Keeper if melee_side == Side::Keeper => melee += 1,
                _ => ranged += 1,
            }
        }
    }
    (melee, ranged)
}

/// Доля партий равных карт, которые выиграл тот, кто ходил первым.
/// 100 % — карты ни при чём, решает очередь.
fn initiative_decides(rules: Rules, seeds: u32) -> f64 {
    let a = CardSnapshot::new("а", 2, 10, 5);
    let b = CardSnapshot::new("б", 2, 8, 4).with_armor(2);
    let (mut first, mut total) = (0u32, 0u32);
    for seed in 0..seeds {
        for swap in [false, true] {
            let mut r = Roll(seed * 31 + 7);
            let mut cells: Vec<(u8, u8)> = Vec::new();
            while cells.len() < 3 {
                let c = (r.next(3) as u8, r.next(3) as u8);
                if !cells.contains(&c) { cells.push(c); }
            }
            let (x, y) = if swap { (&b, &a) } else { (&a, &b) };
            let setup = Setup {
                player_board: cells.iter().map(|(cx, cy)| (x.clone(), Cell::new(*cx, 5 - cy).unwrap())).collect(),
                keeper_board: cells.iter().map(|(cx, cy)| (y.clone(), Cell::new(*cx, *cy).unwrap())).collect(),
                ..Default::default()
            };
            let (st, ..) = watch(setup, rules);
            total += 1;
            if st.outcome == Some(Outcome::Player) { first += 1; }
        }
    }
    100.0 * first as f64 / total as f64
}

/// Доля побед первого хода при заданной глубине руки. §13.3: правило,
/// проверенное на одной глубине, не проверено.
fn first_share_hand(runs: u32, rules: Rules, hand: u32) -> f64 {
    let mut first = 0u32;
    for seed in 0..runs {
        let (st, ..) = watch(mirrored_hand(seed, hand), rules);
        if st.outcome == Some(Outcome::Player) { first += 1; }
    }
    100.0 * first as f64 / runs as f64
}

fn thinker_under(seeds: u32, rules: Rules) -> f64 {
    let (mut thinker, mut total) = (0u32, 0u32);
    for seed in 0..seeds {
        for greedy_is_player in [true, false] {
            let mut r = Roll(seed * 2_654_435_761 + 11);
            let mut st = MatchState::begin_with(mirrored(seed), rules);
            while st.outcome.is_none() {
                let mine = st.active == Side::Player;
                let action = if mine == greedy_is_player { bot::choose(&st) } else { choose_random(&st, &mut r) };
                let (next, _) = reduce(&st, &action).unwrap();
                st = next;
            }
            total += 1;
            let g = if greedy_is_player { Side::Player } else { Side::Keeper };
            match st.outcome.unwrap() {
                Outcome::Player if g == Side::Player => thinker += 1,
                Outcome::Keeper if g == Side::Keeper => thinker += 1,
                _ => {}
            }
        }
    }
    100.0 * thinker as f64 / total as f64
}

fn main() {
    const SEEDS: u32 = 400;

    println!("── 1. Предсказывают ли весы исход? ──");
    println!("Пары, которым весы дают ОДИНАКОВОЕ число очков при одной цене.");
    println!("Если весы верны, доля побед каждой ≈ 50 %.\n");
    let builds = vec![
        build("сила  h10 p5 r1", 10, 5, 1, 1, 0, 0, 1),
        build("даль  h10 p4 r3", 10, 4, 3, 1, 0, 0, 1),
        build("даль  h10 p3 r5", 10, 3, 5, 1, 0, 0, 1),
        build("тело  h14 p3 r1", 14, 3, 1, 1, 0, 0, 1),
        build("бронь h08 p4 a2", 8, 4, 1, 1, 2, 0, 1),
        build("прыть h10 p5 s3", 10, 5, 1, 3, 0, 0, 1),
        build("скор  h10 p5 v5", 10, 5, 1, 1, 0, 0, 5),
    ];
    println!("{:<20} {:>7}", "сложение", "очки");
    for b in &builds { println!("{:<20} {:>7.1}", b.name, body_points(&b.card, b.speed)); }
    println!("\n{:<20} {:<20} {:>12} {:>13}", "против", "", "побед первой", "решил ход");
    for i in 0..builds.len() {
        for j in (i + 1)..builds.len() {
            let (a, b, d, first) = duel(&builds[i], &builds[j], SEEDS);
            let total = (a + b + d).max(1);
            let sh = 100.0 * a as f64 / total as f64;
            let fm = 100.0 * first as f64 / total as f64;
            // «Решил ход» ≈ 100 % означает, что победила не карта, а право
            // первого хода, и число слева ничего про карты не говорит.
            let flag = if fm > 90.0 { "  ◀ мерил не карты" }
                else if (sh - 50.0).abs() > 15.0 { "  ◀ весы врут" } else { "" };
            println!("{:<20} {:<20} {:>11.1}% {:>12.1}%{}", builds[i].name, builds[j].name, sh, fm, flag);
        }
    }

    println!("\n── 1b. Настоящий курс «здоровье за силу» ──");
    println!("Опора: h10 p5 r1. Сколько здоровья нужно телу силы p, чтобы сравняться.");
    println!("Весы считают 1 силу = 2 здоровья (0.5 за здоровье, 1.0 за силу).\n");
    println!("{:>6} {:>16} {:>18} {:>14}", "сила", "нужно здоровья", "по весам хватило бы", "во сколько раз");
    for p in [2, 3, 4, 6, 7, 8] {
        match health_for_power(p, 120) {
            Some(h) => {
                let by_scale = 10 + 2 * (5 - p);
                println!("{:>6} {:>16} {:>18} {:>13.1}x", p, h, by_scale, h as f64 / by_scale as f64);
            }
            None => println!("{:>6} {:>16} {:>18} {:>14}", p, "не хватает 80", 10 + 2 * (5 - p), "—"),
        }
    }

    println!("\n── 1c. Обрыв: насколько тонко вообще можно настраивать ──");
    println!("Опора p5, против неё тело p3 с растущим здоровьем.");
    println!("Если доля побед прыгает с 0 % на 100 % за одно очко —");
    println!("никакая линейная формула предсказать исход не может.\n");
    for scale in [1i32, 2, 3] {
        let reference = build("опора", 10 * scale, 5, 1, 1, 0, 0, 1);
        print!("здоровье ×{scale}:  ");
        let mut band: Vec<i32> = Vec::new();
        for h in (6 * scale)..=(26 * scale) {
            let s = share(&build("проба", h, 3, 1, 1, 0, 0, 1), &reference, 60);
            if s > 25.0 && s < 75.0 { band.push(h); }
        }
        let widths: Vec<String> = ((6 * scale)..=(26 * scale)).step_by(2 * scale as usize)
            .map(|h| format!("{:.0}", share(&build("проба", h, 3, 1, 1, 0, 0, 1), &reference, 60)))
            .collect();
        println!("{}", widths.join(" "));
        println!("             полоса перехода (25–75 %): {} очк. здоровья", band.len());
    }

    println!("\n── 1d. Связь очков с победами на всей полке ──");
    println!("24 случайных тела, круговой турнир, каждое против каждого.");
    println!("Если весы — инструмент, очки должны предсказывать долю побед.\n");
    let mut r = Roll(4_242);
    let mut zoo: Vec<Build> = Vec::new();
    for _ in 0..24 {
        let h = 4 + r.next(16) as i32;
        let pw = 1 + r.next(7) as i32;
        let reach = 1 + r.next(3) as u8;
        let step = r.next(3) as u8;
        let armor = r.next(4) as i32;
        zoo.push(build("тело", h, pw, reach, step, armor, 0, 1));
    }
    let mut points = Vec::new();
    let mut wins = Vec::new();
    for i in 0..zoo.len() {
        let mut w = 0.0;
        let mut n = 0.0;
        for j in 0..zoo.len() {
            if i == j { continue; }
            w += share(&zoo[i], &zoo[j], 40);
            n += 1.0;
        }
        points.push(body_points(&zoo[i].card, zoo[i].speed));
        wins.push(w / n);
    }
    let mean = |v: &Vec<f64>| v.iter().sum::<f64>() / v.len() as f64;
    let (mp, mw) = (mean(&points), mean(&wins));
    let cov: f64 = points.iter().zip(&wins).map(|(p, w)| (p - mp) * (w - mw)).sum();
    let sp: f64 = points.iter().map(|p| (p - mp).powi(2)).sum::<f64>().sqrt();
    let sw: f64 = wins.iter().map(|w| (w - mw).powi(2)).sum::<f64>().sqrt();
    println!("связь очков и доли побед (Пирсон):  {:.2}", cov / (sp * sw));
    println!("(1.0 — весы предсказывают точно, 0 — не предсказывают ничего)\n");
    let mut order: Vec<usize> = (0..zoo.len()).collect();
    order.sort_by(|a, b| wins[*b].partial_cmp(&wins[*a]).unwrap());
    println!("{:>6} {:>8} {:>7} {:>6} {:>6} {:>7} {:>9}", "очки", "побед", "здор.", "сила", "даль", "бронь", "шаг");
    for i in order.iter().take(6).chain(order.iter().rev().take(3)) {
        let c = &zoo[*i].card;
        println!("{:>6.1} {:>7.0}% {:>7} {:>6} {:>6} {:>7} {:>9}",
            points[*i], wins[*i], c.health, c.power, c.reach, c.armor, c.step);
    }

    println!("\n── 1e. Где весы перестают работать ──");
    println!("Та же связь, но только среди тел, близких по очкам.\n");
    println!("{:>26} {:>8} {:>10}", "разброс очков в выборке", "тел", "связь");
    for width in [16.0f64, 8.0, 4.0, 2.0] {
        let mut r2 = Roll(90_210);
        let mut band: Vec<Build> = Vec::new();
        let mut guard = 0;
        while band.len() < 30 && guard < 200_000 {
            guard += 1;
            let h = 4 + r2.next(20) as i32;
            let pw = 1 + r2.next(8) as i32;
            let reach = 1 + r2.next(3) as u8;
            let step = r2.next(3) as u8;
            let armor = r2.next(4) as i32;
            let b = build("тело", h, pw, reach, step, armor, 0, 1);
            let pts = body_points(&b.card, 1);
            if (pts - 12.0).abs() <= width / 2.0 { band.push(b); }
        }
        let mut pts = Vec::new();
        let mut wr = Vec::new();
        for i in 0..band.len() {
            let mut w = 0.0; let mut n = 0.0;
            for j in 0..band.len() { if i != j { w += share(&band[i], &band[j], 30); n += 1.0; } }
            pts.push(body_points(&band[i].card, 1));
            wr.push(w / n);
        }
        let mean = |v: &Vec<f64>| v.iter().sum::<f64>() / v.len() as f64;
        let (mp, mw) = (mean(&pts), mean(&wr));
        let cov: f64 = pts.iter().zip(&wr).map(|(p, w)| (p - mp) * (w - mw)).sum();
        let sp: f64 = pts.iter().map(|p| (p - mp).powi(2)).sum::<f64>().sqrt();
        let sw: f64 = wr.iter().map(|w| (w - mw).powi(2)).sum::<f64>().sqrt();
        let rho = if sp * sw > 0.0 { cov / (sp * sw) } else { 0.0 };
        println!("{:>24} {:>10} {:>10.2}", format!("+/-{:.0}", width / 2.0), band.len(), rho);
    }

    println!("\n── 2. Имеет ли значение выбор? ──");
    println!("Жадный бот против случайного из тех же законных действий.");
    println!("50 % — решения не значат ничего; 100 % — значат всё.");
    println!("побед у думающего   {:>6.1}%", thinker_versus_dice(SEEDS));

    println!("\n── 3. Форма партии ──");
    let (early, mean, median, forced) = shape_of_a_match(SEEDS);
    println!("ведёт к 3-му кругу → и побеждает  {:>6.1}%", early);
    println!("законных действий на ходу, среднее{:>7.1}", mean);
    println!("                        медиана   {:>7.0}", median);
    println!("ходов без выбора (1 действие)     {:>6.1}%", forced);

    // ── 4 ────────────────────────────────────────────────────────────────
    println!("\n\n══ 4. ЭКОНОМИКА ДЕЙСТВИЙ ══\n");
    // «Как было» — экономика до правки: шаг тратил ход целиком, монета в две
    // маны. Оставлена в сетке, чтобы разницу было видно, а не помнить.
    let was = Rules {
        walk_spends_turn: true,
        second_side_coin: 2,
        point_blank_power: 100,
        ..Default::default()
    };
    // Штраф за упор в этой таблице держится выключенным: она про экономику
    // действий, и мешать в неё второе правило значило бы не измерить ни одного.
    let bare = Rules { point_blank_power: 100, ..Default::default() };
    let variants: Vec<(&str, Rules)> = vec![
        ("как было", was),
        ("шаг+удар (без штрафа)", bare),
        ("как было + сдача", Rules { retaliation: true, ..was }),
        ("шаг+удар + сдача", Rules { retaliation: true, ..bare }),
        ("одно действие за ход", Rules { acts_per_turn: 1, ..bare }),
        ("два действия за ход", Rules { acts_per_turn: 2, ..bare }),
    ];

    println!("{:<24} {:>8} {:>8} {:>8} {:>10} {:>9}", "правило", "1-й ход", "кругов", "лимитом", "дейст./ход", "без выбора");
    let mut shapes = Vec::new();
    for (name, rules) in &variants {
        let sh = shape_under(1_000, *rules);
        println!("{:<24} {:>7.1}% {:>8.1} {:>7.1}% {:>10.1} {:>8.1}%",
            name, sh.first, sh.rounds, sh.by_limit, sh.acts_per_turn, sh.forced);
        shapes.push(sh);
    }

    println!("\n§12.2 — ближний бой против стрелков, 18 партий (шаг 1–3 × дальность 2–4 × обе стороны)");
    println!("{:<24} {:>14} {:>10}", "правило", "ближний бой", "стрелки");
    for (name, rules) in &variants {
        let (m, r) = melee_versus_ranged(*rules);
        let flag = if m * 3 >= r { "  ◀ ближний бой проходим" } else { "" };
        println!("{:<24} {:>14} {:>10}{}", name, m, r, flag);
    }

    println!("\nКто выигрывает у равных карт: карта или очередь");
    println!("{:<24} {:>16} {:>18}", "правило", "решила очередь", "думающий/случайный");
    for (name, rules) in &variants {
        println!("{:<24} {:>15.1}% {:>17.1}%", name, initiative_decides(*rules, 200), thinker_under(200, *rules));
    }

    // Любая правка экономики отменяет калибровку правил открытия (§13):
    // они выбирались под нынешний темп. Пересчитываем для двух кандидатов.
    println!("\nПравила открытия надо перекалибровать под новую экономику");
    println!("{:<24} {:>10} {:>12} {:>10}", "экономика", "монета", "ударов в 1-м", "1-й ход");
    for (name, base) in [
        ("шаг+удар", bare),
        ("шаг+удар и сдача", Rules { retaliation: true, ..bare }),
    ] {
        for coin in [0, 1, 2, 3] {
            for opening in [1u8, 2] {
                let rules = Rules { second_side_coin: coin, opening_attacks: opening, ..base };
                let sh = shape_under(600, rules);
                let mark = if (sh.first - 50.0).abs() <= 5.0 { "  ◀ в коридоре" } else { "" };
                println!("{:<24} {:>10} {:>12} {:>9.1}%{}", name, coin, opening, sh.first, mark);
            }
        }
    }

    // ── 5 ────────────────────────────────────────────────────────────────
    println!("\n\n══ 5. СТРЕЛОК В УПОР ══\n");
    println!("Ручка: какую долю силы стрелок сохраняет, когда к нему подошли вплотную.");
    println!("100 — как сейчас, никакого штрафа.\n");

    // Тот же крайний случай, что в §12.2: голые стрелки против голого ближнего боя.
    // И честный: одинаковый бюджет, потраченный по-разному, — вопрос, который на
    // самом деле стоит перед хранителем, когда он рисует карту.
    let melee = build("ближний h14 p5 д1", 14, 5, 1, 1, 0, 0, 1);
    let archer = build("стрелок h14 p4 д3", 14, 4, 3, 1, 0, 0, 1);
    println!("равный бюджет: ближний {:.1} очк. · стрелок {:.1} очк.\n",
        body_points(&melee.card, 1), body_points(&archer.card, 1));

    // Одна пара ничего не доказывает: она могла оказаться удачной. Четыре
    // способа потратить один бюджет — ближний бой против стрелка каждый раз.
    let pairs: Vec<(&str, Build, Build)> = vec![
        ("крепкий / неподвижный", build("б", 14, 5, 1, 1, 0, 0, 1), build("с", 14, 4, 3, 0, 0, 0, 1)),
        ("крепкий / подвижный",   build("б", 14, 5, 1, 1, 0, 0, 1), build("с", 14, 4, 3, 1, 0, 0, 1)),
        ("сильный / живучий",     build("б", 10, 7, 1, 1, 0, 0, 1), build("с", 16, 4, 3, 1, 0, 0, 1)),
        ("живучий / сильный",     build("б", 18, 3, 1, 1, 0, 0, 1), build("с", 10, 6, 3, 1, 0, 0, 1)),
    ];
    print!("{:>8} {:>18}", "в упор", "§12.2");
    for (name, ..) in &pairs { print!(" {:>22}", *name); }
    println!(" {:>9} {:>9}", "1-й ход", "кругов");
    print!("{:>8} {:>18}", "", "ближний/стрелки");
    for _ in &pairs { print!(" {:>22}", "побед у ближнего"); }
    println!();
    for pb in [100u8, 75, 70, 65, 60, 50, 25, 0] {
        let rules = Rules { point_blank_power: pb, ..Default::default() };
        let (m, r) = melee_versus_ranged(rules);
        print!("{:>7}% {:>18}", pb, format!("{m}/{r}"));
        for (_, near, far) in &pairs {
            let (wm, wa, d, _) = duel_under(near, far, 200, rules);
            print!(" {:>21.0}%", 100.0 * wm as f64 / (wm + wa + d).max(1) as f64);
        }
        let sh = shape_under(600, rules);
        println!(" {:>8.1}% {:>9.1}", sh.first, sh.rounds);
    }

    println!("\nШтраф сдвинул честность — пересчёт монеты при штрафе в половину");
    let _ = 0;
    println!("{:>10} {:>14} {:>10}", "сила в упор", "монета", "1-й ход");
    for coin in [0i32, 1] {
        for opening in [0u8, 1, 2] {
            let (pb, rules) = (50u8, Rules { point_blank_power: 50, second_side_coin: coin, opening_attacks: opening, ..Default::default() });
            let sh = shape_under(1_000, rules);
            let (m, r) = melee_versus_ranged(rules);
            let mark = if (sh.first - 50.0).abs() <= 5.0 { "  ◀ в коридоре" } else { "" };
            let _ = pb;
            println!("{:>10} {:>14} {:>9.1}%   §12.2 {}/{}  очередь {:.0}%{}",
                format!("монета {coin}"), format!("ударов {opening}"), sh.first, m, r,
                initiative_decides(rules, 150), mark);
        }
    }

    // ── 6 ────────────────────────────────────────────────────────────────
    println!("\n\n══ 6. СТОИТ ЛИ ШАГ ЧЕГО-НИБУДЬ ТЕПЕРЬ ══\n");
    println!("Прежний замер («шаг 3 проигрывает шагу 1») сделан при старых правилах,");
    println!("когда шаг тратил ход целиком. Правила изменились — меряю заново.\n");
    let s1 = build("шаг 1", 10, 5, 1, 1, 0, 0, 1);
    let s2 = build("шаг 2", 10, 5, 1, 2, 0, 0, 1);
    let s3 = build("шаг 3", 10, 5, 1, 3, 0, 0, 1);
    println!("Все трое весят по {:.1} очк. — шаг в весах не стоит ничего.\n", body_points(&s1.card, 1));
    println!("{:<26} {:>12} {:>14} {:>14}", "", "в стычке", "на расстоянии", "решила очередь");
    for (name, a, b) in [
        ("шаг 2 против шага 1", &s2, &s1),
        ("шаг 3 против шага 1", &s3, &s1),
        ("шаг 3 против шага 2", &s3, &s2),
    ] {
        let (wa, wb, d, _) = duel(a, b, 300);
        let (far, queue) = duel_far(a, b, 2, Rules::default());
        println!("{:<26} {:>11.0}% {:>13.0}% {:>13.0}%", name,
            100.0 * wa as f64 / (wa + wb + d).max(1) as f64, far, queue);
    }

    println!("\nСколько он стоит: добавляем медленному телу здоровья, пока не сравняется");
    println!("(0.5 очка за единицу здоровья — по нынешней формуле)\n");
    println!("{:>10} {:>18} {:>16} {:>14}", "шаг", "здоровья вровень", "это очков", "в стычке / вдали");
    // Числа крупные намеренно: при здоровье 10 и силе 5 одно очко здоровья —
    // это целый лишний удар, и измерять шаг такой линейкой всё равно что
    // мерить миллиметры метровой палкой. При здоровье 50 удар стоит десятую
    // часть, и цена шага наконец становится различимой.
    for (base, step) in [(50i32, 2u8), (50, 3)] {
        let fast = build("быстрый", base, 5, 1, step, 0, 0, 1);
        let mut found: Option<i32> = None;
        for extra in 0..=30 {
            let slow = build("медленный", base + extra, 5, 1, 1, 0, 0, 1);
            let (wa, wb, d, _) = duel(&slow, &fast, 200);
            let close = 100.0 * wa as f64 / (wa + wb + d).max(1) as f64;
            let (far, _) = duel_far(&slow, &fast, 2, Rules::default());
            if close >= 50.0 && far >= 50.0 {
                found = Some(extra);
                println!("{:>10} {:>18} {:>15.1} {:>9.0}% / {:.0}%",
                    format!("шаг {step}"), format!("+{extra} к {base}"), 0.5 * extra as f64, close, far);
                break;
            }
        }
        if found.is_none() {
            println!("{:>10} {:>18} {:>15} {:>14}", format!("шаг {step}"), "не хватило +30", ">15", "—");
        }
    }

    println!("\n══ ВАШ ПРИМЕР: быстрый первый чин против стрелка третьего ══\n");
    // Бюджеты: первый чин 8 очков, третий — 20.
    let rusher = build("1 чин: h10 p3 шаг3", 10, 3, 1, 3, 0, 0, 1);
    let sniper = build("3 чин: h20 p8 д3", 20, 8, 3, 1, 0, 0, 1);
    println!("первый чин  {:.1} очк. (бюджет 8)", body_points(&rusher.card, 1));
    println!("третий чин  {:.1} очк. (бюджет 20)\n", body_points(&sniper.card, 1));
    for bodies in [1usize, 2, 3] {
        let (share, queue) = duel_far(&rusher, &sniper, bodies, Rules::default());
        println!("{} против {}: у первого чина {:.0}% побед  (очередь решила {:.0}%)",
            bodies, bodies, share, queue);
    }
    // И то же самое при равном бюджете: три дешёвых против одного дорогого —
    // так это и выглядит в колоде из шести карт.
    println!("\n3 тела первого чина (24 очка) против 1 тела третьего (20 очк.): {:.0}%",
        {
            let mut st_wins = 0u32; let mut total = 0u32;
            for swap in [false, true] {
                let three: Vec<(CardSnapshot, Cell)> = (0..3u8)
                    .map(|x| (rusher.card.clone(), Cell::new(x, if swap { 0 } else { 5 }).unwrap())).collect();
                let one = vec![(sniper.card.clone(), Cell::new(1, if swap { 5 } else { 0 }).unwrap())];
                let (player_board, keeper_board) = if swap { (one, three) } else { (three, one) };
                let mut st = MatchState::begin_with(Setup { player_board, keeper_board, ..Default::default() }, Rules::default());
                while st.outcome.is_none() {
                    let (next, _) = reduce(&st, &bot::choose(&st)).unwrap();
                    st = next;
                }
                total += 1;
                let rush_side = if swap { Side::Keeper } else { Side::Player };
                match st.outcome.unwrap() {
                    Outcome::Player if rush_side == Side::Player => st_wins += 1,
                    Outcome::Keeper if rush_side == Side::Keeper => st_wins += 1,
                    _ => {}
                }
            }
            100.0 * st_wins as f64 / total as f64
        });

    // ── 7 ────────────────────────────────────────────────────────────────
    println!("\n\n══ 7. ПЕРЕБОР У БОТА ══\n");
    println!("Глубина 1 — прежний жадный. Проверяю, что глубже действительно сильнее");
    println!("и что глубина 1 осталась ровно тем же ботом.\n");
    for depth in [1u8, 2, 3] {
        let (mut deep, mut total) = (0u32, 0u32);
        for seed in 0..200u32 {
            for deep_is_player in [true, false] {
                let mut st = MatchState::begin(mirrored(seed));
                while st.outcome.is_none() {
                    let mine = st.active == Side::Player;
                    let action = if mine == deep_is_player {
                        bot::choose_at(&st, depth)
                    } else {
                        bot::choose(&st)
                    };
                    let (next, _) = reduce(&st, &action).unwrap();
                    st = next;
                }
                total += 1;
                let deep_side = if deep_is_player { Side::Player } else { Side::Keeper };
                match st.outcome.unwrap() {
                    Outcome::Player if deep_side == Side::Player => deep += 1,
                    Outcome::Keeper if deep_side == Side::Keeper => deep += 1,
                    _ => {}
                }
            }
        }
        println!("глубина {} против глубины 1:  {:>5.1}% побед", depth,
            100.0 * deep as f64 / total as f64);
    }

    // Ступени должны отличаться не только от жадного, но и друг от друга —
    // иначе ручка снова обещает три положения и даёт два.
    {
        let (mut three, mut total) = (0u32, 0u32);
        for seed in 0..120u32 {
            for three_is_player in [true, false] {
                let mut st = MatchState::begin(mirrored(seed));
                while st.outcome.is_none() {
                    let mine = st.active == Side::Player;
                    let action = bot::choose_at(&st, if mine == three_is_player { 3 } else { 2 });
                    let (next, _) = reduce(&st, &action).unwrap();
                    st = next;
                }
                total += 1;
                let s3 = if three_is_player { Side::Player } else { Side::Keeper };
                match st.outcome.unwrap() {
                    Outcome::Player if s3 == Side::Player => three += 1,
                    Outcome::Keeper if s3 == Side::Keeper => three += 1,
                    _ => {}
                }
            }
        }
        println!("глубина 3 против глубины 2:  {:>5.1}% побед", 100.0 * three as f64 / total as f64);
    }

    // Сколько времени стоит один ход. На сервере это задержка ответа хранителя.
    {
        let st = MatchState::begin(mirrored(7));
        for depth in [1u8, 2, 3] {
            let start = std::time::Instant::now();
            for _ in 0..20 { std::hint::black_box(bot::choose_at(&st, depth)); }
            println!("один ход на глубине {}: {:>8.1} мс", depth,
                start.elapsed().as_secs_f64() * 1000.0 / 20.0);
        }
    }

    println!("\n── Решающая проверка (§13.3): держится ли кандидат при разной руке ──\n");
    let finalists: Vec<(&str, Rules)> = vec![
        ("как было", was),
        ("шаг+удар, без штрафа", bare),
        ("штраф ½ · монета 0 · ударов 0", Rules { second_side_coin: 0, opening_attacks: 0, ..Default::default() }),
        ("штраф ½ · монета 1 · удар 1", Rules { second_side_coin: 1, opening_attacks: 1, ..Default::default() }),
        ("штраф ½ · монета 1 · удара 2", Rules { second_side_coin: 1, opening_attacks: 2, ..Default::default() }),
    ];
    print!("{:<32}", "правило");
    for h in 1..=4 { print!(" {:>8}", format!("рука {h}")); }
    println!(" {:>9} {:>10} {:>8}", "разброс", "очередь", "кругов");
    for (name, rules) in &finalists {
        print!("{:<32}", name);
        let mut shares = Vec::new();
        for h in 1..=4 {
            let s = first_share_hand(800, *rules, h);
            shares.push(s);
            print!(" {:>7.1}%", s);
        }
        let spread = shares.iter().cloned().fold(f64::MIN, f64::max)
            - shares.iter().cloned().fold(f64::MAX, f64::min);
        let sh = shape_under(800, *rules);
        println!(" {:>6.1} п.п. {:>9.1}% {:>8.1}", spread, initiative_decides(*rules, 200), sh.rounds);
    }
}
