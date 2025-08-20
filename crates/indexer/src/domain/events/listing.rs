use thiserror::Error;

use radroots_events::{
    listing::models::{RadrootsListing, RadrootsListingEventIndex, RadrootsListingEventMetadata},
    RadrootsNostrEvent,
};

use crate::relay::event::RelayIndexerEvent;

#[derive(Debug, Error)]
pub enum RadrootsListingEventIndexError {
    #[error("Failed to parse listing JSON: {0}")]
    ParseError(#[from] serde_json::Error),
}

fn create_radroots_listing_event_metadata(
    id: String,
    author: String,
    published_at: u32,
    content: String,
    _tags: Vec<Vec<String>>,
) -> Result<RadrootsListingEventMetadata, RadrootsListingEventIndexError> {
    let listing: RadrootsListing = serde_json::from_str(&content)?;
    Ok(RadrootsListingEventMetadata {
        id,
        author,
        published_at,
        listing,
    })
}

pub trait ToRadrootsListingEventIndex {
    fn to_radroots_listing_event(
        self,
    ) -> Result<RadrootsListingEventIndex, RadrootsListingEventIndexError>;
}

impl ToRadrootsListingEventIndex for RelayIndexerEvent {
    fn to_radroots_listing_event(
        self,
    ) -> Result<RadrootsListingEventIndex, RadrootsListingEventIndexError> {
        let metadata = create_radroots_listing_event_metadata(
            self.id.clone(),
            self.author.clone(),
            self.created_at,
            self.content.clone(),
            self.tags.clone(),
        )?;

        Ok(RadrootsListingEventIndex {
            event: RadrootsNostrEvent {
                id: self.id,
                author: self.author,
                created_at: self.created_at,
                kind: self.kind.as_u64() as u32,
                tags: self.tags,
                content: self.content,
                sig: self.sig,
            },
            metadata,
        })
    }
}
