use anyhow::Result;
use radroots_common::models::events::{
    RadrootsMetadataEvent, RadrootsMetadataEventData, RadrootsMetadataEventDataMetadata,
    RadrootsNostrEvent,
};
use std::collections::HashMap;
use thiserror::Error;

use crate::domain::events::RequiredField;
use crate::{opt_required, relay::event::RelayIndexerEvent};

#[derive(Debug, Error)]
pub enum RadrootsMetadataEventError {
    #[error("Failed to parse metadata content JSON: {0}")]
    ParseError(#[from] serde_json::Error),

    #[error("Missing or empty 'name' field in profile data")]
    MissingNameField,

    #[error("Missing or invalid 'published_at' tag")]
    MissingPublishedAt,
}

pub fn create_radroots_metadata_event_data(
    id: String,
    public_key: String,
    content: String,
    tags: Vec<Vec<String>>,
) -> Result<RadrootsMetadataEventData, RadrootsMetadataEventError> {
    let mut tag_map: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    for tag in tags {
        if let Some(key) = tag.get(0).map(String::as_str) {
            tag_map.entry(key.to_string()).or_default().push(tag);
        }
    }

    let get = |key: &str| -> Option<String> { tag_map.get(key)?.get(0)?.get(1).cloned() };

    let published_at_str = opt_required!(get("published_at"))
        .map_err(|_| RadrootsMetadataEventError::MissingPublishedAt)?;
    let published_at = published_at_str
        .parse::<u32>()
        .map_err(|_| RadrootsMetadataEventError::MissingPublishedAt)?;

    let metadata: RadrootsMetadataEventDataMetadata = serde_json::from_str(&content)?;
    if metadata.name.trim().is_empty() {
        return Err(RadrootsMetadataEventError::MissingNameField);
    }

    Ok(RadrootsMetadataEventData {
        id,
        public_key,
        metadata,
        published_at,
    })
}

pub trait ToRadrootsMetadataEvent {
    fn to_radroots_metadata_event(
        self,
    ) -> Result<RadrootsMetadataEvent, RadrootsMetadataEventError>;
}

impl ToRadrootsMetadataEvent for RelayIndexerEvent {
    fn to_radroots_metadata_event(
        self,
    ) -> Result<RadrootsMetadataEvent, RadrootsMetadataEventError> {
        let data = create_radroots_metadata_event_data(
            self.id.clone(),
            self.pubkey.clone(),
            self.content.clone(),
            self.tags.clone(),
        )?;

        let kind = self.kind.as_u64();

        Ok(RadrootsMetadataEvent {
            event: RadrootsNostrEvent {
                id: self.id,
                author: self.author,
                created_at: self.created_at,
                kind: kind.try_into().unwrap(),
                content: self.content,
                tags: self.tags,
                sig: self.sig,
            },
            data,
        })
    }
}
