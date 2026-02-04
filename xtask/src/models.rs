use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};

use crate::cache::{cache_size, clean_model, get_cache_dir, list_cached_models};
use crate::download::download_model;
use crate::util::format_size;
use crate::verify::verify_model;
use crate::wasm_bundle::bundle_model_for_wasm;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum ModelType {
    BiRefNet,
    U2Net,
    ISNet,
}

pub struct ModelInfo {
    pub name: &'static str,
    pub model_type: ModelType,
    pub url: &'static str,
    pub filename: &'static str,
    pub size_bytes: u64,
    pub sha256: &'static str,
    pub input_size: u32,
    pub description: &'static str,
}

pub const MODELS: &[ModelInfo] = &[
    ModelInfo {
        name: "birefnet-lite",
        model_type: ModelType::BiRefNet,
        url: "https://github.com/danielgatis/rembg/releases/download/v0.0.0/BiRefNet-general-bb_swin_v1_tiny-epoch_232.onnx",
        filename: "BiRefNet-general-bb_swin_v1_tiny-epoch_232.onnx",
        size_bytes: 224_005_088,
        sha256: "5600024376f572a557870a5eb0afb1e5961636bef4e1e22132025467d0f03333",
        input_size: 1024,
        description: "BiRefNet lite - high quality, detailed edges",
    },
    ModelInfo {
        name: "u2net",
        model_type: ModelType::U2Net,
        url: "https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2net.onnx",
        filename: "u2net.onnx",
        size_bytes: 176_631_213,
        sha256: "8d10d2f3bb75ae3b6d527c77944fc5e7dcd94b29809d47a739a7a728a912b491",
        input_size: 320,
        description: "U2Net - fast, good quality",
    },
    ModelInfo {
        name: "isnet",
        model_type: ModelType::ISNet,
        url: "https://github.com/danielgatis/rembg/releases/download/v0.0.0/isnet-general-use.onnx",
        filename: "isnet-general-use.onnx",
        size_bytes: 169_024_454,
        sha256: "60920e99c45464f2ba57bee2ad08c919a52bbf852739e96947fbb4358c0d964a",
        input_size: 1024,
        description: "ISNet - balanced quality and speed",
    },
];

pub fn find_model_by_name(name: &str) -> Option<&'static ModelInfo> {
    MODELS.iter().find(|m| m.name == name)
}

#[derive(Args)]
pub struct ModelsCmd {
    #[command(subcommand)]
    pub command: ModelsSubCmd,
}

#[derive(Subcommand)]
pub enum ModelsSubCmd {
    /// List all available models
    List,
    /// Download a model
    Download {
        /// Model name (e.g., birefnet-lite, u2net, isnet)
        name: Option<String>,
        /// Download all models
        #[arg(long)]
        all: bool,
    },
    /// Verify model integrity
    Verify {
        /// Model name to verify
        name: Option<String>,
        /// Verify all cached models
        #[arg(long)]
        all: bool,
    },
    /// Show cache information
    Info,
    /// Clean model cache
    Clean {
        /// Clean all models
        #[arg(long)]
        all: bool,
        /// Specific model to clean
        name: Option<String>,
    },
    /// Bundle model for WASM deployment
    Bundle {
        /// Model name to bundle
        name: String,
        /// Destination directory
        #[arg(long, default_value = "zanbergify-wasm/www/models")]
        dest: String,
    },
    /// Upload model to Cloudflare R2
    UploadR2 {
        /// Model name to upload
        name: String,
        /// R2 bucket name
        #[arg(long, default_value = "zanbergify-models")]
        bucket: String,
    },
}

impl ModelsCmd {
    pub fn run(self) -> Result<()> {
        match self.command {
            ModelsSubCmd::List => list_models(),
            ModelsSubCmd::Download { name, all } => download_models(name, all),
            ModelsSubCmd::Verify { name, all } => verify_models(name, all),
            ModelsSubCmd::Info => show_cache_info(),
            ModelsSubCmd::Clean { all, name } => clean_cache(all, name),
            ModelsSubCmd::Bundle { name, dest } => bundle_model(name, dest),
            ModelsSubCmd::UploadR2 { name, bucket } => upload_to_r2(name, bucket),
        }
    }
}

fn list_models() -> Result<()> {
    println!("Available Models:\n");
    println!(
        "{:<16} {:<10} {:<12} {:<10} Description",
        "Name", "Type", "Resolution", "Size"
    );
    println!("{}", "-".repeat(80));

    for model in MODELS {
        println!(
            "{:<16} {:<10?} {}x{:<7} {:<10} {}",
            model.name,
            model.model_type,
            model.input_size,
            model.input_size,
            format_size(model.size_bytes),
            model.description
        );
    }

    Ok(())
}

