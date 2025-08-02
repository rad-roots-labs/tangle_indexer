use indexer_utils::sqlite::{RustqliteError, SqliteResult, SqliteRow, SqliteType};
use serde::Serialize;

use crate::domain::event::{IndexerEvent, IndexerEventParseError};

#[derive(Clone, Debug, Serialize)]
pub struct RelayEventRecord {
    pub event_hash: String,
    pub author: String,
    pub created_at: u32,
    pub kind: IndexerEvent,
    pub content: String,
}

impl RelayEventRecord {
    pub fn from_row(row: &SqliteRow) -> SqliteResult<Self> {
        let event_hash: String = row.get(0)?;
        let author: String = row.get(1)?;
        let created_at: u32 = row.get(2)?;
        let kind_num: u32 = row.get(3)?;

        let kind =
            IndexerEvent::try_from(kind_num as u64).map_err(|e: IndexerEventParseError| {
                RustqliteError::FromSqlConversionFailure(3, SqliteType::Integer, Box::new(e))
            })?;

        let content: String = row.get(4)?;
        Ok(RelayEventRecord {
            event_hash,
            author,
            created_at,
            kind,
            content,
        })
    }
}
