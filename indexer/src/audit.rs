#![cfg(feature = "audit")]

use std::collections::HashSet;
use std::sync::RwLock;

use crate::utils::nostr::public_key_to_npub;
use once_cell::sync::Lazy;
use regex::Regex;
use tracing::info;

use crate::domain::resolvers::profile::ProfileResolver;
use crate::relay::event::RelayIndexerEvent;
use radroots_events::listing::models::RadrootsListingEventIndex;
use radroots_events::profile::models::RadrootsProfileEventIndex;

#[derive(Clone, Debug)]
pub struct AuditFilter {
    pub enabled: bool,
    pub kinds: Option<HashSet<u64>>,
    pub authors: HashSet<String>,
    pub npubs: HashSet<String>,
    pub nip05_full: HashSet<String>,
    pub nip05_local: HashSet<String>,
    pub content_re: Option<Regex>,
    pub created_at_min: Option<u32>,
    pub created_at_max: Option<u32>,
}

impl Default for AuditFilter {
    fn default() -> Self {
        Self {
            enabled: false,
            kinds: None,
            authors: HashSet::new(),
            npubs: HashSet::new(),
            nip05_full: HashSet::new(),
            nip05_local: HashSet::new(),
            content_re: None,
            created_at_min: None,
            created_at_max: None,
        }
    }
}

impl AuditFilter {
    pub fn from_env() -> Self {
        let mut f = Self::default();

        f.enabled = std::env::var("AUDIT_ENABLED")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);

        if let Ok(v) = std::env::var("AUDIT_KINDS") {
            let set = v
                .split(',')
                .filter_map(|s| s.trim().parse::<u64>().ok())
                .collect::<HashSet<_>>();
            if !set.is_empty() {
                f.kinds = Some(set);
            }
        }

        let parse_set = |key: &str| -> HashSet<String> {
            std::env::var(key)
                .ok()
                .map(|s| {
                    s.split(',')
                        .map(|x| x.trim().to_lowercase())
                        .filter(|x| !x.is_empty())
                        .collect()
                })
                .unwrap_or_default()
        };

        f.authors = parse_set("AUDIT_AUTHORS");
        f.npubs = parse_set("AUDIT_NPUBS");
        f.nip05_full = parse_set("AUDIT_NIP05");
        f.nip05_local = parse_set("AUDIT_NIP05_LOCAL");

        if let Ok(rx) = std::env::var("AUDIT_CONTENT_RE") {
            if !rx.trim().is_empty() {
                if let Ok(re) = Regex::new(&format!("(?i){}", rx)) {
                    f.content_re = Some(re);
                }
            }
        }

        f.created_at_min = std::env::var("AUDIT_CREATED_AT_MIN")
            .ok()
            .and_then(|s| s.parse().ok());
        f.created_at_max = std::env::var("AUDIT_CREATED_AT_MAX")
            .ok()
            .and_then(|s| s.parse().ok());

        f
    }
}

#[derive(Clone)]
struct AuditState {
    filter: AuditFilter,
    resolver: Option<ProfileResolver>,
}

static STATE: Lazy<RwLock<AuditState>> = Lazy::new(|| {
    RwLock::new(AuditState {
        filter: AuditFilter::from_env(),
        resolver: None,
    })
});

pub fn reload_from_env() {
    if let Ok(mut w) = STATE.write() {
        w.filter = AuditFilter::from_env();
    }
}

pub fn set_profile_resolver(resolver: ProfileResolver) {
    if let Ok(mut w) = STATE.write() {
        w.resolver = Some(resolver);
    }
}

fn nip05_parts_from_metadata(nip05: &str) -> (String, String) {
    let lower = nip05.to_lowercase();
    if let Some((name, domain)) = lower.split_once('@') {
        (format!("{name}@{domain}"), name.to_string())
    } else {
        (lower.clone(), lower)
    }
}

fn should_log(
    author_hex: &str,
    kind_u64: u64,
    created_at: u32,
    content: &str,
    npub_opt: Option<String>,
    nip05_full_opt: Option<String>,
    nip05_local_opt: Option<String>,
) -> bool {
    let filter = STATE.read().ok().map(|s| s.filter.clone());
    let Some(f) = filter else {
        return false;
    };
    if !f.enabled {
        return false;
    }

    if let Some(kinds) = &f.kinds {
        if !kinds.contains(&kind_u64) {
            return false;
        }
    }

    if !f.authors.is_empty() && !f.authors.contains(&author_hex.to_lowercase()) {
        return false;
    }

    if !f.npubs.is_empty() {
        let pass = npub_opt
            .as_ref()
            .map(|n| f.npubs.contains(&n.to_lowercase()))
            .unwrap_or(false);
        if !pass {
            return false;
        }
    }

    if !f.nip05_full.is_empty() {
        let pass = nip05_full_opt
            .as_ref()
            .map(|n| f.nip05_full.contains(&n.to_lowercase()))
            .unwrap_or(false);
        if !pass {
            return false;
        }
    }

    if !f.nip05_local.is_empty() {
        let pass = nip05_local_opt
            .as_ref()
            .map(|n| f.nip05_local.contains(&n.to_lowercase()))
            .unwrap_or(false);
        if !pass {
            return false;
        }
    }

    if let Some(min) = f.created_at_min {
        if created_at < min {
            return false;
        }
    }
    if let Some(max) = f.created_at_max {
        if created_at > max {
            return false;
        }
    }

    if let Some(re) = &f.content_re {
        if !re.is_match(content) {
            return false;
        }
    }

    true
}

