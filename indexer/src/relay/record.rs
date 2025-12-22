use crate::utils::sqlite::{RustqliteError, SqliteResult, SqliteRow, SqliteType};
use serde::Serialize;

use crate::domain::indexer::kind::{IndexerEventKind, IndexerEventKindParseError};

#[derive(Clone, Debug, Serialize)]
pub struct RelayEventRecord {
    pub rowid: Option<u64>,
    pub event_hash: String,
    pub author: String,
    pub created_at: u32,
    pub kind: IndexerEventKind,
    pub content: String,
}

impl RelayEventRecord {
    pub fn from_row(row: &SqliteRow) -> SqliteResult<Self> {
        let event_hash: String = row.get(0)?;
        let author: String = row.get(1)?;
        let created_at: u32 = row.get(2)?;
        let kind_num: u32 = row.get(3)?;

        let kind = IndexerEventKind::try_from(kind_num as u64).map_err(
            |e: IndexerEventKindParseError| {
                RustqliteError::FromSqlConversionFailure(3, SqliteType::Integer, Box::new(e))
            },
        )?;

        let content: String = row.get(4)?;
        Ok(RelayEventRecord {
            rowid: None,
            event_hash,
            author,
            created_at,
            kind,
            content,
        })
    }

    pub fn from_row_with_rowid(row: &SqliteRow) -> SqliteResult<Self> {
        let rowid: u64 = row.get(0)?;
        let event_hash: String = row.get(1)?;
        let author: String = row.get(2)?;
        let created_at: u32 = row.get(3)?;
        let kind_num: u32 = row.get(4)?;

        let kind = IndexerEventKind::try_from(kind_num as u64).map_err(
            |e: IndexerEventKindParseError| {
                RustqliteError::FromSqlConversionFailure(4, SqliteType::Integer, Box::new(e))
            },
        )?;

        let content: String = row.get(5)?;
        Ok(RelayEventRecord {
            rowid: Some(rowid),
            event_hash,
            author,
            created_at,
            kind,
            content,
        })
    }
}
