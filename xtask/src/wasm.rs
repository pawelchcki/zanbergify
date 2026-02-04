use anyhow::{bail, Context, Result};
use base64::Engine;
use clap::{Args, Subcommand};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::util::project_root;

#[derive(Args)]
pub struct WasmCmd {
    #[command(subcommand)]
    pub command: WasmSubCmd,
}

#[derive(Subcommand)]
pub enum WasmSubCmd {
    /// Build the WASM package only
    Build {
        /// Build in release mode (optimized)
        #[arg(long, short)]
        release: bool,
    },
    /// Build and serve locally for development
    Serve {
        /// Port to serve on
        #[arg(long, short, default_value = "8080")]
        port: u16,
        /// Build in release mode (optimized)
        #[arg(long, short)]
        release: bool,
        /// Automatically open browser
        #[arg(long)]
        open: bool,
    },
    /// Package WASM site for deployment
    Package {
        /// Output directory for packaged files
        #[arg(long, default_value = "target/wasm-site")]
        output: String,
        /// Build in release mode (optimized)
        #[arg(long, short)]
        release: bool,
    },
    /// Deploy WASM site to Cloudflare Pages
    Deploy {
        /// Branch name for deployment (auto-detected from git if not specified)
        /// Production = main/master, others = preview
        #[arg(long, short)]
        branch: Option<String>,
        /// Cloudflare Pages project name
        #[arg(long, default_value = "zanbergify-wasm")]
        project_name: String,
        /// Output directory to deploy
        #[arg(long, default_value = "target/wasm-site")]
        output: String,
        /// Skip packaging step (deploy pre-packaged files)
        #[arg(long)]
        skip_package: bool,
        /// Build in release mode (only if packaging)
        #[arg(long, short)]
        release: bool,
    },
    /// Clean up old deployments
    Cleanup {
        /// Cloudflare Pages project name
        #[arg(long, default_value = "zanbergify-wasm")]
        project_name: String,
        /// Keep this many recent deployments per environment (production/preview)
        #[arg(long, default_value = "5")]
        keep: usize,
        /// Actually delete (without this flag, only shows what would be deleted)
        #[arg(long)]
        yes: bool,
    },
}

impl WasmCmd {
    pub fn run(self) -> Result<()> {
        match self.command {
            WasmSubCmd::Build { release } => build_wasm(release),
            WasmSubCmd::Serve {
                port,
                release,
                open,
            } => serve_wasm(port, release, open),
            WasmSubCmd::Package { output, release } => package_wasm(&output, release),
            WasmSubCmd::Deploy {
                branch,
                project_name,
                output,
                skip_package,
                release,
            } => deploy_wasm(branch, &project_name, &output, skip_package, release),
            WasmSubCmd::Cleanup {
                project_name,
                keep,
                yes,
            } => cleanup_deployments(&project_name, keep, yes),
        }
    }
}

fn build_wasm(release: bool) -> Result<()> {
    let project_root = project_root();
    let wasm_dir = project_root.join("zanbergify-wasm");

    if !wasm_dir.exists() {
        bail!(
            "zanbergify-wasm directory not found at {}",
            wasm_dir.display()
        );
    }

    println!("Building WASM package...");
    let mode = if release { "release" } else { "dev" };
    println!("Mode: {}", mode);

    let mut cmd = Command::new("wasm-pack");
    cmd.arg("build")
        .arg("--target")
        .arg("web")
        .current_dir(&wasm_dir);

    if release {
        cmd.arg("--release");
    } else {
        cmd.arg("--dev");
    }

    let status = cmd.status().context(
        "Failed to run wasm-pack. Is wasm-pack installed? Install with: cargo install wasm-pack",
    )?;

    if !status.success() {
        bail!("wasm-pack build failed");
    }

    let pkg_dir = wasm_dir.join("pkg");
    println!("✓ WASM build complete: {}", pkg_dir.display());

    Ok(())
}

