[English](README.md) | [中文](README_zh.md)

# encoding-vfs

透明虚拟文件系统，自动将遗留编码文件（GBK、Shift_JIS、Big5 等）转换为 UTF-8 以供读取，并在写入时将 UTF-8 转换回原始编码。挂载为虚拟驱动器后，任何应用无需任何配置即可看到干净的 UTF-8 内容。

A transparent virtual filesystem that automatically converts legacy-encoded files (GBK, Shift_JIS, Big5, etc.) to UTF-8 on read, and converts UTF-8 back to the original encoding on write. Mounts as a virtual drive so that any application sees clean UTF-8 content with zero configuration.

## 问题

现代工具期望 UTF-8。但老项目常使用 GBK、Shift_JIS、Big5 等编码，显示为乱码或替换字符。本方案在**系统层面**解决——无需 IDE 插件、无需原地转换文件、无需手动干预。

## 快速上手

### Windows

```powershell
# 1. 安装 WinFsp 运行环境（一次性）
winget install WinFsp.WinFsp

# 2. 从 Releases 下载 encoding-vfs-Windows-x64.exe
# 3. 拷贝 winfsp-x64.dll 到 exe 同目录:
Copy-Item "C:\Program Files (x86)\WinFsp\bin\winfsp-x64.dll" .\

# 4. 挂载即可
.\encoding-vfs.exe -b C:\legacy-project -d X
```

对 `X:\` 的所有读写都会自动在源编码和 UTF-8 之间透明转换。

### Linux

```bash
# 1. 安装 FUSE3 运行环境
sudo apt-get install -y libfuse3-2 fuse3

# 2. 从 Releases 下载 encoding-vfs-Linux-x64

# 3. 挂载即可
./encoding-vfs -b /home/user/legacy-project -m /mnt/vfs
```

---

## 前置依赖

### Windows — WinFsp 2.1

虚拟驱动器挂载必需：

```powershell
winget install WinFsp.WinFsp
```

验证服务是否运行：

```powershell
Get-Service WinFsp.Launcher
# 应显示 Status: Running
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

## 从源码编译

### Windows

```powershell
cargo build --release
# 拷贝 WinFsp DLL 到二进制目录:
Copy-Item "C:\Program Files (x86)\WinFsp\bin\winfsp-x64.dll" target\release\
```

### Linux

```bash
# 安装编译依赖
sudo apt-get install -y libfuse3-dev pkg-config

cargo build --release
```

---

## 命令行用法

```
encoding-vfs --help
```

| 参数 | 说明 | 默认值 |
|------|------|--------|
| `-b, --backend <DIR>` | 后端源文件目录 | — |
| `-d, --drive <LETTER>` | Windows 挂载盘符 | `X` |
| `-m, --mount <PATH>` | Linux FUSE 挂载点 | `/mnt/vfs` |
| `-s, --source-encoding <ENC>` | 源编码：`auto`、`GBK`、`Shift_JIS`、`Big5` | `auto` |
| `-t, --target-encoding <ENC>` | 呈现给应用的编码 | `UTF-8` |
| `-L, --log-level <LEVEL>` | 日志级别 | `info` |
| `-c, --config <FILE>` | 可选 TOML 配置文件 | — |

### 示例

```powershell
# 自动检测 → UTF-8
encoding-vfs.exe -b C:\legacy-project -d X

# 固定源编码（更快，跳过检测）
encoding-vfs.exe -b C:\sjis-project -d X -s Shift_JIS

# Big5 → UTF-8
encoding-vfs.exe -b C:\big5-project -d X -s Big5

# 使用配置文件
encoding-vfs.exe -b C:\legacy-project -d X -c encoding-vfs.toml

# 命令行覆盖配置文件
encoding-vfs.exe -b C:\legacy-project -d X -c config.toml -s Big5
```

### 卸载

```powershell
# Windows: 终端按 Ctrl+C，或
net use X: /delete /y
```

