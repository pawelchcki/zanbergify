use anyhow::{bail, Result};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::Write;
use std::path::PathBuf;

use crate::cache::get_cache_dir;
use crate::models::ModelInfo;
use crate::verify::verify_checksum;

pub async fn download_model(info: &ModelInfo) -> Result<PathBuf> {
    let cache_dir = get_cache_dir()?;
    std::fs::create_dir_all(&cache_dir)?;

    let dest = cache_dir.join(info.filename);

    // Skip if already exists and valid
    if dest.exists() {
        if verify_checksum(&dest, info.sha256)? {
            println!("✓ Model already downloaded and verified: {}", info.name);
            return Ok(dest);
        }
        println!("⚠ Existing file checksum mismatch, re-downloading");
    }

    println!(
        "Downloading {} ({:.1} MB)...",
        info.name,
        info.size_bytes as f64 / 1_000_000.0
    );

    // Download with progress bar
    let client = reqwest::Client::new();
    let response = client.get(info.url).send().await?;

    if !response.status().is_success() {
        bail!("Failed to download: HTTP {}", response.status());
    }

    let total_size = response.content_length().unwrap_or(info.size_bytes);

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    let mut file = std::fs::File::create(&dest)?;
    let mut downloaded = 0u64;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }

    pb.finish_with_message("Download complete");

    // Verify checksum
    println!("Verifying checksum...");
    if !verify_checksum(&dest, info.sha256)? {
        std::fs::remove_file(&dest)?;
        bail!("Downloaded file failed checksum verification");
    }

    println!("✓ Model verified and cached at: {}", dest.display());
    Ok(dest)
}
