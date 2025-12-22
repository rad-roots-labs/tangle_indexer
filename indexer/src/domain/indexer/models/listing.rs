use anyhow::anyhow;
use crate::utils::io::{fs_mkdir, safe_path_segment, write_json_if_changed};
use crate::utils::nostr::public_key_to_npub;
use crate::utils::strings::truncate_log;
use radroots_events::listing::{RadrootsListingEventIndex, RadrootsListingEventMetadata};
use radroots_events_indexed::{RadrootsEventsIndexedManifest, RadrootsEventsIndexedShardMetadata};
use std::{collections::BTreeMap, path::PathBuf};
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

#[derive(Debug)]
pub struct EventListingIndexes {
    events: Vec<RadrootsListingEventIndex>,
    events_id: BTreeMap<String, usize>,
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
        let mut events_id: BTreeMap<String, usize> = BTreeMap::new();
        let mut country_ids: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut author_ids: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut npub_ids: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut nip05_ids: BTreeMap<String, Vec<String>> = BTreeMap::new();

        for raw in raw_events {
            match raw.to_radroots_listing_event() {
                Ok(evt) => {
                    audit::log_listing_event(&evt);

                    let id = evt.metadata.id.clone();
                    let author_hex = evt.metadata.author.to_lowercase();

                    let npub = public_key_to_npub(&author_hex)
                        .map(|mut s| {
                            s.make_ascii_lowercase();
                            s
                        })
                        .ok();
                    let author_nip05 = profiles.nip05_for_author(&author_hex).map(str::to_owned);

                    let country_opt = evt
                        .metadata
                        .listing
                        .location
                        .as_ref()
                        .and_then(|loc| loc.country.as_ref())
                        .map(|c| c.to_lowercase());

                    events.push(evt);
                    let idx = events.len() - 1;
                    events_id.insert(id.clone(), idx);

                    if let Some(country) = country_opt {
                        country_ids.entry(country).or_default().push(id.clone());
                    }

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
                        "Skipping malformed listing event"
                    );
                }
            }
        }

        let sort_ids = |ids: &mut Vec<String>,
                        map: &BTreeMap<String, usize>,
                        events: &[RadrootsListingEventIndex]| {
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

        for ids in country_ids.values_mut() {
            sort_ids(ids, &events_id, &events);
        }
        for ids in author_ids.values_mut() {
            sort_ids(ids, &events_id, &events);
        }
        for ids in npub_ids.values_mut() {
            sort_ids(ids, &events_id, &events);
        }
        for ids in nip05_ids.values_mut() {
            ids.sort_unstable_by(|a, b| {
                let pa = events_id
                    .get(a)
                    .map(|idx| events[*idx].metadata.published_at)
                    .unwrap_or_default();
                let pb = events_id
                    .get(b)
                    .map(|idx| events[*idx].metadata.published_at)
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
        if items.is_empty() {
            return Vec::new();
        }
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

    fn manifest_shard_size(configured: usize, len: usize) -> usize {
        if configured == 0 {
            len
        } else {
            configured
        }
    }

    fn effective_shard_size(configured: usize, len: usize) -> usize {
        let size = Self::manifest_shard_size(configured, len);
        size.max(1)
    }

    fn usize_to_u32(value: usize, label: &str) -> anyhow::Result<u32> {
        u32::try_from(value).map_err(|_| anyhow!("{label} too large for u32"))
    }
}

impl WriteEventIndexes for EventListingIndexes {
    fn write(&self, settings: &Settings, updated: &mut Vec<PathBuf>) -> anyhow::Result<()> {
        let base: PathBuf = IndexerEventKind::Listing.base_path(&settings.indexer.data_dir)?;
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
                    warn!(id = %id, "Skipping unsafe listing id path segment");
                    continue;
                };
                let dir = sub.join(dir_key);
                let evt = &self.events[*idx];
                fs_mkdir(&[&dir])?;
                write_json_if_changed(&dir.join("event.json"), &evt.event, updated)?;
                write_json_if_changed(&dir.join("data.json"), &evt.metadata, updated)?;
            }
        }

        {
            let sub_country = base.join(crate::domain::indexer::key::IndexerKey::Country.as_str());
            fs_mkdir(&[&sub_country])?;
            let country_codes: Vec<String> = self
                .country_ids
                .keys()
                .filter_map(|cc| safe_path_segment(cc))
                .collect();
            write_json_if_changed(&sub_country.join("indexes.json"), &country_codes, updated)?;

            for (cc, ids) in &self.country_ids {
                let Some(dir_key) = safe_path_segment(cc) else {
                    warn!(country = %cc, "Skipping unsafe country path segment");
                    continue;
                };
                let cc_dir = sub_country.join(dir_key);
                let shards_dir = cc_dir.join("shards");
                fs_mkdir(&[&cc_dir])?;
                fs_mkdir(&[&shards_dir])?;

                let mut data_items: Vec<&RadrootsListingEventMetadata> =
                    Vec::with_capacity(ids.len());
                for id in ids {
                    if let Some(idx) = self.events_id.get(id) {
                        data_items.push(&self.events[*idx].metadata);
                    }
                }

                let shard_size = settings.listings.country_shard_size;
                let manifest_shard_size =
                    Self::manifest_shard_size(shard_size, data_items.len());
                let effective_shard_size =
                    Self::effective_shard_size(shard_size, data_items.len());

                let shards = Self::shard_vec(&data_items, shard_size);

                let (country_first_pub, country_last_pub) =
                    if let (Some(f), Some(l)) = (data_items.first(), data_items.last()) {
                        (f.published_at, l.published_at)
                    } else {
                        (0, 0)
                    };

                let mut manifest = RadrootsEventsIndexedManifest {
                    country: cc.clone(),
                    total: Self::usize_to_u32(data_items.len(), "data items")?,
                    shard_size: Self::usize_to_u32(manifest_shard_size, "shard_size")?,
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

                    let sha = write_json_if_changed(&file_abs, &chunk, updated)?;

                    let (first_id, first_pub, last_id, last_pub) = if let (Some(f), Some(l)) = (
                        data_items.get(ix * effective_shard_size),
                        data_items
                            .get(((ix + 1) * effective_shard_size).saturating_sub(1)),
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
                        count: Self::usize_to_u32(chunk.len(), "chunk length")?,
                        first_id,
                        last_id,
                        first_published_at: first_pub,
                        last_published_at: last_pub,
                        sha256: sha,
                    });
                }

                write_json_if_changed(&cc_dir.join("manifest.json"), &manifest, updated)?;
            }
        }

        {
            let sub_author = base.join(crate::domain::indexer::key::IndexerKey::Author.as_str());
            fs_mkdir(&[&sub_author])?;
            let authors: Vec<String> = self
                .author_ids
                .keys()
                .filter_map(|author| safe_path_segment(author))
                .collect();
            write_json_if_changed(&sub_author.join("indexes.json"), &authors, updated)?;

            for (author, ids) in &self.author_ids {
                let Some(dir_key) = safe_path_segment(author) else {
                    warn!(author = %author, "Skipping unsafe author path segment");
                    continue;
                };
                let dir = sub_author.join(dir_key);
                let shards_dir = dir.join("shards");
                fs_mkdir(&[&dir, &shards_dir])?;

                let mut data_items: Vec<&RadrootsListingEventMetadata> =
                    Vec::with_capacity(ids.len());
                for id in ids {
                    if let Some(idx) = self.events_id.get(id) {
                        data_items.push(&self.events[*idx].metadata);
                    }
                }

                let shard_size = settings.listings.profile_shard_size;
                let manifest_shard_size =
                    Self::manifest_shard_size(shard_size, data_items.len());
                let effective_shard_size =
                    Self::effective_shard_size(shard_size, data_items.len());
                let shards = Self::shard_vec(&data_items, shard_size);

                let (first_pub, last_pub) =
                    if let (Some(f), Some(l)) = (data_items.first(), data_items.last()) {
                        (f.published_at, l.published_at)
                    } else {
                        (0, 0)
                    };

                let mut manifest = RadrootsEventsIndexedManifest {
                    country: author.clone(),
                    total: Self::usize_to_u32(data_items.len(), "data items")?,
                    shard_size: Self::usize_to_u32(manifest_shard_size, "shard_size")?,
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

                    let sha = write_json_if_changed(&file_abs, &chunk, updated)?;

                    let (first_id, first_published_at, last_id, last_published_at) =
                        if let (Some(f), Some(l)) = (
                            data_items.get(ix * effective_shard_size),
                            data_items
                                .get(((ix + 1) * effective_shard_size).saturating_sub(1)),
                        ) {
                            (f.id.clone(), f.published_at, l.id.clone(), l.published_at)
                        } else {
                            let fp = data_items
                                .get(ix * effective_shard_size)
                                .map(|x| (x.id.clone(), x.published_at))
                                .or_else(|| chunk.first().map(|x| (x.id.clone(), x.published_at)))
                                .unwrap_or_default();

                            let lp = data_items
                                .get(((ix + 1) * effective_shard_size).saturating_sub(1))
                                .map(|x| (x.id.clone(), x.published_at))
                                .or_else(|| chunk.last().map(|x| (x.id.clone(), x.published_at)))
                                .unwrap_or_default();

                            (fp.0, fp.1, lp.0, lp.1)
                        };

                    manifest.shards.push(RadrootsEventsIndexedShardMetadata {
                        file: file_rel,
                        count: Self::usize_to_u32(chunk.len(), "chunk length")?,
                        first_id,
                        last_id,
                        first_published_at,
                        last_published_at,
                        sha256: sha,
                    });
                }

                write_json_if_changed(&dir.join("manifest.json"), &manifest, updated)?;
            }
        }

        {
            let sub_npub = base.join(crate::domain::indexer::key::IndexerKey::Npub.as_str());
            fs_mkdir(&[&sub_npub])?;
            let npubs: Vec<String> = self
                .npub_ids
                .keys()
                .filter_map(|npub| safe_path_segment(npub))
                .collect();
            write_json_if_changed(&sub_npub.join("indexes.json"), &npubs, updated)?;

            for (npub, ids) in &self.npub_ids {
                let Some(dir_key) = safe_path_segment(npub) else {
                    warn!(npub = %npub, "Skipping unsafe npub path segment");
                    continue;
                };
                let dir = sub_npub.join(dir_key);
                let shards_dir = dir.join("shards");
                fs_mkdir(&[&dir, &shards_dir])?;

                let mut data_items: Vec<&RadrootsListingEventMetadata> =
                    Vec::with_capacity(ids.len());
                for id in ids {
                    if let Some(idx) = self.events_id.get(id) {
                        data_items.push(&self.events[*idx].metadata);
                    }
                }

                let shard_size = settings.listings.profile_shard_size;
                let manifest_shard_size =
                    Self::manifest_shard_size(shard_size, data_items.len());
                let effective_shard_size =
                    Self::effective_shard_size(shard_size, data_items.len());
                let shards = Self::shard_vec(&data_items, shard_size);

                let (first_pub, last_pub) =
                    if let (Some(f), Some(l)) = (data_items.first(), data_items.last()) {
                        (f.published_at, l.published_at)
                    } else {
                        (0, 0)
                    };

                let mut manifest = RadrootsEventsIndexedManifest {
                    country: npub.clone(),
                    total: Self::usize_to_u32(data_items.len(), "data items")?,
                    shard_size: Self::usize_to_u32(manifest_shard_size, "shard_size")?,
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

                    let sha = write_json_if_changed(&file_abs, &chunk, updated)?;

                    let (first_id, first_published_at, last_id, last_published_at) =
                        if let (Some(f), Some(l)) = (
                            data_items.get(ix * effective_shard_size),
                            data_items
                                .get(((ix + 1) * effective_shard_size).saturating_sub(1)),
                        ) {
                            (f.id.clone(), f.published_at, l.id.clone(), l.published_at)
                        } else {
                            let fp = data_items
                                .get(ix * effective_shard_size)
                                .map(|x| (x.id.clone(), x.published_at))
                                .or_else(|| chunk.first().map(|x| (x.id.clone(), x.published_at)))
                                .unwrap_or_default();

                            let lp = data_items
                                .get(((ix + 1) * effective_shard_size).saturating_sub(1))
                                .map(|x| (x.id.clone(), x.published_at))
                                .or_else(|| chunk.last().map(|x| (x.id.clone(), x.published_at)))
                                .unwrap_or_default();

                            (fp.0, fp.1, lp.0, lp.1)
                        };

                    manifest.shards.push(RadrootsEventsIndexedShardMetadata {
                        file: file_rel,
                        count: Self::usize_to_u32(chunk.len(), "chunk length")?,
                        first_id,
                        last_id,
                        first_published_at,
                        last_published_at,
                        sha256: sha,
                    });
                }

                write_json_if_changed(&dir.join("manifest.json"), &manifest, updated)?;
            }

            {
                let sub_nip05 = base.join(crate::domain::indexer::key::IndexerKey::Nip05.as_str());
                fs_mkdir(&[&sub_nip05])?;
                let names: Vec<String> = self
                    .nip05_ids
                    .keys()
                    .filter_map(|name| safe_path_segment(name))
                    .collect();
                write_json_if_changed(&sub_nip05.join("indexes.json"), &names, updated)?;

                for (name, ids) in &self.nip05_ids {
                    let Some(dir_key) = safe_path_segment(name) else {
                        warn!(nip05 = %name, "Skipping unsafe nip05 path segment");
                        continue;
                    };
                    let dir = sub_nip05.join(dir_key);
                    let shards_dir = dir.join("shards");
                    fs_mkdir(&[&dir, &shards_dir])?;

                    let mut data_items: Vec<&RadrootsListingEventMetadata> =
                        Vec::with_capacity(ids.len());
                    for id in ids {
                        if let Some(idx) = self.events_id.get(id) {
                            data_items.push(&self.events[*idx].metadata);
                        }
                    }

                    let shard_size = settings.listings.profile_shard_size;
                    let manifest_shard_size =
                        Self::manifest_shard_size(shard_size, data_items.len());
                    let effective_shard_size =
                        Self::effective_shard_size(shard_size, data_items.len());
                    let shards = Self::shard_vec(&data_items, shard_size);

                    let (first_pub, last_pub) =
                        if let (Some(f), Some(l)) = (data_items.first(), data_items.last()) {
                            (f.published_at, l.published_at)
                        } else {
                            (0, 0)
                        };

                    let mut manifest = RadrootsEventsIndexedManifest {
                        country: name.clone(),
                        total: Self::usize_to_u32(data_items.len(), "data items")?,
                        shard_size: Self::usize_to_u32(manifest_shard_size, "shard_size")?,
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

                        let sha = write_json_if_changed(&file_abs, &chunk, updated)?;

                        let (first_id, first_pub, last_id, last_pub) = if let (Some(f), Some(l)) = (
                            data_items.get(ix * effective_shard_size),
                            data_items
                                .get(((ix + 1) * effective_shard_size).saturating_sub(1)),
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
                            count: Self::usize_to_u32(chunk.len(), "chunk length")?,
                            first_id,
                            last_id,
                            first_published_at: first_pub,
                            last_published_at: last_pub,
                            sha256: sha,
                        });
                    }

                    write_json_if_changed(&dir.join("manifest.json"), &manifest, updated)?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::EventListingIndexes;

    #[test]
    fn shard_vec_empty_returns_empty() {
        let items: Vec<u32> = Vec::new();
        let shards = EventListingIndexes::shard_vec(&items, 0);
        assert!(shards.is_empty());
    }

    #[test]
    fn shard_vec_zero_size_groups_all() {
        let items = vec![1u32, 2, 3];
        let shards = EventListingIndexes::shard_vec(&items, 0);
        assert_eq!(shards.len(), 1);
        assert_eq!(shards[0], items);
    }
}