```bash
# Linux: 终端按 Ctrl+C，或
fusermount3 -u /mnt/vfs
```

---

## 配置文件

创建 `encoding-vfs.toml`（通过 `-c` 传入）：

```toml
[backend]
backend_dir = "C:\\legacy-project"    # Windows
# backend_dir = "/home/user/legacy"   # Linux

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

### 配置项详解

| 节 | 键 | 说明 | 默认值 |
|---------|-----|------|--------|
| `backend` | `backend_dir` | 原始文件所在目录 | `.` |
| `mount` | `drive_letter` | Windows 盘符 | `X` |
| `mount` | `mount_point` | Linux FUSE 挂载点 | `/mnt/vfs` |
| `encoding` | `source_encoding` | 后端编码：`"auto"` 逐文件检测，或固定编码 | `auto` |
| `encoding` | `target_encoding` | 呈现给应用的编码 | `UTF-8` |
| `encoding` | `default_encoding` | 自动检测失败时的回退 | `GBK` |
| `encoding` | `detect_sample_bytes` | 编码检测读取的字节数 | `8192` |
| `encoding` | `cache_max_entries` | 编码缓存最大条目数（LRU） | `10000` |
| `encoding` | `cache_ttl_seconds` | 缓存过期时间（秒） | `3600` |
| `encoding.filter` | `mode` | 过滤模式：`"blacklist"`（全可见）或 `"whitelist"`（全隐藏） | `blacklist` |
| `encoding.filter` | `rules` | 内联 glob 规则 | `[]` |
| `log` | `level` | 日志级别 | `info` |

### 编码配置示例

**自动检测 → UTF-8（最常用）：**

```toml
[encoding]
source_encoding = "auto"
target_encoding = "UTF-8"
default_encoding = "GBK"
```

**固定源编码（更快）：**

```toml
[encoding]
source_encoding = "Shift_JIS"
target_encoding = "UTF-8"
```

**挂载为 GBK（适配需要 GBK 的应用）：**

```toml
[encoding]
source_encoding = "auto"
target_encoding = "GBK"
default_encoding = "GBK"
```

### 优先级

命令行 > 配置文件 > 默认值：

```powershell
# 配置文件写的是 Big5，命令行覆盖为 GBK
encoding-vfs.exe -b C:\legacy -d X -c config.toml -s GBK
```

---

## 过滤器

控制挂载时哪些文件可见、哪些隐藏、哪些跳过编码转换。

### 规则来源（两处合并）

1. **`.encodingvfs-filter`** — 放在 backend 目录根下
2. **`[encoding.filter] rules`** — TOML 配置中的内联规则

文件规则先加载，配置规则后追加。

### 两种模式

**黑名单（默认）** — 默认全部可见，规则标记隐藏或跳过编码：

```
# .encodingvfs-filter
# 隐藏
*.dll
build/
.git/

# 跳过编码，直接返回原始字节
@passthrough *.png
@passthrough *.jpg
```

**白名单** — 默认全部隐藏，`@allow` 标记才可见：

```
# .encodingvfs-filter
# 只显示 C/C++ 源码
@allow *.h
@allow *.cpp

