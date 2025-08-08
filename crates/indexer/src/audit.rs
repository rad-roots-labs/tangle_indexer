#![cfg(feature = "audit")]

use tracing::info;

use crate::relay::event::RelayIndexerEvent;
use radroots_common::models::events::{RadrootsListingEvent, RadrootsMetadataEvent};

static AUDIT_PUBLIC_KEY: Option<&'static str> = option_env!("AUDIT_PUBLIC_KEY");

#[inline]
fn matches(pk: &str) -> bool {
    match AUDIT_PUBLIC_KEY {
        Some(w) => pk == w,
        None => false,
    }
}

#[inline]
pub fn log_indexer_event(idx: &RelayIndexerEvent) {
    if !matches(&idx.author) {
        return;
    }

    let tags_json = match serde_json::to_string(&idx.tags) {
        Ok(json) => json,
        Err(_) => String::from("Error serializing tags"),
    };
    info!(
        target: "audit",
        kind = idx.kind.as_u64(),
        id = %idx.id,
        author = %idx.author,
        created_at = idx.created_at,
        tags = %tags_json,
        content = %idx.content,
        "AUDIT: relay indexer event"
    );
}

#[inline]
pub fn log_metadata_event(evt: &RadrootsMetadataEvent) {
    if !matches(&evt.event.author) {
        return;
    }
    if let Ok(json) = serde_json::to_string(evt) {
        info!(
            target = "audit",
            kind = evt.event.kind,
            id = %evt.event.id,
            author = %evt.event.author,
            created_at = evt.event.created_at,
            processed_json = %json,
            "AUDIT: processed metadata"
        );
    }
}

#[inline]
pub fn log_listing_event(evt: &RadrootsListingEvent) {
    if !matches(&evt.event.author) {
        return;
    }
    if let Ok(json) = serde_json::to_string(evt) {
        info!(
            target = "audit",
            kind = evt.event.kind,
            id = %evt.event.id,
            author = %evt.event.author,
            created_at = evt.event.created_at,
            processed_json = %json,
            "AUDIT: processed listing"
        );
    }
}
