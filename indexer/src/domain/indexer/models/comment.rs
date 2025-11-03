use crate::domain::indexer::key::{IndexerKey, COMMENT_INDEX_DIRECTORY};
use crate::utils::crypto::compute_hash;
use crate::utils::io::fs_mkdir;
use crate::utils::io::{write_hash, write_json};
use crate::utils::nostr::public_key_to_npub;
use crate::utils::strings::truncate_log;
use crate::{
    audit,
    domain::{
        events::comment::ToRadrootsCommentEventIndex,
        indexer::{
            kind::IndexerEventKind,
            models::{EventIndexes, NostrEventsStaticError, WriteEventIndexes},
        },
        resolvers::profile::ProfileResolver,
    },
    relay::event::RelayIndexerEvent,
    Settings,
};
use radroots_events::comment::models::{RadrootsCommentEventIndex, RadrootsCommentEventMetadata};
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
pub struct EventCommentIndexes {
    events: Vec<RadrootsCommentEventIndex>,
    events_id: BTreeMap<String, RadrootsCommentEventIndex>,
    root_ids: BTreeMap<String, Vec<String>>,
    author_ids: BTreeMap<String, Vec<String>>,
    npub_ids: BTreeMap<String, Vec<String>>,
    nip05_ids: BTreeMap<String, Vec<String>>,
}

impl EventCommentIndexes {
    pub fn build_with_profiles(
        raw_events: &[RelayIndexerEvent],
        profiles: &ProfileResolver,
    ) -> Result<Self, NostrEventsStaticError> {
        let mut events = Vec::with_capacity(raw_events.len());
        let mut events_id = BTreeMap::new();
        let mut root_ids: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut author_ids: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut npub_ids: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut nip05_ids: BTreeMap<String, Vec<String>> = BTreeMap::new();

        for raw in raw_events {
            match raw.clone().to_radroots_comment_event() {
                Ok(evt) => {
                    audit::log_comment_event(&evt);
                    let id = evt.metadata.id.clone();
                    let author_hex = evt.metadata.author.to_lowercase();

                    let npub = public_key_to_npub(&author_hex)
                        .map(|s| s.to_lowercase())
                        .ok();
                    let author_nip05 = profiles.nip05_for_author(&author_hex).map(str::to_owned);

                    let root = evt.metadata.comment.root.id.to_lowercase();

                    events_id.insert(id.clone(), evt.clone());
                    events.push(evt.clone());

                    root_ids.entry(root).or_default().push(id.clone());
                    author_ids.entry(author_hex).or_default().push(id.clone());
                    if let Some(n) = npub {
                        npub_ids.entry(n).or_default().push(id.clone());
                    }
                    if let Some(n05) = author_nip05 {
                        nip05_ids
                            .entry(n05.to_lowercase())
                            .or_default()
                            .push(id.clone());
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
                        "Skipping malformed comment event"
                    );
                }
            }
        }

        let sort_ids = |ids: &mut Vec<String>,
                        map: &BTreeMap<String, RadrootsCommentEventIndex>| {
            ids.sort_unstable_by(|a, b| {
                let pa = map
                    .get(a)
                    .map(|e| e.metadata.published_at)
                    .unwrap_or_default();
                let pb = map
                    .get(b)
                    .map(|e| e.metadata.published_at)
                    .unwrap_or_default();
                pb.cmp(&pa).then(a.cmp(b))
            });
        };

        for ids in root_ids.values_mut() {
            sort_ids(ids, &events_id);
        }
        for ids in author_ids.values_mut() {
            sort_ids(ids, &events_id);
        }
        for ids in npub_ids.values_mut() {
            sort_ids(ids, &events_id);
        }
        for ids in nip05_ids.values_mut() {
            sort_ids(ids, &events_id);
        }

        Ok(Self {
            events,
            events_id,
            root_ids,
            author_ids,
            npub_ids,
            nip05_ids,
        })
    }
}

impl EventIndexes for EventCommentIndexes {
    type Event = RelayIndexerEvent;

    fn subdirs() -> &'static [IndexerKey] {
        &COMMENT_INDEX_DIRECTORY
    }

    #[instrument(skip(raw_events), fields(event_count = raw_events.len()))]
    fn build(raw_events: &[Self::Event]) -> Result<Self, NostrEventsStaticError> {
        let empty = ProfileResolver::default();
        Self::build_with_profiles(raw_events, &empty)
    }
}