fn prepare_site_dir(output_dir: &Path, release: bool) -> Result<()> {
    let project_root = project_root();
    let wasm_dir = project_root.join("zanbergify-wasm");
    let pkg_dir = wasm_dir.join("pkg");
    let www_dir = wasm_dir.join("www");

    // Ensure WASM is built
    if !pkg_dir.exists() {
        println!("WASM package not found, building first...");
        build_wasm(release)?;
    }

    // Clean and create output directory
    if output_dir.exists() {
        fs::remove_dir_all(output_dir).context(format!(
            "Failed to clean directory: {}",
            output_dir.display()
        ))?;
    }
    fs::create_dir_all(output_dir).context(format!(
        "Failed to create directory: {}",
        output_dir.display()
    ))?;

    println!("Preparing site directory: {}", output_dir.display());

    // Copy index.html
    let src_html = www_dir.join("index.html");
    let dst_html = output_dir.join("index.html");
    if src_html.exists() {
        fs::copy(&src_html, &dst_html).context(format!("Failed to copy {}", src_html.display()))?;
        println!("  ✓ Copied index.html");
    }

    // Copy and rewrite index.js
    rewrite_index_js(&www_dir, output_dir)?;
    println!("  ✓ Copied and rewritten index.js");

    // Copy _headers
    let src_headers = www_dir.join("_headers");
    let dst_headers = output_dir.join("_headers");
    if src_headers.exists() {
        fs::copy(&src_headers, &dst_headers)
            .context(format!("Failed to copy {}", src_headers.display()))?;
        println!("  ✓ Copied _headers");
    }

    // Copy models directory
    let src_models = www_dir.join("models");
    let dst_models = output_dir.join("models");
    if src_models.exists() {
        copy_dir_recursive(&src_models, &dst_models)?;
        println!("  ✓ Copied models/");
    }

    // Copy pkg directory
    let dst_pkg = output_dir.join("pkg");
    copy_dir_recursive(&pkg_dir, &dst_pkg)?;
    println!("  ✓ Copied pkg/");

    // Create deployment marker with timestamp in filename to force upload
    // This ensures Cloudflare API provides upload URLs even when all other files are cached
    println!("✓ Site directory ready");

    Ok(())
}

fn rewrite_index_js(www_dir: &Path, output_dir: &Path) -> Result<()> {
    let src = www_dir.join("index.js");
    let dst = output_dir.join("index.js");

    let content = fs::read_to_string(&src).context(format!("Failed to read {}", src.display()))?;

    // Rewrite import paths: '../pkg/' -> './pkg/'
    let rewritten = content.replace("'../pkg/", "'./pkg/");

    fs::write(&dst, rewritten).context(format!("Failed to write {}", dst.display()))?;

    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst).context(format!("Failed to create directory: {}", dst.display()))?;

    for entry in
        fs::read_dir(src).context(format!("Failed to read directory: {}", src.display()))?
    {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .context(format!("Failed to copy {}", src_path.display()))?;
        }
    }

    Ok(())
}

fn serve_wasm(port: u16, release: bool, open: bool) -> Result<()> {
    // Validate port
    if port < 1024 {
        bail!("Port must be >= 1024 (non-privileged). Got: {}", port);
    }

    let project_root = project_root();
    let output_dir = project_root.join("target").join("wasm-site");

    // Prepare site
    prepare_site_dir(&output_dir, release)?;

    // Start server
    let addr = format!("127.0.0.1:{}", port);
    let server = tiny_http::Server::http(&addr)
        .map_err(|e| anyhow::anyhow!("Failed to start server on {}: {}", addr, e))?;

    let url = format!("http://localhost:{}", port);

    println!("\n{}", "=".repeat(60));
    println!("WASM Development Server");
    println!("{}", "=".repeat(60));
    println!("  URL:       {}", url);
    println!("  Port:      {}", port);
    println!("  Directory: {}", output_dir.display());
    println!("  Mode:      {}", if release { "release" } else { "dev" });
    println!("{}", "=".repeat(60));
    println!("\nPress Ctrl+C to stop the server\n");

    if open {
        if let Err(e) = open_browser(&url) {
            println!("Failed to open browser: {}", e);
            println!("Please open {} manually", url);
        }
    }

    serve_static_files(&server, &output_dir)?;

    Ok(())
}

