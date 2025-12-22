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
    let bech32 = match pubkey.to_bech32() {
        Ok(value) => value,
        Err(err) => match err {},
    };
    Ok(bech32)
}

pub(crate) fn normalize_nip05(nip05: &str) -> (String, String, String) {
    let lower = nip05.to_lowercase();
    let local = lower
        .split_once('@')
        .map(|(name, _)| name.to_string())
        .unwrap_or_else(|| lower.clone());
    let index_key = lower
        .strip_suffix("@radroots.market")
        .map(|s| s.to_string())
        .unwrap_or_else(|| lower.clone());
    (lower, local, index_key)
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
