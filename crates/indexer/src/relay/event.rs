use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::{
    domain::indexer::kind::{IndexerEventKind, IndexerEventKindParseError},
    RelayEventRecord,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RelayRawEvent {
    pub id: String,
    pub pubkey: String,
    pub created_at: u32,
    pub kind: u32,
    pub tags: Vec<Vec<String>>,
    pub content: String,
    pub sig: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RelayIndexerEvent {
    pub id: String,
    pub author: String,
    pub created_at: u32,
    pub pubkey: String,
    pub kind: IndexerEventKind,
    pub tags: Vec<Vec<String>>,
    pub content: String,
    pub hash: String,
    pub sig: String,
}

impl TryFrom<RelayEventRecord> for RelayIndexerEvent {
    type Error = anyhow::Error;

    fn try_from(rec: RelayEventRecord) -> Result<Self> {
        let raw: RelayRawEvent = serde_json::from_str(&rec.content)
            .with_context(|| format!("Failed to parse relay JSON for event {}", rec.event_hash))?;

        let kind = IndexerEventKind::try_from(raw.kind as u64)
            .map_err(|e: IndexerEventKindParseError| anyhow::anyhow!(e))?;

        Ok(RelayIndexerEvent {
            id: raw.id.to_lowercase(),
            author: rec.author.to_lowercase(),
            created_at: raw.created_at,
            pubkey: raw.pubkey.to_lowercase(),
            kind,
            tags: raw.tags,
            content: raw.content,
            hash: rec.event_hash,
            sig: raw.sig.to_lowercase(),
        })
    }
}