fn serve_static_files(server: &tiny_http::Server, root: &Path) -> Result<()> {
    loop {
        let request = match server.recv() {
            Ok(rq) => rq,
            Err(e) => {
                eprintln!("Error receiving request: {}", e);
                continue;
            }
        };

        let url_path = request.url();

        // Map URL path to file path
        let file_path = if url_path == "/" || url_path.is_empty() {
            root.join("index.html")
        } else {
            // Remove leading slash
            let path = url_path.trim_start_matches('/');
            root.join(path)
        };

        // Security: prevent directory traversal
        let canonical_root = root
            .canonicalize()
            .context("Failed to canonicalize root directory")?;

        let response = if let Ok(canonical_file) = file_path.canonicalize() {
            if canonical_file.starts_with(&canonical_root) && canonical_file.is_file() {
                // File exists and is within root
                match fs::read(&canonical_file) {
                    Ok(data) => {
                        let mime_type = get_mime_type(&canonical_file);

                        // Log request
                        let now = SystemTime::now();
                        println!("[{}] {} -> {}", format_time(now), url_path, mime_type);

                        let mut response = tiny_http::Response::from_data(data);

                        // Add CORS headers required for WASM
                        response = response
                            .with_header(
                                tiny_http::Header::from_bytes(
                                    &b"Cross-Origin-Embedder-Policy"[..],
                                    &b"require-corp"[..],
                                )
                                .expect("Header name is valid ASCII"),
                            )
                            .with_header(
                                tiny_http::Header::from_bytes(
                                    &b"Cross-Origin-Opener-Policy"[..],
                                    &b"same-origin"[..],
                                )
                                .expect("Header name is valid ASCII"),
                            )
                            .with_header(
                                tiny_http::Header::from_bytes(
                                    &b"Content-Type"[..],
                                    mime_type.as_bytes(),
                                )
                                .expect("Header name is valid ASCII"),
                            );

                        response
                    }
                    Err(e) => {
                        eprintln!("Error reading file {}: {}", canonical_file.display(), e);
                        tiny_http::Response::from_string("500 Internal Server Error")
                            .with_status_code(500)
                    }
                }
            } else {
                // File not found or outside root
                tiny_http::Response::from_string("404 Not Found").with_status_code(404)
            }
        } else {
            // File doesn't exist
            tiny_http::Response::from_string("404 Not Found").with_status_code(404)
        };

        if let Err(e) = request.respond(response) {
            eprintln!("Error sending response: {}", e);
        }
    }
}

fn get_mime_type(path: &Path) -> &'static str {
    match path.extension().and_then(|s| s.to_str()) {
        Some("html") => "text/html",
        Some("js") => "application/javascript",
        Some("wasm") => "application/wasm",
        Some("json") => "application/json",
        Some("css") => "text/css",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("onnx") => "application/octet-stream",
        _ => "application/octet-stream",
    }
}

fn format_time(time: SystemTime) -> String {
    if let Ok(duration) = time.duration_since(UNIX_EPOCH) {
        let secs = duration.as_secs();
        let hours = (secs / 3600) % 24;
        let minutes = (secs / 60) % 60;
        let seconds = secs % 60;
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        "??:??:??".to_string()
    }
}

fn open_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", url])
            .spawn()
            .context("Failed to open browser")?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(url)
            .spawn()
            .context("Failed to open browser")?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(url)
            .spawn()
            .context("Failed to open browser")?;
    }

    Ok(())
}

