use anyhow::{Context, Result};
use indexer_utils::sqlite::{sqlite_conn, sqlite_stmt};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use tracing::info;

pub mod cli;
pub mod config;
pub mod telemetry;

pub mod domain {
    pub mod event;
    pub mod indexer;
}

pub mod relay {
    pub mod event;
    pub mod record;
}

pub use config::Settings;
pub use domain::event::{IndexerEventKind, IndexerKey};
pub use relay::record::RelayEventRecord;

use crate::relay::event::RelayIndexerEvent;

pub async fn run(settings: Settings) -> Result<()> {
    let select_event_kinds = IndexerEventKind::ALL
        .iter()
        .map(|k| k.as_u64().to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let select_events_query = format!(
        "SELECT hex(event_hash), hex(author), created_at, kind, content \
         FROM event WHERE kind IN ({select_event_kinds})"
    );

    loop {
        let iteration_start = Instant::now();

        let relay_db_conn = sqlite_conn(&settings.relay.database_path).with_context(|| {
            format!(
                "Could not open relay database at {}",
                settings.relay.database_path
            )
        })?;

        let mut stmt = sqlite_stmt(&relay_db_conn, &select_events_query)
            .context("Could not prepare event query")?;

        let records: Vec<RelayEventRecord> = stmt
            .query_map([], RelayEventRecord::from_row)?
            .collect::<Result<_, _>>()
            .context("collecting RelayEventRecord rows")?;

        info!(record_count = records.len(), "Loaded RelayEventRecords");

        let records_by_kind: HashMap<IndexerEventKind, Vec<RelayIndexerEvent>> = records
            .into_iter()
            .map(RelayIndexerEvent::try_from)
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .fold(
                HashMap::<IndexerEventKind, Vec<RelayIndexerEvent>>::new(),
                |mut acc, ev| {
                    acc.entry(ev.kind).or_default().push(ev);
                    acc
                },
            );

        info!(
            records_count_by_kind = records_by_kind.len(),
            "Loaded RelayIndexerEvents"
        );

        // sleep
        let elapsed = iteration_start.elapsed();
        let interval = Duration::from_secs(settings.service.flush_interval);
        tokio::time::sleep(interval.saturating_sub(elapsed)).await;
    }

    #[allow(unreachable_code)]
    Ok(())
}
