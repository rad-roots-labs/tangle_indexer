use serde::ser::Serializer;
use serde::Serialize;
use std::fmt;

use crate::domain::event::{IndexerKey, METADATA_INDEX_DIRECTORY};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IndexerEvent {
    Metadata,
}

impl IndexerEvent {
    pub const ALL: [IndexerEvent; 1] = [IndexerEvent::Metadata];

    pub const fn as_u64(self) -> u64 {
        match self {
            IndexerEvent::Metadata => 0,
        }
    }

    pub const fn paths(self) -> &'static [IndexerKey] {
        match self {
            IndexerEvent::Metadata => &METADATA_INDEX_DIRECTORY,
        }
    }
}

impl fmt::Display for IndexerEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_u64())
    }
}

#[derive(thiserror::Error, Debug)]
#[error("unknown event kind: {0}")]
pub struct IndexerEventParseError(pub u64);

impl TryFrom<u64> for IndexerEvent {
    type Error = IndexerEventParseError;

    fn try_from(val: u64) -> Result<Self, Self::Error> {
        match val {
            0 => Ok(IndexerEvent::Metadata),
            other => Err(IndexerEventParseError(other)),
        }
    }
}

impl Serialize for IndexerEvent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(self.as_u64())
    }
}
