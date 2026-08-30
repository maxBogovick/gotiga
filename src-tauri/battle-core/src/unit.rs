//! The body: what a card becomes once it stands on the board.
//!
//! A card is a template and never changes. A unit is one copy of it in one
//! match, with its own wounds, its own riders and its own place. Two units of
//! the same card in one match is an ordinary thing, so they cannot be the same
//! type — merging them is the single most expensive mistake available here.

/// Identity of one unit inside one match. Not a card id: the same card played
/// twice yields two units.
pub type UnitId = u32;

/// What a rider modifies. A closed list — a new card brings a new combination,
/// never a new stat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Stat {
    /// Strength of what this unit deals.
    Power,
    /// Flat reduction of bodily damage.
    Armor,
    /// Flat reduction of charmed damage.
    Ward,
    /// Added to *everything* incoming, which is why it is priced above the
    /// others: it stacks with every ally's damage, not only with its own caster's.
    Vulnerable,
}

/// One rider on a unit: a blessing or a curse, with a name and a term.
///
/// The name is not decoration. It is what the stacking rule reads: a second
/// "Дым из печи" refreshes the term of the first instead of doubling its
/// magnitude — the difference between a game whose numbers can be budgeted and
/// one whose numbers avalanche.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub name: String,
    pub stat: Stat,
    /// Signed. A blessing is positive, a curse negative; `Vulnerable` is
    /// positive because it *adds* to incoming damage.
    pub amount: i32,
    /// Turns of the *bearer* remaining. Ticked by whoever owns the turn loop,
    /// which does not exist yet in this slice.
    pub turns: u8,
}

impl Status {
    pub fn new(name: &str, stat: Stat, amount: i32, turns: u8) -> Self {
        Self { name: name.to_string(), stat, amount, turns }
    }
}

/// Health is a number with a ceiling, and nothing else.
///
/// Mitigation deliberately does not live here. Armour is read by the damage
/// pipeline, by the card in the archive and by the points calculator; buried
/// inside a wrapper around health, every one of those would have to unwrap it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    pub current: i32,
    pub max: i32,
}

impl Health {
    pub fn full(max: i32) -> Self {
        Self { current: max, max }
    }

    pub fn is_dead(&self) -> bool {
        self.current <= 0
    }
}

fn strikes_by_default() -> bool {
    true
}

/// How many riders one unit may carry at once. Not a balance rule — a reading
/// rule. Past five the card can no longer be understood at a glance.
pub const STATUS_CAP: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Unit {
    pub id: UnitId,
    /// Whose body this is. Sides, not players: the keeper's side is a bot, and
    /// the rules must not care which.
    pub owner: crate::board::Side,
    /// How far its ordinary blow carries, in king's steps.
    pub reach: u8,
    /// How many cells it walks in one move.
    pub step: u8,
    /// How much it mends in one act of mending. Zero — it does not mend.
    pub mend: i32,
    /// Which defence answers its ordinary blow.
    pub channel: crate::damage::Channel,
    /// Бьёт ли это тело вообще. Читается с карты и в партии не меняется.
    #[serde(default = "strikes_by_default")]
    pub strikes: bool,
    /// Whether it has already struck, mended — or walked, when the rules say a
    /// walk spends the whole turn. One such act per body per turn is the whole
    /// of the action economy.
    pub acted: bool,
    /// Whether it has already walked this turn. Read when the rules let a body
    /// walk without spending its whole turn — which is the default; otherwise
    /// `acted` says it.
    /// Kept apart from `acted` so a body that walked can still be asked whether
    /// it may strike, which is the entire question the dial exists to answer.
    #[serde(default)]
    pub moved: bool,
    /// Whether it has already struck back during the enemy's turn. Retaliation
    /// is once per turn: without the cap, focusing one body would cost the
    /// attacker more than it costs the defender, and holding still would beat
    /// attacking at every board.
    #[serde(default)]
    pub retaliated: bool,
    /// The card this body was raised from, frozen. Kept because the fields
    /// below are the *current* numbers — wounded, blessed, cursed — and the
    /// card is what was printed. The name lives here and nowhere else.
    pub card: crate::card::CardSnapshot,
    pub health: Health,
    pub power: i32,
    pub armor: i32,
    pub ward: i32,
    /// Absorbs any channel and melts as it does. Placed before health and
    /// *after* the floor, so even the guaranteed single point of damage is
    /// caught by a shield rather than leaking through it.
    pub shield: i32,
    pub statuses: Vec<Status>,
    /// The one channel this unit does not feel at all. Rare, tier 5, one turn.
    pub immune: Option<crate::damage::Channel>,
}

