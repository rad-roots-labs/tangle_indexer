pub use metadata::Event0StaticIndexes;

use crate::{config::Settings, domain::indexer::IndexerKey};
use anyhow::Result;
use serde_json::Value;
use std::path::PathBuf;
use thiserror::Error;

pub mod metadata;

#[derive(Debug, Error)]
pub enum NostrEventsStaticError {
    #[error("Failed to build static indexes: {0}")]
    BuildError(#[from] anyhow::Error),
}

pub trait EventIndexes {
    type Event;

    fn subdirs() -> &'static [IndexerKey];

    fn build(events: &[Self::Event]) -> Result<Self, NostrEventsStaticError>
    where
        Self: Sized;

    fn index_json(&self, subdir: IndexerKey) -> Option<Value>;
}

pub trait WriteEventIndexes {
    fn write(&self, settings: &Settings, updated: &mut Vec<PathBuf>) -> Result<()>;
}
