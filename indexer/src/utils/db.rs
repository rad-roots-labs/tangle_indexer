use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};
use sled::{Config as SledConfig, Db, IVec};
use std::collections::HashMap;
use std::sync::Mutex;

use crate::utils::serde_utils::{deserialize, serialize};

pub struct IndexerDb {
    pub db: Db,
    trees: Mutex<HashMap<String, sled::Tree>>,
}

impl IndexerDb {
    pub fn open(path: &str) -> Result<Self> {
        let db = SledConfig::new().path(path).open()?;
        Ok(Self {
            db,
            trees: Mutex::new(HashMap::new()),
        })
    }

    fn tree(&self, name: &str) -> Result<sled::Tree> {
        let mut trees = self.trees.lock().unwrap_or_else(|err| err.into_inner());
        if let Some(tree) = trees.get(name) {
            return Ok(tree.clone());
        }
        let tree = self.db.open_tree(name)?;
        trees.insert(name.to_string(), tree.clone());
        Ok(tree)
    }

    pub fn insert<T: Serialize>(&self, tree: &str, key: &str, value: &T) -> Result<()> {
        let t = self.tree(tree)?;
        let blob: Vec<u8> = serialize(value)?;
        t.insert(key, blob)?;
        Ok(())
    }

    pub fn get<T: DeserializeOwned>(&self, tree: &str, key: &str) -> Result<Option<T>> {
        let t = self.tree(tree)?;
        if let Some(bytes) = t.get(key)? {
            let v: T = deserialize(&bytes)?;
            Ok(Some(v))
        } else {
            Ok(None)
        }
    }

    pub fn get_all<T: DeserializeOwned>(&self, tree: &str) -> Result<Vec<T>> {
        let t = self.tree(tree)?;
        let mut out = Vec::new();
        for res in t.iter().values() {
            let bytes = res?;
            let v: T = deserialize(&bytes)?;
            out.push(v);
        }
        Ok(out)
    }

    pub fn insert_raw(&self, tree: &str, key: &str, bytes: &[u8]) -> Result<()> {
        self.tree(tree)?.insert(key, bytes)?;
        Ok(())
    }

    pub fn get_raw(&self, tree: &str, key: &str) -> Result<Option<IVec>> {
        Ok(self.tree(tree)?.get(key)?)
    }

    pub fn flush(&self) -> Result<()> {
        self.db.flush()?;
        Ok(())
    }
}
