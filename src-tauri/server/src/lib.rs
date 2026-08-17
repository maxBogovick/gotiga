// sqlx row mappings naturally produce wide tuples, and some repo writers take
// many columns as arguments; these style lints add no value here.
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

pub mod analytics;
pub mod api;
pub mod config;
pub mod db;
pub mod depth;
pub mod embed;
pub mod error;
pub mod geo;
pub mod logs;
pub mod models;
pub mod observability;
pub mod search;
pub mod services;
pub mod slug;
