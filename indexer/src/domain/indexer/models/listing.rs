use crate::utils::crypto::compute_hash;
use crate::utils::io::fs_mkdir;
use crate::utils::io::{write_hash, write_json};
use crate::utils::nostr::public_key_to_npub;
use crate::utils::strings::truncate_log;
use radroots_events::listing::models::{RadrootsListingEventIndex, RadrootsListingEventMetadata};
use radroots_events_indexed::{RadrootsEventsIndexedManifest, RadrootsEventsIndexedShardMetadata};
use std::{collections::BTreeMap, fs, path::PathBuf};
use tracing::{instrument, warn};

use crate::{
    audit,
    domain::{
        events::ToRadrootsListingEventIndex,
        indexer::{
            key::LISTING_INDEX_DIRECTORY,
            kind::IndexerEventKind,
            models::{EventIndexes, NostrEventsStaticError, WriteEventIndexes},
        },
        resolvers::profile::ProfileResolver,
    },
    relay::event::RelayIndexerEvent,
    Settings,
};

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

        hash
    }};
}

#[derive(Debug)]
pub struct EventListingIndexes {
    events: Vec<RadrootsListingEventIndex>,
    events_id: BTreeMap<String, RadrootsListingEventIndex>,
    country_ids: BTreeMap<String, Vec<String>>,
    author_ids: BTreeMap<String, Vec<String>>,
    npub_ids: BTreeMap<String, Vec<String>>,
    nip05_ids: BTreeMap<String, Vec<String>>,
}

impl EventListingIndexes {
    pub fn build_with_profiles(
        raw_events: &[RelayIndexerEvent],
        profiles: &ProfileResolver,
    ) -> Result<Self, NostrEventsStaticError> {
        let mut events: Vec<RadrootsListingEventIndex> = Vec::with_capacity(raw_events.len());
        let mut events_id: BTreeMap<String, RadrootsListingEventIndex> = BTreeMap::new();
        let mut country_ids: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut author_ids: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut npub_ids: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut nip05_ids: BTreeMap<String, Vec<String>> = BTreeMap::new();

        for raw in raw_events {
            match raw.clone().to_radroots_listing_event() {
                Ok(evt) => {
                    audit::log_listing_event(&evt);

                    let id = evt.metadata.id.clone();
                    let author_hex = evt.metadata.author.to_lowercase();

                    let npub = public_key_to_npub(&author_hex)
                        .map(|s| s.to_lowercase())
                        .ok();
                    let author_nip05 = profiles.nip05_for_author(&author_hex).map(str::to_owned);

                    let country_opt = evt
                        .metadata
                        .listing
                        .location
                        .as_ref()
                        .and_then(|loc| loc.country.as_ref())
                        .map(|c| c.to_lowercase());

                    events_id.insert(id.clone(), evt.clone());
                    events.push(evt.clone());

                    if let Some(country) = country_opt {
                        country_ids.entry(country).or_default().push(id.clone());
                    }

                    author_ids.entry(author_hex).or_default().push(id.clone());

                    if let Some(n) = npub {
                        npub_ids.entry(n).or_default().push(id.clone());
                    }
                    if let Some(n05) = author_nip05 {
                        let n05 = n05.to_lowercase();
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
                        "Skipping malformed listing event"
                    );
                }
            }
        }

        let sort_ids = |ids: &mut Vec<String>,
                        map: &BTreeMap<String, RadrootsListingEventIndex>| {
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

        for ids in country_ids.values_mut() {
            sort_ids(ids, &events_id);
        }
        for ids in author_ids.values_mut() {
            sort_ids(ids, &events_id);
        }
        for ids in npub_ids.values_mut() {
            sort_ids(ids, &events_id);
        }
        for ids in nip05_ids.values_mut() {
            ids.sort_unstable_by(|a, b| {
                let pa = events_id
                    .get(a)
                    .map(|e| e.metadata.published_at)
                    .unwrap_or_default();
                let pb = events_id
                    .get(b)
                    .map(|e| e.metadata.published_at)
                    .unwrap_or_default();
                pb.cmp(&pa).then(a.cmp(b))
            });
        }

        Ok(EventListingIndexes {
            events,
            events_id,
            country_ids,
            author_ids,
            npub_ids,
            nip05_ids,
        })
    }
}

impl EventIndexes for EventListingIndexes {
    type Event = RelayIndexerEvent;

    fn subdirs() -> &'static [crate::domain::indexer::key::IndexerKey] {
        &LISTING_INDEX_DIRECTORY
    }

