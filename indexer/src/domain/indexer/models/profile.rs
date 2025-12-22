use crate::domain::indexer::key::{IndexerKey, PROFILE_INDEX_DIRECTORY};
use crate::utils::io::{fs_mkdir, safe_path_segment, write_json_if_changed};
use crate::utils::nostr::{normalize_nip05, public_key_to_npub};
use crate::utils::strings::truncate_log;
use crate::{
    audit,
    domain::{
        events::ToRadrootsProfileEventIndex,
        indexer::{
            kind::IndexerEventKind,
            models::{EventIndexes, NostrEventsStaticError, WriteEventIndexes},
        },
    },
    relay::event::RelayIndexerEvent,
    Settings,
};
use radroots_events::profile::RadrootsProfileEventIndex;
use std::{collections::BTreeMap, path::PathBuf};
use tracing::{instrument, warn};

#[derive(Debug)]
pub struct EventProfileIndexes {
    events: Vec<RadrootsProfileEventIndex>,
    events_id: BTreeMap<String, usize>,
    events_author: BTreeMap<String, usize>,
    events_nip05: BTreeMap<String, usize>,
    events_npub: BTreeMap<String, usize>,
}

impl EventIndexes for EventProfileIndexes {
    type Event = RelayIndexerEvent;

    fn subdirs() -> &'static [IndexerKey] {
        &PROFILE_INDEX_DIRECTORY
    }

    #[instrument(skip(raw_events), fields(event_count = raw_events.len()))]
    fn build(raw_events: &[Self::Event]) -> Result<Self, NostrEventsStaticError> {
        let mut events = Vec::with_capacity(raw_events.len());
        let mut events_id: BTreeMap<String, usize> = BTreeMap::new();
        let mut events_author: BTreeMap<String, usize> = BTreeMap::new();
        let mut events_nip05: BTreeMap<String, usize> = BTreeMap::new();
        let mut events_npub: BTreeMap<String, usize> = BTreeMap::new();

        let should_replace = |existing_idx: usize,
                              candidate_idx: usize,
                              events: &[RadrootsProfileEventIndex]| {
            let existing = &events[existing_idx];
            let candidate = &events[candidate_idx];
            let new_ts = candidate.metadata.published_at;
            let old_ts = existing.metadata.published_at;
            if new_ts > old_ts {
                true
            } else if new_ts < old_ts {
                false
            } else {
                candidate.event.id < existing.event.id
            }
        };

        for raw in raw_events {
            match raw.to_radroots_profile_event() {
                Ok(evt) => {
                    audit::log_profile_event(&evt);
                    let id = evt.event.id.clone();
                    let author = evt.event.author.clone();
                    let npub_key = public_key_to_npub(&author).ok().map(|mut npub| {
                        npub.make_ascii_lowercase();
                        npub
                    });
                    let nip05_index_key = evt
                        .metadata
                        .profile
                        .nip05
                        .as_deref()
                        .map(normalize_nip05)
                        .map(|(_, _, index_key)| index_key);
                    events.push(evt);
                    let idx = events.len() - 1;
                    events_id.insert(id, idx);

                    let replace_author = events_author
                        .get(&author)
                        .map(|&existing_idx| should_replace(existing_idx, idx, &events))
                        .unwrap_or(true);
                    if replace_author {
                        events_author.insert(author, idx);
                    }

                    if let Some(key) = npub_key {
                        let replace_npub = events_npub
                            .get(&key)
                            .map(|&existing_idx| should_replace(existing_idx, idx, &events))
                            .unwrap_or(true);
                        if replace_npub {
                            events_npub.insert(key, idx);
                        }
                    }
                    if let Some(index_key) = nip05_index_key {
                        if !index_key.is_empty() {
                            let replace_nip05 = events_nip05
                                .get(&index_key)
                                .map(|&existing_idx| should_replace(existing_idx, idx, &events))
                                .unwrap_or(true);
                            if replace_nip05 {
                                events_nip05.insert(index_key, idx);
                            }
                        }
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
        write_json_if_changed(&idxs_root, &ids, updated)?;

        for &subdir in Self::subdirs().iter() {
            let sub_base = base.join(subdir.as_str());
            fs_mkdir(&[&sub_base])?;

            let keys_lower: Vec<String> = match subdir {
                IndexerKey::Id => self
                    .events_id
                    .keys()
                    .filter_map(|k| safe_path_segment(&k.to_lowercase()))
                    .collect(),
                IndexerKey::Author => self
                    .events_author
                    .keys()
                    .filter_map(|k| safe_path_segment(&k.to_lowercase()))
                    .collect(),
                IndexerKey::Nip05 => self
                    .events_nip05
                    .keys()
                    .filter_map(|k| safe_path_segment(&k.to_lowercase()))
                    .collect(),
                IndexerKey::Npub => self
                    .events_npub
                    .keys()
                    .filter_map(|k| safe_path_segment(&k.to_lowercase()))
                    .collect(),
                _ => Vec::new(),
            };
            let idxs_subdir = sub_base.join("indexes.json");
            write_json_if_changed(&idxs_subdir, &keys_lower, updated)?;

            match subdir {
                IndexerKey::Id => {
                    for (key, idx) in &self.events_id {
                        let key_lower = key.to_lowercase();
                        let Some(dir_key) = safe_path_segment(&key_lower) else {
                            warn!(key = %key, "Skipping unsafe profile id path segment");
                            continue;
                        };
                        let dir = sub_base.join(dir_key);
                        let evt = &self.events[*idx];
                        fs_mkdir(&[&dir])?;
                        write_json_if_changed(&dir.join("event.json"), &evt.event, updated)?;
                        write_json_if_changed(&dir.join("metadata.json"), &evt.metadata, updated)?;
                    }
                }
                IndexerKey::Author => {
                    for (key, idx) in &self.events_author {
                        let key_lower = key.to_lowercase();
                        let Some(dir_key) = safe_path_segment(&key_lower) else {
                            warn!(key = %key, "Skipping unsafe profile author path segment");
                            continue;
                        };
                        let dir = sub_base.join(dir_key);
                        let evt = &self.events[*idx];
                        fs_mkdir(&[&dir])?;
                        write_json_if_changed(&dir.join("event.json"), &evt.event, updated)?;
                        write_json_if_changed(&dir.join("metadata.json"), &evt.metadata, updated)?;
                    }
                }
                IndexerKey::Nip05 => {
                    for (key, idx) in &self.events_nip05 {
                        let key_lower = key.to_lowercase();
                        let Some(dir_key) = safe_path_segment(&key_lower) else {
                            warn!(key = %key, "Skipping unsafe profile nip05 path segment");
                            continue;
                        };
                        let dir = sub_base.join(dir_key);
                        let evt = &self.events[*idx];
                        fs_mkdir(&[&dir])?;
                        write_json_if_changed(&dir.join("event.json"), &evt.event, updated)?;
                        write_json_if_changed(&dir.join("metadata.json"), &evt.metadata, updated)?;
                    }
                }
                IndexerKey::Npub => {
                    for (key, idx) in &self.events_npub {
                        let key_lower = key.to_lowercase();
                        let Some(dir_key) = safe_path_segment(&key_lower) else {
                            warn!(key = %key, "Skipping unsafe profile npub path segment");
                            continue;
                        };
                        let dir = sub_base.join(dir_key);
                        let evt = &self.events[*idx];
                        fs_mkdir(&[&dir])?;
                        write_json_if_changed(&dir.join("event.json"), &evt.event, updated)?;
                        write_json_if_changed(&dir.join("metadata.json"), &evt.metadata, updated)?;
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::EventProfileIndexes;
    use crate::domain::indexer::kind::IndexerEventKind;
    use crate::relay::event::RelayIndexerEvent;

    fn make_profile_event(id: &str, author: &str, created_at: u32, name: &str) -> RelayIndexerEvent {
        RelayIndexerEvent {
            id: id.to_string(),
            author: author.to_string(),
            created_at,
            pubkey: author.to_string(),
            kind: IndexerEventKind::Profile,
            tags: Vec::new(),
            content: format!(r#"{{"name":"{}"}}"#, name),
            hash: id.to_string(),
            sig: "sig".to_string(),
        }
    }

    #[test]
    fn profile_index_uses_latest_event() {
        let author = "a".repeat(64);
        let older = make_profile_event(&"b".repeat(64), &author, 10, "old");
        let newer = make_profile_event(&"c".repeat(64), &author, 20, "new");

        let indexes = EventProfileIndexes::build(&[older, newer]).expect("build");
        let idx = *indexes.events_author.get(&author).expect("author index");
        assert_eq!(indexes.events[idx].metadata.profile.name, "new");
    }

    #[test]
    fn profile_index_tiebreaks_by_id() {
        let author = "b".repeat(64);
        let low_id = "0".repeat(64);
        let high_id = "f".repeat(64);
        let first = make_profile_event(&high_id, &author, 10, "high");
        let second = make_profile_event(&low_id, &author, 10, "low");

        let indexes = EventProfileIndexes::build(&[first, second]).expect("build");
        let idx = *indexes.events_author.get(&author).expect("author index");
        assert_eq!(indexes.events[idx].event.id, low_id);
    }
}
