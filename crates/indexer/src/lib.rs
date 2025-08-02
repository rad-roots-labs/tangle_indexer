use anyhow::{Context, Result};
use indexer_utils::sqlite::{sqlite_conn, sqlite_stmt};
use std::time::{Duration, Instant};
use tracing::info;

pub mod cli;
pub mod config;
pub mod telemetry;

pub mod domain {
    pub mod event;
    pub mod indexer;
}

pub mod relay {
    pub mod model;
}

pub use config::Settings;
pub use domain::event::{IndexerEvent, IndexerKey};
pub use relay::model::RelayEventRecord;

pub async fn run(settings: Settings) -> Result<()> {
    let select_event_kinds = IndexerEvent::ALL
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

        // sleep
        let elapsed = iteration_start.elapsed();
        let interval = Duration::from_secs(settings.service.flush_interval);
        tokio::time::sleep(interval.saturating_sub(elapsed)).await;
    }

    #[allow(unreachable_code)]
    Ok(())
}
