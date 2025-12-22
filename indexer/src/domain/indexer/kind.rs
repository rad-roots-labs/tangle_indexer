use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::path::PathBuf;

use crate::domain::indexer::key::{
    COMMENT_INDEX_DIRECTORY, FOLLOW_INDEX_DIRECTORY, IndexerKey, JOB_FEEDBACK_INDEX_DIRECTORY,
    JOB_REQUEST_INDEX_DIRECTORY, JOB_RESULT_INDEX_DIRECTORY, LISTING_INDEX_DIRECTORY,
    POST_INDEX_DIRECTORY, PROFILE_INDEX_DIRECTORY, REACTION_INDEX_DIRECTORY,
};
use crate::utils::io::{paths_join, PathsError};
use radroots_events::kinds::{
    is_request_kind, is_result_kind, KIND_JOB_FEEDBACK, KIND_JOB_REQUEST_MAX, KIND_JOB_REQUEST_MIN,
    KIND_JOB_RESULT_MAX, KIND_JOB_RESULT_MIN,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IndexerEventKind {
    Profile,
    Post,
    Follow,
    Reaction,
    Listing,
    Comment,
    JobRequest(u32),
    JobResult(u32),
    JobFeedback,
}

impl IndexerEventKind {
    pub const GROUPS: [IndexerEventKind; 9] = [
        IndexerEventKind::Profile,
        IndexerEventKind::Post,
        IndexerEventKind::Follow,
        IndexerEventKind::Reaction,
        IndexerEventKind::Listing,
        IndexerEventKind::Comment,
        IndexerEventKind::JobRequest(KIND_JOB_REQUEST_MIN),
        IndexerEventKind::JobResult(KIND_JOB_RESULT_MIN),
        IndexerEventKind::JobFeedback,
    ];

    pub const fn as_u64(self) -> u64 {
        match self {
            IndexerEventKind::Profile => 0,
            IndexerEventKind::Post => 1,
            IndexerEventKind::Follow => 3,
            IndexerEventKind::Reaction => 7,
            IndexerEventKind::Listing => 30402,
            IndexerEventKind::Comment => 1111,
            IndexerEventKind::JobRequest(kind) => kind as u64,
            IndexerEventKind::JobResult(kind) => kind as u64,
            IndexerEventKind::JobFeedback => KIND_JOB_FEEDBACK as u64,
        }
    }

    pub const fn paths(self) -> &'static [IndexerKey] {
        match self {
            IndexerEventKind::Profile => &PROFILE_INDEX_DIRECTORY,
            IndexerEventKind::Post => &POST_INDEX_DIRECTORY,
            IndexerEventKind::Follow => &FOLLOW_INDEX_DIRECTORY,
            IndexerEventKind::Reaction => &REACTION_INDEX_DIRECTORY,
            IndexerEventKind::Listing => &LISTING_INDEX_DIRECTORY,
            IndexerEventKind::Comment => &COMMENT_INDEX_DIRECTORY,
            IndexerEventKind::JobRequest(_) => &JOB_REQUEST_INDEX_DIRECTORY,
            IndexerEventKind::JobResult(_) => &JOB_RESULT_INDEX_DIRECTORY,
            IndexerEventKind::JobFeedback => &JOB_FEEDBACK_INDEX_DIRECTORY,
        }
    }

    pub fn base_path<P: AsRef<std::path::Path>>(self, data_dir: P) -> Result<PathBuf, PathsError> {
        let kind_dir = self.as_u64().to_string();
        paths_join([
            data_dir.as_ref(),
            std::path::Path::new("static"),
            std::path::Path::new("events"),
            std::path::Path::new(&kind_dir),
        ])
    }

    pub fn group(self) -> Self {
        match self {
            IndexerEventKind::JobRequest(_) => IndexerEventKind::JobRequest(KIND_JOB_REQUEST_MIN),
            IndexerEventKind::JobResult(_) => IndexerEventKind::JobResult(KIND_JOB_RESULT_MIN),
            other => other,
        }
    }

    pub fn relay_kind_filter_sql() -> String {
        let exact = [
            IndexerEventKind::Profile.as_u64(),
            IndexerEventKind::Post.as_u64(),
            IndexerEventKind::Follow.as_u64(),
            IndexerEventKind::Reaction.as_u64(),
            IndexerEventKind::Listing.as_u64(),
            IndexerEventKind::Comment.as_u64(),
            IndexerEventKind::JobFeedback.as_u64(),
        ];
        let list = exact
            .iter()
            .map(|k| k.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let req = format!(
            "(kind BETWEEN {} AND {})",
            KIND_JOB_REQUEST_MIN, KIND_JOB_REQUEST_MAX
        );
        let res = format!(
            "(kind BETWEEN {} AND {})",
            KIND_JOB_RESULT_MIN, KIND_JOB_RESULT_MAX
        );
        format!("kind IN ({}) OR {} OR {}", list, req, res)
    }
}

impl fmt::Display for IndexerEventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_u64())
    }
}

