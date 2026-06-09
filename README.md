# encoding-vfs

[English](README.md) | [中文](README_zh.md)

透明虚拟文件系统，自动将遗留编码文件（GBK、Shift_JIS、Big5 等）转换为 UTF-8 以供读取，并在写入时将 UTF-8 转换回原始编码。挂载为虚拟驱动器后，任何应用无需任何配置即可看到干净的 UTF-8 内容。现代工具期望 UTF-8，但老项目常使用非 UTF-8 编码导致乱码，本方案在系统层面解决此问题——无需 IDE 插件、无需原地转换文件、无需手动干预。

A transparent virtual filesystem that automatically converts legacy-encoded files (GBK, Shift_JIS, Big5, etc.) to UTF-8 on read, and converts UTF-8 back to the original encoding on write. Mounts as a virtual drive so that any application sees clean UTF-8 content with zero configuration. Modern tools expect UTF-8, but legacy projects often use non-UTF-8 encodings that render as garbled text — this project solves the problem at the system level, with no IDE plugins, no file-in-place conversion, and no manual intervention.

## Quick Start

### Windows

```powershell
# 1. Install WinFsp runtime (one-time)
winget install WinFsp.WinFsp

# 2. Download encoding-vfs-Windows-x64.exe from Releases
# 3. Copy winfsp-x64.dll next to the exe:
Copy-Item "C:\Program Files (x86)\WinFsp\bin\winfsp-x64.dll" .\

# 4. Mount and go
.\encoding-vfs.exe -b C:\legacy-project -d X
```

Any read/write to `X:\` is now transparently converted between the source encoding and UTF-8.

### Linux

```bash
# 1. Install FUSE3 runtime
sudo apt-get install -y libfuse3-2 fuse3

# 2. Download encoding-vfs-Linux-x64 from Releases

# 3. Mount and go
./encoding-vfs -b /home/user/legacy-project -m /mnt/vfs
```

---

## Prerequisites

### Windows — WinFsp 2.1

Required for virtual drive mount. Install via:

```powershell
winget install WinFsp.WinFsp
```

Verify the service is running:

```powershell
Get-Service WinFsp.Launcher
# Should show Status: Running
```

### Linux — FUSE3

**Ubuntu/Debian:**

```bash
sudo apt-get install -y libfuse3-2 fuse3
```

**Fedora:**

```bash
sudo dnf install -y fuse3
```

**Arch:**

```bash
sudo pacman -S fuse3
```

---

## Build from Source

### Windows

```powershell
cargo build --release
# Copy WinFsp DLL next to the binary:
Copy-Item "C:\Program Files (x86)\WinFsp\bin\winfsp-x64.dll" target\release\
```

### Linux

```bash
# Install build deps
sudo apt-get install -y libfuse3-dev pkg-config

cargo build --release
```

---

## CLI Usage

```
encoding-vfs --help
```

| Flag | Description | Default |
|------|-------------|---------|
| `-b, --backend <DIR>` | Backend directory with original files | — |
| `-d, --drive <LETTER>` | Windows drive letter to mount | `X` |
| `-m, --mount <PATH>` | Linux FUSE mount point | `/mnt/vfs` |
| `-s, --source-encoding <ENC>` | Source encoding: `auto`, `GBK`, `Shift_JIS`, `Big5` | `auto` |
| `-t, --target-encoding <ENC>` | Encoding presented to applications | `UTF-8` |
| `-L, --log-level <LEVEL>` | Log level: trace, debug, info, warn, error | `info` |
| `-c, --config <FILE>` | Optional TOML config file | — |

### Examples

```powershell
# Auto-detect all files, mount as UTF-8
encoding-vfs.exe -b C:\legacy-project -d X

# Fixed source encoding (faster, skips per-file detection)
encoding-vfs.exe -b C:\sjis-project -d X -s Shift_JIS

# Big5 → UTF-8
encoding-vfs.exe -b C:\big5-project -d X -s Big5

# With config file (CLI overrides config values)
encoding-vfs.exe -b C:\legacy-project -d X -c encoding-vfs.toml