fn cleanup_deployments(project_name: &str, keep: usize, yes: bool) -> Result<()> {
    println!("\n{}", "=".repeat(60));
    println!("Deployment Cleanup");
    println!("{}", "=".repeat(60));
    println!("  Project:      {}", project_name);
    println!("  Keep:         {} most recent per environment", keep);
    println!("  Mode:         {}", if yes { "DELETE" } else { "DRY RUN" });
    println!("{}", "=".repeat(60));
    println!();

    // Get API credentials
    let api_token = check_cloudflare_credentials()?;

    // Create async runtime
    let runtime = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;

    runtime.block_on(async {
        // Get account ID
        let account_id = get_account_id(&api_token).await?;

        // Fetch all deployments
        println!("Fetching deployments...");
        let client = reqwest::Client::new();
        let url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/pages/projects/{}/deployments",
            account_id, project_name
        );

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_token))
            .send()
            .await
            .context("Failed to fetch deployments")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("Failed to fetch deployments (status: {}): {}", status, body);
        }

        let deployments: DeploymentsListResponse = response
            .json()
            .await
            .context("Failed to parse deployments")?;

        println!("✓ Found {} total deployments\n", deployments.result.len());

        // Group by environment
        let mut production: Vec<DeploymentInfo> = Vec::new();
        let mut preview: Vec<DeploymentInfo> = Vec::new();

        for deployment in deployments.result {
            match deployment.environment.as_str() {
                "production" => production.push(deployment),
                "preview" => preview.push(deployment),
                _ => {}
            }
        }

        // Sort by creation date (newest first)
        production.sort_by(|a, b| b.created_on.cmp(&a.created_on));
        preview.sort_by(|a, b| b.created_on.cmp(&a.created_on));

        println!("Environment breakdown:");
        println!("  Production:   {} deployments", production.len());
        println!("  Preview:      {} deployments", preview.len());
        println!();

        // Determine what to delete
        let prod_to_delete: Vec<_> = production.iter().skip(keep).collect();
        let preview_to_delete: Vec<_> = preview.iter().skip(keep).collect();

        let total_to_delete = prod_to_delete.len() + preview_to_delete.len();

        if total_to_delete == 0 {
            println!("✓ No deployments to clean up");
            return Ok(());
        }

        println!("Deployments to delete ({}):", total_to_delete);
        println!();

        if !prod_to_delete.is_empty() {
            println!("Production ({} to delete):", prod_to_delete.len());
            for dep in &prod_to_delete {
                let alias = dep
                    .aliases
                    .as_ref()
                    .and_then(|a| a.first())
                    .map(|s| s.as_str())
                    .unwrap_or("no alias");
                println!("  - {} | {} | {}", dep.short_id, dep.created_on, alias);
            }
            println!();
        }

        if !preview_to_delete.is_empty() {
            println!("Preview ({} to delete):", preview_to_delete.len());
            for dep in &preview_to_delete {
                let alias = dep
                    .aliases
                    .as_ref()
                    .and_then(|a| a.first())
                    .map(|s| s.as_str())
                    .unwrap_or("no alias");
                println!("  - {} | {} | {}", dep.short_id, dep.created_on, alias);
            }
            println!();
        }

        if !yes {
            println!("ℹ️  This is a DRY RUN. Use --yes to actually delete these deployments.");
            return Ok(());
        }

        // Actually delete
        println!("Deleting deployments...");
        let pb = indicatif::ProgressBar::new(total_to_delete as u64);
        pb.set_style(
            indicatif::ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("=>-"),
        );

        let mut deleted = 0;
        let mut failed = 0;

        for dep in prod_to_delete.iter().chain(preview_to_delete.iter()) {
            pb.set_message(dep.short_id.to_string());

            let delete_url = format!(
                "https://api.cloudflare.com/client/v4/accounts/{}/pages/projects/{}/deployments/{}",
                account_id, project_name, dep.id
            );

            let result = client
                .delete(&delete_url)
                .header("Authorization", format!("Bearer {}", api_token))
                .send()
                .await;

            match result {
                Ok(resp) if resp.status().is_success() => deleted += 1,
                _ => failed += 1,
            }

            pb.inc(1);
        }

        pb.finish_with_message("Complete");

        println!();
        println!("✓ Deleted: {}", deleted);
        if failed > 0 {
            println!("✗ Failed:  {}", failed);
        }

        Ok(())
    })
}

