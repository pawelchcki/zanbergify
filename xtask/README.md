# xtask - Model Management

Cargo-xtask pattern for automated model downloads, verification, and bundling.

## Usage

```bash
cargo xtask models list                    # List available models
cargo xtask models download <name>         # Download model (birefnet-lite, u2net, isnet)
cargo xtask models info                    # Show cache status
cargo xtask models verify <name>           # Verify checksum
cargo xtask models clean <name>            # Remove from cache
cargo xtask models bundle <name>           # Copy to WASM assets
```

## Models

| Name | Type | Resolution | Size | Use Case |
|------|------|------------|------|----------|
| birefnet-lite | BiRefNet | 1024×1024 | 214 MB | High quality, detailed edges |
| u2net | U2Net | 320×320 | 178 MB | Fast, simple images |
| isnet | ISNet | 1024×1024 | 200 MB | Balanced |

## Cache

Models cached at `~/.razemify/models/` (primary) or `~/.u2net/` (legacy).

Search priority:
1. Explicit path / env var (`RAZEMIFY_MODEL_PATH`)
2. xtask cache
3. Legacy cache
4. Current directory

## Implementation

- **Registry**: `models.rs` - Centralized metadata (URLs, checksums, sizes)
- **Download**: `download.rs` - Async with progress bars (reqwest + indicatif)
- **Verify**: `verify.rs` - SHA-256 validation (sha2)
- **Cache**: `cache.rs` - Directory operations
- **Bundle**: `wasm_bundle.rs` - WASM asset copying

Uses `rustls-tls` for cross-platform compatibility.
