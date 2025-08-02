use indexer_utils::{
    file::{fs_mkdir, fs_write_with_hash_check},
    logs::truncate_log,
    paths::paths_join,
};
use radroots_common::models::events::RadrootsMetadataEvent;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;
use tracing::{instrument, warn};

use crate::{
    domain::{
        events::metadata::ToRadrootsMetadataEvent,
        indexer::{
            kind::IndexerEventKind,
            models::{EventIndexes, NostrEventsStaticError, WriteEventIndexes},
            IndexerKey, METADATA_INDEX_DIRECTORY,
        },
    },
    relay::event::RelayIndexerEvent,
    Settings,
};

#[derive(Debug)]
pub struct Event0StaticIndexes {
    events: Vec<RadrootsMetadataEvent>,
    events_id: BTreeMap<String, RadrootsMetadataEvent>,
    events_nip05: BTreeMap<String, RadrootsMetadataEvent>,
    events_author: BTreeMap<String, Vec<RadrootsMetadataEvent>>,
}

impl EventIndexes for Event0StaticIndexes {
    type Event = RelayIndexerEvent;

    fn subdirs() -> &'static [IndexerKey] {
        &METADATA_INDEX_DIRECTORY
    }

    #[instrument(skip(raw_events), fields(event_count = raw_events.len()))]
    fn build(raw_events: &[Self::Event]) -> Result<Self, NostrEventsStaticError> {
        let mut events = Vec::with_capacity(raw_events.len());
        let mut events_id = BTreeMap::new();
        let mut events_nip05 = BTreeMap::new();
        let mut events_author: BTreeMap<String, Vec<RadrootsMetadataEvent>> = BTreeMap::new();

        for raw in raw_events {
            match raw.clone().to_radroots_metadata_event() {
                Ok(parsed) => {
                    let id = parsed.event.id.clone();
                    let author = parsed.event.author.clone();

                    events.push(parsed.clone());
                    events_id.insert(id.clone(), parsed.clone());
                    events_author
                        .entry(author.clone())
                        .or_default()
                        .push(parsed.clone());

                    if let Some(nip05) = &parsed.data.metadata.nip05 {
                        let normalized = nip05.replace("@radroots.market", "");
                        events_nip05.insert(normalized, parsed);
                    }
                }
                Err(err) => {
                    warn!(
                        kind = raw.kind.as_u64(),
                        id = %raw.id,
                        author = %raw.author,
                        content = %truncate_log(&raw.content, 1000),
                        tags = ?raw.tags,
                        error = %err,
                        "Skipping malformed metadata event"
                    );
                }
            }
        }

        Ok(Event0StaticIndexes {
            events,
            events_id,
            events_nip05,
            events_author,
        })
    }

    fn index_json(&self, subdir: IndexerKey) -> Option<Value> {
        match subdir {
            IndexerKey::Id => serde_json::to_value(&self.events_id).ok(),
            IndexerKey::Author => {
                // Map author -> [event IDs]
                let map: BTreeMap<&String, Vec<String>> = self
                    .events_author
                    .iter()
                    .map(|(author, evts)| {
                        let ids = evts.iter().map(|e| e.event.id.clone()).collect();
                        (author, ids)
                    })
                    .collect();
                serde_json::to_value(&map).ok()
            }
            IndexerKey::Nip05 => serde_json::to_value(&self.events_nip05).ok(),
            _ => None,
        }
    }
}

impl WriteEventIndexes for Event0StaticIndexes {
    fn write(&self, settings: &Settings, updated: &mut Vec<PathBuf>) -> anyhow::Result<()> {
        let base = paths_join(&[
            settings.service.output_dir.as_str(),
            "events",
            &IndexerEventKind::Metadata.as_u64().to_string(),
        ])?;
        fs_mkdir(&[&base])?;

        // Write top-level events.json with all event IDs
        let all_ids: Vec<&String> = self.events.iter().map(|e| &e.event.id).collect();
        let top_events = base.join("events.json");
        if fs_write_with_hash_check(&top_events, &all_ids)? {
            updated.push(top_events.clone());
        }

        // Per-subdir indices
        for &subdir in Event0StaticIndexes::subdirs().iter() {
            let sub_base = base.join(subdir.as_str());
            fs_mkdir(&[sub_base.to_str().unwrap()])?;

            // Write indexes.json (list of keys)
            let keys_lower: Vec<String> = match subdir {
                IndexerKey::Id => self.events_id.keys().map(|k| k.to_lowercase()).collect(),
                IndexerKey::Author => self
                    .events_author
                    .keys()
                    .map(|k| k.to_lowercase())
                    .collect(),
                IndexerKey::Nip05 => self.events_nip05.keys().map(|k| k.to_lowercase()).collect(),
                other => {
                    warn!("No index keys for subdir {:?}", other);
                    Vec::new()
                }
            };
            let idxs = sub_base.join("indexes.json");
            if fs_write_with_hash_check(&idxs, &keys_lower)? {
                updated.push(idxs.clone());
            }

            // Write events.json according to subdir variant
            match subdir {
                IndexerKey::Author => {
                    // One events.json per-author, mapping event_id -> full RadrootsMetadataEvent
                    for (author, evts_list) in &self.events_author {
                        let author_dir = sub_base.join(author.to_lowercase());
                        fs_mkdir(&[author_dir.to_str().unwrap()])?;

                        let mut map: BTreeMap<String, &RadrootsMetadataEvent> = BTreeMap::new();
                        for ev in evts_list {
                            map.insert(ev.event.id.clone(), ev);
                        }

                        let evts_path = author_dir.join("events.json");
                        if fs_write_with_hash_check(&evts_path, &map)? {
                            updated.push(evts_path.clone());
                        }
                    }
                }
                IndexerKey::Id => {
                    // Flat events.json at subdir root
                    let ids: Vec<&String> = self.events_id.values().map(|e| &e.event.id).collect();
                    let evts = sub_base.join("events.json");
                    if fs_write_with_hash_check(&evts, &ids)? {
                        updated.push(evts.clone());
                    }
                }
                IndexerKey::Nip05 => {
                    // Flat events.json at subdir root
                    let ids: Vec<&String> =
                        self.events_nip05.values().map(|e| &e.event.id).collect();
                    let evts = sub_base.join("events.json");
                    if fs_write_with_hash_check(&evts, &ids)? {
                        updated.push(evts.clone());
                    }
                }
                other => {
                    // Default fallback: no writer implemented
                    warn!("No static writer implemented for subdir {:?}", other);
                }
            }
        }

        Ok(())
    }
}
