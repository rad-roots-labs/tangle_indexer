use indexer_utils::paths::{paths_join, PathsError};
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::path::PathBuf;

use crate::domain::indexer::key::LISTING_INDEX_DIRECTORY;
use crate::domain::indexer::{IndexerKey, METADATA_INDEX_DIRECTORY};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IndexerEventKind {
    Metadata,
    Listing,
}

impl IndexerEventKind {
    pub const ALL: [IndexerEventKind; 2] = [IndexerEventKind::Metadata, IndexerEventKind::Listing];

    pub const fn as_u64(self) -> u64 {
        match self {
            IndexerEventKind::Metadata => 0,
            IndexerEventKind::Listing => 30402,
        }
    }

    pub const fn paths(self) -> &'static [IndexerKey] {
        match self {
            IndexerEventKind::Metadata => &METADATA_INDEX_DIRECTORY,
            IndexerEventKind::Listing => &LISTING_INDEX_DIRECTORY,
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
            0 => Ok(IndexerEventKind::Metadata),
            30402 => Ok(IndexerEventKind::Listing),
            other => Err(IndexerEventKindParseError(other)),
        }
    }
}
