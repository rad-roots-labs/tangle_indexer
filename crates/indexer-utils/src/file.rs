use anyhow::Result;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use thiserror::Error;
use tracing::debug;

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
