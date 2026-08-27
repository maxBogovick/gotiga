//! Damage: the packet, the pipeline that shapes it, and the two events it leaves.
//!
//! The shape here is the alternative to a chain of decorators. Damage is plain
//! data; the "spices" are a fixed, ordered array of small pure functions over
//! it. Adding one is adding a line to `PIPELINE`; removing one is removing that
//! line. Unlike a wrapper chain, the order is written down in a single visible
//! place rather than implied by whoever built the chain, the packet can be
//! written to the journal and compared for equality, and every step can be
//! asked afterwards what it did — which is where `Breakdown` comes from.

use crate::unit::{Stat, Unit};

/// What kind of damage this is, and therefore which defence answers it.
///
/// Three, and there will not be more. An element wheel grows quadratically and,
/// worse, asks the player to remember which element they have already tried on
/// this card. Fire and ice stay words in the name of an ability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// Bodily. Answered by armour.
    Physical,
    /// Charmed. Answered by ward.
    Magic,
    /// The evil eye. Answered by nothing.
    Pure,
}

/// Where the damage came from.
///
/// Not decoration: thorns must not answer thorns or the engine loops forever,
/// and poison must not provoke thorns every turn. Later slices read this field;
/// it is here from the first line so that adding them is not a migration
/// through every call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Attack,
    Ability,
    Thorns,
    Dot,
    Zone,
    Splash,
    Recoil,
}

impl Source {
    /// Whether retaliation answers this kind of damage at all.
    pub fn provokes_thorns(self) -> bool {
        matches!(self, Source::Attack | Source::Ability)
    }

    /// Whether "when I am damaged" triggers fire for it.
    pub fn is_felt(self) -> bool {
        matches!(self, Source::Attack | Source::Ability | Source::Thorns | Source::Splash)
    }
}

/// One blow, before anything has been done to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamagePacket {
    pub amount: i32,
    pub channel: Channel,
    pub source: Source,
}

impl DamagePacket {
    pub fn new(amount: i32, channel: Channel, source: Source) -> Self {
        Self { amount, channel, source }
    }
}

/// What one step of the pipeline did. Kept only when it changed the number, so
/// the trail reads as an explanation rather than a log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Breakdown {
    pub step: &'static str,
    pub from: i32,
    pub to: i32,
}

/// The finished sum: what the pipeline decided, and how.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    /// After everything, including the shield. This is what health loses.
    pub to_health: i32,
    /// What the shield caught. Reported separately because the scene shows it
    /// differently and because the shield melts by exactly this much.
    pub to_shield: i32,
    pub channel: Channel,
    pub source: Source,
    pub trail: Vec<Breakdown>,
}

impl Resolution {
    /// Everything that got through, on either side of the shield.
    pub fn total(&self) -> i32 {
        self.to_health + self.to_shield
    }
}

/// What actually happened, for the scene and for the journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Damaged {
        target: crate::unit::UnitId,
        to_health: i32,
        to_shield: i32,
        channel: Channel,
        source: Source,
    },
    /// The blow was felt as nothing at all — a different picture from zero
    /// damage, and the scene should say so.
    Immune { target: crate::unit::UnitId, channel: Channel },
    Died { target: crate::unit::UnitId },
}

/// What a step is allowed to see: the blow, the one dealing it (there may be
/// none — a zone has no author), and the one receiving it.
struct Ctx<'a> {
    packet: &'a DamagePacket,
    attacker: Option<&'a Unit>,
    target: &'a Unit,
}

