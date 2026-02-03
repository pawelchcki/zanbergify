use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::path::Path;

use crate::cache::get_cache_dir;
use crate::models::ModelInfo;

pub fn verify_checksum(path: &Path, expected_sha256: &str) -> Result<bool> {
    let mut file = std::fs::File::open(path)
        .context(format!("Failed to open file: {}", path.display()))?;

    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;

    let hash = format!("{:x}", hasher.finalize());
    Ok(hash == expected_sha256)
}

#[allow(dead_code)]
pub fn compute_checksum(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn verify_model(info: &ModelInfo) -> Result<()> {
    let cache_dir = get_cache_dir()?;
    let path = cache_dir.join(info.filename);

    if !path.exists() {
        bail!("Model not found in cache: {}", info.name);
    }

    print!("Verifying {}... ", info.name);

    if verify_checksum(&path, info.sha256)? {
        println!("✓ Valid");
        Ok(())
    } else {
        println!("✗ Checksum mismatch");
        bail!("Model failed verification: {}", info.name);
    }
}
