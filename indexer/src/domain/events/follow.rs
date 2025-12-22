use radroots_events::follow::RadrootsFollowEventIndex;
use radroots_events_codec::{error::EventParseError, follow::decode as follow_decode};
use thiserror::Error;

use crate::relay::event::RelayIndexerEvent;

#[derive(Debug, Error)]
pub enum RadrootsFollowEventIndexError {
    #[error("Failed to parse follow event: {0}")]
    ParseError(#[from] EventParseError),
}

pub trait ToRadrootsFollowEventIndex {
    fn to_radroots_follow_event(
        &self,
    ) -> Result<RadrootsFollowEventIndex, RadrootsFollowEventIndexError>;
}

impl ToRadrootsFollowEventIndex for RelayIndexerEvent {
    fn to_radroots_follow_event(
        &self,
    ) -> Result<RadrootsFollowEventIndex, RadrootsFollowEventIndexError> {
        let kind_u32 = self.kind.as_u64() as u32;
        let index = follow_decode::index_from_event(
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
