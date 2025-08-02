use anyhow::{Context, Result};
use indexer_utils::file::fs_mkdir;

use crate::{config::Settings, IndexerEventKind};

pub fn create_index_dirs(settings: &Settings) -> Result<()> {
    for kind in IndexerEventKind::ALL {
        let kind_str = kind.as_u64().to_string();

        for subdir in kind.paths() {
            fs_mkdir(&[
                settings.service.output_dir.as_str(),
                "events",
                &kind_str,
                subdir.as_str(),
            ])
            .with_context(|| {
                format!(
                    "Failed to create directory for kind {} / {}",
                    kind_str,
                    subdir.as_str()
                )
            })?;
        }
    }
    Ok(())
}
