# encoding-vfs

**A transparent encoding conversion virtual filesystem. Mount a drive where legacy-encoded files (GBK, Shift_JIS, Big5...) appear as UTF-8 to any application.**

**透明的编码转换虚拟文件系统。挂载一个虚拟磁盘，让遗留编码文件（GBK、Shift_JIS、Big5...）在任何应用中显示为 UTF-8。**

---

# 中文

## 这是什么？

你有一个老项目，代码文件用 GBK 编码。你想用 VSCode、Claude Code 或其他现代工具来编辑，但它们都默认 UTF-8，打开文件就是乱码。

encoding-vfs 解决这个问题的方式很简单：

1. 你有一个 GBK 编码的项目目录，比如 `C:\projects\my-legacy-project`
2. 运行 encoding-vfs，把它挂载到 `Y:` 盘
3. 打开 `Y:\`，所有文件自动显示为 UTF-8
4. 用任何工具打开、编辑、保存 —— 编码转换完全透明
5. 保存时自动转换回 GBK，原目录里的文件保持 GBK 编码不变

## 使用场景

### 场景 1：用 VSCode 编辑 GBK 项目

```powershell
# 挂载
.\encoding-vfs.exe -b C:\projects\my-legacy-project -d Y

# 然后用 VSCode 打开 Y:\ 目录
code Y:\
```

在 VSCode 里正常打开、编辑、保存文件。VSCode 看到的是 UTF-8，实际磁盘上存的是 GBK。

### 场景 2：用 Claude Code / AI 工具处理 GBK 项目

```powershell
# 挂载
.\encoding-vfs.exe -b C:\projects\my-legacy-project -d Y

# 在 Y:\ 目录下启动 Claude Code
cd Y:\
claude
```

AI 工具读到的文件都是 UTF-8，写回去自动转成 GBK。

### 场景 3：在 GBK 项目里用 git

```powershell
# 挂载
.\encoding-vfs.exe -b C:\projects\my-legacy-project -d Y

# 设置 git wrapper（让 git 命令自动映射到源目录）
set PATH=%CD%;%PATH%

# 在 Y:\ 下正常使用 git
cd Y:\
git status
git add .
git commit -m "fix: update"
git push
```

git wrapper 会自动把 git 命令映射到源目录执行，保证 `.git` 操作在正确的地方。

## 安装步骤

### Windows

**第 1 步：安装 WinFsp（必需）**

encoding-vfs 依赖 WinFsp 来挂载虚拟磁盘。

```powershell
winget install WinFsp.WinFsp
```

或者从 https://winfsp.dev/releases/ 下载安装包。

**第 2 步：下载 encoding-vfs**

从 [Releases](https://github.com/skyispainted/encoding-vfs/releases) 下载最新版本的 zip 包，解压到任意目录。

**第 3 步：复制 WinFsp DLL**

```powershell
copy "C:\Program Files (x86)\WinFsp\bin\winfsp-x64.dll" .\
```

把 `winfsp-x64.dll` 复制到 encoding-vfs.exe 所在的目录。

### Linux

```bash
# 安装 FUSE3
sudo apt-get install -y libfuse3-2 fuse3
```

从 Releases 下载 Linux 版本解压即可。

## 基本用法

### Windows

```powershell
# 挂载项目到 Y: 盘
.\encoding-vfs.exe -b C:\projects\my-legacy-project -d Y

# 现在可以访问 Y:\
dir Y:\
type Y:\src\main.cpp
```

挂载后，encoding-vfs 会一直运行。按 `Ctrl+C` 停止并卸载。

### Linux

```bash
# 挂载项目到 /mnt/vfs
./encoding-vfs -b /home/user/my-legacy-project -m /mnt/vfs

# 现在可以访问 /mnt/vfs
ls /mnt/vfs
cat /mnt/vfs/src/main.cpp
```

卸载：`fusermount -u /mnt/vfs` 或按 `Ctrl+C`。

## 命令行参数

| 参数 | 说明 | 默认值 |
|------|------|--------|
| `-b, --backend <目录>` | 源项目目录（存放原始编码文件的地方） | **必填** |
| `-d, --drive <盘符>` | Windows 挂载盘符 | `X` |
| `-m, --mount <路径>` | Linux 挂载点 | `/mnt/vfs` |
| `-s, --source-encoding <编码>` | 源文件编码。`auto` 使用默认编码，也可指定具体编码名 | `auto` |
| `-t, --target-encoding <编码>` | 挂载后显示的编码 | `UTF-8` |
| `-c, --config <文件>` | 配置文件路径 | 无 |
| `-L, --log-level <级别>` | 日志级别：trace/debug/info/warn/error | `info` |

### 支持的编码

GBK, UTF-8, Big5, Shift_JIS, EUC-JP, EUC-KR, ISO-2022-JP, KOI8-R, Windows-1252, UTF-16LE, UTF-16BE 等。

`-s auto` 模式下，实际使用 `-t` 的默认编码（通常是 GBK）作为源编码。因为自动检测 GBK 和 UTF-8 不可靠，建议明确指定。

## 配置文件

可以创建 TOML 配置文件代替命令行参数：

```toml
[backend]
backend_dir = "C:\\projects\\my-legacy-project"

