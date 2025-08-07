use anyhow::Result;
use serde::Serialize;
use sha2::Digest;
use std::{
    fs::{write, File},
    io::{BufWriter, Write},
    path::Path,
};
use tracing::debug;

pub fn write_json<T: Serialize>(path: &Path, data: &T) -> Result<()> {
    let file = File::create(path)?;
    let mut buf = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut buf, data)?;
    buf.flush()?;
    Ok(())
}

pub fn write_hash(path: &Path, hash: &str) -> Result<()> {
    let hash_path = path.with_extension("sha256.txt");
    write(&hash_path, format!("{hash}\n"))?;
    debug!(hash_path = %hash_path.display(), "Wrote new hash file");
    Ok(())
}

struct HasherWriter<'a>(&'a mut sha2::Sha256);
impl<'a> std::io::Write for HasherWriter<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.update(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub fn compute_hash<T: Serialize>(data: &T) -> anyhow::Result<String> {
    let mut hasher = sha2::Sha256::new();
    {
        let writer = HasherWriter(&mut hasher);
        serde_json::to_writer_pretty(writer, data)?;
    }
    Ok(format!("{:x}", hasher.finalize()))
}
