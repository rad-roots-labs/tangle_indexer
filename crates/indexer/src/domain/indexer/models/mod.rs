pub mod listing;
pub mod profile;

pub use listing::EventListingIndexes;
pub use profile::EventProfileIndexes;

use crate::{config::Settings, domain::indexer::IndexerKey};
use anyhow::Result;
use std::path::PathBuf;
use thiserror::Error;

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
}

pub trait WriteEventIndexes {
    fn write(&self, settings: &Settings, updated: &mut Vec<PathBuf>) -> Result<()>;
}
