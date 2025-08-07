use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sled::{Config as SledConfig, Db, IVec};

use crate::serialization::{deserialize, serialize};

pub struct IndexerDb {
    pub db: Db,
}

impl IndexerDb {
    pub fn open(path: &str) -> Result<Self> {
        let db = SledConfig::new().path(path).open()?;
        Ok(Self { db })
    }

    pub fn insert<T: Serialize>(&self, tree: &str, key: &str, v: &T) -> Result<()> {
        let t = self.db.open_tree(tree)?;
        let blob: Vec<u8> = serialize(v)?;
        t.insert(key, blob)?;
        Ok(())
    }

    pub fn get<T: DeserializeOwned>(&self, tree: &str, key: &str) -> Result<Option<T>> {
        let t = self.db.open_tree(tree)?;
        if let Some(bytes) = t.get(key)? {
            let v: T = deserialize(&bytes)?;
            Ok(Some(v))
        } else {
            Ok(None)
        }
    }

    pub fn get_all<T: DeserializeOwned>(&self, tree: &str) -> Result<Vec<T>> {
        let t = self.db.open_tree(tree)?;
        let mut out = Vec::new();
        for res in t.iter().values() {
            let bytes = res?;
            let v: T = crate::serialization::deserialize(&bytes)?;
            out.push(v);
        }
        Ok(out)
    }

    pub fn insert_raw(&self, tree: &str, key: &str, bytes: &[u8]) -> Result<()> {
        self.db.open_tree(tree)?.insert(key, bytes)?;
        Ok(())
    }

    pub fn get_raw(&self, tree: &str, key: &str) -> Result<Option<IVec>> {
        Ok(self.db.open_tree(tree)?.get(key)?)
    }

    pub fn flush(&self) -> Result<()> {
        self.db.flush()?;
        Ok(())
    }
}