type Step = (&'static str, fn(i32, &Ctx) -> i32);

/// The order of application, written once, in one place.
///
/// This is the whole design decision in five lines. Anything a decorator would
/// have expressed by wrapping is expressed here by position, where it can be
/// read, tested and reordered deliberately.
const PIPELINE: &[Step] = &[
    ("благословения бьющего", step_attacker_bless),
    ("проклятия бьющего", step_attacker_curse),
    ("уязвимость цели", step_target_vulnerable),
    ("защита по каналу", step_channel_defence),
    ("минимум 1", step_floor),
];

fn step_attacker_bless(amount: i32, ctx: &Ctx) -> i32 {
    match ctx.attacker {
        Some(a) => amount + a.status_sum(Stat::Power).max(0),
        None => amount,
    }
}

fn step_attacker_curse(amount: i32, ctx: &Ctx) -> i32 {
    match ctx.attacker {
        Some(a) => amount + a.status_sum(Stat::Power).min(0),
        None => amount,
    }
}

fn step_target_vulnerable(amount: i32, ctx: &Ctx) -> i32 {
    amount + ctx.target.status_sum(Stat::Vulnerable)
}

fn step_channel_defence(amount: i32, ctx: &Ctx) -> i32 {
    let t = ctx.target;
    let defence = match ctx.packet.channel {
        Channel::Physical => t.armor + t.status_sum(Stat::Armor),
        Channel::Magic => t.ward + t.status_sum(Stat::Ward),
        // The evil eye skips this step entirely — that is the whole of it.
        Channel::Pure => 0,
    };
    amount - defence.max(0)
}

/// A blow that landed always takes at least one point.
///
/// Without this, armour 4 makes a unit untouchable by half the deck and the
/// match settles into an exchange with no ending. It is the same reason damage
/// does not heal between turns.
fn step_floor(amount: i32, _ctx: &Ctx) -> i32 {
    amount.max(1)
}

/// Work out what a blow does. Changes nothing — the caller decides whether to
/// apply it, and the scene can show the arithmetic either way.
pub fn resolve(attacker: Option<&Unit>, target: &Unit, packet: DamagePacket) -> Resolution {
    if target.immune == Some(packet.channel) {
        return Resolution {
            to_health: 0,
            to_shield: 0,
            channel: packet.channel,
            source: packet.source,
            trail: vec![Breakdown { step: "невосприимчивость", from: packet.amount, to: 0 }],
        };
    }

    let ctx = Ctx { packet: &packet, attacker, target };
    let mut amount = packet.amount;
    let mut trail = Vec::new();

    for (name, step) in PIPELINE {
        let next = step(amount, &ctx);
        if next != amount {
            trail.push(Breakdown { step: name, from: amount, to: next });
        }
        amount = next;
    }

    // The shield stands after the floor, not before it: a shield is supposed to
    // catch the guaranteed point too, otherwise it is weaker than armour of the
    // same size and no one would ever want one.
    let to_shield = amount.min(target.shield.max(0));
    let to_health = amount - to_shield;
    if to_shield > 0 {
        trail.push(Breakdown { step: "щит", from: amount, to: to_health });
    }

    Resolution { to_health, to_shield, channel: packet.channel, source: packet.source, trail }
}

/// Write a resolution into a body, and say what happened.
///
/// Split from `resolve` on purpose: everything above is arithmetic that can be
/// run a thousand times to answer "what if", and only this function is allowed
/// to change anyone.
pub fn apply(target: &mut Unit, res: &Resolution) -> Vec<Event> {
    let mut events = Vec::new();

    if res.total() == 0 && target.immune == Some(res.channel) {
        events.push(Event::Immune { target: target.id, channel: res.channel });
        return events;
    }

    target.shield -= res.to_shield;
    target.health.current -= res.to_health;

    events.push(Event::Damaged {
        target: target.id,
        to_health: res.to_health,
        to_shield: res.to_shield,
        channel: res.channel,
        source: res.source,
    });

    if target.health.is_dead() {
        // Death lifts every rider. Nothing outlives its bearer, which is why
        // the question "the witch died, does the hypnosis lift?" never arises.
        target.statuses.clear();
        events.push(Event::Died { target: target.id });
    }

    events
}

/// The ordinary blow: a unit strikes with its own strength.
pub fn strike(attacker: &Unit, target: &Unit) -> Resolution {
    let packet = DamagePacket::new(attacker.power, Channel::Physical, Source::Attack);
    resolve(Some(attacker), target, packet)
}
