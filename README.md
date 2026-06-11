[English](#english) | [涓枃](#涓枃)

---

<a id="english"></a>

# encoding-vfs

**A transparent encoding conversion virtual filesystem.**
**閫忔槑鐨勭紪鐮佽浆鎹㈣櫄鎷熸枃浠剁郴缁熴€?*

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

<a id="涓枃"></a>

## 涓枃

### 闂

鐜颁唬寮€鍙戝伐鍏凤紙IDE銆丄I 缂栫▼鍔╂墜銆佺粓绔級榛樿浣跨敤 UTF-8銆備絾閬楃暀椤圭洰缁忓父浣跨敤 GBK銆丼hift_JIS銆丅ig5 绛夌紪鐮侊紝瀵艰嚧鏄剧ず涔辩爜鎴栬В鏋愬け璐ャ€?
### 瑙ｅ喅鏂规

encoding-vfs 鍦ㄧ郴缁熺骇鍒寕杞借櫄鎷熺鐩橈紝**閫忔槑杞崲**婧愮紪鐮佸拰鐩爣缂栫爜锛?
- **璇诲彇**锛氶仐鐣欑紪鐮佹枃浠跺湪浠讳綍搴旂敤涓樉绀轰负 UTF-8
- **鍐欏叆**锛歎TF-8 鍐呭鑷姩杞崲鍥炲師濮嬬紪鐮佷繚瀛?- **闆朵镜鍏?*锛氫笉淇敼鏂囦欢锛屼笉闇€瑕佹彃浠讹紝鏃犻渶鎵嬪姩閰嶇疆
- **鍏煎鎵€鏈夊簲鐢?*锛欳laude Code銆乂S Code銆丆ursor銆乧at銆乬it 閮借兘鐩存帴浣跨敤
- **鏅鸿兘闅愯棌**锛氶粯璁ら殣钘?.git 绛夌洰褰?- **閫忔槑 Git**锛欸it 鍛戒护鍦ㄦ寕杞界洰褰曚笅鏃犵紳宸ヤ綔

---

## Quick Start / 蹇€熷紑濮?
### Windows

`powershell
# 1. Install WinFsp / 瀹夎 WinFsp
winget install WinFsp.WinFsp

# 2. Download and extract / 涓嬭浇骞惰В鍘?# encoding-vfs-v0.1.0-windows-x86_64.zip

# 3. Copy DLL / 澶嶅埗 DLL
Copy-Item "C:\Program Files (x86)\WinFsp\bin\winfsp-x64.dll" .\

# 4. Mount / 鎸傝浇
.\encoding-vfs.exe -b C:\legacy-project -d Y

# 5. Install git wrapper / 瀹夎 git wrapper
.\install-git-wrapper.ps1

# 6. Use git / 浣跨敤 git
cd Y:\
git status
`

### Linux

`ash
# 1. Install FUSE3 / 瀹夎 FUSE3
sudo apt-get install -y libfuse3-2 fuse3

# 2. Download and extract / 涓嬭浇骞惰В鍘?# encoding-vfs-v0.1.0-linux-x86_64.tar.gz

# 3. Mount / 鎸傝浇
./encoding-vfs -b /home/user/legacy-project -m /mnt/vfs

# 4. Install git wrapper / 瀹夎 git wrapper
./install-git-wrapper.sh
`

---

## Transparent Git / 閫忔槑 Git

The git wrapper reads `~/.encoding-vfs/mounts.json` to find source directories.

Git wrapper 璇诲彇 `~/.encoding-vfs/mounts.json` 鎵惧埌婧愮洰褰曘€?
`json
{
  "mounts": [
    {
      "mount_point": "Y:",
      "source": "C:\\projects\\my-project",
      "pid": 12345
    }
  ]
}
`

`ash
# All git commands work transparently / 鎵€鏈?git 鍛戒护閫忔槑宸ヤ綔
git status
git log
git diff
git add .
git commit -m "fix: update"
git push
`

---

## Usage / 鐢ㄦ硶

`
encoding-vfs -b <source> [-d drive | -m mount] [-s encoding]
`

| Option / 閫夐」 | Description / 璇存槑 | Default / 榛樿 |
|--------------|-------------------|---------------|
| `-b, --backend` | Source directory / 婧愮洰褰?| *required / 蹇呭～* |
| `-d, --drive` | Windows drive / Windows 鐩樼 | `X` |
| `-m, --mount` | Linux mount / Linux 鎸傝浇鐐?| `/mnt/vfs` |
| `-s, --source-encoding` | Source encoding / 婧愮紪鐮?| `auto` |
| `-c, --config` | Config file / 閰嶇疆鏂囦欢 | - |

---

## Configuration / 閰嶇疆

`	oml
[encoding]
source_encoding = "auto"
target_encoding = "UTF-8"

[encoding.filter]
rules = ["*.png", "*.exe"]
hidden = [".git/", "node_modules/"]
`

---

## Build / 鏋勫缓

`powershell
.\build.ps1
`

---

## Package / 鎵撳寘

`powershell
.\package.ps1
`

---

## CI/CD

`ash
git tag v0.1.0
git push origin v0.1.0
`

---

## Troubleshooting / 鏁呴殰鎺掗櫎

| Problem / 闂 | Solution / 瑙ｅ喅 |
|---------------|----------------|
| Git: "not a git repository" | Check `mounts.json` / 妫€鏌?`mounts.json` |
| Mount fails / 鎸傝浇澶辫触 | Check WinFsp/FUSE / 妫€鏌?WinFsp/FUSE |

---

## Supported Encodings / 鏀寔鐨勭紪鐮?
GBK, Shift_JIS, Big5, UTF-8, UTF-16, EUC-JP, EUC-KR, KOI8-R, ISO-8859-x...

---

## License / 璁稿彲璇?
MIT License
