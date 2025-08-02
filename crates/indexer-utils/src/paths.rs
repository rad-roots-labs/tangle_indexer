use anyhow::Result;
use std::path::{Path, PathBuf};
use thiserror::Error;

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
        let seg_ref = segment.as_ref();

        if seg_ref.as_os_str().is_empty() {
            return Err(PathsError::InvalidSegment {
                index: i,
                segment: seg_ref.display().to_string(),
            });
        }

        path.push(seg_ref);
    }

    Ok(path)
}
