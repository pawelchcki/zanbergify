use anyhow::{Context, Result};
use std::path::PathBuf;

pub fn get_cache_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".zanbergify").join("models"))
}

pub fn list_cached_models() -> Result<Vec<(String, u64)>> {
    let cache_dir = get_cache_dir()?;

    if !cache_dir.exists() {
        return Ok(Vec::new());
    }

    let mut models = Vec::new();

    for entry in std::fs::read_dir(cache_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().is_some_and(|ext| ext == "onnx") {
            let size = entry.metadata()?.len();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .context("Non-UTF8 filename encountered")?
                .to_string();
            models.push((name, size));
        }
    }

    Ok(models)
}

pub fn clean_model(filename: &str) -> Result<()> {
    let cache_dir = get_cache_dir()?;
    let path = cache_dir.join(filename);

    if path.exists() {
        std::fs::remove_file(&path)?;
        println!("✓ Removed: {}", filename);
    } else {
        println!("Model not found: {}", filename);
    }

    Ok(())
}

pub fn cache_size() -> Result<u64> {
    let models = list_cached_models()?;
    Ok(models.iter().map(|(_, size)| size).sum())
}
