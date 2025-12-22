use crate::domain::events::ToRadrootsProfileEventIndex;
use crate::relay::event::RelayIndexerEvent;
use crate::utils::nostr::normalize_nip05;
use std::collections::BTreeMap;

#[derive(Clone)]
struct Nip05Info {
    full: String,
    local: String,
    index_key: String,
}

#[derive(Default, Clone)]
pub struct ProfileResolver {
    author_to_nip05: BTreeMap<String, Nip05Info>,
}

impl ProfileResolver {
    pub fn from_metadata(raw_metadata: &[RelayIndexerEvent]) -> Self {
        let mut latest: BTreeMap<String, (u32, Nip05Info)> = BTreeMap::new();

        for raw in raw_metadata {
            if let Ok(evt) = raw.to_radroots_profile_event() {
                if let Some(n) = &evt.metadata.profile.nip05 {
                    let (full, local, index_key) = normalize_nip05(n);
                    if index_key.is_empty() {
                        continue;
                    }

                    let author = evt.event.author.to_lowercase();
                    let ts: u32 = evt.metadata.published_at;
                    match latest.get(&author) {
                        Some(&(old_ts, _)) if old_ts >= ts => {}
                        _ => {
                            latest.insert(
                                author,
                                (
                                    ts,
                                    Nip05Info {
                                        full,
                                        local,
                                        index_key,
                                    },
                                ),
                            );
                        }
                    }
                }
            }
        }

        let author_to_nip05 = latest.into_iter().map(|(a, (_ts, n))| (a, n)).collect();

        Self { author_to_nip05 }
    }

    #[inline]
    pub fn nip05_for_author(&self, author_hex: &str) -> Option<&str> {
        self.author_to_nip05
            .get(author_hex)
            .map(|info| info.index_key.as_str())
    }

    #[inline]
    pub fn nip05_full_for_author(&self, author_hex: &str) -> Option<&str> {
        self.author_to_nip05
            .get(author_hex)
            .map(|info| info.full.as_str())
    }

    #[inline]
    pub fn nip05_local_for_author(&self, author_hex: &str) -> Option<&str> {
        self.author_to_nip05
            .get(author_hex)
            .map(|info| info.local.as_str())
    }
}
