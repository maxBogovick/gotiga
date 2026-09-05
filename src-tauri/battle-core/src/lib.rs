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
//! The vocabulary is deliberately one word wide: a body strikes a body. Damage
//! over time, healing, shields, riders laid by abilities, zones, summons — all
//! of it arrives later as new verbs dispatched from the same `reduce`, without
//! reshaping the frame below.

//! # Как этим пользоваться
//!
//! Партия целиком — это цикл вокруг [`reduce`]. Служба на сервере будет делать
//! ровно это, только ход игрока придёт из запроса, а не от бота:
//!
//! ```
//! use battle_core::*;
//!
//! let setup = Setup {
//!     player_board: vec![(CardSnapshot::new("Боец", 1, 6, 3), Cell::new(1, 3).unwrap())],
//!     keeper_board: vec![(CardSnapshot::new("Ворон", 1, 4, 2), Cell::new(1, 2).unwrap())],
//!     ..Default::default()
//! };
//!
//! let mut state = MatchState::begin(setup);
//! let mut journal = Vec::new();
//!
//! while state.outcome.is_none() {
//!     // Клиент выбирает из этого списка и не знает ни одного правила.
//!     let offered = legal_actions(&state);
//!     assert!(!offered.is_empty());
//!
//!     let action = bot::choose(&state);
//!     let (next, events) = reduce(&state, &action).expect("законное действие");
//!     state = next;
//!     journal.extend(events);
//! }
//!
//! assert!(matches!(journal.last(), Some(Event::Finished { .. })));
//! ```
//!
//! Отдельный удар считается [`resolve`] и применяется [`apply`]. Считать можно
//! сколько угодно раз — счёт ничего не меняет, и на этом живёт перебор бота:
//!
//! ```
//! use battle_core::*;
//!
//! let mut striker = Unit::new(0, 10, 8);
//! striker.apply_status(Status::new("Проклятие", Stat::Power, -1, 2));
//! let target = Unit::new(1, 10, 0).with_armor(4);
//!
//! let res = resolve(
//!     Some(&striker),
//!     &target,
//!     DamagePacket::new(8, Channel::Physical, Source::Ability),
//! );
//!
//! assert_eq!(res.to_health, 3);
//! // И всегда можно спросить, почему 3, а не 8.
//! assert_eq!(res.trail[0].step, StepId::AttackerCurse);
//! assert_eq!(res.trail[1].step, StepId::ChannelDefence);
//! ```
//!
//! Как добавлять новые механики — в `README.md` рядом.

pub mod board;
pub mod bot;
pub mod card;
pub mod damage;
pub mod event;
pub mod heal;
pub mod state;
pub mod unit;

pub use board::{Board, Cell, Side, Spot};
pub use card::{AbilitySnapshot, CardSnapshot};
pub use damage::{Breakdown, Channel, DamagePacket, Resolution, Source, StepId, apply, resolve, strike};
pub use event::{Event, Outcome};
pub use heal::{Mending, apply_mend, resolve_mend};
pub use state::{Action, Illegal, MatchState, Rules, Setup, SideState, legal_actions, reduce};
pub use unit::{AbilityCooldown, Health, Stat, Status, Unit, UnitId};