# CLI overrides config file
encoding-vfs.exe -b C:\legacy-project -d X -c config.toml -s Big5
```

### Unmount

```powershell
# Windows: Ctrl+C in terminal, or
net use X: /delete /y
```

```bash
# Linux: Ctrl+C in terminal, or
fusermount3 -u /mnt/vfs
```

---

## Configuration File

Create `encoding-vfs.toml` (pass via `-c`):

```toml
[backend]
backend_dir = "C:\\legacy-project"   # Windows
# backend_dir = "/home/user/legacy"  # Linux

[mount]
drive_letter = "X"
# mount_point = "/mnt/vfs"  # Linux only

[encoding]
source_encoding = "auto"
target_encoding = "UTF-8"
default_encoding = "GBK"
detect_sample_bytes = 8192
cache_max_entries = 10000
cache_ttl_seconds = 3600

[encoding.filter]
mode = "blacklist"
rules = ["*.dll", "build/"]

[log]
level = "info"
```

### All Options

| Section | Key | Description | Default |
|---------|-----|-------------|---------|
| `backend` | `backend_dir` | Directory containing original files | `.` |
| `mount` | `drive_letter` | Windows drive letter | `X` |
| `mount` | `mount_point` | Linux FUSE mount point | `/mnt/vfs` |
| `encoding` | `source_encoding` | Backend file encoding: `"auto"` per-file or fixed (`"GBK"`, `"Shift_JIS"`, etc.) | `auto` |
| `encoding` | `target_encoding` | Encoding presented to applications | `UTF-8` |
| `encoding` | `default_encoding` | Fallback when auto-detect fails | `GBK` |
| `encoding` | `detect_sample_bytes` | Bytes read for encoding detection | `8192` |
| `encoding` | `cache_max_entries` | Max encoding cache entries (LRU) | `10000` |
| `encoding` | `cache_ttl_seconds` | Cache entry TTL in seconds | `3600` |
| `encoding.filter` | `mode` | `"blacklist"` (all visible) or `"whitelist"` (all hidden) | `blacklist` |
| `encoding.filter` | `rules` | Inline glob rules (same format as `.encodingvfs-filter` file) | `[]` |
| `log` | `level` | Log verbosity | `info` |

### Encoding Presets

**Auto-detect → UTF-8 (most common):**

```toml
[encoding]
source_encoding = "auto"
target_encoding = "UTF-8"
default_encoding = "GBK"
```

**Fixed source (faster, no detection):**

```toml
[encoding]
source_encoding = "Shift_JIS"
target_encoding = "UTF-8"
```

**Mount as GBK for apps that expect it:**

```toml
[encoding]
source_encoding = "auto"
target_encoding = "GBK"
default_encoding = "GBK"
```

### Priority

CLI flags override config file, which overrides defaults.

```powershell
# config.toml says Big5, but CLI overrides to GBK
encoding-vfs.exe -b C:\legacy -d X -c config.toml -s GBK
```

---

## Filter

Control which files are visible, hidden, or bypass encoding conversion in the mounted drive.

### Sources (merged)

1. **`.encodingvfs-filter`** — place in backend directory root
2. **`[encoding.filter] rules`** — inline in TOML config

File rules load first, config rules append.

### Modes

**Blacklist (default)** — all visible, rules hide or bypass:

```
# .encodingvfs-filter
# Hide
*.dll
build/
.git/

# Bypass encoding, return raw bytes
@passthrough *.png
@passthrough *.jpg
```

**Whitelist** — all hidden, only `@allow` makes visible:

```
# .encodingvfs-filter
# Show only C/C++ sources
@allow *.h
@allow *.cpp

