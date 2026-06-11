[English](#english) | [中文](#中文)

---

<a id="english"></a>

# encoding-vfs

**A transparent encoding conversion virtual filesystem.**
**透明的编码转换虚拟文件系统。**

---

## English

### The Problem

Modern development tools (IDEs, AI coding assistants, terminals) default to UTF-8. But legacy projects often use GBK, Shift_JIS, Big5, or other encodings that render as garbled text or fail to parse.

### The Solution

encoding-vfs mounts a virtual drive that **transparently converts** between source and target encodings at the system level:

- **Read**: Legacy-encoded files appear as UTF-8 to any application
- **Write**: UTF-8 content is converted back to the original encoding on disk
- **Zero intrusion**: No file modification, no IDE plugins, no manual configuration
- **Works with any app**: Claude Code, VS Code, Cursor, cat, git - all work directly
- **Smart hiding**: .git and other directories hidden by default
- **Transparent Git**: Git commands work seamlessly in mounted directories

---

<a id="中文"></a>

## 中文

### 问题

现代开发工具（IDE、AI 编程助手、终端）默认使用 UTF-8。但遗留项目经常使用 GBK、Shift_JIS、Big5 等编码，导致显示乱码或解析失败。

### 解决方案

encoding-vfs 在系统级别挂载虚拟磁盘，**透明转换**源编码和目标编码：

- **读取**：遗留编码文件在任何应用中显示为 UTF-8
- **写入**：UTF-8 内容自动转换回原始编码保存
- **零侵入**：不修改文件，不需要插件，无需手动配置
- **兼容所有应用**：Claude Code、VS Code、Cursor、cat、git 都能直接使用
- **智能隐藏**：默认隐藏 .git 等目录
- **透明 Git**：Git 命令在挂载目录下无缝工作

---

## Quick Start / 快速开始

### Windows

#### PowerShell

```powershell
# 1. Install WinFsp / 安装 WinFsp
winget install WinFsp.WinFsp

# 2. Download and extract / 下载并解压
# encoding-vfs-v0.1.3-windows-x86_64.zip

# 3. Copy DLL / 复制 DLL
Copy-Item "C:\Program Files (x86)\WinFsp\bin\winfsp-x64.dll" .\

# 4. Mount / 挂载
.\encoding-vfs.exe -b C:\legacy-project -d Y

# 5. Setup git wrapper (choose one) / 设置 git wrapper（选一）

# Option A: Current session only / 方案A：仅当前会话
$env:PATH = "$PWD;$env:PATH"

# Option B: Permanent / 方案B：永久设置
[Environment]::SetEnvironmentVariable("PATH", "$PWD;$env:PATH", "User")
# Restart terminal / 重启终端生效

# 6. Use git / 使用 git
cd Y:\
git status
```

#### CMD

```cmd
REM 1. Install WinFsp / 安装 WinFsp
winget install WinFsp.WinFsp

REM 2. Download and extract / 下载并解压
REM encoding-vfs-v0.1.3-windows-x86_64.zip

REM 3. Copy DLL / 复制 DLL
copy "C:\Program Files (x86)\WinFsp\bin\winfsp-x64.dll" .\

REM 4. Mount / 挂载
encoding-vfs.exe -b C:\legacy-project -d Y

REM 5. Setup git wrapper (choose one) / 设置 git wrapper（选一）

REM Option A: Current session only / 方案A：仅当前会话
set PATH=%CD%;%PATH%

REM Option B: Permanent / 方案B：永久设置
setx PATH "%CD%;%PATH%"
REM Restart terminal / 重启终端生效

REM 6. Use git / 使用 git
cd Y:\
git status
```

### Linux

```bash
# 1. Install FUSE3 / 安装 FUSE3
sudo apt-get install -y libfuse3-2 fuse3

# 2. Download and extract / 下载并解压
# encoding-vfs-v0.1.3-linux-x86_64.tar.gz

# 3. Mount / 挂载
./encoding-vfs -b /home/user/legacy-project -m /mnt/vfs

# 4. Setup git wrapper (choose one) / 设置 git wrapper（选一）

# Option A: Current session only / 方案A：仅当前会话
export PATH="$PWD:$PATH"

# Option B: Permanent / 方案B：永久设置
echo 'export PATH="$PWD:$PATH"' >> ~/.bashrc
source ~/.bashrc

# 5. Use git / 使用 git
cd /mnt/vfs
git status
```

---

## Transparent Git / 透明 Git

Git wrapper reads `~/.encoding-vfs/mounts.json` to find source directories.

Git wrapper 读取 `~/.encoding-vfs/mounts.json` 找到源目录。

```json
{
  "mounts": [{ "mount_point": "Y:", "source": "C:\\projects\\my-project", "pid": 12345 }]
}
```

```bash
git status
git log
git diff
git add .
git commit -m "fix: update"
git push
```

---

## Usage / 用法

| Option / 选项 | Description / 说明 | Default / 默认 |
|--------------|-------------------|---------------|
| `-b, --backend` | Source directory / 源目录 | *required* |
| `-d, --drive` | Windows drive / Windows 盘符 | `X` |
| `-m, --mount` | Linux mount / Linux 挂载点 | `/mnt/vfs` |
| `-s, --source-encoding` | Encoding / 编码 | `auto` |

---

## Config / 配置

```toml
[encoding]
source_encoding = "auto"
target_encoding = "UTF-8"

[encoding.filter]
rules = ["*.png", "*.exe"]
hidden = [".git/", "node_modules/"]
```

---

## Build / 构建

```powershell
.\build.ps1
```

---

## Package / 打包

```powershell
.\package.ps1
```

---

## CI/CD

```bash
git tag v0.1.3
git push origin v0.1.3
```

---

## License / 许可证

MIT
