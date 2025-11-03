use crate::relay::event::RelayIndexerEvent;
use radroots_events::{
    comment::models::{RadrootsComment, RadrootsCommentEventIndex, RadrootsCommentEventMetadata},
    RadrootsNostrEvent, RadrootsNostrEventRef,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RadrootsCommentEventIndexError {
    #[error("Failed to parse comment from tags")]
    ParseError,
}

fn parse_comment_from_tags(
    tags: &[Vec<String>],
    content: &str,
) -> Result<RadrootsComment, RadrootsCommentEventIndexError> {
    let mut root_id: Option<String> = None;
    let mut root_relays: Option<Vec<String>> = None;
    let mut root_kind: Option<u32> = None;
    let mut root_author: Option<String> = None;
    let mut root_d: Option<String> = None;

    let mut parent_id: Option<String> = None;
    let mut parent_relays: Option<Vec<String>> = None;
    let mut parent_kind: Option<u32> = None;
    let mut parent_author: Option<String> = None;
    let mut parent_d: Option<String> = None;

    for t in tags {
        if t.first().map(|k| k == "e_root").unwrap_or(false) {
            if let Some(id) = t.get(1).cloned() {
                root_id = Some(id);
            }
            if let Some(r) = t.get(2).cloned() {
                root_relays = Some(vec![r]);
            }
        } else if t.first().map(|k| k == "e_prev").unwrap_or(false) {
            if let Some(id) = t.get(1).cloned() {
                parent_id = Some(id);
            }
            if let Some(r) = t.get(2).cloned() {
                parent_relays = Some(vec![r]);
            }
        } else if t.first().map(|k| k == "e").unwrap_or(false) {
            if root_id.is_none() {
                if let Some(id) = t.get(1).cloned() {
                    root_id = Some(id);
                }
            }
            if root_relays.is_none() {
                if let Some(r) = t.get(2).cloned() {
                    root_relays = Some(vec![r]);
                }
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

    if parent_id.is_none() {
        parent_id = root_id.clone();
        parent_relays = root_relays.clone();
        parent_kind = root_kind;
        parent_author = root_author.clone();
        parent_d = root_d.clone();
    }

    let root = RadrootsNostrEventRef {
        id: root_id.ok_or(RadrootsCommentEventIndexError::ParseError)?,
        author: root_author.unwrap_or_default(),
        kind: root_kind.unwrap_or(1),
        d_tag: root_d,
        relays: root_relays,
    };

    let parent = RadrootsNostrEventRef {
        id: parent_id.ok_or(RadrootsCommentEventIndexError::ParseError)?,
        author: parent_author.unwrap_or_default(),
        kind: parent_kind.unwrap_or(1),
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
    content: String,
    tags: Vec<Vec<String>>,
) -> Result<RadrootsCommentEventMetadata, RadrootsCommentEventIndexError> {
    let comment = parse_comment_from_tags(&tags, &content)?;
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
        self,
    ) -> Result<RadrootsCommentEventIndex, RadrootsCommentEventIndexError>;
}

impl ToRadrootsCommentEventIndex for RelayIndexerEvent {
    fn to_radroots_comment_event(
        self,
    ) -> Result<RadrootsCommentEventIndex, RadrootsCommentEventIndexError> {
        let kind_u32 = self.kind.as_u64() as u32;
        let metadata = create_radroots_comment_event_metadata(
            self.id.clone(),
            self.author.clone(),
            self.created_at,
            kind_u32,
            self.content.clone(),
            self.tags.clone(),
        )?;
        Ok(RadrootsCommentEventIndex {
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
