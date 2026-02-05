use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;

pub struct R2Config {
    pub account_id: String,
    pub api_token: String,
    pub bucket_name: String,
}

pub async fn upload_file_to_r2(
    config: &R2Config,
    local_path: &Path,
    object_key: &str,
    content_type: &str,
) -> Result<String> {
    println!(
        "Uploading {} to R2 bucket '{}'...",
        object_key, config.bucket_name
    );

    let client = reqwest::Client::new();

    // Read file
    let file_size = fs::metadata(local_path)?.len();
    let content =
        fs::read(local_path).context(format!("Failed to read file: {}", local_path.display()))?;

    println!("  File size: {:.1} MB", file_size as f64 / 1_048_576.0);

    // Upload to R2 using Cloudflare API
    let url = format!(
        "https://api.cloudflare.com/client/v4/accounts/{}/r2/buckets/{}/objects/{}",
        config.account_id,
        config.bucket_name,
        urlencoding::encode(object_key)
    );

    let response = client
        .put(&url)
        .header("Authorization", format!("Bearer {}", config.api_token))
        .header("Content-Type", content_type)
        .header("Content-Length", file_size.to_string())
        .body(content)
        .send()
        .await
        .context("Failed to upload to R2")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!("R2 upload failed (status: {}): {}", status, body);
    }

    println!("  ✓ Uploaded successfully");

    // Return the public R2 URL
    // Format: https://<bucket>.<account_id>.r2.cloudflarestorage.com/<key>
    // Or using custom domain if configured
    let public_url = format!(
        "https://{}.{}.r2.cloudflarestorage.com/{}",
        config.bucket_name, config.account_id, object_key
    );

    Ok(public_url)
}

pub async fn set_bucket_cors(config: &R2Config) -> Result<()> {
    println!("Configuring CORS for R2 bucket '{}'...", config.bucket_name);

    let client = reqwest::Client::new();

    // CORS configuration
    let cors_config = serde_json::json!({
        "AllowedOrigins": ["*"],
        "AllowedMethods": ["GET", "HEAD"],
        "AllowedHeaders": ["*"],
        "MaxAgeSeconds": 3600
    });

    let url = format!(
        "https://api.cloudflare.com/client/v4/accounts/{}/r2/buckets/{}/cors",
        config.account_id, config.bucket_name
    );

    let response = client
        .put(&url)
        .header("Authorization", format!("Bearer {}", config.api_token))
        .header("Content-Type", "application/json")
        .json(&cors_config)
        .send()
        .await
        .context("Failed to set CORS configuration")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!("Failed to set CORS (status: {}): {}", status, body);
    }

    println!("  ✓ CORS configured");
    Ok(())
}

pub async fn make_bucket_public(config: &R2Config) -> Result<()> {
    println!(
        "Making R2 bucket '{}' publicly accessible...",
        config.bucket_name
    );

    let client = reqwest::Client::new();

    // Enable public access
    let public_config = serde_json::json!({
        "public": true
    });

    let url = format!(
        "https://api.cloudflare.com/client/v4/accounts/{}/r2/buckets/{}/public",
        config.account_id, config.bucket_name
    );

    let response = client
        .put(&url)
        .header("Authorization", format!("Bearer {}", config.api_token))
        .header("Content-Type", "application/json")
        .json(&public_config)
        .send()
        .await
        .context("Failed to enable public access")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        // Check if bucket is already public
        if body.contains("already") || status.as_u16() == 400 {
            println!("  Note: Bucket may already be public");
            return Ok(());
        }

        bail!(
            "Failed to enable public access (status: {}): {}",
            status,
            body
        );
    }

    println!("  ✓ Bucket is now public");
    Ok(())
}