    #[instrument(skip(raw_events), fields(event_count = raw_events.len()))]
    fn build(raw_events: &[Self::Event]) -> Result<Self, NostrEventsStaticError> {
        let empty = ProfileResolver::default();
        Self::build_with_profiles(raw_events, &empty)
    }
}

impl EventListingIndexes {
    fn format_shard_filename(ix: usize) -> String {
        format!("shards/{:06}.json", ix)
    }

    fn shard_vec<T: Clone>(items: &[T], size: usize) -> Vec<Vec<T>> {
        if size == 0 {
            return vec![items.to_vec()];
        }
        let mut out = Vec::with_capacity((items.len() + size - 1) / size);
        let mut i = 0;
        while i < items.len() {
            let end = (i + size).min(items.len());
            out.push(items[i..end].to_vec());
            i = end;
        }
        out
    }
}

impl WriteEventIndexes for EventListingIndexes {
    fn write(&self, settings: &Settings, updated: &mut Vec<PathBuf>) -> anyhow::Result<()> {
        let base: PathBuf = IndexerEventKind::Listing.base_path(&settings.indexer.data_dir)?;
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
                write_if_stale!(dir.join("data.json"), evt.metadata.clone(), updated);
            }
        }

        {
            let sub_country = base.join(crate::domain::indexer::key::IndexerKey::Country.as_str());
            fs_mkdir(&[&sub_country])?;
            let country_codes: Vec<String> = self.country_ids.keys().cloned().collect();
            write_if_stale!(sub_country.join("indexes.json"), country_codes, updated);

            for (cc, ids) in &self.country_ids {
                let cc_dir = sub_country.join(cc);
                let shards_dir = cc_dir.join("shards");
                fs_mkdir(&[&cc_dir])?;
                fs_mkdir(&[&shards_dir])?;

                let mut data_items: Vec<RadrootsListingEventMetadata> =
                    Vec::with_capacity(ids.len());
                for id in ids {
                    if let Some(evt) = self.events_id.get(id) {
                        data_items.push(evt.metadata.clone());
                    }
                }

                let shard_size = settings.listings.country_shard_size;

                let shards = Self::shard_vec(&data_items, shard_size);

                let (country_first_pub, country_last_pub) =
                    if let (Some(f), Some(l)) = (data_items.first(), data_items.last()) {
                        (f.published_at, l.published_at)
                    } else {
                        (0, 0)
                    };

                let mut manifest = RadrootsEventsIndexedManifest {
                    country: cc.clone(),
                    total: u32::try_from(data_items.len()).expect("too many data items for u32"),
                    shard_size: u32::try_from(shard_size).expect("shard_size too large for u32"),
                    first_published_at: country_first_pub,
                    last_published_at: country_last_pub,
                    shards: Vec::with_capacity(shards.len()),
                };

                for (ix, chunk) in shards.into_iter().enumerate() {
                    let file_rel = Self::format_shard_filename(ix);
                    let file_abs = cc_dir.join(&file_rel);
                    if let Some(parent) = file_abs.parent() {
                        fs_mkdir(&[&parent])?;
                    }

                    let sha = write_if_stale!(file_abs, chunk, updated);

                    let (first_id, first_pub, last_id, last_pub) = if let (Some(f), Some(l)) = (
                        data_items.get(ix * shard_size),
                        data_items.get(((ix + 1) * shard_size).saturating_sub(1)),
                    ) {
                        (f.id.clone(), f.published_at, l.id.clone(), l.published_at)
                    } else {
                        let fp = chunk
                            .first()
                            .map(|x| (x.id.clone(), x.published_at))
                            .unwrap_or_default();
                        let lp = chunk
                            .last()
                            .map(|x| (x.id.clone(), x.published_at))
                            .unwrap_or_default();
                        (fp.0, fp.1, lp.0, lp.1)
                    };

                    manifest.shards.push(RadrootsEventsIndexedShardMetadata {
                        file: file_rel,
                        count: u32::try_from(chunk.len()).expect("chunk length too large for u32"),
                        first_id,
                        last_id,
                        first_published_at: first_pub,
                        last_published_at: last_pub,
                        sha256: sha,
                    });
                }

                write_if_stale!(cc_dir.join("manifest.json"), manifest, updated);
            }
        }

        {
            let sub_author = base.join(crate::domain::indexer::key::IndexerKey::Author.as_str());
            fs_mkdir(&[&sub_author])?;
            let authors: Vec<String> = self.author_ids.keys().cloned().collect();
            write_if_stale!(sub_author.join("indexes.json"), authors, updated);

            for (author, ids) in &self.author_ids {
                let dir = sub_author.join(author);
                let shards_dir = dir.join("shards");
                fs_mkdir(&[&dir, &shards_dir])?;

                let mut data_items: Vec<RadrootsListingEventMetadata> =
                    Vec::with_capacity(ids.len());
                for id in ids {
                    if let Some(evt) = self.events_id.get(id) {
                        data_items.push(evt.metadata.clone());
                    }
                }

                let shard_size = settings.listings.profile_shard_size;
                let shards = Self::shard_vec(&data_items, shard_size);

                let (first_pub, last_pub) =
                    if let (Some(f), Some(l)) = (data_items.first(), data_items.last()) {
                        (f.published_at, l.published_at)
                    } else {
                        (0, 0)
                    };

                let mut manifest = RadrootsEventsIndexedManifest {
                    country: author.clone(),
                    total: u32::try_from(data_items.len()).expect("too many data items for u32"),
                    shard_size: u32::try_from(shard_size).expect("shard_size too large for u32"),
                    first_published_at: first_pub,
                    last_published_at: last_pub,
                    shards: Vec::with_capacity(shards.len()),
                };

                for (ix, chunk) in shards.into_iter().enumerate() {
                    let file_rel = Self::format_shard_filename(ix);
                    let file_abs = dir.join(&file_rel);
                    if let Some(parent) = file_abs.parent() {
                        fs_mkdir(&[&parent])?;
                    }

                    let sha = write_if_stale!(file_abs, chunk, updated);

                    let (first_id, first_published_at, last_id, last_published_at) =
                        if let (Some(f), Some(l)) = (
                            data_items.get(ix * shard_size),
                            data_items.get(((ix + 1) * shard_size).saturating_sub(1)),
                        ) {
                            (f.id.clone(), f.published_at, l.id.clone(), l.published_at)
                        } else {
                            let fp = data_items
                                .get(ix * shard_size)
                                .map(|x| (x.id.clone(), x.published_at))
                                .or_else(|| chunk.first().map(|x| (x.id.clone(), x.published_at)))
                                .unwrap_or_default();

                            let lp = data_items
                                .get(((ix + 1) * shard_size).saturating_sub(1))
                                .map(|x| (x.id.clone(), x.published_at))
                                .or_else(|| chunk.last().map(|x| (x.id.clone(), x.published_at)))
                                .unwrap_or_default();

                            (fp.0, fp.1, lp.0, lp.1)
                        };

                    manifest.shards.push(RadrootsEventsIndexedShardMetadata {
                        file: file_rel,
                        count: u32::try_from(std::cmp::min(
                            shard_size,
                            data_items.len().saturating_sub(ix * shard_size),
                        ))
                        .expect("chunk length too large for u32"),
                        first_id,
                        last_id,
                        first_published_at,
                        last_published_at,
                        sha256: sha,
                    });
                }

                write_if_stale!(dir.join("manifest.json"), manifest, updated);
            }
        }

        {
            let sub_npub = base.join(crate::domain::indexer::key::IndexerKey::Npub.as_str());
            fs_mkdir(&[&sub_npub])?;
            let npubs: Vec<String> = self.npub_ids.keys().cloned().collect();
            write_if_stale!(sub_npub.join("indexes.json"), npubs, updated);

            for (npub, ids) in &self.npub_ids {
                let dir = sub_npub.join(npub);
                let shards_dir = dir.join("shards");
                fs_mkdir(&[&dir, &shards_dir])?;

                let mut data_items: Vec<RadrootsListingEventMetadata> =
                    Vec::with_capacity(ids.len());
                for id in ids {
                    if let Some(evt) = self.events_id.get(id) {
                        data_items.push(evt.metadata.clone());
                    }
                }

                let shard_size = settings.listings.profile_shard_size;
                let shards = Self::shard_vec(&data_items, shard_size);

                let (first_pub, last_pub) =
                    if let (Some(f), Some(l)) = (data_items.first(), data_items.last()) {
                        (f.published_at, l.published_at)
                    } else {
                        (0, 0)
                    };

                let mut manifest = RadrootsEventsIndexedManifest {
                    country: npub.clone(),
                    total: u32::try_from(data_items.len()).expect("too many data items for u32"),
                    shard_size: u32::try_from(shard_size).expect("shard_size too large for u32"),
                    first_published_at: first_pub,
                    last_published_at: last_pub,
                    shards: Vec::with_capacity(shards.len()),
                };

                for (ix, chunk) in shards.into_iter().enumerate() {
                    let file_rel = Self::format_shard_filename(ix);
                    let file_abs = dir.join(&file_rel);
                    if let Some(parent) = file_abs.parent() {
                        fs_mkdir(&[&parent])?;
                    }

                    let sha = write_if_stale!(file_abs, chunk, updated);

                    let (first_id, first_published_at, last_id, last_published_at) =
                        if let (Some(f), Some(l)) = (
                            data_items.get(ix * shard_size),
                            data_items.get(((ix + 1) * shard_size).saturating_sub(1)),
                        ) {
                            (f.id.clone(), f.published_at, l.id.clone(), l.published_at)
                        } else {
                            let fp = data_items
                                .get(ix * shard_size)
                                .map(|x| (x.id.clone(), x.published_at))
                                .or_else(|| chunk.first().map(|x| (x.id.clone(), x.published_at)))
                                .unwrap_or_default();

                            let lp = data_items
                                .get(((ix + 1) * shard_size).saturating_sub(1))
                                .map(|x| (x.id.clone(), x.published_at))
                                .or_else(|| chunk.last().map(|x| (x.id.clone(), x.published_at)))
                                .unwrap_or_default();

                            (fp.0, fp.1, lp.0, lp.1)
                        };

                    manifest.shards.push(RadrootsEventsIndexedShardMetadata {
                        file: file_rel,
                        count: u32::try_from(std::cmp::min(
                            shard_size,
                            data_items.len().saturating_sub(ix * shard_size),
                        ))
                        .expect("chunk length too large for u32"),
                        first_id,
                        last_id,
                        first_published_at,
                        last_published_at,
                        sha256: sha,
                    });
                }

                write_if_stale!(dir.join("manifest.json"), manifest, updated);
            }

            {
                let sub_nip05 = base.join(crate::domain::indexer::key::IndexerKey::Nip05.as_str());
                fs_mkdir(&[&sub_nip05])?;
                let names: Vec<String> = self.nip05_ids.keys().cloned().collect();
                write_if_stale!(sub_nip05.join("indexes.json"), names, updated);

                for (name, ids) in &self.nip05_ids {
                    let dir = sub_nip05.join(name);
                    let shards_dir = dir.join("shards");
                    fs_mkdir(&[&dir, &shards_dir])?;

                    let mut data_items = Vec::with_capacity(ids.len());
                    for id in ids {
                        if let Some(evt) = self.events_id.get(id) {
                            data_items.push(evt.metadata.clone());
                        }
                    }

                    let shard_size = settings.listings.profile_shard_size;
                    let shards = Self::shard_vec(&data_items, shard_size);

                    let (first_pub, last_pub) =
                        if let (Some(f), Some(l)) = (data_items.first(), data_items.last()) {
                            (f.published_at, l.published_at)
                        } else {
                            (0, 0)
                        };

                    let mut manifest = RadrootsEventsIndexedManifest {
                        country: name.clone(),
                        total: u32::try_from(data_items.len()).expect("u32 overflow"),
                        shard_size: u32::try_from(shard_size).expect("u32 overflow"),
                        first_published_at: first_pub,
                        last_published_at: last_pub,
                        shards: Vec::with_capacity(shards.len()),
                    };

                    for (ix, chunk) in shards.into_iter().enumerate() {
                        let file_rel = Self::format_shard_filename(ix);
                        let file_abs = dir.join(&file_rel);
                        if let Some(parent) = file_abs.parent() {
                            fs_mkdir(&[&parent])?;
                        }

                        let sha = write_if_stale!(file_abs, chunk, updated);

                        let (first_id, first_pub, last_id, last_pub) = if let (Some(f), Some(l)) = (
                            data_items.get(ix * shard_size),
                            data_items.get(((ix + 1) * shard_size).saturating_sub(1)),
                        ) {
                            (f.id.clone(), f.published_at, l.id.clone(), l.published_at)
                        } else {
                            let fp = chunk
                                .first()
                                .map(|x| (x.id.clone(), x.published_at))
                                .unwrap_or_default();
                            let lp = chunk
                                .last()
                                .map(|x| (x.id.clone(), x.published_at))
                                .unwrap_or_default();
                            (fp.0, fp.1, lp.0, lp.1)
                        };

                        manifest.shards.push(RadrootsEventsIndexedShardMetadata {
                            file: file_rel,
                            count: u32::try_from(std::cmp::min(
                                shard_size,
                                data_items.len().saturating_sub(ix * shard_size),
                            ))
                            .expect("u32 overflow"),
                            first_id,
                            last_id,
                            first_published_at: first_pub,
                            last_published_at: last_pub,
                            sha256: sha,
                        });
                    }

                    write_if_stale!(dir.join("manifest.json"), manifest, updated);
                }
            }
        }

        Ok(())
    }
}
