use thiserror::Error;

use radroots_events::{
    reaction::models::{
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
    let mut root_id: Option<String> = None;
    let mut root_relays: Option<Vec<String>> = None;
    let mut root_kind: Option<u32> = None;
    let mut root_author: Option<String> = None;
    let mut root_d: Option<String> = None;

    for t in tags {
        if t.first().map(|k| k == "e").unwrap_or(false) {
            if let Some(id) = t.get(1).cloned() {
                root_id = Some(id);
            }
            if let Some(r) = t.get(2).cloned() {
                root_relays = Some(vec![r]);
            }
        } else if t.first().map(|k| k == "a").unwrap_or(false) {
            if let Some(arg) = t.get(1) {
                let parts: Vec<&str> = arg.split(':').collect();
                if parts.len() >= 2 {
                    root_kind = parts[0].parse::<u32>().ok();
                    root_author = Some(parts[1].to_lowercase());
                    if parts.len() >= 3 && !parts[2].is_empty() {
                        root_d = Some(parts[2].to_string());
                    }
                }
            }
            if let Some(r) = t.get(2).cloned() {
                root_relays = Some(vec![r]);
            }
        }
    }

    let id = root_id.ok_or(RadrootsReactionEventIndexError::ParseError)?;
    let kind = root_kind.unwrap_or(1);
    let author = root_author.unwrap_or_default();

    let root = RadrootsNostrEventRef {
        id,
        author,
        kind,
        d_tag: root_d,
        relays: root_relays,
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
    content: String,
    tags: Vec<Vec<String>>,
) -> Result<RadrootsReactionEventMetadata, RadrootsReactionEventIndexError> {
    let reaction = parse_reaction_from_tags(&tags, &content)?;
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
        self,
    ) -> Result<RadrootsReactionEventIndex, RadrootsReactionEventIndexError>;
}

impl ToRadrootsReactionEventIndex for RelayIndexerEvent {
    fn to_radroots_reaction_event(
        self,
    ) -> Result<RadrootsReactionEventIndex, RadrootsReactionEventIndexError> {
        let kind_u32 = self.kind.as_u64() as u32;

        let metadata = create_radroots_reaction_event_metadata(
            self.id.clone(),
            self.author.clone(),
            self.created_at,
            kind_u32,
            self.content.clone(),
            self.tags.clone(),
        )?;

        Ok(RadrootsReactionEventIndex {
            event: RadrootsNostrEvent {
                id: self.id,
                author: self.author,
                created_at: self.created_at,
                kind: kind_u32,
                tags: self.tags,
                content: self.content,
                sig: self.sig,
            },
            metadata,
        })
    }
}
