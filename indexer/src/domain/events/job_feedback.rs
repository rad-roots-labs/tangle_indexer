use radroots_events::job_feedback::RadrootsJobFeedbackEventIndex;
use radroots_events_codec::{job::error::JobParseError, job::feedback::decode as job_feedback_decode};
use thiserror::Error;

use crate::relay::event::RelayIndexerEvent;

#[derive(Debug, Error)]
pub enum RadrootsJobFeedbackEventIndexError {
    #[error("Failed to parse job feedback event: {0}")]
    ParseError(#[from] JobParseError),
}

pub trait ToRadrootsJobFeedbackEventIndex {
    fn to_radroots_job_feedback_event(
        &self,
    ) -> Result<RadrootsJobFeedbackEventIndex, RadrootsJobFeedbackEventIndexError>;
}

impl ToRadrootsJobFeedbackEventIndex for RelayIndexerEvent {
    fn to_radroots_job_feedback_event(
        &self,
    ) -> Result<RadrootsJobFeedbackEventIndex, RadrootsJobFeedbackEventIndexError> {
        let kind_u32 = self.kind.as_u64() as u32;
        let index = job_feedback_decode::index_from_event(
            self.id.clone(),
            self.author.clone(),
            self.created_at,
            kind_u32,
            self.content.clone(),
            self.tags.clone(),
            self.sig.clone(),
        )?;
        Ok(index)
    }
}
