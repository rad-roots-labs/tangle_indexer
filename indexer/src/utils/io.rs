use anyhow::{Context, Result};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::debug;

use crate::utils::crypto::compute_hash;

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

pub fn safe_path_segment(segment: &str) -> Option<String> {
    let mut components = Path::new(segment).components();
    match (components.next(), components.next()) {
        (Some(std::path::Component::Normal(comp)), None) => {
            let value = comp.to_string_lossy();
            if value.is_empty() {
                None
            } else {
                Some(value.into_owned())
            }
        }
        _ => None,
    }
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

pub fn write_json_if_changed<T: serde::Serialize>(
    path: &Path,
    data: &T,
    updated: &mut Vec<PathBuf>,
) -> Result<String> {
    let hash = compute_hash(data)
        .with_context(|| format!("Failed to hash JSON for {}", path.display()))?;
    let hash_path = path.with_extension("sha256.txt");

    let needs_write = if path.exists() && hash_path.exists() {
        let stored = fs::read_to_string(&hash_path)
            .with_context(|| format!("Failed to read {}", hash_path.display()))?;
        stored.trim() != hash
    } else {
        true
    };

    if needs_write {
        write_json(path, data)
            .with_context(|| format!("Failed to write {}", path.display()))?;
        write_hash(path, &hash)
            .with_context(|| format!("Failed to write hash for {}", path.display()))?;
        updated.push(path.to_path_buf());
    }

    Ok(hash)
}

pub fn fs_write_rss(path: &Path, content: &str) -> Result<()> {
    let mut file = File::create(path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{safe_path_segment, write_json_if_changed};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn safe_path_segment_rejects_traversal() {
        assert!(safe_path_segment("..").is_none());
        assert!(safe_path_segment(".").is_none());
        assert!(safe_path_segment("a/b").is_none());
        assert!(safe_path_segment("/abs").is_none());
    }

    #[test]
    fn safe_path_segment_accepts_normal() {
        assert_eq!(safe_path_segment("alpha"), Some("alpha".to_string()));
        assert_eq!(
            safe_path_segment("user@example.com"),
            Some("user@example.com".to_string())
        );
    }

    #[test]
    fn write_json_if_changed_is_idempotent() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("data.json");
        let mut updated = Vec::new();

        let hash_a = write_json_if_changed(&path, &vec![1u32, 2, 3], &mut updated)
            .expect("write json");
        assert_eq!(updated.len(), 1);
        let first = fs::read_to_string(&path).expect("read data");

        updated.clear();
        let hash_b = write_json_if_changed(&path, &vec![1u32, 2, 3], &mut updated)
            .expect("write json");
        assert_eq!(hash_a, hash_b);
        assert!(updated.is_empty());
        let second = fs::read_to_string(&path).expect("read data");
        assert_eq!(first, second);

        updated.clear();
        let _hash_c = write_json_if_changed(&path, &vec![1u32, 2, 4], &mut updated)
            .expect("write json");
        assert_eq!(updated.len(), 1);
    }
}
