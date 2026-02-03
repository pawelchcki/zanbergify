use anyhow::{bail, Result};
use clap::{Args, Subcommand};

use crate::cache::{cache_size, clean_model, get_cache_dir, list_cached_models};
use crate::download::download_model;
use crate::util::format_size;
use crate::verify::verify_model;
use crate::wasm_bundle::bundle_model_for_wasm;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        sha256: "8c125705850b1c4e3ca3859c82b8f89e3c88e723e10de8cd8be90fb8eb839343",
        input_size: 1024,
        description: "BiRefNet lite - high quality, detailed edges",
    },
    ModelInfo {
        name: "u2net",
        model_type: ModelType::U2Net,
        url: "https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2net.onnx",
        filename: "u2net.onnx",
        size_bytes: 176_631_213,
        sha256: "60024c5c889badc19c04ad937298a77bc3e72e6a78a0e865a0f46a0e0f3d4c3b",
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
        }
    }
}

fn list_models() -> Result<()> {
    println!("Available Models:\n");
    println!("{:<16} {:<10} {:<12} {:<10} {}", "Name", "Type", "Resolution", "Size", "Description");
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
    if all {
        for model in MODELS {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(download_model(model))?;
            println!();
        }
    } else if let Some(name) = name {
        let model = find_model_by_name(&name)
            .ok_or_else(|| anyhow::anyhow!("Unknown model: {}", name))?;
        let rt = tokio::runtime::Runtime::new()?;
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
        let model = find_model_by_name(&name)
            .ok_or_else(|| anyhow::anyhow!("Unknown model: {}", name))?;
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
        let model = find_model_by_name(&name)
            .ok_or_else(|| anyhow::anyhow!("Unknown model: {}", name))?;
        clean_model(model.filename)?;
    } else {
        bail!("Specify a model name or use --all");
    }

    Ok(())
}

fn bundle_model(name: String, dest: String) -> Result<()> {
    let model = find_model_by_name(&name)
        .ok_or_else(|| anyhow::anyhow!("Unknown model: {}", name))?;
    bundle_model_for_wasm(model, &dest)
}
