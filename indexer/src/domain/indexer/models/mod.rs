pub mod comment;
pub mod follow;
pub mod job_feedback;
pub mod job_request;
pub mod job_result;
pub mod listing;
pub mod post;
pub mod profile;
pub mod reaction;

pub use comment::EventCommentIndexes;
pub use follow::EventFollowIndexes;
pub use job_feedback::EventJobFeedbackIndexes;
pub use job_request::EventJobRequestIndexes;
pub use job_result::EventJobResultIndexes;
pub use listing::EventListingIndexes;
pub use post::EventPostIndexes;
pub use profile::EventProfileIndexes;
pub use reaction::EventReactionIndexes;

use crate::{config::Settings, domain::indexer::key::IndexerKey};
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
