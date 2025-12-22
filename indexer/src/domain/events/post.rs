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

#[cfg(test)]
mod tests {
    use super::ToRadrootsPostEventIndex;
    use crate::domain::indexer::kind::IndexerEventKind;
    use crate::relay::event::RelayIndexerEvent;

    fn make_event(content: &str) -> RelayIndexerEvent {
        RelayIndexerEvent {
            id: "1".repeat(64),
            author: "a".repeat(64),
            created_at: 10,
            pubkey: "a".repeat(64),
            kind: IndexerEventKind::Post,
            tags: Vec::new(),
            content: content.to_string(),
            hash: "2".repeat(64),
            sig: "3".repeat(64),
        }
    }

    #[test]
    fn post_event_decodes_from_content() {
        let event = make_event("hello");
        let index = event.to_radroots_post_event().expect("post index");
        assert_eq!(index.metadata.post.content, "hello");
    }

    #[test]
    fn post_event_rejects_empty_content() {
        let event = make_event("");
        assert!(event.to_radroots_post_event().is_err());
    }
}
