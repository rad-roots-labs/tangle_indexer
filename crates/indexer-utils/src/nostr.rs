use nostr::key::{Error as PublicKeyError, PublicKey};
use nostr::prelude::ToBech32;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NostrError {
    #[error("Invalid hex for public key: {0}")]
    InvalidPublicKey(#[from] PublicKeyError),
}

pub fn public_key_to_npub(public_key_hex: &str) -> Result<String, NostrError> {
    let pubkey = PublicKey::from_hex(public_key_hex)?;
    Ok(pubkey.to_bech32().expect("to_bech32 is infallible"))
}