# But hide tests
src/test/
```

### Rule Syntax

| Rule | Effect | Example |
|------|--------|---------|
| `*.ext` | Hide matching files | `*.dll`, `*.bin` |
| `dir/` | Hide directory tree | `build/`, `.git/` |
| `**/*.tmp` | Recursive glob | hides all `.tmp` recursively |
| `@passthrough pattern` | Skip encoding, return raw bytes | `@passthrough *.png` |
| `@allow pattern` | Make visible (whitelist mode) | `@allow *.cpp` |

### Evaluation Order

1. `@passthrough` — highest priority, both modes
2. Ignore globs — hide matching files
3. `@allow` — visible (whitelist only)
4. Default — visible (blacklist) / hidden (whitelist)

### Config Examples

**Only convert `.h`/`.cpp`, hide rest:**

```toml
[encoding.filter]
mode = "whitelist"
rules = ["@allow *.h", "@allow *.hpp", "@allow *.cpp", "@allow *.c"]
```

**Skip binary assets, convert sources:**

```toml
[encoding.filter]
mode = "blacklist"
rules = [
    "@passthrough *.png",
    "@passthrough *.jpg",
    "@passthrough *.exe",
    "build/",
    "target/",
]
```

---

## How It Works

### Read (source encoding → target encoding)

```
Application reads mounted file
       │
       ▼
Platform callback → vfs.read_file()
       │
       ├── Read raw bytes from backend (e.g. GBK)
       ├── Detect encoding (BOM + heuristic, cached)
       │   └─ "auto": per-file detection
       │   └─ fixed: skip detection
       ├── Convert via encoding_rs
       └── Return converted bytes to application
```

### Write (target encoding → source encoding)

```
Application writes mounted file (UTF-8)
       │
       ▼
Platform callback → vfs.write_file()
       │
       ├── Detect existing file encoding (cached)
       ├── Convert target → source encoding
       └── Write encoded bytes to backend
```

### Encoding Detection

1. **BOM check** — UTF-8 (`EF BB BF`), UTF-16 LE/BE
2. **Content heuristic** — `encoding_rs_io` style validation
3. **Cache** — per-file encoding cached with TTL
4. **Fallback** — `default_encoding` when undetectable

---

## Supported Encodings

GBK, CP936, GB2312, GB18030, UTF-8, UTF-16LE, UTF-16BE, Big5, EUC-JP, EUC-KR, Shift_JIS, KOI8-R, Windows-1252, ISO-8859-x, IBM866, Macintosh, and more (full list from `encoding_rs`).

---

## Project Structure

```
encoding-vfs/
├── Cargo.toml
├── encoding-vfs-core/       # Cross-platform core
│   ├── config.rs            # TOML config
│   ├── encoding.rs          # Encoding conversion
│   ├── detector.rs          # BOM + heuristic detection
│   ├── cache.rs             # LRU cache with TTL
│   ├── vfs.rs               # Core read/write/dir
│   └── filter.rs            # Glob-based path filter
├── encoding-vfs-windows/    # WinFsp adapter
│   └── winfsp_host.rs       # FileSystemContext impl
├── encoding-vfs-linux/      # FUSE adapter
│   └── fuse_host.rs         # fuser Filesystem impl
├── encoding-vfs-cli/        # Unified CLI binary
│   └── main.rs
└── .github/workflows/
    └── release.yml          # CI build + release
```

## Build Details

| Crate | Role | Key Dependencies |
|-------|------|-----------------|
| `encoding-vfs-core` | Detection, conversion, cache, VFS | `encoding_rs`, `dashmap`, `globset`, `toml`, `serde` |
| `encoding-vfs-windows` | WinFsp 2.1 drive mount | `winfsp 0.12`, `windows 0.61`, `encoding-vfs-core` |
| `encoding-vfs-linux` | FUSE filesystem mount | `fuser 0.14`, `libc`, `encoding-vfs-core` |
| `encoding-vfs-cli` | CLI binary, platform dispatch | `clap 4.4`, `tracing-subscriber` |

### WinFsp Notes

- Uses `winfsp` crate (v0.12.6+winfsp-2.1) with `FileSystemContext` trait
- Standard install: `winget install WinFsp.WinFsp`
- SxS installation supported for custom builds
- Security: returns null security descriptor, WinFsp applies defaults

### FUSE Notes

- Uses `fuser` crate (v0.14) with `Filesystem` trait
- Build requires `libfuse3-dev` + `pkg-config`
- Runtime requires `/dev/fuse` and `fusermount`/`fusermount3`
- For multi-user access, uncomment `user_allow_other` in `/etc/fuse.conf`
