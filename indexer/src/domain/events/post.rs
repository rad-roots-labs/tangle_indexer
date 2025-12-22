use radroots_events::post::RadrootsPostEventIndex;
use radroots_events_codec::{error::EventParseError, post::decode as post_decode};
use thiserror::Error;

use crate::relay::event::RelayIndexerEvent;

#[derive(Debug, Error)]
pub enum RadrootsPostEventIndexError {
    #[error("Failed to parse post event: {0}")]
    ParseError(#[from] EventParseError),
}

pub trait ToRadrootsPostEventIndex {
    fn to_radroots_post_event(
        &self,
    ) -> Result<RadrootsPostEventIndex, RadrootsPostEventIndexError>;
}

impl ToRadrootsPostEventIndex for RelayIndexerEvent {
    fn to_radroots_post_event(
        &self,
    ) -> Result<RadrootsPostEventIndex, RadrootsPostEventIndexError> {
        let kind_u32 = self.kind.as_u64() as u32;
        let index = post_decode::index_from_event(
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