fn package_wasm(output: &str, release: bool) -> Result<()> {
    let project_root = project_root();
    let output_dir = project_root.join(output);

    prepare_site_dir(&output_dir, release)?;

    println!("\n{}", "=".repeat(60));
    println!("Package Ready");
    println!("{}", "=".repeat(60));
    println!("  Location: {}", output_dir.display());
    println!("  Mode:     {}", if release { "release" } else { "dev" });
    println!("{}", "=".repeat(60));
    println!("\nTo deploy to Cloudflare Pages:");
    println!(
        "  npx wrangler pages deploy {} --project-name zanbergify",
        output_dir.display()
    );
    println!();

    Ok(())
}

// ========================================
// Deployment Functions
// ========================================

// Wrangler-style API structures

#[derive(serde::Deserialize, Debug)]
struct UploadTokenResponse {
    result: UploadTokenResult,
    #[allow(dead_code)]
    success: bool,
}

#[derive(serde::Deserialize, Debug)]
struct UploadTokenResult {
    jwt: String,
}

#[derive(serde::Serialize)]
struct CheckMissingRequest {
    hashes: Vec<String>,
}

#[derive(serde::Serialize)]
struct UploadPayloadFile {
    key: String,
    value: String,
    metadata: FileMetadata,
    base64: bool,
}

#[derive(serde::Serialize)]
struct FileMetadata {
    #[serde(rename = "contentType")]
    content_type: String,
}

#[derive(serde::Serialize)]
struct UpsertHashesRequest {
    hashes: Vec<String>,
}

#[derive(serde::Deserialize, Debug)]
struct CreateDeploymentResponse {
    result: DeploymentResult,
    #[allow(dead_code)]
    success: bool,
}

#[derive(serde::Deserialize, Debug)]
struct DeploymentResult {
    id: String,
    #[allow(dead_code)]
    url: String,
}

#[derive(serde::Deserialize, Debug)]
struct DeploymentsListResponse {
    result: Vec<DeploymentInfo>,
}

#[derive(serde::Deserialize, Debug, Clone)]
struct DeploymentInfo {
    id: String,
    short_id: String,
    environment: String,
    #[allow(dead_code)]
    url: String,
    created_on: String,
    #[serde(default)]
    aliases: Option<Vec<String>>,
}

#[derive(serde::Deserialize)]
struct AccountsResponse {
    result: Vec<Account>,
}

#[derive(serde::Deserialize)]
struct Account {
    id: String,
    name: String,
}

fn check_cloudflare_credentials() -> Result<String> {
    let api_token = std::env::var("CLOUDFLARE_API_TOKEN").context(
        "CLOUDFLARE_API_TOKEN environment variable not set\n\n\
             Get your API token from: https://dash.cloudflare.com/profile/api-tokens\n\
             Create a token with 'Cloudflare Pages' permissions\n\n\
             Then set: export CLOUDFLARE_API_TOKEN=your_token",
    )?;

    println!("✓ Cloudflare API token found");
    Ok(api_token)
}

async fn get_account_id(api_token: &str) -> Result<String> {
    // Check if account ID is explicitly provided
    if let Ok(account_id) = std::env::var("CLOUDFLARE_ACCOUNT_ID") {
        println!("✓ Using account ID from environment: {}", account_id);
        return Ok(account_id);
    }

    // Otherwise, fetch it from the API
    println!("Fetching account ID from Cloudflare API...");

    let client = reqwest::Client::new();
    let response = client
        .get("https://api.cloudflare.com/client/v4/accounts")
        .header("Authorization", format!("Bearer {}", api_token))
        .send()
        .await
        .context("Failed to fetch accounts from Cloudflare API")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!(
            "Failed to fetch accounts (status: {})\n\
             Response: {}\n\n\
             Check that your API token has correct permissions",
            status,
            body
        );
    }

    let accounts: AccountsResponse = response
        .json()
        .await
        .context("Failed to parse accounts response")?;

    if accounts.result.is_empty() {
        bail!(
            "No Cloudflare accounts found\n\n\
             Ensure your API token has access to at least one account"
        );
    }

    if accounts.result.len() > 1 {
        println!("\nMultiple accounts found:");
        for (i, account) in accounts.result.iter().enumerate() {
            println!("  {}. {} ({})", i + 1, account.name, account.id);
        }
        println!(
            "\nUsing first account: {} ({})",
            accounts.result[0].name, accounts.result[0].id
        );
        println!("To use a different account, set: export CLOUDFLARE_ACCOUNT_ID=<account_id>");
    }

    let account_id = accounts.result[0].id.clone();
    println!("✓ Account ID: {}", account_id);

    Ok(account_id)
}

