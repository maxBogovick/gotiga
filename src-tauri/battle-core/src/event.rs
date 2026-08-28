//! What happened — as opposed to what was asked for.
//!
//! The client replays events, never actions, because one action becomes a
//! chain: a blow lands, thorns answer, a body falls, a dying gift fires. The
//! scene has to show that chain in order, and the journal has to be able to
//! reproduce it exactly.
//!
//! Events are immutable and ordered. Together they *are* the match.

use crate::board::{Cell, Side};
use crate::damage::{Breakdown, Channel, Source};
use crate::unit::UnitId;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(rename_all_fields = "camelCase")]
pub enum Event {
    /// A card left a hand and became a body.
    Played { side: Side, unit: UnitId, cell: Cell, cost: i32 },
    Moved { unit: UnitId, from: Cell, to: Cell },
    Damaged {
        target: UnitId,
        /// Who struck, when anyone did. `None` for a zone, a poison, a thorn —
        /// damage with no author.
        ///
        /// Carried for the scene, not for the rules: at reach four a blow is
        /// otherwise a number changing on the far side of the board with nobody
        /// visibly doing it, and the reader is left to guess which body moved.
        by: Option<UnitId>,
        to_health: i32,
        to_shield: i32,
        channel: Channel,
        source: Source,
        /// Why that number and not the one on the card. Every step of the
        /// pipeline that changed the figure, in order.
        ///
        /// Carried in the event rather than kept to the calculation because the
        /// question "it says eight, why did it take three" is asked by a reader,
        /// and answering it is the difference between a rule and a mystery.
        trail: Vec<Breakdown>,
    },
    /// Felt as nothing at all — a different picture from zero damage, and the
    /// scene should say so.
    Immune { target: UnitId, by: Option<UnitId>, channel: Channel },
    Healed { target: UnitId, amount: i32 },
    Died { target: UnitId },
    TurnEnded { side: Side, round: u8 },
    Finished { outcome: Outcome },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    Player,
    Keeper,
    Draw,
}
