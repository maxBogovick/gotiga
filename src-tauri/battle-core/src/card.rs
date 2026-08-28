//! The card as the match sees it: frozen at the moment the match began.
//!
//! Not a reference to the row in the archive. The keeper edits cards; if a match
//! pointed at the living row, a rebalance would rewrite the history of every
//! match already played and every replay would start to lie.

use crate::damage::Channel;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardSnapshot {
    pub name: String,
    pub cost: i32,
    pub health: i32,
    pub power: i32,
    pub armor: i32,
    pub ward: i32,
    pub reach: u8,
    /// How many cells it walks in one move. Zero means it does not walk at all —
    /// a cauldron stands where it was put.
    pub step: u8,
    /// How much it mends in one act of mending. Zero — it does not mend.
    pub mend: i32,
    pub channel: Channel,
}

impl CardSnapshot {
    /// A plain body: the two-line card the first slice needs.
    pub fn new(name: &str, cost: i32, health: i32, power: i32) -> Self {
        Self {
            name: name.to_string(),
            cost,
            health,
            power,
            armor: 0,
            ward: 0,
            reach: 1,
            step: 1,
            mend: 0,
            channel: Channel::Physical,
        }
    }

    pub fn with_armor(mut self, armor: i32) -> Self {
        self.armor = armor;
        self
    }

    pub fn with_ward(mut self, ward: i32) -> Self {
        self.ward = ward;
        self
    }

    pub fn with_reach(mut self, reach: u8) -> Self {
        self.reach = reach;
        self
    }

    pub fn with_step(mut self, step: u8) -> Self {
        self.step = step;
        self
    }

    pub fn with_mend(mut self, mend: i32) -> Self {
        self.mend = mend;
        self
    }

    pub fn with_channel(mut self, channel: Channel) -> Self {
        self.channel = channel;
        self
    }
}
