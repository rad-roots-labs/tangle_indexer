use anyhow::Result;
use config::{Config, ConfigError, File};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;
use tracing::error;

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("Configuration loading failed: {0}")]
    Load(#[from] ConfigError),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Indexer {
    pub data_dir: String,
    pub logs_dir: String,
    pub flush_interval: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Relay {
    pub url: String,
    pub database_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Listings {
    pub country_shard_size: usize,
    pub profile_shard_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub indexer: Indexer,
    pub relay: Relay,
    pub listings: Listings,
}

impl Settings {
    pub fn load(config_path: &Option<String>) -> Result<Self, SettingsError> {
        let path = config_path.as_deref().unwrap_or("config.toml");
        let builder = if config_path.is_some() {
            Config::builder().add_source(File::from(Path::new(path)).required(true))
        } else {
            Config::builder().add_source(File::with_name("config").required(true))
        };

        match builder.build() {
            Ok(cfg) => match cfg.try_deserialize::<Settings>() {
                Ok(settings) => Ok(settings),
                Err(err) => {
                    error!("❌ Failed to deserialize configuration: {err}");
                    Err(SettingsError::Load(err))
                }
            },
            Err(err) => {
                error!("❌ Failed to load configuration from '{}': {err}", path);
                Err(SettingsError::Load(err))
            }
        }
    }
}
