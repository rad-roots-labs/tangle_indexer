use indexer_utils::{
    file::fs_mkdir,
    logs::truncate_log,
    write::{compute_hash, write_hash, write_json},
};
use radroots_common::models::{
    events::{RadrootsListingEvent, RadrootsListingEventData},
    indexer::{RadrootsListingIndexCountryManifest, RadrootsListingIndexShardMetadata},
};
use std::{collections::BTreeMap, fs, path::PathBuf};
use tracing::{instrument, warn};

use crate::{
    audit,
    domain::{
        events::ToRadrootsListingEvent,
        indexer::{
            key::LISTING_INDEX_DIRECTORY,
            kind::IndexerEventKind,
            models::{EventIndexes, NostrEventsStaticError, WriteEventIndexes},
            IndexerKey,
        },
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
    events: Vec<RadrootsListingEvent>,
    events_id: BTreeMap<String, RadrootsListingEvent>,
    country_ids: BTreeMap<String, Vec<String>>,
}

impl EventIndexes for EventListingIndexes {
    type Event = RelayIndexerEvent;

    fn subdirs() -> &'static [IndexerKey] {
        &LISTING_INDEX_DIRECTORY
    }

    #[instrument(skip(raw_events), fields(event_count = raw_events.len()))]
    fn build(raw_events: &[Self::Event]) -> Result<Self, NostrEventsStaticError> {
        let mut events: Vec<RadrootsListingEvent> = Vec::with_capacity(raw_events.len());
        let mut events_id: BTreeMap<String, RadrootsListingEvent> = BTreeMap::new();
        let mut country_ids: BTreeMap<String, Vec<String>> = BTreeMap::new();

        for raw in raw_events {
            match raw.clone().to_radroots_listing_event() {
                Ok(evt) => {
                    audit::log_listing_event(&evt);
                    let id = evt.event.id.clone();
                    let country_code = evt.data.location_country.to_lowercase();

                    events_id.insert(id.clone(), evt.clone());
                    events.push(evt.clone());

                    country_ids.entry(country_code).or_default().push(id);
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

        for (_cc, ids) in country_ids.iter_mut() {
            ids.sort_unstable_by(|a, b| {
                let pa = events_id
                    .get(a)
                    .map(|e| e.data.published_at)
                    .unwrap_or_default();
                let pb = events_id
                    .get(b)
                    .map(|e| e.data.published_at)
                    .unwrap_or_default();

                pb.cmp(&pa).then(a.cmp(b))
            });
        }

        Ok(EventListingIndexes {
            events,
            events_id,
            country_ids,
        })
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
                write_if_stale!(dir.join("data.json"), evt.data.clone(), updated);
            }
        }

        {
            let sub_country = base.join(IndexerKey::Country.as_str());
            fs_mkdir(&[&sub_country])?;
            let country_codes: Vec<String> = self.country_ids.keys().cloned().collect();
            write_if_stale!(sub_country.join("indexes.json"), country_codes, updated);

            for (cc, ids) in &self.country_ids {
                let cc_dir = sub_country.join(cc);
                let shards_dir = cc_dir.join("shards");
                fs_mkdir(&[&cc_dir])?;
                fs_mkdir(&[&shards_dir])?;

                let mut data_items: Vec<RadrootsListingEventData> = Vec::with_capacity(ids.len());
                for id in ids {
                    if let Some(evt) = self.events_id.get(id) {
                        data_items.push(evt.data.clone());
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

                let mut manifest = RadrootsListingIndexCountryManifest {
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

                    manifest.shards.push(RadrootsListingIndexShardMetadata {
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

        Ok(())
    }
}
