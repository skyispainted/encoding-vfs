[English](README.md) | [中文](README_zh.md)

# encoding-vfs

透明虚拟文件系统，自动将遗留编码文件转换为 UTF-8 以供读取，并在写入时将 UTF-8 转换回原始编码。挂载为虚拟驱动器后，任何应用（包括 Claude Code、VS Code、`cat` 等）都能看到干净的 UTF-8 内容，无需任何特殊配置。

## 问题

现代工具期望 UTF-8。但老项目常使用 GBK、Shift_JIS、Big5 等编码，显示为乱码或替换字符。本方案在**系统层面**解决——无需 IDE 插件、无需原地转换文件、无需手动干预。

## 架构

```
┌───────────────────────────────────────────────┐
│           encoding-vfs-cli (入口)              │
│         clap → config → 平台挂载              │
├───────────────────────────────────────────────┤
│         encoding-vfs-core (共享)               │
│  config  encoding  detector  cache  vfs error  │
├───────────────────────────────────────────────┤
│  encoding-vfs-windows    │  encoding-vfs-linux │
│  WinFsp 2.1 虚拟驱动器    │  fuser FUSE 挂载    │
│  FileSystemContext trait │  Filesystem trait   │
└───────────────────────────────────────────────┘
```

---

## Windows (WinFsp)

### 前提条件

#### 安装 WinFsp 2.1 运行环境

本工具需要 WinFsp 2.1 来挂载虚拟驱动器：

**方法一：winget（推荐）**
```powershell
winget install WinFsp.WinFsp
```

**方法二：手动下载**

