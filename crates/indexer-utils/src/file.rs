use anyhow::Result;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::debug;

use crate::crypto::sha256_hex;
use crate::paths::{paths_join, PathsError};

#[derive(Error, Debug)]
pub enum FileError {
    #[error("Failed to create directory `{path}`: {source}")]
    CreateDirError {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Invalid path construction: {0}")]
    PathJoinError(#[from] PathsError),

    #[error("Failed to write RSS file: {0}")]
    Io(#[from] std::io::Error),
}

pub fn fs_mkdir<S, I>(segments: I) -> Result<(), FileError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<Path>,
{
    let dir_path = paths_join(segments)?;

    if !dir_path.exists() {
        fs::create_dir_all(&dir_path).map_err(|e| FileError::CreateDirError {
            path: dir_path.display().to_string(),
            source: e,
        })?;
        debug!("Created directory: {}", dir_path.display());
    } else {
        debug!("Directory already exists: {}", dir_path.display());
    }

    Ok(())
}

pub fn fs_write_rss(path: &Path, content: &str) -> Result<(), FileError> {
    let mut file = File::create(path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

pub fn fs_write_with_hash_check<T: serde::Serialize>(path: &Path, data: &T) -> Result<bool> {
    let content = serde_json::to_string_pretty(data)?;
    let new_hash = sha256_hex(content.as_bytes());

    let hash_path = path.with_extension("sha256.txt");

    if path.exists() && hash_path.exists() {
        if let Ok(old_hash) = fs::read_to_string(&hash_path) {
            if old_hash.trim() == new_hash {
                debug!(file_path = %path.display(),"File hash unchanged, not written.");
                return Ok(false);
            }
        }
    }

    debug!(file_path = %path.display(),"File hash changed, writing.");

    fs::write(path, &content)?;
    fs::write(hash_path, format!("{}\n", new_hash))?;
    Ok(true)
}

pub fn fs_write_track_hash_checks<T: serde::Serialize>(
    path: PathBuf,
    data: &T,
    updated_files: &mut Vec<PathBuf>,
) -> Result<()> {
    if fs_write_with_hash_check(&path, data)? {
        updated_files.push(path);
    }
    Ok(())
}
