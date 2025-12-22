use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::{Read, Result as IoResult};

pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

pub fn compute_hash_of_reader<R: Read>(mut reader: R) -> IoResult<String> {
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn compute_hash<T: Serialize>(value: &T) -> anyhow::Result<String> {
    struct HasherWriter<'a>(&'a mut Sha256);
    impl<'a> std::io::Write for HasherWriter<'a> {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.update(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let mut hasher = Sha256::new();
    {
        let writer = HasherWriter(&mut hasher);
        serde_json::to_writer(writer, value)?;
    }
    Ok(format!("{:x}", hasher.finalize()))
}
