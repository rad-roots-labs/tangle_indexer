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

#[cfg(test)]
mod tests {
    use super::ToRadrootsFollowEventIndex;
    use crate::domain::indexer::kind::IndexerEventKind;
    use crate::relay::event::RelayIndexerEvent;

    fn make_event(kind: IndexerEventKind, tags: Vec<Vec<String>>) -> RelayIndexerEvent {
        RelayIndexerEvent {
            id: "1".repeat(64),
            author: "a".repeat(64),
            created_at: 10,
            pubkey: "a".repeat(64),
            kind,
            tags,
            content: String::new(),
            hash: "2".repeat(64),
            sig: "3".repeat(64),
        }
    }

    #[test]
    fn follow_event_decodes_from_tags() {
        let tags = vec![vec!["p".to_string(), "b".repeat(64)]];
        let event = make_event(IndexerEventKind::Follow, tags);
        let index = event.to_radroots_follow_event().expect("follow index");
        assert_eq!(index.metadata.follow.list.len(), 1);
    }

    #[test]
    fn follow_event_rejects_wrong_kind() {
        let tags = vec![vec!["p".to_string(), "b".repeat(64)]];
        let event = make_event(IndexerEventKind::Post, tags);
        assert!(event.to_radroots_follow_event().is_err());
    }
}
