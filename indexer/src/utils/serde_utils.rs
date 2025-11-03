use anyhow::Result;
use bincode::config::standard;
use bincode::serde::{decode_from_slice, encode_to_vec};
use serde::{de::DeserializeOwned, Serialize};

pub fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    Ok(encode_to_vec(value, standard())?)
}

pub fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let (v, _) = decode_from_slice(bytes, standard())?;
    Ok(v)
}
