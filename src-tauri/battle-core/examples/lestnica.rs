//! Лестница сложности: отличаются ли ступени друг от друга и сколько стоят.
//!
//!     cargo run --release --example lestnica

use battle_core::*;

struct Roll(u32);
impl Roll {
    fn next(&mut self, upto: u32) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 16) % upto
    }
}

fn mirrored(seed: u32) -> Setup {
    let mut r = Roll(seed);
    let mut setup = Setup::default();
    for _ in 0..(1 + r.next(3)) {
        let card = CardSnapshot::new("тело", 1, 3 + r.next(6) as i32, 1 + r.next(4) as i32)
            .with_reach(1 + r.next(3) as u8)
            .with_step(1);
        let (x, y) = (r.next(3) as u8, r.next(3) as u8);
        let keeper = Cell::new(x, y).unwrap();
        if setup.keeper_board.iter().any(|(_, c)| *c == keeper) {
            continue;
        }
        setup.keeper_board.push((card.clone(), keeper));
        setup.player_board.push((card, Cell::new(x, 5 - y).unwrap()));
    }
    for _ in 0..3 {
        let held =
            CardSnapshot::new("рука", 1 + r.next(4) as i32, 3 + r.next(5) as i32, 1 + r.next(4) as i32);
        setup.keeper_hand.push(held.clone());
        setup.player_hand.push(held);
    }
    if setup.keeper_board.is_empty() {
        return mirrored(seed + 7_919);
    }
    setup
}

/// Доля побед у стороны, играющей глубиной `a`, против глубины `b`.
fn ladder(a: u8, b: u8, seeds: u32) -> f64 {
    let (mut wins, mut total) = (0u32, 0u32);
    for seed in 0..seeds {
        for a_is_player in [true, false] {
            let mut st = MatchState::begin(mirrored(seed));
            while st.outcome.is_none() {
                let mine = st.active == Side::Player;
                let depth = if mine == a_is_player { a } else { b };
                let (next, _) = reduce(&st, &bot::choose_at(&st, depth)).unwrap();
                st = next;
            }
            total += 1;
            let a_side = if a_is_player { Side::Player } else { Side::Keeper };
            match st.outcome.unwrap() {
                Outcome::Player if a_side == Side::Player => wins += 1,
                Outcome::Keeper if a_side == Side::Keeper => wins += 1,
                _ => {}
            }
        }
    }
    100.0 * wins as f64 / total.max(1) as f64
}

fn main() {
    println!("── сколько стоит один ход ──");
    let st = MatchState::begin(mirrored(7));
    for depth in [1u8, 2] {
        let start = std::time::Instant::now();
        for _ in 0..10 {
            std::hint::black_box(bot::choose_at(&st, depth));
        }
        println!("глубина {}: {:>9.2} мс", depth, start.elapsed().as_secs_f64() * 100.0);
    }

    println!("\n── отличаются ли ступени друг от друга ──");
    for (a, b) in [(1u8, 1u8), (2, 1)] {
        println!("глубина {a} против глубины {b}: {:>6.1}%", ladder(a, b, 60));
    }
}
