use anyhow::Result;
use radroots_events::{
    profile::{RadrootsProfile, RadrootsProfileEventIndex, RadrootsProfileEventMetadata},
    RadrootsNostrEvent,
};
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
    published_at: u32,
    kind: u32,
    content: &str,
) -> Result<RadrootsProfileEventMetadata, RadrootsProfileEventIndexError> {
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
        &self,
    ) -> Result<RadrootsProfileEventIndex, RadrootsProfileEventIndexError>;
}

impl ToRadrootsProfileEventIndex for RelayIndexerEvent {
    fn to_radroots_profile_event(
        &self,
    ) -> Result<RadrootsProfileEventIndex, RadrootsProfileEventIndexError> {
        let kind_u32 = self.kind.as_u64() as u32;
        let id = self.id.clone();
        let author = self.author.clone();

        let metadata = create_radroots_profile_event_metadata(
            id.clone(),
            author.clone(),
            self.created_at,
            kind_u32,
            &self.content,
        )?;

        Ok(RadrootsProfileEventIndex {
            event: RadrootsNostrEvent {
                id,
                author,
                created_at: self.created_at,
                kind: kind_u32,
                content: self.content.clone(),
                tags: self.tags.clone(),
                sig: self.sig.clone(),
            },
            metadata,
        })
    }
}
