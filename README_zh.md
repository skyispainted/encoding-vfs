[English](README.md) | [中文](README_zh.md)

# encoding-vfs

现代开发工具默认使用 UTF-8，但老项目中的文件常使用 GBK、Shift_JIS、Big5 等编码，在 IDE、终端和 AI 编码工具中显示为乱码或无法解析。encoding-vfs 通过挂载虚拟驱动器在系统层面透明解决此问题：

- **读取自动转换** — 将遗留编码文件自动转为 UTF-8，应用看到干净的文本
- **写入自动还原** — 将 UTF-8 内容转回文件原始编码写入磁盘
- **零侵入** — 无需修改文件、无需 IDE 插件、无需手动配置
- **任意应用可用** — 挂载后 Claude Code、VS Code、`cat` 等均可直接使用

Modern tools default to UTF-8, but legacy project files often use GBK, Shift_JIS, Big5, or other encodings that render as garbled text or fail to parse in IDEs, terminals, and AI coding tools. encoding-vfs solves this transparently by mounting a virtual drive at the system level:

- **Automatic read conversion** — legacy-encoded files appear as clean UTF-8 to any application
- **Automatic write restore** — UTF-8 content is converted back to the file's original encoding on disk
- **Zero intrusion** — no file modification, no IDE plugins, no manual configuration needed
- **Works with any app** — Claude Code, VS Code, `cat`, and any other tool works directly on the mounted drive

Modern tools default to UTF-8, but legacy project files often use GBK, Shift_JIS, Big5, or other encodings that render as garbled text or fail to parse in IDEs, terminals, and AI coding tools. encoding-vfs solves this transparently by mounting a virtual drive at the system level.

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
| `encoding.filter` | `rules` | `@passthrough` 规则 | `[]` |
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

控制哪些文件跳过编码转换，挂载后所有文件都可见。

规则遵循 `.gitignore` 格式和语义。

### 规则来源（两处合并）

1. **`.evfsignore`** — 放在 backend 目录根下
2. **`[encoding.filter] rules`** — TOML 配置中的内联规则

文件规则先加载，配置规则后追加。

### 规则语法

| 规则 | 效果 | 示例 |
|------|------|------|
| `*.ext` | 跳过编码，返回原始字节 | `*.png`, `*.exe` |
| `dir/` | 跳过整个目录树 | `assets/`, `lib/` |
| `**/*.tmp` | 递归匹配 | 所有 `.tmp` 文件 |
| `!pattern` | 取反：恢复编码转换 | `!logo.png` |

### 匹配规则

- 规则**按顺序**匹配，**最后一条匹配的规则决定结果**。
- 默认（不匹配任何规则）：正常编码转换。
- `!pattern` 取消之前的匹配 — 为该文件恢复编码转换。

### 配置示例

**跳过二进制资源，转换其他：**

```
# .evfsignore
*.png
*.jpg
*.exe
```

**只转换 `.h`/`.cpp`，其他全部返回原始字节：**

```
# .evfsignore
**/*
!*.h
!*.cpp
```

或者在 TOML 中：

```toml
[encoding.filter]
rules = ["**/*", "!*.h", "!*.cpp"]
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
