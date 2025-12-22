use radroots_events::job_result::RadrootsJobResultEventIndex;
use radroots_events_codec::{job::error::JobParseError, job::result::decode as job_result_decode};
use thiserror::Error;

use crate::relay::event::RelayIndexerEvent;

#[derive(Debug, Error)]
pub enum RadrootsJobResultEventIndexError {
    #[error("Failed to parse job result event: {0}")]
    ParseError(#[from] JobParseError),
}

pub trait ToRadrootsJobResultEventIndex {
    fn to_radroots_job_result_event(
        &self,
    ) -> Result<RadrootsJobResultEventIndex, RadrootsJobResultEventIndexError>;
}

impl ToRadrootsJobResultEventIndex for RelayIndexerEvent {
    fn to_radroots_job_result_event(
        &self,
    ) -> Result<RadrootsJobResultEventIndex, RadrootsJobResultEventIndexError> {
        let kind_u32 = self.kind.as_u64() as u32;
        let index = job_result_decode::index_from_event(
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