fn download_models(name: Option<String>, all: bool) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;

    if all {
        for model in MODELS {
            rt.block_on(download_model(model))?;
            println!();
        }
    } else if let Some(name) = name {
        let model =
            find_model_by_name(&name).ok_or_else(|| anyhow::anyhow!("Unknown model: {}", name))?;
        rt.block_on(download_model(model))?;
    } else {
        bail!("Specify a model name or use --all");
    }

    Ok(())
}

fn verify_models(name: Option<String>, all: bool) -> Result<()> {
    if all {
        let cached = list_cached_models()?;
        if cached.is_empty() {
            println!("No models in cache");
            return Ok(());
        }

        for (filename, _) in cached {
            if let Some(model) = MODELS.iter().find(|m| m.filename == filename) {
                verify_model(model)?;
            } else {
                println!("Unknown model file: {}", filename);
            }
        }
    } else if let Some(name) = name {
        let model =
            find_model_by_name(&name).ok_or_else(|| anyhow::anyhow!("Unknown model: {}", name))?;
        verify_model(model)?;
    } else {
        bail!("Specify a model name or use --all");
    }

    Ok(())
}

fn show_cache_info() -> Result<()> {
    let cache_dir = get_cache_dir()?;
    println!("Model Cache: {}\n", cache_dir.display());

    let cached = list_cached_models()?;

    if cached.is_empty() {
        println!("No models cached");
        println!("\nDownload models with: cargo xtask models download <name>");
        return Ok(());
    }

    println!("Cached Models:");
    for (filename, size) in &cached {
        let status = if let Some(model) = MODELS.iter().find(|m| m.filename == filename) {
            match crate::verify::verify_checksum(&cache_dir.join(filename), model.sha256) {
                Ok(true) => "✓",
                _ => "✗",
            }
        } else {
            "?"
        };
        println!("  {} {:<50} {}", status, filename, format_size(*size));
    }

    let total = cache_size()?;
    println!("\nTotal: {}", format_size(total));

    Ok(())
}

fn clean_cache(all: bool, name: Option<String>) -> Result<()> {
    if all {
        let cached = list_cached_models()?;
        for (filename, _) in cached {
            clean_model(&filename)?;
        }
    } else if let Some(name) = name {
        let model =
            find_model_by_name(&name).ok_or_else(|| anyhow::anyhow!("Unknown model: {}", name))?;
        clean_model(model.filename)?;
    } else {
        bail!("Specify a model name or use --all");
    }

    Ok(())
}

fn bundle_model(name: String, dest: String) -> Result<()> {
    let model =
        find_model_by_name(&name).ok_or_else(|| anyhow::anyhow!("Unknown model: {}", name))?;
    bundle_model_for_wasm(model, &dest)
}

fn upload_to_r2(name: String, bucket: String) -> Result<()> {
    let model =
        find_model_by_name(&name).ok_or_else(|| anyhow::anyhow!("Unknown model: {}", name))?;

    // Get model file path from cache
    let cache_dir = get_cache_dir()?;
    let model_path = cache_dir.join(model.filename);

    if !model_path.exists() {
        bail!(
            "Model not found in cache. Download it first with: cargo xtask models download {}",
            name
        );
    }

    // Verify model before uploading
    verify_model(model)?;

    // Get Cloudflare credentials
    let api_token = std::env::var("CLOUDFLARE_API_TOKEN")
        .or_else(|_| std::env::var("CF_API_TOKEN"))
        .context(
            "CLOUDFLARE_API_TOKEN not found in environment\n\n\
             Set it with: export CLOUDFLARE_API_TOKEN=your_token_here\n\
             Get your token from: https://dash.cloudflare.com/profile/api-tokens"
        )?;

    // Upload to R2
    let rt = tokio::runtime::Runtime::new()?;

    // Get account ID from API
    let account_id = rt.block_on(crate::wasm::get_account_id(&api_token))?;

    let r2_config = crate::r2::R2Config {
        account_id,
        api_token,
        bucket_name: bucket.clone(),
    };

    println!("\nUploading model to R2...");
    println!("  Model: {}", model.name);
    println!("  Bucket: {}", bucket);
    println!();

    let public_url = rt.block_on(async {
        // First, ensure CORS is configured
        if let Err(e) = crate::r2::set_bucket_cors(&r2_config).await {
            println!("Warning: Could not set CORS (bucket may not exist or already configured): {}", e);
        }

        // Make bucket public
        if let Err(e) = crate::r2::make_bucket_public(&r2_config).await {
            println!("Warning: Could not make bucket public: {}", e);
        }

        // Upload the model
        crate::r2::upload_file_to_r2(
            &r2_config,
            &model_path,
            model.filename,
            "application/octet-stream",
        ).await
    })?;

    println!("\n✓ Model uploaded successfully!");
    println!("\nPublic URL: {}", public_url);
    println!("\nUpdate your WASM app to use this URL:");
    println!("  const modelUrl = '{}';", public_url);

    Ok(())
}
