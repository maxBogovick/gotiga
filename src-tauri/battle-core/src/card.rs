//! The card as the match sees it: frozen at the moment the match began.
//!
//! Not a reference to the row in the archive. The keeper edits cards; if a match
//! pointed at the living row, a rebalance would rewrite the history of every
//! match already played and every replay would start to lie.

use crate::damage::Channel;

/// One ability, frozen with the card.
///
/// The archive holds the full dictionary; the match only needs what `reduce`
/// reads. Unknown verbs sit here quietly — `legal_actions` never offers them,
/// so a card printed with a verb the engine does not yet run is not a trap.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbilitySnapshot {
    pub id: String,
    pub verb: String,
    pub amount: i32,
    pub shape: String,
    pub range: u8,
    pub mana_cost: i32,
    pub cooldown: u8,
    pub trigger: String,
}

impl AbilitySnapshot {
    /// An active heal of `amount` at `range`, for tests and the generator.
    pub fn heal(id: &str, amount: i32, range: u8) -> Self {
        Self {
            id: id.to_string(),
            verb: "heal".into(),
            amount,
            shape: "one".into(),
            range,
            mana_cost: 0,
            cooldown: 0,
            trigger: "active".into(),
        }
    }

    pub fn with_mana(mut self, mana: i32) -> Self {
        self.mana_cost = mana;
        self
    }

    pub fn with_cooldown(mut self, cooldown: u8) -> Self {
        self.cooldown = cooldown;
        self
    }

    pub fn on_self(mut self) -> Self {
        self.shape = "self".into();
        self
    }

    pub fn is_active_heal(&self) -> bool {
        self.verb == "heal" && self.trigger == "active" && self.amount > 0
    }
}

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
    /// Interim: a heal ability on the card takes over when present.
    pub mend: i32,
    pub channel: Channel,
    /// Бьёт ли это тело вообще.
    ///
    /// Отдельным полем, а не четвёртым каналом: канала три, и четвёртым он был
    /// бы не «ещё один вид урона», а его отсутствие — то есть слово не из того
    /// словаря. Котёл, знамя, лекарь без оружия стоят на поле и не наносят
    /// ударов; до этого поля хранитель мог сказать «не бьёт» в форме, а карта
    /// всё равно выходила и била.
    #[serde(default = "strikes_by_default")]
    pub strikes: bool,
    /// Abilities frozen with the card. Empty on every match begun before this
    /// field existed — `default` keeps their board caches readable.
    #[serde(default)]
    pub abilities: Vec<AbilitySnapshot>,
}

fn strikes_by_default() -> bool {
    true
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
            strikes: true,
            abilities: Vec::new(),
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

    pub fn with_ability(mut self, ability: AbilitySnapshot) -> Self {
        self.abilities.push(ability);
        self
    }

    /// Тело, которое стоит на поле и не наносит ударов.
    pub fn without_a_blow(mut self) -> Self {
        self.strikes = false;
        self
    }
}
