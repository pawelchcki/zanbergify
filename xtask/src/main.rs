use anyhow::Result;
use clap::Parser;

mod cache;
mod download;
mod models;
mod r2;
mod util;
mod verify;
mod wasm;
mod wasm_bundle;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Zanbergify automation tasks", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Model management commands
    Models(models::ModelsCmd),
    /// WASM build and serve commands
    Wasm(wasm::WasmCmd),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Models(cmd) => cmd.run()?,
        Command::Wasm(cmd) => cmd.run()?,
    }

    Ok(())
}
