**English** | [中文](README_zh.md)

---

# encoding-vfs

**A transparent encoding conversion virtual filesystem. Mount a drive where legacy-encoded files (GBK, Shift_JIS, Big5...) appear as UTF-8 to any application.**

**透明的编码转换虚拟文件系统。挂载一个虚拟磁盘，让遗留编码文件（GBK、Shift_JIS、Big5...）在任何应用中显示为 UTF-8。**

---

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