impl WriteEventIndexes for EventCommentIndexes {
    fn write(&self, settings: &Settings, updated: &mut Vec<PathBuf>) -> anyhow::Result<()> {
        let base: PathBuf = IndexerEventKind::Comment.base_path(&settings.indexer.data_dir)?;
        fs_mkdir(&[&base])?;

        {
            let idxs_root = base.join("events.json");
            let ids: Vec<&String> = self.events.iter().map(|e| &e.event.id).collect();
            write_if_stale!(idxs_root, ids, updated);
        }

        {
            let sub = base.join("id");
            fs_mkdir(&[&sub])?;
            let keys: Vec<String> = self.events_id.keys().cloned().collect();
            write_if_stale!(sub.join("indexes.json"), keys, updated);

            for (id, evt) in &self.events_id {
                let dir = sub.join(id.to_lowercase());
                fs_mkdir(&[&dir])?;
                write_if_stale!(dir.join("event.json"), evt.event.clone(), updated);
                write_if_stale!(dir.join("metadata.json"), evt.metadata.clone(), updated);
            }
        }

        {
            let sub = base.join(IndexerKey::RootId.as_str());
            fs_mkdir(&[&sub])?;
            let roots: Vec<String> = self.root_ids.keys().cloned().collect();
            write_if_stale!(sub.join("indexes.json"), roots, updated);

            for (root, ids) in &self.root_ids {
                let dir = sub.join(root);
                fs_mkdir(&[&dir])?;
                let metas: Vec<RadrootsCommentEventMetadata> = ids
                    .iter()
                    .filter_map(|id| self.events_id.get(id))
                    .map(|e| e.metadata.clone())
                    .collect();
                write_if_stale!(dir.join("events.json"), ids.clone(), updated);
                write_if_stale!(dir.join("metadata.json"), metas, updated);
            }
        }

        {
            let sub = base.join(IndexerKey::Author.as_str());
            fs_mkdir(&[&sub])?;
            let authors: Vec<String> = self.author_ids.keys().cloned().collect();
            write_if_stale!(sub.join("indexes.json"), authors, updated);

            for (author, ids) in &self.author_ids {
                let dir = sub.join(author);
                fs_mkdir(&[&dir])?;
                let metas: Vec<RadrootsCommentEventMetadata> = ids
                    .iter()
                    .filter_map(|id| self.events_id.get(id))
                    .map(|e| e.metadata.clone())
                    .collect();
                write_if_stale!(dir.join("events.json"), ids.clone(), updated);
                write_if_stale!(dir.join("metadata.json"), metas, updated);
            }
        }

        {
            let sub = base.join(IndexerKey::Npub.as_str());
            fs_mkdir(&[&sub])?;
            let npubs: Vec<String> = self.npub_ids.keys().cloned().collect();
            write_if_stale!(sub.join("indexes.json"), npubs, updated);

            for (npub, ids) in &self.npub_ids {
                let dir = sub.join(npub);
                fs_mkdir(&[&dir])?;
                let metas: Vec<RadrootsCommentEventMetadata> = ids
                    .iter()
                    .filter_map(|id| self.events_id.get(id))
                    .map(|e| e.metadata.clone())
                    .collect();
                write_if_stale!(dir.join("events.json"), ids.clone(), updated);
                write_if_stale!(dir.join("metadata.json"), metas, updated);
            }
        }

        {
            let sub = base.join(IndexerKey::Nip05.as_str());
            fs_mkdir(&[&sub])?;
            let names: Vec<String> = self.nip05_ids.keys().cloned().collect();
            write_if_stale!(sub.join("indexes.json"), names, updated);

            for (name, ids) in &self.nip05_ids {
                let dir = sub.join(name);
                fs_mkdir(&[&dir])?;
                let metas: Vec<RadrootsCommentEventMetadata> = ids
                    .iter()
                    .filter_map(|id| self.events_id.get(id))
                    .map(|e| e.metadata.clone())
                    .collect();
                write_if_stale!(dir.join("events.json"), ids.clone(), updated);
                write_if_stale!(dir.join("metadata.json"), metas, updated);
            }
        }

        Ok(())
    }
}