fn determine_branch(explicit_branch: Option<String>) -> Result<String> {
    if let Some(branch) = explicit_branch {
        println!("Using explicit branch: {}", branch);
        return Ok(branch);
    }

    // Auto-detect from git
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .context("Failed to get current git branch")?;

    if !output.status.success() {
        bail!(
            "Not in a git repository or unable to determine branch\n\n\
             Use --branch <name> to specify deployment branch explicitly"
        );
    }

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if branch.is_empty() {
        bail!(
            "Not on a git branch (detached HEAD?)\n\n\
             Use --branch <name> to specify deployment branch explicitly"
        );
    }

    println!("Auto-detected branch: {}", branch);
    Ok(branch)
}

fn get_git_commit_hash() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

fn get_git_commit_message() -> Option<String> {
    let output = Command::new("git")
        .args(["log", "-1", "--pretty=%B"])
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

fn hash_file(path: &Path) -> Result<String> {
    let content = fs::read(path).context(format!("Failed to read file: {}", path.display()))?;
    let hash = md5::compute(&content);
    Ok(format!("{:x}", hash))
}

fn collect_files(dir: &Path) -> Result<HashMap<String, (PathBuf, String)>> {
    let mut files = HashMap::new();

    fn visit_dir(
        dir: &Path,
        base: &Path,
        files: &mut HashMap<String, (PathBuf, String)>,
    ) -> Result<()> {
        for entry in
            fs::read_dir(dir).context(format!("Failed to read directory: {}", dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                visit_dir(&path, base, files)?;
            } else if path.is_file() {
                let relative = path
                    .strip_prefix(base)
                    .context("Failed to get relative path")?
                    .to_string_lossy()
                    .replace('\\', "/");

                // Don't prepend / - Cloudflare expects paths without leading slash
                let hash = hash_file(&path)?;

                files.insert(relative.to_string(), (path.clone(), hash));
            }
        }
        Ok(())
    }

    visit_dir(dir, dir, &mut files)?;
    Ok(files)
}

fn deploy_wasm(
    branch: Option<String>,
    project_name: &str,
    output: &str,
    skip_package: bool,
    release: bool,
) -> Result<()> {
    println!("\n{}", "=".repeat(60));
    println!("WASM Deployment");
    println!("{}", "=".repeat(60));

    // Pre-flight checks
    let api_token = check_cloudflare_credentials()?;

    // Determine deployment target
    let branch = determine_branch(branch)?;
    let is_production = branch == "main" || branch == "master";

    let project_root = project_root();
    let output_dir = project_root.join(output);

    // Package if needed
    if !skip_package {
        println!("\nPackaging WASM site...");
        prepare_site_dir(&output_dir, release)?;
        println!("✓ Packaging complete");
    } else {
        if !output_dir.exists() {
            bail!(
                "Output directory does not exist: {}\n\n\
                 Run without --skip-package to build first",
                output_dir.display()
            );
        }
        println!("✓ Using pre-packaged files from: {}", output_dir.display());
    }

    // Execute deployment (async operations)
    let runtime = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;

    runtime.block_on(async {
        // Fetch account ID (or use from env)
        let account_id = get_account_id(&api_token).await?;

        // Display deployment info
        println!("\n{}", "=".repeat(60));
        println!("Deploying to Cloudflare Pages");
        println!("{}", "=".repeat(60));
        println!("  Project:      {}", project_name);
        println!("  Branch:       {}", branch);
        println!(
            "  Type:         {}",
            if is_production {
                "Production"
            } else {
                "Preview"
            }
        );
        println!("  Source:       {}", output_dir.display());
        println!(
            "  Mode:         {}",
            if release { "Release" } else { "Debug" }
        );
        println!("{}", "=".repeat(60));
        println!();

        // Execute deployment
        deploy_to_cloudflare(&output_dir, project_name, &branch, &api_token, &account_id).await?;

        // Success message
        println!("\n{}", "=".repeat(60));
        println!("✓ Deployment Successful");
        println!("{}", "=".repeat(60));

        if is_production {
            println!("  Production URL: https://{}.pages.dev", project_name);
        } else {
            println!(
                "  Preview URL:    https://{}.{}.pages.dev",
                branch, project_name
            );
        }

        println!();

        Ok(())
    })
}

async fn deploy_to_cloudflare(
    output_dir: &Path,
    project_name: &str,
    branch: &str,
    api_token: &str,
    account_id: &str,
) -> Result<()> {
    println!("Collecting and hashing files...");
    let files = collect_files(output_dir)?;
    println!("✓ Found {} files", files.len());

    let client = reqwest::Client::new();

    // Step 1: Get upload token (JWT)
    println!("\nGetting upload token...");
    let token_url = format!(
        "https://api.cloudflare.com/client/v4/accounts/{}/pages/projects/{}/upload-token",
        account_id, project_name
    );

    let token_response = client
        .get(&token_url)
        .header("Authorization", format!("Bearer {}", api_token))
        .send()
        .await
        .context("Failed to get upload token")?;

    if !token_response.status().is_success() {
        let status = token_response.status();
        let body = token_response.text().await.unwrap_or_default();
        bail!(
            "Failed to get upload token (status: {})\n\
             Response: {}\n\n\
             Possible solutions:\n\
             - Verify project '{}' exists in Cloudflare Pages dashboard\n\
             - Check API token has 'Cloudflare Pages' permissions",
            status,
            body,
            project_name
        );
    }

    let token_data: UploadTokenResponse = token_response
        .json()
        .await
        .context("Failed to parse upload token response")?;
    let jwt = token_data.result.jwt;
    println!("✓ Upload token obtained");

    // Step 2: Check which file hashes are missing
    // Exclude special files (_headers, _redirects) - they go directly in deployment FormData
    println!("\nChecking which files need upload...");
    let hashes: Vec<String> = files
        .iter()
        .filter(|(path, _)| *path != "_headers" && *path != "_redirects")
        .map(|(_, (_, hash))| hash.clone())
        .collect();

    let check_missing_url =
        "https://api.cloudflare.com/client/v4/pages/assets/check-missing".to_string();

    let check_response = client
        .post(&check_missing_url)
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .json(&CheckMissingRequest {
            hashes: hashes.clone(),
        })
        .send()
        .await
        .context("Failed to check missing hashes")?;

    if !check_response.status().is_success() {
        let status = check_response.status();
        let body = check_response.text().await.unwrap_or_default();
        bail!(
            "Failed to check missing hashes (status: {}): {}",
            status,
            body
        );
    }

    let missing_hashes: Vec<String> = check_response
        .json::<serde_json::Value>()
        .await
        .context("Failed to parse check-missing response")?
        .get("result")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let skipped = files.len() - missing_hashes.len();
    if skipped > 0 {
        println!("✓ {} files already cached", skipped);
    }

    // Step 3: Upload missing files
    if !missing_hashes.is_empty() {
        println!("Uploading {} new files...", missing_hashes.len());

        let missing_set: std::collections::HashSet<String> =
            missing_hashes.iter().cloned().collect();

        // Build upload payload
        let mut upload_files = Vec::new();
        for (_path, (full_path, hash)) in &files {
            if missing_set.contains(hash) {
                let content = fs::read(full_path)
                    .context(format!("Failed to read file: {}", full_path.display()))?;
                let encoded = base64::engine::general_purpose::STANDARD.encode(&content);

                upload_files.push(UploadPayloadFile {
                    key: hash.clone(),
                    value: encoded,
                    metadata: FileMetadata {
                        content_type: get_mime_type(full_path).to_string(),
                    },
                    base64: true,
                });
            }
        }

        let upload_url = "https://api.cloudflare.com/client/v4/pages/assets/upload".to_string();

        let upload_response = client
            .post(&upload_url)
            .header("Authorization", format!("Bearer {}", jwt))
            .header("Content-Type", "application/json")
            .json(&upload_files)
            .send()
            .await
            .context("Failed to upload files")?;

        if !upload_response.status().is_success() {
            let status = upload_response.status();
            let body = upload_response.text().await.unwrap_or_default();
            bail!("Failed to upload files (status: {}): {}", status, body);
        }

        println!("✓ Files uploaded successfully");
    } else {
        println!("✓ No new files to upload");
    }

    // Step 4: Upsert hashes
    println!("\nRegistering file hashes...");
    let upsert_url = "https://api.cloudflare.com/client/v4/pages/assets/upsert-hashes".to_string();

    let upsert_response = client
        .post(&upsert_url)
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .json(&UpsertHashesRequest { hashes })
        .send()
        .await
        .context("Failed to upsert hashes")?;

    if !upsert_response.status().is_success() {
        let status = upsert_response.status();
        let body = upsert_response.text().await.unwrap_or_default();
        bail!("Failed to upsert hashes (status: {}): {}", status, body);
    }

    println!("✓ File hashes registered");

    // Step 5: Create deployment with manifest
    println!("\nCreating deployment...");

    // Build manifest - wrangler uses leading slashes!
    // But exclude special files like _headers, _redirects which must be uploaded as File objects
    let mut manifest_map = HashMap::new();
    for (path, (_full_path, hash)) in &files {
        // Skip special files that need to be uploaded directly
        if path == "_headers" || path == "_redirects" {
            continue;
        }
        manifest_map.insert(format!("/{}", path), hash.clone());
    }

    let manifest_json =
        serde_json::to_string(&manifest_map).context("Failed to serialize manifest")?;

    println!("  Manifest has {} entries", manifest_map.len());
    if manifest_map.len() < 15 {
        println!("  Manifest: {}", manifest_json);
    }

    let commit_hash = get_git_commit_hash();
    let commit_message = get_git_commit_message();

    let deployment_url = format!(
        "https://api.cloudflare.com/client/v4/accounts/{}/pages/projects/{}/deployments",
        account_id, project_name
    );

    // Build FormData in same order as wrangler
    let mut form = reqwest::multipart::Form::new().text("manifest", manifest_json);

    if let Some(message) = commit_message {
        form = form.text("commit_message", message);
    }
    if let Some(hash) = commit_hash {
        form = form.text("commit_hash", hash);
    }

    // Always mark as dirty to match wrangler behavior
    form = form.text("commit_dirty", "true");

    // Branch after metadata
    form = form.text("branch", branch.to_string());

    // Add special files as File objects after text fields
    if let Some((full_path, _hash)) = files.get("_headers") {
        let content = fs::read(full_path).context("Failed to read _headers")?;
        form = form.part(
            "_headers",
            reqwest::multipart::Part::bytes(content).file_name("_headers"),
        );
        println!("  ✓ Including _headers file");
    }

    if let Some((full_path, _hash)) = files.get("_redirects") {
        let content = fs::read(full_path).context("Failed to read _redirects")?;
        form = form.part(
            "_redirects",
            reqwest::multipart::Part::bytes(content).file_name("_redirects"),
        );
        println!("  ✓ Including _redirects file");
    }

    let deployment_response = client
        .post(&deployment_url)
        .header("Authorization", format!("Bearer {}", api_token))
        .multipart(form)
        .send()
        .await
        .context("Failed to create deployment")?;

    if !deployment_response.status().is_success() {
        let status = deployment_response.status();
        let body = deployment_response.text().await.unwrap_or_default();
        bail!(
            "Failed to create deployment (status: {})\n\
             Response: {}",
            status,
            body
        );
    }

    let deployment: CreateDeploymentResponse = deployment_response
        .json()
        .await
        .context("Failed to parse deployment response")?;

    println!("✓ Deployment created: {}", deployment.result.id);

    Ok(())
}