impl Unit {
    pub fn new(id: UnitId, health: i32, power: i32) -> Self {
        Self {
            id,
            owner: crate::board::Side::Player,
            reach: 1,
            step: 1,
            mend: 0,
            channel: crate::damage::Channel::Physical,
            strikes: true,
            acted: false,
            moved: false,
            retaliated: false,
            card: crate::card::CardSnapshot::new("", 0, health, power),
            health: Health::full(health),
            power,
            armor: 0,
            ward: 0,
            shield: 0,
            statuses: Vec::new(),
            immune: None,
        }
    }

    /// Raise a body from a frozen card. The only way a unit is made in a match.
    pub fn from_card(id: UnitId, card: &crate::card::CardSnapshot, owner: crate::board::Side) -> Self {
        Self {
            id,
            owner,
            reach: card.reach,
            step: card.step,
            mend: card.mend,
            channel: card.channel,
            strikes: card.strikes,
            // A body placed this turn does not swing this turn. Without it,
            // mana buys damage outright and holding the field means nothing.
            acted: true,
            moved: false,
            retaliated: false,
            card: card.clone(),
            health: Health::full(card.health),
            power: card.power,
            armor: card.armor,
            ward: card.ward,
            shield: 0,
            statuses: Vec::new(),
            immune: None,
        }
    }

    pub fn with_owner(mut self, owner: crate::board::Side) -> Self {
        self.owner = owner;
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

    /// How much of this body is missing. What mending can actually put back.
    pub fn wound(&self) -> i32 {
        self.health.max - self.health.current
    }

    /// What the card says, as opposed to what the body currently is.
    pub fn name(&self) -> &str {
        &self.card.name
    }

    pub fn with_armor(mut self, armor: i32) -> Self {
        self.armor = armor;
        self
    }

    pub fn with_ward(mut self, ward: i32) -> Self {
        self.ward = ward;
        self
    }

    pub fn with_shield(mut self, shield: i32) -> Self {
        self.shield = shield;
        self
    }

    /// Sum of every rider touching one stat. Riders of different names add up;
    /// that is the whole of rule two.
    pub fn status_sum(&self, stat: Stat) -> i32 {
        self.statuses.iter().filter(|s| s.stat == stat).map(|s| s.amount).sum()
    }

    /// Lay a rider on this unit.
    ///
    /// Same name: the term is refreshed and the magnitude left alone. Different
    /// name: it joins the others. At the cap the oldest rider is displaced —
    /// the list is kept in the order laid, so the oldest is the first.
    ///
    /// This is the operation a chain of wrappers cannot perform without being
    /// taken apart and rebuilt, and it is required by the rules on page one.
    pub fn apply_status(&mut self, status: Status) {
        if let Some(existing) = self.statuses.iter_mut().find(|s| s.name == status.name) {
            existing.turns = existing.turns.max(status.turns);
            return;
        }
        if self.statuses.len() >= STATUS_CAP {
            self.statuses.remove(0);
        }
        self.statuses.push(status);
    }

    /// Lift every rider of one name — what `cleanse` and `dispel` are made of.
    /// Returns how many were lifted.
    pub fn clear_status(&mut self, name: &str) -> usize {
        let before = self.statuses.len();
        self.statuses.retain(|s| s.name != name);
        before - self.statuses.len()
    }

    /// Strength as a card would print it after riders — for showing, never for
    /// striking. The pipeline applies blessings and curses itself, and adding
    /// them here too would count them twice. Two places that compute the same
    /// number is the mistake this whole crate is arranged to avoid.
    pub fn printed_power(&self) -> i32 {
        (self.power + self.status_sum(Stat::Power)).max(0)
    }
}
