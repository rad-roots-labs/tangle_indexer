use anyhow::Result;
use radroots_events::{
    profile::models::{RadrootsProfile, RadrootsProfileEventIndex, RadrootsProfileEventMetadata},
    RadrootsNostrEvent,
};
use std::collections::HashMap;
use thiserror::Error;

use crate::relay::event::RelayIndexerEvent;

#[derive(Debug, Error)]
pub enum RadrootsProfileEventIndexError {
    #[error("Failed to parse metadata content JSON: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error("Missing or empty 'name' field in profile data")]
    MissingNameField,
    #[error("Missing or invalid 'published_at' tag")]
    MissingPublishedAt,
}

pub fn create_radroots_profile_event_metadata(
    id: String,
    author: String,
    published_at: u64,
    kind: u32,
    content: String,
    tags: Vec<Vec<String>>,
) -> Result<RadrootsProfileEventMetadata, RadrootsProfileEventIndexError> {
    let mut tag_map: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    for tag in tags {
        if let Some(key) = tag.get(0).map(String::as_str) {
            tag_map.entry(key.to_string()).or_default().push(tag);
        }
    }

    let profile: RadrootsProfile = serde_json::from_str(&content)?;
    if profile.name.trim().is_empty() {
        return Err(RadrootsProfileEventIndexError::MissingNameField);
    }

    Ok(RadrootsProfileEventMetadata {
        id,
        author,
        published_at,
        kind,
        profile,
    })
}

pub trait ToRadrootsProfileEventIndex {
    fn to_radroots_profile_event(
        self,
    ) -> Result<RadrootsProfileEventIndex, RadrootsProfileEventIndexError>;
}

impl ToRadrootsProfileEventIndex for RelayIndexerEvent {
    fn to_radroots_profile_event(
        self,
    ) -> Result<RadrootsProfileEventIndex, RadrootsProfileEventIndexError> {
        let kind_u32 = self.kind.as_u64() as u32;

        let metadata = create_radroots_profile_event_metadata(
            self.id.clone(),
            self.author.clone(),
            self.created_at as u64,
            kind_u32,
            self.content.clone(),
            self.tags.clone(),
        )?;

        Ok(RadrootsProfileEventIndex {
            event: RadrootsNostrEvent {
                id: self.id,
                author: self.author,
                created_at: self.created_at,
                kind: kind_u32,
                content: self.content,
                tags: self.tags,
                sig: self.sig,
            },
            metadata,
        })
    }
}
