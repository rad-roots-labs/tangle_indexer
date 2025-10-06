use crate::domain::events::ToRadrootsProfileEventIndex;
use crate::relay::event::RelayIndexerEvent;
use std::collections::BTreeMap;

#[derive(Default, Clone)]
pub struct ProfileResolver {
    author_to_nip05: BTreeMap<String, String>,
}

impl ProfileResolver {
    pub fn from_metadata(raw_metadata: &[RelayIndexerEvent]) -> Self {
        let mut latest: BTreeMap<String, (u64, String)> = BTreeMap::new();

        for raw in raw_metadata {
            if let Ok(evt) = raw.clone().to_radroots_profile_event() {
                if let Some(n) = &evt.metadata.profile.nip05 {
                    let normalized = n.replace("@radroots.market", "").to_lowercase();
                    if normalized.is_empty() {
                        continue;
                    }

                    let author = evt.event.author.to_lowercase();
                    let ts: u64 = evt.metadata.published_at;
                    match latest.get(&author) {
                        Some(&(old_ts, _)) if old_ts >= ts => {}
                        _ => {
                            latest.insert(author, (ts, normalized));
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
            .get(&author_hex.to_lowercase())
            .map(|s| s.as_str())
    }
}