#[inline]
pub fn log_indexer_event(idx: &RelayIndexerEvent) {
    let need_npub = STATE
        .read()
        .ok()
        .map(|s| !s.filter.npubs.is_empty())
        .unwrap_or(false);
    let npub_opt = if need_npub {
        public_key_to_npub(&idx.author).ok()
    } else {
        None
    };

    let (need_full, need_local) = STATE
        .read()
        .ok()
        .map(|s| {
            (
                !s.filter.nip05_full.is_empty(),
                !s.filter.nip05_local.is_empty(),
            )
        })
        .unwrap_or((false, false));

    let (nip05_full_opt, nip05_local_opt) = if need_full || need_local {
        if let Ok(s) = STATE.read() {
            if let Some(res) = s.resolver.as_ref() {
                let local = res.nip05_for_author(&idx.author).map(|s| s.to_string());
                (None, local)
            } else {
                (None, None)
            }
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    if !should_log(
        &idx.author,
        idx.kind.as_u64(),
        idx.created_at,
        &idx.content,
        npub_opt,
        nip05_full_opt,
        nip05_local_opt,
    ) {
        return;
    }

    let tags_json =
        serde_json::to_string(&idx.tags).unwrap_or_else(|_| "Error serializing tags".into());
    info!(
        target: "audit",
        kind = idx.kind.as_u64(),
        id = %idx.id,
        author = %idx.author,
        created_at = idx.created_at,
        tags = %tags_json,
        content = %idx.content,
        "AUDIT: relay indexer event"
    );
}

#[inline]
pub fn log_profile_event(evt: &RadrootsProfileEventIndex) {
    let (nip05_full_opt, nip05_local_opt) = evt
        .metadata
        .profile
        .nip05
        .as_ref()
        .map(|n| {
            let (full, local) = nip05_parts_from_metadata(n);
            (Some(full), Some(local))
        })
        .unwrap_or((None, None));

    let need_npub = STATE
        .read()
        .ok()
        .map(|s| !s.filter.npubs.is_empty())
        .unwrap_or(false);
    let npub_opt = if need_npub {
        public_key_to_npub(&evt.event.author).ok()
    } else {
        None
    };

    if !should_log(
        &evt.event.author,
        evt.event.kind.try_into().unwrap(),
        evt.event.created_at,
        &evt.event.content,
        npub_opt,
        nip05_full_opt,
        nip05_local_opt,
    ) {
        return;
    }

    if let Ok(json) = serde_json::to_string(evt) {
        info!(
            target = "audit",
            kind = evt.event.kind,
            id = %evt.event.id,
            author = %evt.event.author,
            created_at = evt.event.created_at,
            processed_json = %json,
            "AUDIT: processed metadata"
        );
    }
}

#[inline]
pub fn log_listing_event(evt: &RadrootsListingEventIndex) {
    let need_npub = STATE
        .read()
        .ok()
        .map(|s| !s.filter.npubs.is_empty())
        .unwrap_or(false);
    let npub_opt = if need_npub {
        public_key_to_npub(&evt.event.author).ok()
    } else {
        None
    };

    let (need_full, need_local) = STATE
        .read()
        .ok()
        .map(|s| {
            (
                !s.filter.nip05_full.is_empty(),
                !s.filter.nip05_local.is_empty(),
            )
        })
        .unwrap_or((false, false));

    let (nip05_full_opt, nip05_local_opt) = if need_full || need_local {
        if let Ok(s) = STATE.read() {
            if let Some(res) = s.resolver.as_ref() {
                let local = res
                    .nip05_for_author(&evt.event.author)
                    .map(|s| s.to_string());
                (None, local)
            } else {
                (None, None)
            }
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    if !should_log(
        &evt.event.author,
        evt.event.kind as u64,
        evt.event.created_at,
        &evt.event.content,
        npub_opt,
        nip05_full_opt,
        nip05_local_opt,
    ) {
        return;
    }

    if let Ok(json) = serde_json::to_string(evt) {
        info!(
            target = "audit",
            kind = evt.event.kind,
            id = %evt.event.id,
            author = %evt.event.author,
            created_at = evt.event.created_at,
            processed_json = %json,
            "AUDIT: processed listing"
        );
    }
}
