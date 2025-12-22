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

#[cfg(test)]
mod tests {
    use super::ToRadrootsJobRequestEventIndex;
    use crate::domain::indexer::kind::IndexerEventKind;
    use crate::relay::event::RelayIndexerEvent;
    use radroots_events::kinds::KIND_JOB_REQUEST_MIN;

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
    fn job_request_decodes_minimal_tags() {
        let event = make_event(
            IndexerEventKind::JobRequest(KIND_JOB_REQUEST_MIN),
            Vec::new(),
        );
        let index = event.to_radroots_job_request_event().expect("job request index");
        assert_eq!(index.metadata.kind, KIND_JOB_REQUEST_MIN);
    }

    #[test]
    fn job_request_rejects_wrong_kind() {
        let event = make_event(IndexerEventKind::Post, Vec::new());
        assert!(event.to_radroots_job_request_event().is_err());
    }
}
