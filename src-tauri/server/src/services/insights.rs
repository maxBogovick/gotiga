//! The reads behind the "Behaviour" tab: how people use the rooms, what breaks,
//! how fast it loads, and whether the numbers can be trusted.
//!
//! A child module of `services` so it can reach `AppService::repo` and the
//! range helpers without widening their visibility.

use super::AppService;
use crate::analytics::health;
use crate::db::{
    BattleFunnelRow, DataTrust, DeadLinkSource, EngagementShape, ErrorRow, ExitRow,
    InteractionRow, SectionRow, TaleReadRow, VitalsRow,
};
use crate::error::Result;
use crate::models::AdminAnalyticsQuery;
use chrono::NaiveDate;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BehaviourReport {
    pub from: NaiveDate,
    pub to: NaiveDate,
    /// Rollups reach the whole range; the sections, the tales and the dead-link
    /// sources come from raw events and reach only this far back.
    pub raw_data_from: NaiveDate,
    pub shape: Vec<EngagementShape>,
    pub exits: Vec<ExitRow>,
    pub sections: Vec<SectionRow>,
    pub interactions: Vec<InteractionRow>,
    pub tales: Vec<TaleReadRow>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineHealth {
    /// Since the server last started: the counters live in memory.
    pub accepted: u64,
    pub written: u64,
    pub dropped_queue_full: u64,
    pub failed_batches: u64,
    pub failed_events: u64,
    pub skipped_bot: u64,
    pub skipped_admin: u64,
    pub skipped_dnt: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthReport {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub raw_data_from: NaiveDate,
    pub vitals: Vec<VitalsRow>,
    pub errors: Vec<ErrorRow>,
    pub dead_links: Vec<DeadLinkSource>,
    pub trust: DataTrust,
    pub pipeline: PipelineHealth,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleReport {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub challenges: Vec<BattleFunnelRow>,
    /// Signed-in people who came through the battles door at all.
    pub visitors: i64,
}

impl AppService {
    pub async fn admin_behaviour(&self, query: AdminAnalyticsQuery) -> Result<BehaviourReport> {
        let (from, to) = Self::analytics_range(query.from, query.to)?;
        let raw_from = Self::raw_data_floor().max(from);
        let (shape, exits, interactions) = tokio::try_join!(
            self.repo.get_engagement_shape(from, to),
            self.repo.get_exits(from, to),
            self.repo.get_interactions(from, to),
        )?;
        let (sections, tales) = tokio::try_join!(
            self.repo.get_last_sections(raw_from, to),
            self.repo.get_tale_reading(raw_from, to),
        )?;
        Ok(BehaviourReport {
            from,
            to,
            raw_data_from: raw_from,
            shape,
            exits,
            sections,
            interactions,
            tales,
        })
    }

    pub async fn admin_health(&self, query: AdminAnalyticsQuery) -> Result<HealthReport> {
        let (from, to) = Self::analytics_range(query.from, query.to)?;
        let raw_from = Self::raw_data_floor().max(from);
        let (vitals, errors) = tokio::try_join!(
            self.repo.get_vitals(from, to),
            self.repo.get_errors(from, to),
        )?;
        let (dead_links, trust) = tokio::try_join!(
            self.repo.get_dead_link_sources(raw_from, to),
            self.repo.get_data_trust(raw_from, to),
        )?;
        Ok(HealthReport {
            from,
            to,
            raw_data_from: raw_from,
            vitals,
            errors,
            dead_links,
            trust,
            pipeline: PipelineHealth {
                accepted: health::read(&health::ACCEPTED),
                written: health::read(&health::WRITTEN),
                dropped_queue_full: self.analytics.dropped_total(),
                failed_batches: health::read(&health::FAILED_BATCHES),
                failed_events: health::read(&health::FAILED_EVENTS),
                skipped_bot: health::read(&health::SKIPPED_BOT),
                skipped_admin: health::read(&health::SKIPPED_ADMIN),
                skipped_dnt: health::read(&health::SKIPPED_DNT),
            },
        })
    }

    pub async fn admin_battle_report(&self, query: AdminAnalyticsQuery) -> Result<BattleReport> {
        let (from, to) = Self::analytics_range(query.from, query.to)?;
        let (challenges, visitors) = tokio::try_join!(
            self.repo.get_battle_funnel(from, to),
            self.repo.count_battle_visitors(from, to),
        )?;
        Ok(BattleReport {
            from,
            to,
            challenges,
            visitors,
        })
    }

    /// Put a note on the analytics trend without anyone typing it: a release, a
    /// tale going out, a letter or a channel post. These are the causes the
    /// admin would otherwise have to remember a month later, when a spike needs
    /// explaining. Best-effort — a note that fails to save must never fail the
    /// thing it describes.
    pub async fn note_on_chart(&self, label: &str) {
        let day = chrono::Utc::now().date_naive();
        let label: String = label.chars().take(120).collect();
        if let Err(e) = self.repo.add_auto_annotation(day, &label).await {
            tracing::warn!(error = %e, %label, "could not add an automatic chart note");
        }
    }

    /// A new server binary means a release, and a release is the first thing to
    /// look for when a curve bends. The binary's own modification time is the
    /// fingerprint: it changes with every image build, needs no environment
    /// variable the container does not have, and stays put across a plain
    /// restart. Local runs are skipped — a developer's rebuild is not an event.
    pub async fn note_release_if_new(&self) {
        if self.config.public_url_is_local() {
            return;
        }
        let Some(built) = std::env::current_exe()
            .ok()
            .and_then(|p| std::fs::metadata(p).ok())
            .and_then(|m| m.modified().ok())
            .map(chrono::DateTime::<chrono::Utc>::from)
        else {
            return;
        };
        const KEY: &str = "analytics_build_seen";
        let stamp = built.to_rfc3339();
        match self.repo.get_setting(KEY).await {
            Ok(Some(seen)) if seen == stamp => return,
            Ok(_) => {}
            Err(_) => return,
        }
        if self.repo.upsert_setting(KEY, &stamp).await.is_err() {
            return;
        }
        self.note_on_chart(&format!("Release: server built {}", built.format("%d.%m %H:%M")))
            .await;
    }
}
