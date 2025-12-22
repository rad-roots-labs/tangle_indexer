use radroots_events::job_request::RadrootsJobRequestEventIndex;
use radroots_events_codec::{job::error::JobParseError, job::request::decode as job_request_decode};
use thiserror::Error;

use crate::relay::event::RelayIndexerEvent;

#[derive(Debug, Error)]
pub enum RadrootsJobRequestEventIndexError {
    #[error("Failed to parse job request event: {0}")]
    ParseError(#[from] JobParseError),
}

pub trait ToRadrootsJobRequestEventIndex {
    fn to_radroots_job_request_event(
        &self,
    ) -> Result<RadrootsJobRequestEventIndex, RadrootsJobRequestEventIndexError>;
}

impl ToRadrootsJobRequestEventIndex for RelayIndexerEvent {
    fn to_radroots_job_request_event(
        &self,
    ) -> Result<RadrootsJobRequestEventIndex, RadrootsJobRequestEventIndexError> {
        let kind_u32 = self.kind.as_u64() as u32;
        let index = job_request_decode::index_from_event(
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
