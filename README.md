# encoding-vfs

Transparent virtual filesystem that automatically converts legacy-encoded files to UTF-8 on read, and converts UTF-8 back to the original encoding on write. Mounts as a virtual drive so that any application — including Claude Code, VS Code, or `cat` — sees clean UTF-8 content without needing any special configuration.

## Problem

Modern tools expect UTF-8. Legacy projects often contain GBK, Shift_JIS, Big5, or other encoded source files, which render as garbled characters or replacement characters. This project solves the problem at the **system level** — no IDE plugins, no file-in-place conversion, no manual intervention.

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

---

## Windows (WinFsp)

### Prerequisites

#### 1. Install WinFsp 2.1 runtime

This project requires WinFsp 2.1 for the virtual drive mount. Install it first:

**Method A: winget (recommended)**
```powershell
winget install WinFsp.WinFsp
```

**Method B: Manual download**

Download the installer from [WinFsp releases](https://github.com/winfsp/winfsp/releases) and run `winfsp-*.msi`.

Verify the service is running:
```powershell
Get-Service WinFsp.Launcher
# Should show Status: Running
```

#### 2. Install Rust toolchain

```powershell
rustup toolchain install stable
```

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

---

## Linux (FUSE)

### Prerequisites

#### 1. Install FUSE3 and build dependencies

**Ubuntu/Debian:**
```bash
sudo apt-get install -y libfuse3-dev pkg-config
```

**Fedora:**
```bash
sudo dnf install -y fuse3-devel pkg-config
```

**Arch:**
```bash
sudo pacman -S fuse3 pkg-config
```

#### 2. Install Rust toolchain

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### Build

```bash
cargo build --release
```

### Run

```bash
# Mount FUSE filesystem at /mnt/gbk-vfs
./target/release/encoding-vfs -b /path/to/legacy-project -m /mnt/gbk-vfs
```

All read/write to `/mnt/gbk-vfs/` will be transparently converted between the source encoding and UTF-8.

### Unmount

```bash
# Method 1: press Ctrl+C in the terminal (auto-unmount)

# Method 2: from another terminal
fusermount3 -u /mnt/gbk-vfs
# or: fusermount -u /mnt/gbk-vfs  (older systems)
```

### Troubleshooting

**"option allow_other only allowed if 'user_allow_other' is set in /etc/fuse.conf"**

Uncomment the line in `/etc/fuse.conf`:
```bash
sudo sed -i 's/^#user_allow_other/user_allow_other/' /etc/fuse.conf
```

---

## CLI Usage

```
encoding-vfs --help
```

| Flag | Description | Default (Win) | Default (Linux) |
|------|-------------|---------------|-----------------|
| `-b, --backend <PATH>` | Backend directory with original files | `.` | `.` |
| `-d, --drive <LETTER>` | Windows drive letter to mount | `X` | — |
| `-m, --mount <PATH>` | Linux FUSE mount point | — | `/mnt/gbk-vfs` |
| `-s, --source-encoding <ENC>` | Source encoding: `auto`, `GBK`, `Shift_JIS`, `Big5`, etc. | `auto` | `auto` |
| `-t, --target-encoding <ENC>` | Target encoding presented to applications | `UTF-8` | `UTF-8` |
| `-L, --log-level <LEVEL>` | Log level: trace, debug, info, warn, error | `info` | `info` |
| `-c, --config <FILE>` | Optional TOML config file | — | — |

### Examples

**Windows:**
```powershell
# Basic mount: auto-detect source encoding → UTF-8
encoding-vfs.exe -b C:\legacy-project -d X

# Fixed source encoding (faster, skips detection)
encoding-vfs.exe -b C:\sjis-project -d X -s Shift_JIS

# Big5 → UTF-8
encoding-vfs.exe -b C:\big5-project -d X -s Big5

# With config file (CLI overrides config values)
encoding-vfs.exe -b C:\legacy-project -d X -c encoding-vfs.toml

# CLI overrides config file
encoding-vfs.exe -b C:\legacy-project -d X -c config.toml -s Big5
```

**Linux:**
```bash
# Basic mount
./encoding-vfs -b /home/user/legacy-project -m /mnt/gbk-vfs

# Fixed source encoding
./encoding-vfs -b /home/user/sjis-project -m /mnt/gbk-vfs -s Shift_JIS

# With config file
./encoding-vfs -b /home/user/legacy-project -m /mnt/gbk-vfs -c encoding-vfs.toml
```

## Configuration

Create `encoding-vfs.toml`:

```toml
[backend]
backend_dir = "C:\\projects\\original"   # Windows
# backend_dir = "/home/user/legacy-project"  # Linux

[mount]
drive_letter = "X"       # Windows: drive letter
# mount_point = "/mnt/gbk-vfs"  # Linux: mount point (optional)

[encoding]
source_encoding = "auto"        # "auto" | "GBK" | "Shift_JIS" | "Big5" | ...
target_encoding = "UTF-8"       # "UTF-8" | "GBK" | ...
default_encoding = "GBK"        # fallback when auto-detect fails
detect_sample_bytes = 8192
cache_max_entries = 10000
cache_ttl_seconds = 3600

[encoding.filter]
mode = "blacklist"              # "blacklist" (default) or "whitelist"
rules = ["*.dll", "logs/"]      # inline rules, same format as filter file

[log]
level = "info"
```

### Config Options

| Section | Key | Description | Default |
|---------|-----|-------------|---------|
| `backend` | `backend_dir` | Directory containing original files | `.` |
| `mount` | `drive_letter` | Windows drive letter | `X` |
| `mount` | `mount_point` | Linux FUSE mount point | `/mnt/gbk-vfs` |
| `encoding` | `source_encoding` | Source encoding (`auto` for detection) | `auto` |
| `encoding` | `target_encoding` | Target encoding presented to apps | `UTF-8` |
| `encoding` | `default_encoding` | Fallback when auto-detect fails | `GBK` |
| `encoding` | `detect_sample_bytes` | Bytes to read for encoding detection | `8192` |
| `encoding` | `cache_max_entries` | Max entries in encoding cache | `10000` |
| `encoding` | `cache_ttl_seconds` | Cache entry time-to-live | `3600` |
| `log` | `level` | Log verbosity | `info` |

## How It Works

### Read Path (source → target)

```
Application reads mounted file
       │
       ▼
Platform callback (WinFsp/FUSE) → vfs.read_file()
       │
       ├── Read raw bytes from backend (e.g. GBK)
       ├── Detect encoding (BOM + content heuristic, cached)
       │   └─ "auto" mode: heuristic detection
       │   └─ fixed encoding: skip detection, use specified
       ├── Convert source → target encoding via encoding_rs
       └── Return target encoding bytes to application
```

### Write Path (target → source)

```
Application writes mounted file (target encoding, e.g. UTF-8)
       │
       ▼
Platform callback (WinFsp/FUSE) → vfs.write_file()
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
encoding-vfs/
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
│       └── winfsp_host.rs              # FileSystemContext trait + run()
├── encoding-vfs-linux/                 # Linux FUSE adapter
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                      # public exports
│       └── fuse_host.rs                # fuser::Filesystem trait + run()
├── encoding-vfs-cli/                   # unified CLI entry point
│   ├── Cargo.toml
│   └── src/
│       └── main.rs                     # clap → platform dispatch
└── .github/workflows/
    └── release.yml                     # CI build + release for Win + Linux
```

## Build Details

### Workspace Crates

| Crate | Role | Key Dependencies |
|-------|------|-----------------|
| `encoding-vfs-core` | Encoding detection, conversion, cache, VFS | `encoding_rs`, `encoding_rs_io`, `dashmap`, `toml`, `serde`, `thiserror`, `tracing` |
| `encoding-vfs-windows` | WinFsp 2.1 virtual drive mount | `winfsp 0.12`, `widestring 1.0`, `windows 0.61`, `encoding-vfs-core` |
| `encoding-vfs-linux` | Linux FUSE filesystem mount | `fuser 0.14`, `libc 0.2`, `encoding-vfs-core` |
| `encoding-vfs-cli` | Unified CLI binary, platform dispatch | `clap 4.4`, `tracing-subscriber`, platform crates |

### Feature Flags

| Feature | Platform | Description |
|---------|----------|-------------|
| `mount` | Windows | Enable WinFsp virtual drive mount (requires `--features mount`) |

On Linux, the FUSE adapter is always included (no feature flag needed).

### WinFsp Notes

- Uses **winfsp** crate (v0.12.6+winfsp-2.1) with `FileSystemContext` trait
- Standard MSI install via `winget install WinFsp.WinFsp` is recommended for most users
- SxS (side-by-side) installation supported for custom builds with unique driver names
- Security: returns null security descriptor, letting WinFsp apply defaults
- Tested: directory listing, file creation, read, write all verified working

### FUSE Notes

- Uses **fuser** crate (v0.14) with `Filesystem` trait
- Requires `libfuse3-dev` and `pkg-config` at build time
- Runtime requires `/dev/fuse` device and `fusermount`/`fusermount3`
- If other users need access, uncomment `user_allow_other` in `/etc/fuse.conf`

## CI / Release

Push a tag to trigger automated builds:

```bash
git tag v0.1.0
git push origin v0.1.0
```

This creates draft releases on GitHub:
- `encoding-vfs-Windows-x64-v0.1.0.zip` — binary + WinFsp DLL + README
- `encoding-vfs-Linux-x64-v0.1.0.tar.gz` — binary + README
