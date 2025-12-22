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
        let mut latest: BTreeMap<String, (u32, String, Nip05Info)> = BTreeMap::new();

        for raw in raw_metadata {
            if let Ok(evt) = raw.to_radroots_profile_event() {
                if let Some(n) = &evt.metadata.profile.nip05 {
                    let (full, local, index_key) = normalize_nip05(n);
                    if index_key.is_empty() {
                        continue;
                    }

                    let author = evt.event.author.to_lowercase();
                    let ts: u32 = evt.metadata.published_at;
                    let event_id = evt.event.id.clone();
                    let should_replace = match latest.get(&author) {
                        None => true,
                        Some((old_ts, old_id, _)) => {
                            ts > *old_ts || (ts == *old_ts && event_id < *old_id)
                        }
                    };
                    if should_replace {
                        latest.insert(
                            author,
                            (
                                ts,
                                event_id,
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

        let author_to_nip05 = latest
            .into_iter()
            .map(|(a, (_ts, _id, n))| (a, n))
            .collect();

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

#[cfg(test)]
mod tests {
    use super::ProfileResolver;
    use crate::domain::indexer::kind::IndexerEventKind;
    use crate::relay::event::RelayIndexerEvent;

    fn make_profile_event(id: &str, author: &str, created_at: u32, nip05: &str) -> RelayIndexerEvent {
        let content = format!(r#"{{"name":"user","nip05":"{}"}}"#, nip05);
        RelayIndexerEvent {
            id: id.to_string(),
            author: author.to_string(),
            created_at,
            pubkey: author.to_string(),
            kind: IndexerEventKind::Profile,
            tags: Vec::new(),
            content,
            hash: id.to_string(),
            sig: "sig".to_string(),
        }
    }

    #[test]
    fn resolver_tiebreaks_by_event_id() {
        let author = "a".repeat(64);
        let high = make_profile_event("f".repeat(64).as_str(), &author, 10, "high@radroots.market");
        let low = make_profile_event("0".repeat(64).as_str(), &author, 10, "low@radroots.market");

        let resolver = ProfileResolver::from_metadata(&[high, low]);
        let full = resolver
            .nip05_full_for_author(&author)
            .expect("full nip05");
        assert_eq!(full, "low@radroots.market");
    }

    #[test]
    fn resolver_returns_local_and_index_key() {
        let author = "b".repeat(64);
        let event = make_profile_event("1".repeat(64).as_str(), &author, 10, "user@radroots.market");
        let resolver = ProfileResolver::from_metadata(&[event]);
        assert_eq!(
            resolver.nip05_full_for_author(&author),
            Some("user@radroots.market")
        );
        assert_eq!(resolver.nip05_local_for_author(&author), Some("user"));
        assert_eq!(resolver.nip05_for_author(&author), Some("user"));
    }
}
