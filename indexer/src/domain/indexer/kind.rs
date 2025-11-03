use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::path::PathBuf;

use crate::domain::indexer::key::{
    IndexerKey, COMMENT_INDEX_DIRECTORY, LISTING_INDEX_DIRECTORY, PROFILE_INDEX_DIRECTORY,
    REACTION_INDEX_DIRECTORY,
};
use crate::utils::io::{paths_join, PathsError};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IndexerEventKind {
    Profile,
    Reaction,
    Listing,
    Comment,
}

impl IndexerEventKind {
    pub const ALL: [IndexerEventKind; 4] = [
        IndexerEventKind::Profile,
        IndexerEventKind::Reaction,
        IndexerEventKind::Listing,
        IndexerEventKind::Comment,
    ];

    pub const fn as_u64(self) -> u64 {
        match self {
            IndexerEventKind::Profile => 0,
            IndexerEventKind::Reaction => 7,
            IndexerEventKind::Listing => 30402,
            IndexerEventKind::Comment => 1111,
        }
    }

    pub const fn paths(self) -> &'static [IndexerKey] {
        match self {
            IndexerEventKind::Profile => &PROFILE_INDEX_DIRECTORY,
            IndexerEventKind::Reaction => &REACTION_INDEX_DIRECTORY,
            IndexerEventKind::Listing => &LISTING_INDEX_DIRECTORY,
            IndexerEventKind::Comment => &COMMENT_INDEX_DIRECTORY,
        }
    }

    pub fn base_path<P: AsRef<std::path::Path>>(self, data_dir: P) -> Result<PathBuf, PathsError> {
        paths_join(&[
            data_dir.as_ref().to_str().unwrap(),
            "static",
            "events",
            &self.as_u64().to_string(),
        ])
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
        match val {
            0 => Ok(IndexerEventKind::Profile),
            7 => Ok(IndexerEventKind::Reaction),
            30402 => Ok(IndexerEventKind::Listing),
            1111 => Ok(IndexerEventKind::Comment),
            other => Err(IndexerEventKindParseError(other)),
        }
    }
}
