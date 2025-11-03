use crate::utils::crypto::compute_hash;
use crate::utils::io::fs_mkdir;
use crate::utils::io::{write_hash, write_json};
use crate::utils::nostr::public_key_to_npub;
use crate::utils::strings::truncate_log;
use crate::{
    audit,
    domain::{
        events::ToRadrootsProfileEventIndex,
        indexer::{
            kind::IndexerEventKind,
            models::{EventIndexes, NostrEventsStaticError, WriteEventIndexes},
            IndexerKey, PROFILE_INDEX_DIRECTORY,
        },
    },
    relay::event::RelayIndexerEvent,
    Settings,
};
use radroots_events::profile::models::RadrootsProfileEventIndex;
use std::{collections::BTreeMap, fs, path::PathBuf};
use tracing::{instrument, warn};

macro_rules! write_if_stale {
    ($path:expr, $data:expr, $updated:expr) => {{
        let hash = compute_hash(&$data)?;
        let hash_path = $path.with_extension("sha256.txt");

        let needs_write = if $path.exists() && hash_path.exists() {
            let stored = fs::read_to_string(&hash_path)?;
            stored.trim() != hash
        } else {
            true
        };

        if needs_write {
            write_json(&$path, &$data)?;
            write_hash(&$path, &hash)?;
            $updated.push($path.clone());
        }
    }};
}

#[derive(Debug)]
pub struct EventProfileIndexes {
    events: Vec<RadrootsProfileEventIndex>,
    events_id: BTreeMap<String, RadrootsProfileEventIndex>,
    events_author: BTreeMap<String, RadrootsProfileEventIndex>,
    events_nip05: BTreeMap<String, RadrootsProfileEventIndex>,
    events_npub: BTreeMap<String, RadrootsProfileEventIndex>,
}

impl EventIndexes for EventProfileIndexes {
    type Event = RelayIndexerEvent;

    fn subdirs() -> &'static [IndexerKey] {
        &PROFILE_INDEX_DIRECTORY
    }

    #[instrument(skip(raw_events), fields(event_count = raw_events.len()))]
    fn build(raw_events: &[Self::Event]) -> Result<Self, NostrEventsStaticError> {
        let mut events = Vec::with_capacity(raw_events.len());
        let mut events_id = BTreeMap::new();
        let mut events_author = BTreeMap::new();
        let mut events_nip05 = BTreeMap::new();
        let mut events_npub = BTreeMap::new();

        for raw in raw_events {
            match raw.clone().to_radroots_profile_event() {
                Ok(evt) => {
                    audit::log_profile_event(&evt);
                    let id = evt.event.id.clone();
                    let author = evt.event.author.clone();
                    events.push(evt.clone());
                    events_id.insert(id.clone(), evt.clone());
                    events_author.insert(author.clone(), evt.clone());

                    if let Ok(npub) = public_key_to_npub(&author) {
                        events_npub.insert(npub.to_lowercase(), evt.clone());
                    }
                    if let Some(nip05) = &evt.metadata.profile.nip05 {
                        let normalized = nip05.replace("@radroots.market", "");
                        events_nip05.insert(normalized, evt.clone());
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

        Ok(EventProfileIndexes {
            events,
            events_id,
            events_author,
            events_nip05,
            events_npub,
        })
    }
}

impl WriteEventIndexes for EventProfileIndexes {
    fn write(&self, settings: &Settings, updated: &mut Vec<PathBuf>) -> anyhow::Result<()> {
        let base: PathBuf = IndexerEventKind::Profile.base_path(&settings.indexer.data_dir)?;
        fs_mkdir(&[&base])?;

        let idxs_root = base.join("events.json");
        let ids: Vec<&String> = self.events.iter().map(|e| &e.event.id).collect();
        write_if_stale!(idxs_root, ids, updated);

        for &subdir in Self::subdirs().iter() {
            let sub_base = base.join(subdir.as_str());
            fs_mkdir(&[sub_base.to_str().unwrap()])?;

            let keys_lower: Vec<String> = match subdir {
                IndexerKey::Id => self.events_id.keys().map(|k| k.to_lowercase()).collect(),
                IndexerKey::Author => self
                    .events_author
                    .keys()
                    .map(|k| k.to_lowercase())
                    .collect(),
                IndexerKey::Nip05 => self.events_nip05.keys().map(|k| k.to_lowercase()).collect(),
                IndexerKey::Npub => self.events_npub.keys().map(|k| k.to_lowercase()).collect(),
                _ => Vec::new(),
            };
            let idxs_subdir = sub_base.join("indexes.json");
            write_if_stale!(idxs_subdir, keys_lower, updated);

            match subdir {
                IndexerKey::Id => {
                    for (key, evt) in &self.events_id {
                        let dir = sub_base.join(key.to_lowercase());
                        fs_mkdir(&[dir.to_str().unwrap()])?;
                        write_if_stale!(dir.join("event.json"), evt.event.clone(), updated);
                        write_if_stale!(dir.join("metadata.json"), evt.metadata.clone(), updated);
                    }
                }
                IndexerKey::Author => {
                    for (key, evt) in &self.events_author {
                        let dir = sub_base.join(key.to_lowercase());
                        fs_mkdir(&[dir.to_str().unwrap()])?;
                        write_if_stale!(dir.join("event.json"), evt.event.clone(), updated);
                        write_if_stale!(dir.join("metadata.json"), evt.metadata.clone(), updated);
                    }
                }
                IndexerKey::Nip05 => {
                    for (key, evt) in &self.events_nip05 {
                        let dir = sub_base.join(key.to_lowercase());
                        fs_mkdir(&[dir.to_str().unwrap()])?;
                        write_if_stale!(dir.join("event.json"), evt.event.clone(), updated);
                        write_if_stale!(dir.join("metadata.json"), evt.metadata.clone(), updated);
                    }
                }
                IndexerKey::Npub => {
                    for (key, evt) in &self.events_npub {
                        let dir = sub_base.join(key.to_lowercase());
                        fs_mkdir(&[dir.to_str().unwrap()])?;
                        write_if_stale!(dir.join("event.json"), evt.event.clone(), updated);
                        write_if_stale!(dir.join("metadata.json"), evt.metadata.clone(), updated);
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}
