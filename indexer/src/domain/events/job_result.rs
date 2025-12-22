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

#[cfg(test)]
mod tests {
    use super::ToRadrootsJobResultEventIndex;
    use crate::domain::indexer::kind::IndexerEventKind;
    use crate::relay::event::RelayIndexerEvent;
    use radroots_events::kinds::KIND_JOB_RESULT_MIN;

    fn make_event(tags: Vec<Vec<String>>) -> RelayIndexerEvent {
        RelayIndexerEvent {
            id: "1".repeat(64),
            author: "a".repeat(64),
            created_at: 10,
            pubkey: "a".repeat(64),
            kind: IndexerEventKind::JobResult(KIND_JOB_RESULT_MIN),
            tags,
            content: String::new(),
            hash: "2".repeat(64),
            sig: "3".repeat(64),
        }
    }

    #[test]
    fn job_result_decodes_request_reference() {
        let tags = vec![vec!["e".to_string(), "req123".to_string()]];
        let event = make_event(tags);
        let index = event.to_radroots_job_result_event().expect("job result index");
        assert_eq!(index.metadata.job_result.request_event.id, "req123");
    }

    #[test]
    fn job_result_requires_request_tag() {
        let event = make_event(Vec::new());
        assert!(event.to_radroots_job_result_event().is_err());
    }
}
