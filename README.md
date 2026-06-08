# encoding-vfs

Transparent virtual filesystem that automatically converts legacy-encoded files to UTF-8 on read, and converts UTF-8 back to the original encoding on write. Mounts as a virtual drive so that any application — including Claude Code, VS Code, or `type` — sees clean UTF-8 content without needing any special configuration.

## Problem

Claude Code (and most modern tools) expect files to be UTF-8. Legacy projects often contain GBK, Shift_JIS, Big5, or other encoded source files, which render as garbled characters or replacement characters. This project solves the problem at the **system level** — no IDE plugins, no file-in-place conversion, no manual intervention.

## Architecture

```
┌───────────────────────────────────────────────┐
│           encoding-vfs-cli (entry)             │
│         clap → config → platform mount         │
├───────────────────────────────────────────────┤
│         encoding-vfs-core (shared)             │
│  config  encoding  detector  cache  vfs error  │
├───────────────────────────────────────────────┤
│  encoding-vfs-windows    │  encoding-vfs-linux │
│  WinFsp 2.1 virtual drive│  fuser FUSE mount   │
│  FileSystemContext trait │  Filesystem trait   │
└───────────────────────────────────────────────┘
```

## Quick Start (Windows)

### Prerequisites

- **Rust toolchain** — `rustup toolchain install stable`
- **WinFsp 2.1 runtime** — SxS installation (custom build with unique driver name, e.g. `WinFsp+20260608T063400Z`), or standard installation from [GitHub releases](https://github.com/winfsp/winfsp/releases)

### Build

```powershell
cargo build --release --features mount
```

### Run

```powershell
# Mount virtual drive Y: backed by C:\projects\original
.\target\release\encoding-vfs.exe -b C:\projects\original -d Y
```

That's it. Any read/write to `Y:\` will be transparently converted between the source encoding and UTF-8.

### Unmount

Two methods, both graceful:

```powershell
# Method 1: press Ctrl+C in the terminal

# Method 2: from any terminal
net use Y: /delete /y
```

## CLI Usage

```
encoding-vfs --help
```

| Flag | Description | Default |
|------|-------------|---------|
| `-b, --backend-dir <PATH>` | Directory containing original files | `.` |
| `-d, --drive <LETTER>` | Windows drive letter to mount | `X` |
| `-s, --source-encoding <ENC>` | Source encoding: `auto`, `GBK`, `Shift_JIS`, `Big5`, etc. | `auto` |
| `-t, --target-encoding <ENC>` | Target encoding presented to applications | `UTF-8` |
| `-L, --log-level <LEVEL>` | Log level: trace, debug, info, warn, error | `info` |
| `-c, --config <FILE>` | Optional TOML config file | — |

### Examples

```powershell
# Basic mount: auto-detect source encoding → UTF-8
encoding-vfs.exe -b C:\legacy-project -d X

# Fixed source encoding (faster, skips detection)
encoding-vfs.exe -b C:\sjis-project -d X -s Shift_JIS

# Big5 → UTF-8
encoding-vfs.exe -b C:\big5-project -d X -s Big5

# With config file (CLI overrides config values)
encoding-vfs.exe -b C:\legacy-project -d X -c encoding-vfs.toml
```

## Configuration

Create `encoding-vfs.toml`:

```toml
[backend]
backend_dir = "C:\\projects\\original"

[mount]
drive_letter = "X"

[encoding]
source_encoding = "auto"        # "auto" | "GBK" | "Shift_JIS" | "Big5" | ...
target_encoding = "UTF-8"       # "UTF-8" | "GBK" | ...
default_encoding = "GBK"        # fallback when auto-detect fails
detect_sample_bytes = 8192
cache_max_entries = 10000
cache_ttl_seconds = 3600

[log]
level = "info"
```

### Config Options

| Section | Key | Description | Default |
|---------|-----|-------------|---------|
| `backend` | `backend_dir` | Directory containing original files | `.` |
| `mount` | `drive_letter` | Windows drive letter | `X` |
| `encoding` | `source_encoding` | Source encoding (`auto` for detection) | `auto` |
| `encoding` | `target_encoding` | Target encoding presented to apps | `UTF-8` |
| `encoding` | `default_encoding` | Fallback when auto-detect fails | `GBK` |
| `encoding` | `detect_sample_bytes` | Bytes to read for encoding detection | `8192` |
| `encoding` | `cache_max_entries` | Max entries in encoding cache | `10000` |
| `encoding` | `cache_ttl_seconds` | Cache entry time-to-live | `3600` |
| `log` | `level` | Log verbosity | `info` |

## How It Works

### Read Path (source → UTF-8)

```
Application reads Y:\file.c
       │
       ▼
WinFsp callback → vfs.read_file()
       │
       ├── Read raw bytes from backend (e.g. GBK)
       ├── Detect encoding (BOM + content heuristic, cached)
       │   └─ "auto" mode: heuristic detection
       │   └─ fixed encoding: skip detection, use specified
       ├── Convert source → target encoding via encoding_rs
       └── Return target encoding bytes to application
```

### Write Path (UTF-8 → source)

```
Application writes Y:\file.c (UTF-8)
       │
       ▼
WinFsp callback → vfs.write_file()
       │
       ├── Detect existing file encoding (cached)
       ├── Convert target → source encoding
       └── Write encoded bytes to backend
```

### Encoding Detection

1. **BOM check** — UTF-8 BOM (`EF BB BF`), UTF-16 LE/BE BOM
2. **Content heuristic** — `encoding_rs_io` style validation
3. **Cache** — per-file encoding cached with TTL to avoid re-scanning
4. **Fallback** — uses `default_encoding` when undetectable

## Supported Encodings

GBK, CP936, GB2312, GB18030, UTF-8, UTF-16LE, UTF-16BE, Big5, EUC-JP, EUC-KR, Shift_JIS, KOI8-R, Windows-1252, ISO-8859-x, IBM866, Macintosh, and more (full list from `encoding_rs`).

## Project Structure

```
C:\projects\file-io-proxy\
├── Cargo.toml                          # workspace root
├── encoding-vfs-core/                  # platform-agnostic core
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                      # public exports
│       ├── config.rs                   # TOML config + defaults
│       ├── encoding.rs                 # source ↔ target encoding conversion
│       ├── detector.rs                 # BOM + heuristic encoding detection
│       ├── cache.rs                    # thread-safe LRU cache with TTL
│       ├── vfs.rs                      # EncodingVfs: core read/write/dir
│       └── error.rs                    # unified error types
├── encoding-vfs-windows/               # Windows WinFsp adapter
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                      # feature-gated exports
│       └── winfsp_host.rs             # FileSystemContext trait + run()
├── encoding-vfs-linux/                 # Linux FUSE adapter (stub)
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs                      # placeholder
├── encoding-vfs-cli/                   # unified CLI entry point
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs                     # clap → platform dispatch
│       └── test_drive.rs              # drive accessibility tests
└── encoding-vfs-test/                  # integration test harness
    ├── Cargo.toml
    └── src/
        └── main.rs                     # end-to-end verification
```

## Build Details

### Workspace Crates

| Crate | Role | Key Dependencies |
|-------|------|-----------------|
| `encoding-vfs-core` | Encoding detection, conversion, cache, VFS | `encoding_rs`, `encoding_rs_io`, `dashmap`, `toml`, `serde`, `thiserror`, `tracing` |
| `encoding-vfs-windows` | WinFsp 2.1 virtual drive mount | `winfsp 0.12.6+winfsp-2.1`, `widestring 1.0`, `windows 0.61`, `encoding-vfs-core` |
| `encoding-vfs-cli` | CLI binary, platform dispatch | `clap 4.4`, `tracing-subscriber`, platform crates |

### WinFsp Notes

- Uses **winfsp** crate (v0.12.6+winfsp-2.1) with `FileSystemContext` trait
- SxS (side-by-side) installation supported — each WinFsp build gets a unique driver name
- Security: returns null security descriptor, letting WinFsp apply defaults
- Tested: directory listing, file creation, read, write all verified working
