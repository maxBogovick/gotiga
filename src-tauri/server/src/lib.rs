// sqlx row mappings naturally produce wide tuples, and some repo writers take
// many columns as arguments; these style lints add no value here.
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

pub mod api;
pub mod config;
pub mod db;
pub mod error;
pub mod geo;
pub mod models;
pub mod services;