# 但隐藏测试文件
src/test/
```

### 规则语法

| 规则 | 效果 | 示例 |
|------|------|------|
| `*.ext` | 隐藏匹配的后缀 | `*.dll`、`*.bin` |
| `dir/` | 隐藏目录树 | `build/`、`.git/` |
| `**/*.tmp` | 递归 glob | 隐藏所有 `.tmp` |
| `@passthrough pattern` | 跳过编码转换，返回原始字节 | `@passthrough *.png` |
| `@allow pattern` | 白名单模式下可见 | `@allow *.cpp` |

### 优先级

1. `@passthrough` — 最高优先级
2. 忽略规则（普通 glob）
3. `@allow` — 白名单模式才有效
4. 默认行为 — 黑名单可见 / 白名单隐藏

### 配置示例

**只转换 `.h`/`.cpp`，隐藏其他：**

```toml
[encoding.filter]
mode = "whitelist"
rules = ["@allow *.h", "@allow *.hpp", "@allow *.cpp", "@allow *.c"]
```

**跳过二进制资源，转换源码：**

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

## 工作原理

### 读取（源编码 → 目标编码）

```
应用读取挂载文件
       │
       ▼
平台回调 → vfs.read_file()
       │
       ├── 从后端读取原始字节（如 GBK）
       ├── 检测编码（BOM + 启发式，已缓存）
       │   └─ "auto": 逐文件检测
       │   └─ 固定编码: 跳过检测
       ├── encoding_rs 转换
       └── 返回转换后字节给应用
```

### 写入（目标编码 → 源编码）

```
应用写入挂载文件（UTF-8）
       │
       ▼
平台回调 → vfs.write_file()
       │
       ├── 检测文件现有编码（缓存）
       ├── 从目标编码转为源编码
       └── 写入编码后字节到后端
```

### 编码检测

1. **BOM 检测** — UTF-8（`EF BB BF`）、UTF-16 LE/BE
2. **内容启发式** — `encoding_rs_io` 风格验证
3. **缓存** — 每个文件的编码带 TTL 缓存
4. **回退** — 无法检测时使用 `default_encoding`

---

## 支持的编码

GBK、CP936、GB2312、GB18030、UTF-8、UTF-16LE、UTF-16BE、Big5、EUC-JP、EUC-KR、Shift_JIS、KOI8-R、Windows-1252、ISO-8859-x、IBM866、Macintosh 等（完整列表见 `encoding_rs`）。

---

## 项目结构

```
encoding-vfs/
├── Cargo.toml
├── encoding-vfs-core/       # 跨平台核心
│   ├── config.rs            # TOML 配置
│   ├── encoding.rs          # 编码转换
│   ├── detector.rs          # BOM + 启发式检测
│   ├── cache.rs             # LRU 缓存 + TTL
│   ├── vfs.rs               # 核心读写/目录
│   └── filter.rs            # Glob 路径过滤器
├── encoding-vfs-windows/    # WinFsp 适配器
│   └── winfsp_host.rs       # FileSystemContext 实现
├── encoding-vfs-linux/      # FUSE 适配器
│   └── fuse_host.rs         # fuser Filesystem 实现
├── encoding-vfs-cli/        # 统一 CLI 二进制
│   └── main.rs
└── .github/workflows/
    └── release.yml          # CI 构建 + 发布
```

## 编译详情

| Crate | 职责 | 核心依赖 |
|-------|------|---------|
| `encoding-vfs-core` | 编码检测、转换、缓存、VFS | `encoding_rs`, `dashmap`, `globset`, `toml`, `serde` |
| `encoding-vfs-windows` | WinFsp 2.1 虚拟驱动器 | `winfsp 0.12`, `windows 0.61`, `encoding-vfs-core` |
| `encoding-vfs-linux` | FUSE 文件系统 | `fuser 0.14`, `libc`, `encoding-vfs-core` |
| `encoding-vfs-cli` | CLI 入口，平台分发 | `clap 4.4`, `tracing-subscriber` |

### WinFsp 注意事项

- 使用 `winfsp` crate（v0.12.6+winfsp-2.1）`FileSystemContext` trait
- 推荐 `winget install WinFsp.WinFsp` 安装
- 支持 SxS（并排）安装，自定义构建可使用独立驱动名
- 安全：返回空安全描述符，由 WinFsp 应用默认策略

### FUSE 注意事项

- 使用 `fuser` crate（v0.14）`Filesystem` trait
- 编译需要 `libfuse3-dev` + `pkg-config`
- 运行需要 `/dev/fuse` 和 `fusermount`/`fusermount3`
- 多用户访问：取消 `/etc/fuse.conf` 中 `user_allow_other` 的注释
