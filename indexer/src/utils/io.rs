use anyhow::Result;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::debug;

#[derive(Debug, Error)]
pub enum PathsError {
    #[error("Invalid path segment at index {index}: `{segment}`")]
    InvalidSegment { index: usize, segment: String },
}

pub fn paths_join<I, S>(segments: I) -> Result<PathBuf, PathsError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<Path>,
{
    let mut path = PathBuf::new();
    for (i, segment) in segments.into_iter().enumerate() {
        let seg = segment.as_ref();
        if seg.as_os_str().is_empty() {
            return Err(PathsError::InvalidSegment {
                index: i,
                segment: seg.display().to_string(),
            });
        }
        path.push(seg);
    }
    Ok(path)
}

#[derive(thiserror::Error, Debug)]
pub enum FileError {
    #[error("Failed to create directory `{path}`: {source}")]
    CreateDirError {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Path join error: {0}")]
    PathJoinError(#[from] PathsError),
    #[error("I/O error: {0}")]
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

pub fn write_json<T: serde::Serialize>(path: &Path, data: &T) -> Result<()> {
    let file = File::create(path)?;
    let mut buf = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(&mut buf, data)?;
    buf.flush()?;
    Ok(())
}

pub fn write_hash(path: &Path, hash: &str) -> Result<()> {
    let hash_path = path.with_extension("sha256.txt");
    fs::write(&hash_path, format!("{hash}\n"))?;
    debug!(hash_path = %hash_path.display(), "Wrote new hash file");
    Ok(())
}

pub fn fs_write_rss(path: &Path, content: &str) -> Result<()> {
    let mut file = File::create(path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}