[mount]
drive_letter = "Y"

[encoding]
source_encoding = "GBK"
target_encoding = "UTF-8"
default_encoding = "GBK"

[encoding.filter]
# 不做编码转换的文件（直接返回原始字节），.gitignore 语法
rules = ["*.png", "*.exe", "*.dll", "*.jpg"]
# 完全隐藏的文件/目录
hidden = [".git/", "node_modules/", "*.log"]

[log]
level = "info"
# file = "vfs.log"  # 可选：输出到文件
```

使用：

```powershell
.\encoding-vfs.exe -c config.toml
```

### 过滤规则说明

- `rules`：匹配的文件不做编码转换，直接返回原始字节。适合二进制文件。
- `hidden`：匹配的文件/目录在挂载点完全不可见。
- 语法同 `.gitignore`：`*.png` 匹配所有 png，`dir/` 匹配整个目录，`!pattern` 取反。
- `.git/` 默认隐藏。

## Git 透明支持

在挂载目录下使用 git 时，需要设置 git wrapper：

```powershell
# Windows CMD（当前会话）
set PATH=%CD%;%PATH%

# Windows PowerShell（当前会话）
$env:PATH = "$PWD;$env:PATH"

# 永久生效
setx PATH "%CD%;%PATH%"
```

设置后，在挂载目录执行 `git status`、`git commit` 等命令时，git wrapper 会自动：
1. 识别当前目录对应的源项目
2. 把路径映射回源目录
3. 在源目录执行真正的 git 命令

这样 `.git` 操作总是在正确的位置执行，不会因为编码转换出问题。

## 常见问题

### Q: 挂载后文件能正常打开，但保存后原文件损坏？

A: 请确保下载的是最新版本（v0.1.10+）。旧版本有分块读取时编码转换的 bug。

### Q: 为什么用 `-s auto` 而不是自动检测编码？

A: 自动检测 GBK 和 UTF-8 非常不可靠，两者有大量重叠的字节序列。建议明确指定源编码。如果你的项目混合了多种编码，目前不支持，需要统一编码后再使用。

### Q: WinFsp 是什么？必须安装吗？

A: WinFsp 是 Windows 文件系统代理框架，encoding-vfs 用它来实现虚拟磁盘。Windows 上必须安装。

### Q: 挂载后性能怎么样？

A: 对于大文件（>500KB），每次读取会完整转换整个文件并缓存。首次读取稍慢，后续读取很快。写入时也是完整转换后保存。日常使用不会感到明显延迟。

### Q: 能同时挂载多个项目吗？

A: 可以。每个项目用不同的盘符：

```powershell
.\encoding-vfs.exe -b C:\projects\project1 -d Y
.\encoding-vfs.exe -b C:\projects\project2 -d Z
```

---

# English

## What is this?

You have a legacy project with source files encoded in GBK. You want to use VSCode, Claude Code, or other modern tools, but they all default to UTF-8 and show garbled text.

encoding-vfs solves this simply:

1. You have a GBK-encoded project directory, e.g., `C:\projects\my-legacy-project`
2. Run encoding-vfs to mount it as `Y:` drive
3. Open `Y:\` — all files appear as UTF-8 automatically
4. Open, edit, save with any tool — encoding conversion is fully transparent
5. On save, content is converted back to GBK — original files stay GBK-encoded

## Use Cases

### Use Case 1: Edit GBK project with VSCode

```powershell
# Mount
.\encoding-vfs.exe -b C:\projects\my-legacy-project -d Y

# Open Y:\ in VSCode
code Y:\
```

Edit and save files normally in VSCode. VSCode sees UTF-8; the disk stores GBK.

### Use Case 2: Use Claude Code / AI tools on GBK project

```powershell
# Mount
.\encoding-vfs.exe -b C:\projects\my-legacy-project -d Y

# Start Claude Code in Y:\
cd Y:\
claude
```

AI tools read files as UTF-8; writes are converted back to GBK automatically.

### Use Case 3: Use git in GBK project

```powershell
# Mount
.\encoding-vfs.exe -b C:\projects\my-legacy-project -d Y

# Set up git wrapper (maps git commands to source directory)
set PATH=%CD%;%PATH%

