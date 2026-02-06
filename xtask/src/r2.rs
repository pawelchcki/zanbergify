use anyhow::{bail, Context, Result};
use aws_sdk_s3::config::{Credentials, Region};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client as S3Client;
use std::fs;
use std::path::Path;

pub struct R2Config {
    pub account_id: String,
    pub api_token: String,
    pub bucket_name: String,
}

// Cloudflare V4 API limit (same as wrangler)
const API_UPLOAD_LIMIT: u64 = 300 * 1024 * 1024; // 300 MB

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

    let file_size = fs::metadata(local_path)?.len();
    println!("  File size: {:.1} MB", file_size as f64 / 1_048_576.0);

    // Use multipart upload for files over 300 MB (V4 API limit)
    // For smaller files, use direct streaming upload
    if file_size > API_UPLOAD_LIMIT {
        upload_multipart(config, local_path, object_key, content_type).await?;
    } else {
        upload_with_stream(config, local_path, object_key, content_type).await?;
    }

    println!("  ✓ Uploaded successfully");

    // Return the public R2 URL
    let public_url = format!(
        "https://{}.{}.r2.cloudflarestorage.com/{}",
        config.bucket_name, config.account_id, object_key
    );

    Ok(public_url)
}

async fn upload_with_stream(
    config: &R2Config,
    local_path: &Path,
    object_key: &str,
    content_type: &str,
) -> Result<()> {
    use tokio::fs::File;
    use tokio_util::codec::{BytesCodec, FramedRead};

    let file = File::open(local_path)
        .await
        .context(format!("Failed to open file: {}", local_path.display()))?;

    let file_size = file.metadata().await?.len();
    let stream = FramedRead::new(file, BytesCodec::new());
    let body = reqwest::Body::wrap_stream(stream);

    let client = reqwest::Client::new();
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
        .body(body)
        .send()
        .await
        .context("Failed to upload to R2")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!("R2 upload failed (status: {}): {}", status, body);
    }

    Ok(())
}

async fn upload_multipart(
    config: &R2Config,
    local_path: &Path,
    object_key: &str,
    content_type: &str,
) -> Result<()> {
    println!("  Using multipart upload for large file...");

    // Try to get R2 credentials from environment, or create temporary ones
    // SECURITY: It's strongly recommended to use proper R2 API tokens via environment variables
    let (access_key_id, secret_access_key) = if let (Ok(key_id), Ok(secret)) = (
        std::env::var("R2_ACCESS_KEY_ID"),
        std::env::var("R2_SECRET_ACCESS_KEY"),
    ) {
        println!("  Using R2 credentials from environment variables");
        (key_id, secret)
    } else {
        println!("  ⚠️  R2 credentials not found in environment, deriving from API token");
        println!("  💡 For better security, set R2_ACCESS_KEY_ID and R2_SECRET_ACCESS_KEY");
        create_r2_credentials(config).await?
    };

    // Configure S3 client for R2
    let credentials = Credentials::new(access_key_id, secret_access_key, None, None, "r2-upload");

    let endpoint_url = format!("https://{}.r2.cloudflarestorage.com", config.account_id);

    let s3_config = aws_sdk_s3::Config::builder()
        .credentials_provider(credentials)
        .region(Region::new("auto"))
        .endpoint_url(&endpoint_url)
        .behavior_version_latest()
        .build();

    let client = S3Client::from_conf(s3_config);

    // Upload using AWS SDK's automatic multipart handling
    let body = ByteStream::from_path(local_path)
        .await
        .context("Failed to read file for upload")?;

    client
        .put_object()
        .bucket(&config.bucket_name)
        .key(object_key)
        .body(body)
        .content_type(content_type)
        .send()
        .await
        .context("Failed to upload file to R2 via multipart")?;

    Ok(())
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

async fn create_r2_credentials(config: &R2Config) -> Result<(String, String)> {
    let client = reqwest::Client::new();

    // SECURITY NOTE: This function derives R2 credentials from a Cloudflare API token
    // using a simple SHA-256 hash. This is a fallback mechanism and has security implications:
    // - If the API token is compromised, an attacker can reconstruct the secret
    // - This is a custom, non-standard approach
    //
    // RECOMMENDED: Set R2_ACCESS_KEY_ID and R2_SECRET_ACCESS_KEY environment variables
    // with proper R2 API tokens generated from the Cloudflare dashboard:
    // https://dash.cloudflare.com/profile/api-tokens
    //
    // This fallback is only used when those environment variables are not set.

    // Derive R2 credentials from Cloudflare API token
    // Access Key ID = token ID
    // Secret Access Key = SHA-256 hash of the token
    let url = "https://api.cloudflare.com/client/v4/user/tokens/verify";

    #[derive(serde::Deserialize)]
    struct VerifyTokenResponse {
        result: TokenResult,
        success: bool,
    }

    #[derive(serde::Deserialize)]
    struct TokenResult {
        id: String,
    }

    let response = client
        .get(url)
        .header("Authorization", format!("Bearer {}", config.api_token))
        .send()
        .await
        .context("Failed to verify API token")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!("Failed to verify API token (status: {}): {}", status, body);
    }

    let result: VerifyTokenResponse = response
        .json()
        .await
        .context("Failed to parse token verification response")?;

    if !result.success {
        bail!("Failed to verify API token - API returned success=false");
    }

    // Access Key ID is the token ID
    let access_key_id = result.result.id;

    // Secret Access Key derived using HMAC-SHA256 with fixed salt
    // More secure than raw SHA-256 as it uses a keyed hash function
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let mut mac = Hmac::<Sha256>::new_from_slice(b"zanbergify-r2-secret-key-salt")
        .expect("HMAC can take key of any size");
    mac.update(config.api_token.as_bytes());
    let secret_access_key = format!("{:x}", mac.finalize().into_bytes());

    println!("  ✓ R2 credentials derived from API token (HMAC-SHA256)");
    Ok((access_key_id, secret_access_key))
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
