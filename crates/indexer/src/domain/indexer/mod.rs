use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Result};
use indexer_utils::file::fs_mkdir;

use crate::{
    config::Settings,
    domain::indexer::{
        kind::IndexerEventKind,
        models::{Event0StaticIndexes, EventIndexes, WriteEventIndexes},
    },
    relay::event::RelayIndexerEvent,
};

pub mod key;
pub mod kind;
pub mod models;

pub use key::{IndexerKey, METADATA_INDEX_DIRECTORY};

pub fn create_index_dirs(settings: &Settings) -> Result<()> {
    for kind in IndexerEventKind::ALL {
        let kind_str = kind.as_u64().to_string();

        for subdir in kind.paths() {
            fs_mkdir(&[
                settings.service.output_dir.as_str(),
                "events",
                &kind_str,
                subdir.as_str(),
            ])
            .with_context(|| {
                format!(
                    "Failed to create directory for kind {} / {}",
                    kind_str,
                    subdir.as_str()
                )
            })?;
        }
    }
    Ok(())
}

pub fn write_index_events(
    settings: &Settings,
    events_by_kind: &HashMap<IndexerEventKind, Vec<RelayIndexerEvent>>,
) -> Result<Vec<PathBuf>> {
    let mut updated = Vec::new();

    for &kind in &IndexerEventKind::ALL {
        let events = events_by_kind.get(&kind).cloned().unwrap_or_default();
        match kind {
            IndexerEventKind::Metadata => {
                let idx =
                    Event0StaticIndexes::build(&events).context("building indexes for Metadata")?;
                idx.write(settings, &mut updated)?;
            }
        }
    }

    Ok(updated)
}
