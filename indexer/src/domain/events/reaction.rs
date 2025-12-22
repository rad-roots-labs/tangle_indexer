use thiserror::Error;

use radroots_events::{
    reaction::{
        RadrootsReaction, RadrootsReactionEventIndex, RadrootsReactionEventMetadata,
    },
    RadrootsNostrEvent, RadrootsNostrEventRef,
};

use crate::relay::event::RelayIndexerEvent;

#[derive(Debug, Error)]
pub enum RadrootsReactionEventIndexError {
    #[error("Failed to parse reaction from tags")]
    ParseError,
}

fn parse_reaction_from_tags(
    tags: &[Vec<String>],
    content: &str,
) -> Result<RadrootsReaction, RadrootsReactionEventIndexError> {
    let parse_address = |addr: &str| -> Option<(u32, String, Option<String>)> {
        let mut parts = addr.splitn(3, ':');
        let kind = parts.next()?.parse::<u32>().ok()?;
        let author = parts.next()?.to_lowercase();
        let d_tag = parts
            .next()
            .and_then(|d| if d.is_empty() { None } else { Some(d.to_string()) });
        Some((kind, author, d_tag))
    };

    let mut root_id: Option<String> = None;
    let mut root_relays_e: Option<Vec<String>> = None;
    let mut root_relays_a: Option<Vec<String>> = None;
    let mut root_kind_tag: Option<u32> = None;
    let mut root_kind_addr: Option<u32> = None;
    let mut root_author_tag: Option<String> = None;
    let mut root_author_addr: Option<String> = None;
    let mut root_author_e: Option<String> = None;
    let mut root_d: Option<String> = None;

    for t in tags {
        match t.first().map(|k| k.as_str()) {
            Some("e") => {
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
            Some("a") => {
                if let Some(addr) = t.get(1).cloned() {
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
            Some("k") => {
                if let Some(kind) = t.get(1).and_then(|v| v.parse::<u32>().ok()) {
                    root_kind_tag = Some(kind);
                }
            }
            Some("p") => {
                if let Some(pk) = t.get(1).filter(|s| !s.is_empty()) {
                    root_author_tag = Some(pk.to_lowercase());
                }
            }
            _ => {}
        }
    }

    let id = root_id.ok_or(RadrootsReactionEventIndexError::ParseError)?;
    let kind = root_kind_tag.or(root_kind_addr).unwrap_or(1);
    let author = root_author_tag
        .or(root_author_addr)
        .or(root_author_e)
        .unwrap_or_default();
    let relays = root_relays_e.or(root_relays_a);

    let root = RadrootsNostrEventRef {
        id,
        author,
        kind,
        d_tag: root_d,
        relays,
    };

    Ok(RadrootsReaction {
        root,
        content: content.to_string(),
    })
}

fn create_radroots_reaction_event_metadata(
    id: String,
    author: String,
    published_at: u32,
    kind: u32,
    content: &str,
    tags: &[Vec<String>],
) -> Result<RadrootsReactionEventMetadata, RadrootsReactionEventIndexError> {
    let reaction = parse_reaction_from_tags(tags, content)?;
    Ok(RadrootsReactionEventMetadata {
        id,
        author,
        published_at,
        kind,
        reaction,
    })
}

pub trait ToRadrootsReactionEventIndex {
    fn to_radroots_reaction_event(
        &self,
    ) -> Result<RadrootsReactionEventIndex, RadrootsReactionEventIndexError>;
}

impl ToRadrootsReactionEventIndex for RelayIndexerEvent {
    fn to_radroots_reaction_event(
        &self,
    ) -> Result<RadrootsReactionEventIndex, RadrootsReactionEventIndexError> {
        let kind_u32 = self.kind.as_u64() as u32;
        let id = self.id.clone();
        let author = self.author.clone();

        let metadata = create_radroots_reaction_event_metadata(
            id.clone(),
            author.clone(),
            self.created_at,
            kind_u32,
            &self.content,
            &self.tags,
        )?;

        Ok(RadrootsReactionEventIndex {
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
    use super::parse_reaction_from_tags;

    #[test]
    fn reaction_parses_event_reference() {
        let tags = vec![
            vec!["e".to_string(), "root123".to_string()],
            vec!["k".to_string(), "1".to_string()],
            vec!["p".to_string(), "a".repeat(64)],
        ];
        let reaction = parse_reaction_from_tags(&tags, "+").expect("parse reaction");
        assert_eq!(reaction.root.id, "root123");
        assert_eq!(reaction.root.kind, 1);
    }

    #[test]
    fn reaction_parses_address_reference() {
        let pubkey = "b".repeat(64);
        let addr = format!("30023:{}:dtag", pubkey);
        let tags = vec![
            vec!["e".to_string(), "root123".to_string()],
            vec!["a".to_string(), addr.clone()],
            vec!["k".to_string(), "30023".to_string()],
            vec!["p".to_string(), pubkey.clone()],
        ];
        let reaction = parse_reaction_from_tags(&tags, "+").expect("parse reaction");
        assert_eq!(reaction.root.kind, 30023);
        assert_eq!(reaction.root.author, pubkey);
        assert_eq!(reaction.root.d_tag.as_deref(), Some("dtag"));
    }
}
