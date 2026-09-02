//! Mending: the same two functions as damage, in the same order.
//!
//! `resolve_mend` works out what would happen and changes nobody;
//! `apply_mend` writes it and says what happened. The split is not decoration —
//! it is what lets the bot ask "what if" without a world of its own.
//!
//! No pipeline here yet. Mending has exactly one modifier so far — the ceiling
//! of the body being mended — and a one-step pipeline would be ceremony. When
//! the second modifier arrives (a curse that halves mending, a blessing that
//! deepens it), this grows the same array `damage.rs` already has.

use crate::event::Event;
use crate::unit::{Unit, UnitId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mending {
    /// What the mender offered.
    pub offered: i32,
    /// What the body could actually take. Never above what is missing.
    pub restored: i32,
}

/// Work out a mending. Changes nobody.
pub fn resolve_mend(target: &Unit, amount: i32) -> Mending {
    // Health does not rise above what the card says. Overflow is dropped rather
    // than carried, the mirror of overkill on the damage side.
    Mending { offered: amount, restored: amount.max(0).min(target.wound()) }
}

/// Write a mending into a body, and say what happened.
///
/// `by` is carried through rather than looked up afterwards: the scene shows
/// mending as a movement that starts at the mender, and the only place that
/// still knows who offered it is the call site.
pub fn apply_mend(by: Option<UnitId>, target: &mut Unit, mending: &Mending) -> Vec<Event> {
    if mending.restored == 0 {
        return Vec::new();
    }
    target.health.current += mending.restored;
    vec![Event::Healed { target: target.id, by, amount: mending.restored }]
}
