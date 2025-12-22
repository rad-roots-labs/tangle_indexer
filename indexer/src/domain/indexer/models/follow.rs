use crate::domain::indexer::key::{FOLLOW_INDEX_DIRECTORY, IndexerKey};
use crate::utils::io::{fs_mkdir, safe_path_segment, write_json_if_changed};
use crate::utils::nostr::public_key_to_npub;
use crate::utils::strings::truncate_log;
use crate::{
    audit,
    domain::{
        events::follow::ToRadrootsFollowEventIndex,
        indexer::{
            kind::IndexerEventKind,
            models::{EventIndexes, NostrEventsStaticError, WriteEventIndexes},
        },
        resolvers::profile::ProfileResolver,
    },
    relay::event::RelayIndexerEvent,
    Settings,
};
use radroots_events::follow::{RadrootsFollowEventIndex, RadrootsFollowEventMetadata};
use std::{collections::BTreeMap, path::PathBuf};
use tracing::{instrument, warn};

#[derive(Debug)]
pub struct EventFollowIndexes {
    events: Vec<RadrootsFollowEventIndex>,
    events_id: BTreeMap<String, usize>,
    author_ids: BTreeMap<String, Vec<String>>,
    npub_ids: BTreeMap<String, Vec<String>>,
    nip05_ids: BTreeMap<String, Vec<String>>,
}

impl EventFollowIndexes {
    pub fn build_with_profiles(
        raw_events: &[RelayIndexerEvent],
        profiles: &ProfileResolver,
    ) -> Result<Self, NostrEventsStaticError> {
        let mut events = Vec::with_capacity(raw_events.len());
        let mut events_id: BTreeMap<String, usize> = BTreeMap::new();
        let mut author_ids: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut npub_ids: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut nip05_ids: BTreeMap<String, Vec<String>> = BTreeMap::new();

        for raw in raw_events {
            match raw.to_radroots_follow_event() {
                Ok(evt) => {
                    audit::log_indexer_event(&raw);
                    let id = evt.metadata.id.clone();
                    let author_hex = evt.metadata.author.to_lowercase();

                    let npub = public_key_to_npub(&author_hex)
                        .map(|mut s| {
                            s.make_ascii_lowercase();
                            s
                        })
                        .ok();
                    let author_nip05 = profiles.nip05_for_author(&author_hex).map(str::to_owned);

                    events.push(evt);
                    let idx = events.len() - 1;
                    events_id.insert(id.clone(), idx);

                    author_ids.entry(author_hex).or_default().push(id.clone());
                    if let Some(n) = npub {
                        npub_ids.entry(n).or_default().push(id.clone());
                    }
                    if let Some(n05) = author_nip05 {
                        nip05_ids.entry(n05).or_default().push(id.clone());
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
                        "Skipping malformed follow event"
                    );
                }
            }
        }

        let sort_ids = |ids: &mut Vec<String>,
                        map: &BTreeMap<String, usize>,
                        events: &[RadrootsFollowEventIndex]| {
            ids.sort_unstable_by(|a, b| {
                let pa = map
                    .get(a)
                    .map(|idx| events[*idx].metadata.published_at)
                    .unwrap_or_default();
                let pb = map
                    .get(b)
                    .map(|idx| events[*idx].metadata.published_at)
                    .unwrap_or_default();
                pb.cmp(&pa).then(a.cmp(b))
            });
        };

        for ids in author_ids.values_mut() {
            sort_ids(ids, &events_id, &events);
        }
        for ids in npub_ids.values_mut() {
            sort_ids(ids, &events_id, &events);
        }
        for ids in nip05_ids.values_mut() {
            sort_ids(ids, &events_id, &events);
        }

        Ok(Self {
            events,
            events_id,
            author_ids,
            npub_ids,
            nip05_ids,
        })
    }
}

impl EventIndexes for EventFollowIndexes {
    type Event = RelayIndexerEvent;

    fn subdirs() -> &'static [IndexerKey] {
        &FOLLOW_INDEX_DIRECTORY
    }

    #[instrument(skip(raw_events), fields(event_count = raw_events.len()))]
    fn build(raw_events: &[Self::Event]) -> Result<Self, NostrEventsStaticError> {
        let empty = ProfileResolver::default();
        Self::build_with_profiles(raw_events, &empty)
    }
}

