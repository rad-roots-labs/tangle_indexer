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

#[cfg(test)]
mod tests {
    use super::ToRadrootsJobFeedbackEventIndex;
    use crate::domain::indexer::kind::IndexerEventKind;
    use crate::relay::event::RelayIndexerEvent;
    use radroots_events::kinds::KIND_JOB_FEEDBACK;

    fn make_event(tags: Vec<Vec<String>>) -> RelayIndexerEvent {
        RelayIndexerEvent {
            id: "1".repeat(64),
            author: "a".repeat(64),
            created_at: 10,
            pubkey: "a".repeat(64),
            kind: IndexerEventKind::JobFeedback,
            tags,
            content: String::new(),
            hash: "2".repeat(64),
            sig: "3".repeat(64),
        }
    }

    #[test]
    fn job_feedback_decodes_status() {
        let tags = vec![
            vec!["e".to_string(), "req123".to_string()],
            vec!["status".to_string(), "success".to_string()],
        ];
        let event = make_event(tags);
        let index = event
            .to_radroots_job_feedback_event()
            .expect("job feedback index");
        assert_eq!(index.metadata.kind, KIND_JOB_FEEDBACK);
    }

    #[test]
    fn job_feedback_requires_status_tag() {
        let tags = vec![vec!["e".to_string(), "req123".to_string()]];
        let event = make_event(tags);
        assert!(event.to_radroots_job_feedback_event().is_err());
    }
}
