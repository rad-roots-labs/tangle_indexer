use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};

pub fn compute_hash_of_bytes<R: Read>(mut rdr: R) -> io::Result<String> {
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8 * 1024];
    loop {
        let n = rdr.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn is_file_hash_valid(path: &Path) -> io::Result<bool> {
    let hash_path = path.with_extension("sha256.txt");
    if !path.exists() || !hash_path.exists() {
        return Ok(false);
    }
    let file = File::open(path)?;
    let computed = compute_hash_of_bytes(file)?;
    let stored = std::fs::read_to_string(hash_path)?;
    Ok(stored.trim() == computed)
}
