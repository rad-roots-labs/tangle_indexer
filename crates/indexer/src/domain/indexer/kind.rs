use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

use crate::domain::indexer::{IndexerKey, METADATA_INDEX_DIRECTORY};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IndexerEventKind {
    Metadata,
}

impl IndexerEventKind {
    pub const ALL: [IndexerEventKind; 1] = [IndexerEventKind::Metadata];

    pub const fn as_u64(self) -> u64 {
        match self {
            IndexerEventKind::Metadata => 0,
        }
    }

    pub const fn paths(self) -> &'static [IndexerKey] {
        match self {
            IndexerEventKind::Metadata => &METADATA_INDEX_DIRECTORY,
        }
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
            other => Err(IndexerEventKindParseError(other)),
        }
    }
}