从 [WinFsp releases](https://github.com/winfsp/winfsp/releases) 下载安装包，运行 `winfsp-*.msi`。

验证服务是否运行：
```powershell
Get-Service WinFsp.Launcher
# 应显示 Status: Running
```

### 快速开始 — 预编译二进制

从 [Releases](https://github.com/skyispainted/encoding-vfs/releases) 下载最新版本：

```powershell
# 下载 encoding-vfs-Windows-x64-vX.Y.Z，解压到任意目录
# 直接运行：
.\encoding-vfs.exe -b C:\projects\original -d X
```

### 从源码编译

```powershell
cargo build --release --features mount
```

### 运行

```powershell
# 挂载虚拟驱动器 Y:，后端目录为 C:\projects\original
.\target\release\encoding-vfs.exe -b C:\projects\original -d Y
```

就这样。任何对 `Y:\` 的读写都会自动在源编码和 UTF-8 之间进行透明转换。

### 卸载

两种方式，均可优雅退出：

```powershell
# 方式一：在终端按 Ctrl+C

# 方式二：从任意终端
net use Y: /delete /y
```

---

## Linux (FUSE)

### 前提条件

#### 安装 FUSE3 运行时

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

### 快速开始 — 预编译二进制

从 [Releases](https://github.com/skyispainted/encoding-vfs/releases) 下载最新版本：

```bash
# 下载后解压
tar xzf encoding-vfs-Linux-x64-vX.Y.Z.tar.gz
cd encoding-vfs-Linux-x64-vX.Y.Z

# 直接运行：
./encoding-vfs -b /home/user/legacy-project -m /mnt/gbk-vfs
```

### 从源码编译

编译前置依赖：

**Ubuntu/Debian:**
```bash
sudo apt-get install -y libfuse3-dev pkg-config
```

```bash
cargo build --release
```

### 运行

```bash
# 挂载 FUSE 文件系统到 /mnt/gbk-vfs
./target/release/encoding-vfs -b /path/to/legacy-project -m /mnt/gbk-vfs
```

所有对 `/mnt/gbk-vfs/` 的读写都会自动进行编码转换。

### 卸载

```bash
# 方式一：在终端按 Ctrl+C（自动卸载）

# 方式二：从另一个终端
fusermount3 -u /mnt/gbk-vfs
# 或：fusermount -u /mnt/gbk-vfs  （旧系统）
```

### 常见问题

**"option allow_other only allowed if 'user_allow_other' is set in /etc/fuse.conf"**

取消 `/etc/fuse.conf` 中该行的注释：
```bash
sudo sed -i 's/^#user_allow_other/user_allow_other/' /etc/fuse.conf
```

---

## 命令行用法

```
encoding-vfs --help
```

| 参数 | 说明 | 默认值 (Win) | 默认值 (Linux) |
|------|------|-------------|---------------|
| `-b, --backend <DIR>` | 源文件目录路径 | — | — |
| `-d, --drive <LETTER>` | Windows 驱动器盘符 | `X` | — |
| `-m, --mount <PATH>` | Linux FUSE 挂载点 | — | `/mnt/gbk-vfs` |
| `-s, --source-encoding <ENC>` | 源编码：`auto`、`GBK`、`Shift_JIS`、`Big5` 等 | `auto` | `auto` |
| `-t, --target-encoding <ENC>` | 挂载后呈现给应用的编码 | `UTF-8` | `UTF-8` |
| `-L, --log-level <LEVEL>` | 日志级别：trace、debug、info、warn、error | `info` | `info` |
| `-c, --config <FILE>` | 可选的 TOML 配置文件 | — | — |

### 示例

**Windows:**
```powershell
# 基本挂载：自动检测源编码 → UTF-8
encoding-vfs.exe -b C:\legacy-project -d X

# 固定源编码（更快，跳过检测）
encoding-vfs.exe -b C:\sjis-project -d X -s Shift_JIS

# Big5 → UTF-8
encoding-vfs.exe -b C:\big5-project -d X -s Big5

# 带配置文件（命令行覆盖配置值）
encoding-vfs.exe -b C:\legacy-project -d X -c encoding-vfs.toml

# 命令行覆盖配置文件
encoding-vfs.exe -b C:\legacy-project -d X -c config.toml -s Big5
```

**Linux:**
```bash
# 基本挂载
./encoding-vfs -b /home/user/legacy-project -m /mnt/gbk-vfs

# 固定源编码
./encoding-vfs -b /home/user/sjis-project -m /mnt/gbk-vfs -s Shift_JIS

# 带配置文件
./encoding-vfs -b /home/user/legacy-project -m /mnt/gbk-vfs -c encoding-vfs.toml
```

## 配置文件

创建 `encoding-vfs.toml`：

```toml
[backend]
backend_dir = "C:\\projects\\original"   # Windows
# backend_dir = "/home/user/legacy-project"  # Linux

[mount]
drive_letter = "X"       # Windows: 驱动器盘符
# mount_point = "/mnt/gbk-vfs"  # Linux: 挂载点（可选）

[encoding]
source_encoding = "auto"        # "auto" | "GBK" | "Shift_JIS" | "Big5" | ...
target_encoding = "UTF-8"       # "UTF-8" | "GBK" | ...
default_encoding = "GBK"        # 自动检测失败时的回退
detect_sample_bytes = 8192
cache_max_entries = 10000
cache_ttl_seconds = 3600

[encoding.filter]
mode = "blacklist"              # "blacklist"（默认）或 "whitelist"
rules = ["*.dll", "logs/"]      # 内联规则，格式同过滤器文件

[log]
level = "info"
```

### 配置项

| 节 | 键 | 说明 | 默认值 |
|---------|-----|-------------|---------|
| `backend` | `backend_dir` | 包含原始文件的目录 | `.` |
| `mount` | `drive_letter` | Windows 驱动器盘符 | `X` |
| `mount` | `mount_point` | Linux FUSE 挂载点 | `/mnt/gbk-vfs` |
| `encoding` | `source_encoding` | 源编码（`auto` 为自动检测） | `auto` |
| `encoding` | `target_encoding` | 呈现给应用的编码 | `UTF-8` |
| `encoding` | `default_encoding` | 自动检测失败时的回退 | `GBK` |
| `encoding` | `detect_sample_bytes` | 编码检测读取的字节数 | `8192` |
| `encoding` | `cache_max_entries` | 编码缓存最大条目数 | `10000` |
| `encoding` | `cache_ttl_seconds` | 缓存条目过期时间 | `3600` |
| `log` | `level` | 日志级别 | `info` |

## 过滤器

在 backend 目录根下创建 `.encodingvfs-filter` 文件，控制哪些文件可见、编码转换或隐藏。

### 两种模式

**黑名单（默认）**：默认所有文件可见，规则标记的文件被隐藏或不经编码转换。

```
# 注释行
*.dll                  # 所有 .dll 文件不可见
logs/                  # logs/ 目录不可见
src/test/              # 排除 src/test/ 下的文件

# 不经编码转换，原样返回
@passthrough *.png
@passthrough *.jpg
```

**白名单**：默认所有文件隐藏，`@allow` 规则标记的文件才可见。

```
# 只显示 C/C++ 源文件和头文件
@allow *.h
@allow *.cpp
@allow *.c
@allow *.hpp

# 排除子目录
src/test/
```

### 配置文件中使用

```toml
[encoding.filter]
mode = "whitelist"
rules = ["@allow *.h", "@allow *.cpp"]
```

### 优先级

`@passthrough` > 黑名单规则 > `@allow` > 默认行为

## 工作原理

### 读取流程（源编码 → 目标编码）

```
应用读取挂载文件
       │
       ▼
平台回调 (WinFsp/FUSE) → vfs.read_file()
       │
       ├── 从后端读取原始字节（如 GBK）
       ├── 检测编码（BOM + 内容启发式，已缓存）
       │   └─ auto 模式：启发式检测
       │   └─ 固定编码：跳过检测
       ├── 通过 encoding_rs 转换为目标编码
       └── 返回目标编码字节给应用
```

### 写入流程（目标编码 → 源编码）

```
应用写入挂载文件（目标编码，如 UTF-8）
       │
       ▼
平台回调 (WinFsp/FUSE) → vfs.write_file()
       │
       ├── 检测现有文件编码（缓存）
       ├── 从目标编码转换为源编码
       └── 写入编码后的字节到后端
```

### 编码检测

1. **BOM 检测** — UTF-8 BOM（`EF BB BF`）、UTF-16 LE/BE BOM
2. **内容启发式** — `encoding_rs_io` 风格验证
3. **缓存** — 每个文件的编码带 TTL 缓存，避免重复扫描
4. **回退** — 无法检测时使用 `default_encoding`

## 支持的编码

GBK、CP936、GB2312、GB18030、UTF-8、UTF-16LE、UTF-16BE、Big5、EUC-JP、EUC-KR、Shift_JIS、KOI8-R、Windows-1252、ISO-8859-x、IBM866、Macintosh 等（完整列表见 `encoding_rs`）。

## 项目结构

```
encoding-vfs/
├── Cargo.toml                          # workspace 根
├── encoding-vfs-core/                  # 跨平台核心
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                      # 公共导出
│       ├── config.rs                   # TOML 配置 + 默认值
│       ├── encoding.rs                 # 源 ↔ 目标编码转换
│       ├── detector.rs                 # BOM + 启发式编码检测
│       ├── cache.rs                    # 线程安全 LRU 缓存 + TTL
│       ├── vfs.rs                      # EncodingVfs：核心读写/目录
│       └── error.rs                    # 统一错误类型
├── encoding-vfs-windows/               # Windows WinFsp 适配器
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                      # feature 门控导出
│       └── winfsp_host.rs              # FileSystemContext trait + run()
├── encoding-vfs-linux/                 # Linux FUSE 适配器
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                      # 公共导出
│       └── fuse_host.rs                # fuser::Filesystem trait + run()
├── encoding-vfs-cli/                   # 统一 CLI 入口
│   ├── Cargo.toml
│   └── src/
│       └── main.rs                     # clap → 平台分发
└── .github/workflows/
    └── release.yml                     # CI 构建 + 发布（Win + Linux）
```

## 编译详情

### Workspace Crates

| Crate | 角色 | 核心依赖 |
|-------|------|---------|
| `encoding-vfs-core` | 编码检测、转换、缓存、VFS | `encoding_rs`, `encoding_rs_io`, `dashmap`, `toml`, `serde`, `thiserror`, `tracing` |
| `encoding-vfs-windows` | WinFsp 2.1 虚拟驱动器挂载 | `winfsp 0.12`, `widestring 1.0`, `windows 0.61`, `encoding-vfs-core` |
| `encoding-vfs-linux` | Linux FUSE 文件系统挂载 | `fuser 0.14`, `libc 0.2`, `encoding-vfs-core` |
| `encoding-vfs-cli` | 统一 CLI 二进制，平台分发 | `clap 4.4`, `tracing-subscriber`, 平台 crate |

### Feature Flags

| Feature | 平台 | 说明 |
|---------|------|------|
| `mount` | Windows | 启用 WinFsp 虚拟驱动器挂载（需要 `--features mount`） |

Linux 上 FUSE 适配器始终包含（无需 feature flag）。

### WinFsp 注意事项

- 使用 **winfsp** crate（v0.12.6+winfsp-2.1），`FileSystemContext` trait
- 推荐 `winget install WinFsp.WinFsp` 标准安装
- 支持 SxS（并排）安装，自定义构建可使用独立驱动名
- 安全：返回空安全描述符，由 WinFsp 应用默认安全策略
- 已验证：目录列表、文件创建、读写均正常工作

### FUSE 注意事项

- 使用 **fuser** crate（v0.14），`Filesystem` trait
- 编译时需要 `libfuse3-dev` 和 `pkg-config`
- 运行时需要 `/dev/fuse` 设备和 `fusermount`/`fusermount3`
- 如果其他用户需要访问，取消注释 `/etc/fuse.conf` 中的 `user_allow_other`