impl WriteEventIndexes for EventFollowIndexes {
    fn write(&self, settings: &Settings, updated: &mut Vec<PathBuf>) -> anyhow::Result<()> {
        let base: PathBuf = IndexerEventKind::Follow.base_path(&settings.indexer.data_dir)?;
        fs_mkdir(&[&base])?;

        {
            let idxs_root = base.join("events.json");
            let ids = super::sorted_event_ids(
                &self.events,
                |event| event.metadata.published_at,
                |event| &event.event.id,
            );
            write_json_if_changed(&idxs_root, &ids, updated)?;
        }

        {
            let sub = base.join("id");
            fs_mkdir(&[&sub])?;
            let keys: Vec<String> = self
                .events_id
                .keys()
                .filter_map(|key| safe_path_segment(&key.to_lowercase()))
                .collect();
            write_json_if_changed(&sub.join("indexes.json"), &keys, updated)?;

            for (id, idx) in &self.events_id {
                let id_lower = id.to_lowercase();
                let Some(dir_key) = safe_path_segment(&id_lower) else {
                    warn!(id = %id, "Skipping unsafe follow id path segment");
                    continue;
                };
                let dir = sub.join(dir_key);
                let evt = &self.events[*idx];
                fs_mkdir(&[&dir])?;
                write_json_if_changed(&dir.join("event.json"), &evt.event, updated)?;
                write_json_if_changed(&dir.join("metadata.json"), &evt.metadata, updated)?;
            }
        }

        {
            let sub = base.join(IndexerKey::Author.as_str());
            fs_mkdir(&[&sub])?;
            let authors: Vec<String> = self
                .author_ids
                .keys()
                .filter_map(|author| safe_path_segment(author))
                .collect();
            write_json_if_changed(&sub.join("indexes.json"), &authors, updated)?;

            for (author, ids) in &self.author_ids {
                let Some(dir_key) = safe_path_segment(author) else {
                    warn!(author = %author, "Skipping unsafe follow author path segment");
                    continue;
                };
                let dir = sub.join(dir_key);
                fs_mkdir(&[&dir])?;
                let metas: Vec<&RadrootsFollowEventMetadata> = ids
                    .iter()
                    .filter_map(|id| self.events_id.get(id))
                    .map(|idx| &self.events[*idx].metadata)
                    .collect();
                write_json_if_changed(&dir.join("events.json"), ids, updated)?;
                write_json_if_changed(&dir.join("metadata.json"), &metas, updated)?;
            }
        }

        {
            let sub = base.join(IndexerKey::Npub.as_str());
            fs_mkdir(&[&sub])?;
            let npubs: Vec<String> = self
                .npub_ids
                .keys()
                .filter_map(|npub| safe_path_segment(npub))
                .collect();
            write_json_if_changed(&sub.join("indexes.json"), &npubs, updated)?;

            for (npub, ids) in &self.npub_ids {
                let Some(dir_key) = safe_path_segment(npub) else {
                    warn!(npub = %npub, "Skipping unsafe follow npub path segment");
                    continue;
                };
                let dir = sub.join(dir_key);
                fs_mkdir(&[&dir])?;
                let metas: Vec<&RadrootsFollowEventMetadata> = ids
                    .iter()
                    .filter_map(|id| self.events_id.get(id))
                    .map(|idx| &self.events[*idx].metadata)
                    .collect();
                write_json_if_changed(&dir.join("events.json"), ids, updated)?;
                write_json_if_changed(&dir.join("metadata.json"), &metas, updated)?;
            }
        }

        {
            let sub = base.join(IndexerKey::Nip05.as_str());
            fs_mkdir(&[&sub])?;
            let names: Vec<String> = self
                .nip05_ids
                .keys()
                .filter_map(|name| safe_path_segment(name))
                .collect();
            write_json_if_changed(&sub.join("indexes.json"), &names, updated)?;

            for (name, ids) in &self.nip05_ids {
                let Some(dir_key) = safe_path_segment(name) else {
                    warn!(nip05 = %name, "Skipping unsafe follow nip05 path segment");
                    continue;
                };
                let dir = sub.join(dir_key);
                fs_mkdir(&[&dir])?;
                let metas: Vec<&RadrootsFollowEventMetadata> = ids
                    .iter()
                    .filter_map(|id| self.events_id.get(id))
                    .map(|idx| &self.events[*idx].metadata)
                    .collect();
                write_json_if_changed(&dir.join("events.json"), ids, updated)?;
                write_json_if_changed(&dir.join("metadata.json"), &metas, updated)?;
            }
        }

        Ok(())
    }
}