impl Serialize for IndexerEventKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(self.as_u64())
    }
}

impl<'de> Deserialize<'de> for IndexerEventKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = u64::deserialize(deserializer)?;
        IndexerEventKind::try_from(v)
            .map_err(|_| DeError::custom(format!("invalid event kind: {}", v)))
    }
}

#[derive(thiserror::Error, Debug)]
#[error("unknown event kind: {0}")]
pub struct IndexerEventKindParseError(pub u64);

impl TryFrom<u64> for IndexerEventKind {
    type Error = IndexerEventKindParseError;

    fn try_from(val: u64) -> Result<Self, Self::Error> {
        let v = u32::try_from(val).map_err(|_| IndexerEventKindParseError(val))?;
        match v {
            0 => Ok(IndexerEventKind::Profile),
            1 => Ok(IndexerEventKind::Post),
            3 => Ok(IndexerEventKind::Follow),
            7 => Ok(IndexerEventKind::Reaction),
            30402 => Ok(IndexerEventKind::Listing),
            1111 => Ok(IndexerEventKind::Comment),
            KIND_JOB_FEEDBACK => Ok(IndexerEventKind::JobFeedback),
            _ if is_request_kind(v) => Ok(IndexerEventKind::JobRequest(v)),
            _ if is_result_kind(v) => Ok(IndexerEventKind::JobResult(v)),
            other => Err(IndexerEventKindParseError(other as u64)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::IndexerEventKind;
    use radroots_events::kinds::{KIND_JOB_REQUEST_MIN, KIND_JOB_RESULT_MIN};

    #[test]
    fn kind_grouping_uses_job_min() {
        let req = IndexerEventKind::JobRequest(KIND_JOB_REQUEST_MIN + 1);
        let res = IndexerEventKind::JobResult(KIND_JOB_RESULT_MIN + 1);
        assert_eq!(
            req.group(),
            IndexerEventKind::JobRequest(KIND_JOB_REQUEST_MIN)
        );
        assert_eq!(
            res.group(),
            IndexerEventKind::JobResult(KIND_JOB_RESULT_MIN)
        );
    }

    #[test]
    fn kind_try_from_job_ranges() {
        let req = IndexerEventKind::try_from(KIND_JOB_REQUEST_MIN as u64 + 5).unwrap();
        let res = IndexerEventKind::try_from(KIND_JOB_RESULT_MIN as u64 + 5).unwrap();
        assert_eq!(req.as_u64(), KIND_JOB_REQUEST_MIN as u64 + 5);
        assert_eq!(res.as_u64(), KIND_JOB_RESULT_MIN as u64 + 5);
    }

    #[test]
    fn kind_try_from_rejects_overflow() {
        let too_large = u64::from(u32::MAX) + 1;
        let err = IndexerEventKind::try_from(too_large).expect_err("overflow");
        assert_eq!(err.0, too_large);
    }
}