# Use git normally in Y:\
cd Y:\
git status
git add .
git commit -m "fix: update"
git push
```

The git wrapper automatically maps git commands to the source directory, ensuring `.git` operations happen in the right place.

## Installation

### Windows

**Step 1: Install WinFsp (required)**

encoding-vfs requires WinFsp for virtual drive mounting.

```powershell
winget install WinFsp.WinFsp
```

Or download from https://winfsp.dev/releases/

**Step 2: Download encoding-vfs**

Download the latest zip from [Releases](https://github.com/skyispainted/encoding-vfs/releases) and extract to any directory.

**Step 3: Copy WinFsp DLL**

```powershell
copy "C:\Program Files (x86)\WinFsp\bin\winfsp-x64.dll" .\
```

Copy `winfsp-x64.dll` to the same directory as encoding-vfs.exe.

### Linux

```bash
# Install FUSE3
sudo apt-get install -y libfuse3-2 fuse3
```

Download and extract the Linux version from Releases.

## Basic Usage

### Windows

```powershell
# Mount project to Y: drive
.\encoding-vfs.exe -b C:\projects\my-legacy-project -d Y

# Now you can access Y:\
dir Y:\
type Y:\src\main.cpp
```

encoding-vfs keeps running after mounting. Press `Ctrl+C` to stop and unmount.

### Linux

```bash
# Mount project to /mnt/vfs
./encoding-vfs -b /home/user/my-legacy-project -m /mnt/vfs

# Now you can access /mnt/vfs
ls /mnt/vfs
cat /mnt/vfs/src/main.cpp
```

Unmount: `fusermount -u /mnt/vfs` or press `Ctrl+C`.

## Command Line Options

| Option | Description | Default |
|--------|-------------|---------|
| `-b, --backend <dir>` | Source project directory (where original-encoded files are) | **Required** |
| `-d, --drive <letter>` | Windows drive letter | `X` |
| `-m, --mount <path>` | Linux mount point | `/mnt/vfs` |
| `-s, --source-encoding <enc>` | Source file encoding. `auto` uses default encoding, or specify encoding name | `auto` |
| `-t, --target-encoding <enc>` | Encoding shown when mounted | `UTF-8` |
| `-c, --config <file>` | Config file path | None |
| `-L, --log-level <level>` | Log level: trace/debug/info/warn/error | `info` |

### Supported Encodings

GBK, UTF-8, Big5, Shift_JIS, EUC-JP, EUC-KR, ISO-2022-JP, KOI8-R, Windows-1252, UTF-16LE, UTF-16BE, etc.

In `-s auto` mode, the default encoding (usually GBK) is used as source encoding. Auto-detection between GBK and UTF-8 is unreliable, so explicit specification is recommended.

## Configuration File

You can create a TOML config file instead of command-line arguments:

```toml
[backend]
backend_dir = "C:\\projects\\my-legacy-project"

[mount]
drive_letter = "Y"

[encoding]
source_encoding = "GBK"
target_encoding = "UTF-8"
default_encoding = "GBK"

[encoding.filter]
# Files that skip encoding conversion (raw bytes returned). .gitignore syntax.
rules = ["*.png", "*.exe", "*.dll", "*.jpg"]
# Files/directories completely hidden from mount point
hidden = [".git/", "node_modules/", "*.log"]

[log]
level = "info"
# file = "vfs.log"  # Optional: output to file
```

Usage:

```powershell
.\encoding-vfs.exe -c config.toml
```

### Filter Rules

- `rules`: Matched files skip encoding conversion, returning raw bytes. Suitable for binary files.
- `hidden`: Matched files/directories are completely invisible at the mount point.
- Syntax follows `.gitignore`: `*.png` matches all pngs, `dir/` matches entire directory, `!pattern` negates.
- `.git/` is hidden by default.

## Transparent Git Support

When using git in the mounted directory, set up the git wrapper:

```powershell
# Windows CMD (current session)
set PATH=%CD%;%PATH%

# Windows PowerShell (current session)
$env:PATH = "$PWD;$env:PATH"

# Permanent
setx PATH "%CD%;%PATH%"
```

After setup, when you run `git status`, `git commit`, etc. in the mounted directory, the git wrapper automatically:
1. Identifies the source project for the current directory
2. Maps paths back to the source directory
3. Executes the real git command in the source directory

This ensures `.git` operations always happen in the correct location, avoiding issues from encoding conversion.

## FAQ

### Q: Files open fine after mounting, but original files get corrupted after saving?

A: Make sure you're using the latest version (v0.1.10+). Older versions had a bug in chunked read encoding conversion.

### Q: Why use `-s auto` instead of auto-detecting encoding?

A: Auto-detection between GBK and UTF-8 is very unreliable — they share many overlapping byte sequences. Explicitly specifying the source encoding is recommended. If your project has mixed encodings, that's not currently supported — unify encodings first.

### Q: What is WinFsp? Is it required?

A: WinFsp is the Windows File System Proxy framework that encoding-vfs uses to implement virtual drives. Required on Windows.

### Q: How's the performance after mounting?

A: For large files (>500KB), each read converts the entire file and caches it. First read is slightly slow; subsequent reads are fast. Writes also convert the full content before saving. No noticeable delay in daily use.

### Q: Can I mount multiple projects simultaneously?

A: Yes. Use different drive letters for each:

```powershell
.\encoding-vfs.exe -b C:\projects\project1 -d Y
.\encoding-vfs.exe -b C:\projects\project2 -d Z
```

---

## License

MIT
