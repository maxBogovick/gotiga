//! Скромные эпические битвы — the rules, and only the rules.
//!
//! This crate holds one idea: a battle is a pure function of its state. Nothing
//! here reads a clock, touches a database, opens a socket or draws a random
//! number. Given the same input it returns the same output, byte for byte,
//! which is what makes three separate things possible at once — replaying a
//! recorded match to check a rules change, running ten thousand matches to
//! measure balance, and one day compiling the same code to WebAssembly instead
//! of writing the rules a second time in TypeScript.
//!
//! The first slice is deliberately narrow: one damage packet, one target, one
//! body. No board, no turns, no abilities. Everything that comes later adds
//! vocabulary — verbs, shapes, triggers — without reshaping what is here.

pub mod damage;
pub mod unit;

pub use damage::{Breakdown, Channel, DamagePacket, Event, Resolution, Source, apply, resolve, strike};
pub use unit::{Health, Stat, Status, Unit, UnitId};
