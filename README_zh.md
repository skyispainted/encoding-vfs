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

## 蹇€熷紑濮?
### Windows

`powershell
# 1. 瀹夎 WinFsp
winget install WinFsp.WinFsp

# 2. 涓嬭浇骞惰В鍘?# encoding-vfs-v0.1.0-windows-x86_64.zip

# 3. 澶嶅埗 DLL
Copy-Item "C:\Program Files (x86)\WinFsp\bin\winfsp-x64.dll" .\

# 4. 鎸傝浇椤圭洰
.\encoding-vfs.exe -b C:\legacy-project -d Y

# 5. 瀹夎 git wrapper
.\install-git-wrapper.ps1

# 6. 浣跨敤 git
cd Y:\
git status
`

### Linux

`ash
# 1. 瀹夎 FUSE3
sudo apt-get install -y libfuse3-2 fuse3

# 2. 涓嬭浇骞惰В鍘?# encoding-vfs-v0.1.0-linux-x86_64.tar.gz

# 3. 鎸傝浇椤圭洰
./encoding-vfs -b /home/user/legacy-project -m /mnt/vfs

# 4. 瀹夎 git wrapper
./install-git-wrapper.sh
`

---

## 閫忔槑 Git

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
# 鎵€鏈?git 鍛戒护閫忔槑宸ヤ綔
git status
git log
git diff
git add .
git commit -m "fix: 鏇存柊浠ｇ爜"
git push
`

---

## 鍛戒护琛岀敤娉?
`
encoding-vfs -b <婧愮洰褰? [-d 鐩樼 | -m 鎸傝浇鐐筣 [-s 缂栫爜]
`

| 閫夐」 | 璇存槑 | 榛樿鍊?|
|------|------|--------|
| `-b, --backend` | 婧愮洰褰?| *蹇呭～* |
| `-d, --drive` | Windows 鐩樼 | `X` |
| `-m, --mount` | Linux 鎸傝浇鐐?| `/mnt/vfs` |
| `-s, --source-encoding` | 婧愮紪鐮?| `auto` |
| `-c, --config` | 閰嶇疆鏂囦欢 | - |

---

## 閰嶇疆鏂囦欢

`	oml
[encoding]
source_encoding = "auto"
target_encoding = "UTF-8"

[encoding.filter]
rules = ["*.png", "*.exe"]
hidden = [".git/", "node_modules/"]
`

---

## 鏋勫缓

`powershell
.\build.ps1
`

---

## 鎵撳寘

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

## 鏁呴殰鎺掗櫎

| 闂 | 瑙ｅ喅 |
|------|------|
| Git: "not a git repository" | 妫€鏌?`mounts.json` |
| 鎸傝浇澶辫触 | 妫€鏌?WinFsp/FUSE |

---

## 鏀寔鐨勭紪鐮?
GBK, Shift_JIS, Big5, UTF-8, UTF-16, EUC-JP, EUC-KR, KOI8-R, ISO-8859-x...

---

## 璁稿彲璇?
MIT License
