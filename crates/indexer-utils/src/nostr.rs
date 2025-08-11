use std::collections::HashMap;

use nostr::key::{Error as PublicKeyError, PublicKey};
use nostr::prelude::ToBech32;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NostrUtilsError {
    #[error("Invalid hex for public key: {0}")]
    InvalidPublicKey(#[from] PublicKeyError),

    #[error("Tag parsing error: {0}")]
    TagParseError(String),
}

pub fn public_key_to_npub(public_key_hex: &str) -> Result<String, NostrUtilsError> {
    let pubkey = PublicKey::from_hex(public_key_hex)?;
    Ok(pubkey.to_bech32().expect("to_bech32 is infallible"))
}

pub fn get_tag_value<'a>(
    tags_map: &'a HashMap<String, Vec<String>>,
    key: &str,
    idx: usize,
) -> Result<Option<String>, NostrUtilsError> {
    match tags_map.get(key) {
        Some(values) => Ok(values.get(idx).cloned()),
        None => Err(NostrUtilsError::TagParseError(format!(
            "Tag '{}' not found",
            key
        ))),
    }
}
