use crate::relay::event::RelayIndexerEvent;
use radroots_events::{
    comment::{RadrootsComment, RadrootsCommentEventIndex, RadrootsCommentEventMetadata},
    RadrootsNostrEvent, RadrootsNostrEventRef,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RadrootsCommentEventIndexError {
    #[error("Failed to parse comment from tags")]
    ParseError,
}

fn parse_address(addr: &str) -> Option<(u32, String, Option<String>)> {
    let mut parts = addr.splitn(3, ':');
    let kind = parts.next()?.parse::<u32>().ok()?;
    let author = parts.next()?.to_lowercase();
    let d_tag = parts
        .next()
        .and_then(|d| if d.is_empty() { None } else { Some(d.to_string()) });
    Some((kind, author, d_tag))
}

fn parse_comment_from_tags(
    tags: &[Vec<String>],
    content: &str,
) -> Result<RadrootsComment, RadrootsCommentEventIndexError> {
    let mut root_id: Option<String> = None;
    let mut root_addr: Option<String> = None;
    let mut root_relays_e: Option<Vec<String>> = None;
    let mut root_relays_a: Option<Vec<String>> = None;
    let mut root_kind_tag: Option<u32> = None;
    let mut root_kind_addr: Option<u32> = None;
    let mut root_author_tag: Option<String> = None;
    let mut root_author_addr: Option<String> = None;
    let mut root_author_e: Option<String> = None;
    let mut root_d: Option<String> = None;

    let mut parent_id: Option<String> = None;
    let mut parent_addr: Option<String> = None;
    let mut parent_relays_e: Option<Vec<String>> = None;
    let mut parent_relays_a: Option<Vec<String>> = None;
    let mut parent_kind_tag: Option<u32> = None;
    let mut parent_kind_addr: Option<u32> = None;
    let mut parent_author_tag: Option<String> = None;
    let mut parent_author_addr: Option<String> = None;
    let mut parent_author_e: Option<String> = None;
    let mut parent_d: Option<String> = None;

    let mut legacy_root_id: Option<String> = None;
    let mut legacy_root_relays: Option<Vec<String>> = None;
    let mut legacy_parent_id: Option<String> = None;
    let mut legacy_parent_relays: Option<Vec<String>> = None;
    let mut legacy_root_kind: Option<u32> = None;
    let mut legacy_root_author: Option<String> = None;
    let mut legacy_root_d: Option<String> = None;

    for t in tags {
        match t.first().map(|k| k.as_str()) {
            Some("E") => {
                if let Some(id) = t.get(1).cloned() {
                    root_id = Some(id);
                }
                if let Some(r) = t.get(2).filter(|s| !s.is_empty()).cloned() {
                    root_relays_e = Some(vec![r]);
                }
                if let Some(pk) = t.get(3).filter(|s| !s.is_empty()) {
                    root_author_e = Some(pk.to_lowercase());
                }
            }
            Some("A") => {
                if let Some(addr) = t.get(1).cloned() {
                    root_addr = Some(addr.clone());
                    if let Some((kind, author, d_tag)) = parse_address(&addr) {
                        root_kind_addr = Some(kind);
                        root_author_addr = Some(author);
                        root_d = d_tag;
                    }
                }
                if let Some(r) = t.get(2).filter(|s| !s.is_empty()).cloned() {
                    root_relays_a = Some(vec![r]);
                }
            }
            Some("K") => {
                if let Some(kind) = t.get(1).and_then(|v| v.parse::<u32>().ok()) {
                    root_kind_tag = Some(kind);
                }
            }
            Some("P") => {
                if let Some(pk) = t.get(1).filter(|s| !s.is_empty()) {
                    root_author_tag = Some(pk.to_lowercase());
                }
            }
            Some("e_root") => {
                if let Some(id) = t.get(1).cloned() {
                    legacy_root_id = Some(id);
                }
                if let Some(r) = t.get(2).filter(|s| !s.is_empty()).cloned() {
                    legacy_root_relays = Some(vec![r]);
                }
            }
            Some("e_prev") => {
                if let Some(id) = t.get(1).cloned() {
                    legacy_parent_id = Some(id);
                }
                if let Some(r) = t.get(2).filter(|s| !s.is_empty()).cloned() {
                    legacy_parent_relays = Some(vec![r]);
                }
            }
            Some("e") => {
                if let Some(id) = t.get(1).cloned() {
                    parent_id = Some(id.clone());
                    if legacy_root_id.is_none() {
                        legacy_root_id = Some(id);
                    }
                }
                if let Some(r) = t.get(2).filter(|s| !s.is_empty()).cloned() {
                    parent_relays_e = Some(vec![r.clone()]);
                    if legacy_root_relays.is_none() {
                        legacy_root_relays = Some(vec![r]);
                    }
                }
                if let Some(pk) = t.get(3).filter(|s| !s.is_empty()) {
                    parent_author_e = Some(pk.to_lowercase());
                }
            }
            Some("a") => {
                if let Some(addr) = t.get(1).cloned() {
                    parent_addr = Some(addr.clone());
                    if let Some((kind, author, d_tag)) = parse_address(&addr) {
                        parent_kind_addr = Some(kind);
                        parent_author_addr = Some(author.clone());
                        parent_d = d_tag.clone();
                        if legacy_root_kind.is_none() {
                            legacy_root_kind = Some(kind);
                        }
                        if legacy_root_author.is_none() {
                            legacy_root_author = Some(author);
                        }
                        if legacy_root_d.is_none() {
                            legacy_root_d = d_tag;
                        }
                    }
                }
                if let Some(r) = t.get(2).filter(|s| !s.is_empty()).cloned() {
                    parent_relays_a = Some(vec![r]);
                }
            }
            Some("k") => {
                if let Some(kind) = t.get(1).and_then(|v| v.parse::<u32>().ok()) {
                    parent_kind_tag = Some(kind);
                }
            }
            Some("p") => {
                if let Some(pk) = t.get(1).filter(|s| !s.is_empty()) {
                    parent_author_tag = Some(pk.to_lowercase());
                }
            }
            _ => {}
        }
    }

    let has_nip22_root = root_id.is_some() || root_addr.is_some();
    if !has_nip22_root {
        if root_id.is_none() {
            root_id = legacy_root_id;
            if root_relays_e.is_none() {
                root_relays_e = legacy_root_relays;
            }
        }
        if root_kind_tag.is_none() {
            root_kind_tag = legacy_root_kind;
        }
        if root_author_tag.is_none() {
            root_author_tag = legacy_root_author;
        }
        if root_d.is_none() {
            root_d = legacy_root_d;
        }
        if parent_id.is_none() && parent_addr.is_none() {
            parent_id = legacy_parent_id;
            if parent_relays_e.is_none() {
                parent_relays_e = legacy_parent_relays;
            }
        }
    }

    let root_id = root_id.or(root_addr.clone()).ok_or(RadrootsCommentEventIndexError::ParseError)?;
    let root_kind = root_kind_tag.or(root_kind_addr).unwrap_or(1);
    let root_author = root_author_tag
        .or(root_author_addr)
        .or(root_author_e)
        .unwrap_or_default();
    let root_relays = root_relays_e.or(root_relays_a);

    let mut parent_id = parent_id.or(parent_addr.clone());
    let parent_kind = parent_kind_tag
        .or(parent_kind_addr)
        .unwrap_or(root_kind);
    let parent_author = parent_author_tag
        .or(parent_author_addr)
        .or(parent_author_e)
        .unwrap_or_else(|| root_author.clone());
    let parent_relays = parent_relays_e
        .or(parent_relays_a)
        .or_else(|| root_relays.clone());
    let parent_d = parent_d.or(root_d.clone());

    if parent_id.is_none() {
        parent_id = Some(root_id.clone());
    }

    let root = RadrootsNostrEventRef {
        id: root_id,
        author: root_author,
        kind: root_kind,
        d_tag: root_d,
        relays: root_relays,
    };

    let parent = RadrootsNostrEventRef {
        id: parent_id.ok_or(RadrootsCommentEventIndexError::ParseError)?,
        author: parent_author,
        kind: parent_kind,
        d_tag: parent_d,
        relays: parent_relays,
    };

    Ok(RadrootsComment {
        root,
        parent,
        content: content.to_string(),
    })
}

fn create_radroots_comment_event_metadata(
    id: String,
    author: String,
    published_at: u32,
    kind: u32,
    content: &str,
    tags: &[Vec<String>],
) -> Result<RadrootsCommentEventMetadata, RadrootsCommentEventIndexError> {
    let comment = parse_comment_from_tags(tags, content)?;
    Ok(RadrootsCommentEventMetadata {
        id,
        author,
        published_at,
        kind,
        comment,
    })
}

pub trait ToRadrootsCommentEventIndex {
    fn to_radroots_comment_event(
        &self,
    ) -> Result<RadrootsCommentEventIndex, RadrootsCommentEventIndexError>;
}

impl ToRadrootsCommentEventIndex for RelayIndexerEvent {
    fn to_radroots_comment_event(
        &self,
    ) -> Result<RadrootsCommentEventIndex, RadrootsCommentEventIndexError> {
        let kind_u32 = self.kind.as_u64() as u32;
        let id = self.id.clone();
        let author = self.author.clone();
        let metadata = create_radroots_comment_event_metadata(
            id.clone(),
            author.clone(),
            self.created_at,
            kind_u32,
            &self.content,
            &self.tags,
        )?;
        Ok(RadrootsCommentEventIndex {
            event: RadrootsNostrEvent {
                id,
                author,
                created_at: self.created_at,
                kind: kind_u32,
                tags: self.tags.clone(),
                content: self.content.clone(),
                sig: self.sig.clone(),
            },
            metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::parse_comment_from_tags;

    #[test]
    fn comment_parses_address_only_root_and_parent() {
        let pubkey = "f".repeat(64);
        let addr = format!("30023:{}:dtag", pubkey);
        let tags = vec![
            vec!["A".to_string(), addr.clone()],
            vec!["K".to_string(), "30023".to_string()],
            vec!["a".to_string(), addr.clone()],
            vec!["k".to_string(), "30023".to_string()],
        ];

        let comment = parse_comment_from_tags(&tags, "hello").expect("parse comment");
        assert_eq!(comment.root.id, addr);
        assert_eq!(comment.root.kind, 30023);
        assert_eq!(comment.root.author, pubkey);
        assert_eq!(comment.root.d_tag.as_deref(), Some("dtag"));
        assert_eq!(comment.parent.id, comment.root.id);
        assert_eq!(comment.parent.kind, 30023);
    }

    #[test]
    fn comment_defaults_parent_to_root_when_missing() {
        let pubkey = "e".repeat(64);
        let addr = format!("30023:{}:root", pubkey);
        let tags = vec![
            vec!["A".to_string(), addr.clone()],
            vec!["K".to_string(), "30023".to_string()],
        ];

        let comment = parse_comment_from_tags(&tags, "hello").expect("parse comment");
        assert_eq!(comment.parent.id, comment.root.id);
        assert_eq!(comment.parent.kind, comment.root.kind);
        assert_eq!(comment.parent.author, comment.root.author);
    }
}
