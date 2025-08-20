use anyhow::{Context, Result};
use indexer_utils::{
    database::IndexerDb,
    sqlite::{sqlite_conn, sqlite_stmt},
};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use tracing::info;

pub mod cli;
pub mod config;
pub mod telemetry;
pub mod domain {
    pub mod events;
    pub mod indexer;
    pub mod resolvers;
}
pub mod relay {
    pub mod event;
    pub mod record;
}

#[cfg(feature = "audit")]
pub mod audit;

#[cfg(not(feature = "audit"))]
pub mod audit {
    use radroots_events::{
        listing::models::RadrootsListingEventIndex, profile::models::RadrootsProfileEventIndex,
    };

    pub fn log_indexer_event(_: &crate::relay::event::RelayIndexerEvent) {}
    pub fn log_profile_event(_: &RadrootsProfileEventIndex) {}
    pub fn log_listing_event(_: &RadrootsListingEventIndex) {}
}

use crate::{
    domain::{
        indexer::{
            kind::IndexerEventKind,
            models::{EventIndexes, EventListingIndexes, EventProfileIndexes, WriteEventIndexes},
        },
        resolvers::profile::ProfileResolver,
    },
    relay::event::RelayIndexerEvent,
};
pub use config::Settings;
pub use relay::record::RelayEventRecord;

pub async fn run(settings: Settings) -> Result<()> {
    let db_idx = IndexerDb::open(&format!("{}/indexer_db", settings.indexer.data_dir))?;
    let tree_raw = "hashes";
    let tree_events_profile = "profile_events";
    let tree_events_listing = "listing_events";
    let tree_stats = "stats";

    let last_created_at_db: u32 = db_idx
        .get_raw(tree_stats, "last_created_at")?
        .map(|ivec| {
            let arr: [u8; 4] = ivec.as_ref().try_into().unwrap();
            u32::from_be_bytes(arr)
        })
        .unwrap_or(0);
    let mut last_created_at = last_created_at_db;

    let event_kinds = IndexerEventKind::ALL
        .iter()
        .map(|k| k.as_u64().to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let relay_query = format!(
        "SELECT hex(event_hash), hex(author), created_at, kind, content \
         FROM event WHERE kind IN ({}) AND created_at > ?",
        event_kinds
    );

    loop {
        let iteration_start = Instant::now();
        let relay_db = sqlite_conn(&settings.relay.database_path).with_context(|| {
            format!(
                "Could not open relay DB at {}",
                settings.relay.database_path
            )
        })?;
        let mut stmt =
            sqlite_stmt(&relay_db, &relay_query).context("Could not prepare event query")?;

        let records: Vec<RelayEventRecord> = stmt
            .query_map([last_created_at], RelayEventRecord::from_row)?
            .collect::<Result<_, _>>()
            .context("collecting RelayEventRecord rows")?;
        info!(record_count = records.len(), "Loaded relay records");

        let mut records_kind: HashMap<IndexerEventKind, Vec<RelayIndexerEvent>> = HashMap::new();
        for rec in records.into_iter() {
            let iev = RelayIndexerEvent::try_from(rec)?;
            audit::log_indexer_event(&iev);
            records_kind.entry(iev.kind).or_default().push(iev);
        }

        let mut need_rebuild_listing = false;

        if let Some(profile_events) = records_kind.remove(&IndexerEventKind::Profile) {
            if !profile_events.is_empty() {
                for ev in &profile_events {
                    last_created_at = last_created_at.max(ev.created_at);
                    let id = &ev.id;
                    let hash = &ev.hash;
                    let skip = if let Some(old) = db_idx.get_raw(tree_raw, id)? {
                        old.as_ref() == hash.as_bytes()
                    } else {
                        false
                    };
                    if skip {
                        continue;
                    }

                    db_idx.insert(tree_events_profile, id, ev)?;
                    db_idx.insert_raw(tree_raw, id, hash.as_bytes())?;
                }

                db_idx.insert_raw(
                    tree_stats,
                    "last_created_at",
                    &last_created_at.to_be_bytes(),
                )?;
                db_idx.flush()?;

                let raw_profile_events: Vec<RelayIndexerEvent> =
                    db_idx.get_all(tree_events_profile)?;
                let indexed_profile_events = EventProfileIndexes::build(&raw_profile_events)?;
                let mut updated_indexes = Vec::new();
                indexed_profile_events.write(&settings, &mut updated_indexes)?;
                info!(
                    written = updated_indexes.len(),
                    "Written {} index files",
                    updated_indexes.len()
                );

                need_rebuild_listing = true;
            }
        }

        let raw_profile_events: Vec<RelayIndexerEvent> = db_idx.get_all(tree_events_profile)?;
        let profiles = ProfileResolver::from_metadata(&raw_profile_events);

        if let Some(listing_events) = records_kind.remove(&IndexerEventKind::Listing) {
            if !listing_events.is_empty() {
                for ev in &listing_events {
                    last_created_at = last_created_at.max(ev.created_at);
                    let id = &ev.id;
                    let hash = &ev.hash;
                    let skip = if let Some(old) = db_idx.get_raw(tree_raw, id)? {
                        old.as_ref() == hash.as_bytes()
                    } else {
                        false
                    };
                    if skip {
                        continue;
                    }
                    db_idx.insert(tree_events_listing, id, ev)?;
                    db_idx.insert_raw(tree_raw, id, hash.as_bytes())?;
                }

                db_idx.insert_raw(
                    tree_stats,
                    "last_created_at",
                    &last_created_at.to_be_bytes(),
                )?;
                db_idx.flush()?;

                let raw_listing_events: Vec<RelayIndexerEvent> =
                    db_idx.get_all(tree_events_listing)?;
                let listing_indexes = EventListingIndexes::build(&raw_listing_events)?;
                let mut updated_listing = Vec::new();
                listing_indexes.write(&settings, &mut updated_listing)?;
                info!(
                    written = updated_listing.len(),
                    "Written {} listing index files",
                    updated_listing.len()
                );

                need_rebuild_listing = true;
            }
        }

        if need_rebuild_listing {
            let raw_listing_events: Vec<RelayIndexerEvent> = db_idx.get_all(tree_events_listing)?;
            let listing_indexes =
                EventListingIndexes::build_with_profiles(&raw_listing_events, &profiles)?;
            let mut updated_listing = Vec::new();
            listing_indexes.write(&settings, &mut updated_listing)?;
            info!(
                written = updated_listing.len(),
                "Written {} listing index files",
                updated_listing.len()
            );
        }

        let elapsed = iteration_start.elapsed();
        let interval = Duration::from_secs(settings.indexer.flush_interval);
        let delay = interval.saturating_sub(elapsed);
        info!(
            elapsed_ms = elapsed.as_millis(),
            sleeping_ms = delay.as_millis(),
            "Iteration complete"
        );
        tokio::time::sleep(delay).await;
    }
}
