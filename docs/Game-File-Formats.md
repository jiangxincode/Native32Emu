# Native32 游戏文件格式详解

本文档详细说明 Native32 模拟器平台中各种游戏资源文件的格式、作用及相互关联。

---

## 目录

- [概述](#概述)
- [目录结构](#目录结构)
- [文件格式一览](#文件格式一览)
- [各格式详细说明](#各格式详细说明)
  - [.smf — YUV Gamemaker 场景/脚本文件](#smf--yuv-gamemaker-场景脚本文件)
  - [.SSL / .ssl — SSL 场景/关卡文件](#ssl--ssl--ssl-场景关卡文件)
  - [.dat — 游戏元数据文件](#dat--游戏元数据文件)
  - [.mpg — MPEG 视频文件](#mpg--mpeg-视频文件)
  - [.ssl_sav — 存档文件](#ssl_sav--存档文件)
  - [.nes — NES ROM 镜像](#nes--nes-rom-镜像)
- [文件关联关系](#文件关联关系)
- [模拟器支持情况](#模拟器支持情况)

---

## 概述

Native32 是一个嵌入式/电视游戏平台，其游戏资源由 **YUV Gamemaker 1.3.12** 工具制作。游戏文件主要分为两大类：

1. **原生游戏** — 使用 YUV Gamemaker 制作的 `.smf` 和 `.SSL` 格式游戏
2. **NES 模拟游戏** — 标准 NES ROM 文件，通过平台内置的 NES 模拟器运行

---

## 目录结构

```
native32_game/
├── FHUI.smf                    # 主前端界面/游戏启动器
├── EACT/                       # 动作游戏分类
│   ├── EBBLADE.smf             # 赤刃（跳板文件）
│   ├── EBBLADE.dat             # 赤刃元数据
│   └── ...
├── EELA/                       # 早期教育分类
├── EPOP/                       # 热门游戏分类
├── EPUZ/                       # 益智游戏分类
├── ESPG/                       # 体育游戏分类
├── ETAB/                       # 桌面/赌场游戏分类
├── NA32SSL/                    # 多文件原生游戏
│   ├── CHINESE/                # 中文教育类游戏
│   │   ├── CS1/                # 中文教育游戏集 1
│   │   ├── CS2/                # 中文教育游戏集 2
│   │   └── LOGO/               # 启动 Logo 视频
│   └── ENGLISH/                # 英文动作冒险类游戏
│       ├── BBLADE/             # 赤刃完整版
│       ├── GUNFIRE/            # 枪火
│       ├── METAL/              # 金属风暴
│       ├── MEXPRESS/           # 摩托快递
│       ├── PIRATE/             # 海盗
│       ├── RUNEWORD/           # 符文
│       ├── STORM/              # 风暴
│       └── LOGO/               # 启动 Logo 视频
└── NESGAME/                    # NES ROM 游戏集合（1008 个）
```

### 游戏分类前缀说明

| 前缀 | 分类 | 说明 |
|------|------|------|
| EACT | Action | 动作游戏 |
| EELA | Early Learning & Education | 早期教育 |
| EPOP | Popular | 热门游戏 |
| EPUZ | Puzzle | 益智游戏 |
| ESPG | Sports | 体育游戏 |
| ETAB | Table/Casino | 桌面/赌场游戏 |

---

## 文件格式一览

| 扩展名 | 数量 | 格式 | 用途 | 模拟器支持 |
|--------|------|------|------|-----------|
| `.smf` | 85 | YUV Gamemaker 1.3.12 二进制 | 游戏逻辑、场景、动画、UI | ✅ 支持 |
| `.SSL`/`.ssl` | 101 | YUV Gamemaker 1.3.12 二进制 | 多文件游戏的各个场景/关卡 | ✅ 支持 |
| `.dat` | 83 | `INFO` 头二进制 | 游戏元数据、缩略图、描述 | ❌ 不支持 |
| `.mpg` | 11 | MPEG-1 系统复用 | Logo 动画、过场视频 | ❌ 不支持（跳过） |
| `.ssl_sav` | 4 | 纯文本数字 | 存档/进度数据 | ✅ 支持 |
| `.nes`/`.NES` | 1008 | NES ROM（可能加密头） | NES 游戏 ROM | ❌ 不在本项目范围 |

---

## 各格式详细说明

### .smf — YUV Gamemaker 场景/脚本文件

**魔数头：** `_YUVGamemaker 1.3.12`

`.smf` 是 YUV Gamemaker 的主要游戏文件格式，包含完整的游戏逻辑和资源。每个 `.smf` 文件是一个独立的可执行游戏单元。

#### 文件结构

```
┌─────────────────────────┐
│  SWFT 缩略图（可选）      │  ← 跳过：SWFT_YUV + 16字节头 + 像素数据
├─────────────────────────┤
│  _YUV / ARGB 色彩空间标记  │  ← 查找此标记定位头部
├─────────────────────────┤
│  生成器字符串（48字节）     │  ← 如 "Resolution_320x240"
├─────────────────────────┤
│  基础偏移量                │  ← colorspace + 0x60
├─────────────────────────┤
│  加密头部（32字节）         │  ← 解密后得到资源表偏移量
├─────────────────────────┤
│  光标数据                  │  ← 2字节宽 + 2字节高 + 像素数据
├─────────────────────────┤
│  声音表                    │
├─────────────────────────┤
│  帧表（Frame Table）       │  ← 定义每帧包含的对象
├─────────────────────────┤
│  图像表（Image Table）      │  ← YUV/ARGB 图像数据
├─────────────────────────┤
│  动作表（Action Table）     │  ← 字节码指令
├─────────────────────────┤
│  影片表（Movie Table）      │  ← 动画序列定义
├─────────────────────────┤
│  按钮表（Button Table）     │  ← 交互按钮定义
└─────────────────────────┘
```

#### 资源对象类型

| 类型 ID | 名称 | 说明 |
|---------|------|------|
| 1 | Image | 图像资源 |
| 2 | Movie | 动画/影片 |
| 3 | Button | 交互按钮 |
| 4 | Action | 动作/脚本指令 |
| 5 | Sound | 音频资源 |

#### 两种使用方式

**1. 独立完整游戏**

位于 `E*` 分类目录下的小型 `.smf` 文件（如教育类游戏），包含完整的游戏逻辑和资源，可直接运行。

**2. 跳板/入口文件**

如 `EACT/EBBLADE.smf`（仅 11 KB），不包含完整游戏内容，而是通过 `SSL_PlayNext` 指令跳转到 `NA32SSL` 目录下的多文件游戏。

跳板文件中可见的跳转路径：
```
/NA32SSL /ENGLISH /LOGO    /NALOGO.MPG     ← 先播放 Logo
/NA32SSL /ENGLISH /BBLADE  /BBSTART.SSL    ← 再启动游戏
SSL+SSL_PlayNext+reValue3
```

---

### .SSL / .ssl — SSL 场景/关卡文件

**魔数头：** `_YUVGamemaker 1.3.12`（与 `.smf` 完全相同）

`.SSL` 文件与 `.smf` 使用完全相同的二进制格式，区别仅在于用途：`.SSL` 是多文件游戏中的单个场景/关卡文件。

#### 与 .smf 的关系

```
.smf（独立游戏）
  └── 包含所有场景，单文件运行

.smf（跳板）──→ 多个 .SSL 文件（多文件游戏）
  │
  ├── BBSTART.SSL    标题画面
  ├── BBMENU.SSL     主菜单
  ├── BBPLAY10.SSL   第 1 关
  ├── BBPLAY20.SSL   第 2 关
  ├── ...
  ├── BBMAPLIB.SSL   地图库
  ├── BBOVER.SSL     游戏结束
  └── BBFINISH.SSL   通关画面
```

#### SSL 文件间的跳转机制

SSL 文件通过 VM 指令实现文件间切换：

| VM API | 作用 |
|--------|------|
| `SSL_PlayNext` | 加载并切换到下一个 SSL 文件 |
| `SSL_PlayPlan` | 按计划播放（当前未实现） |
| `SSL_PlayProg` | 按程序播放（当前未实现） |
| `SSL_GetSSLData` | 从 `.ssl_sav` 文件读取存档数据 |
| `SSL_SaveSSLData` | 将数据保存到 `.ssl_sav` 文件 |

#### 文件内容示例

以赤刃（BBLADE）为例，SSL 文件内部包含：

```
loadBg      7           ← 加载背景资源
loadBar     7           ← 加载进度条
filePath    /NA32SSL /ENGLISH /BBLADE /BBMENU.SSL  ← 下一个要加载的文件
SSL+SSL_PlayNext+loadFileSuccess  ← 触发文件切换
saveData    09600000002  ← 保存进度数据
SSL+SSL_SaveSSLData+saveSuccess  ← 触发存档
```

#### 按语言组织

`NA32SSL` 目录按语言分为两套完全独立的游戏集合：

**CHINESE — 中文教育类游戏**
- CS1：CSAMENU、CSFZ、CSSG、CSSN、CSST、CSTB、CSYC
- CS2：CSBMENU、CSDW、CSKC、CSQC、CSSZ、CSYS、CSZR

**ENGLISH — 英文动作冒险类游戏**
- BBLADE（赤刃）、GUNFIRE（枪火）、METAL（金属风暴）
- MEXPRESS（摩托快递）、PIRATE（海盗）、RUNEWORD（符文）、STORM（风暴）

> **注意：** CHINESE 和 ENGLISH 不是同一批游戏的翻译版本，而是两套完全不同的游戏内容。

---

### .dat — 游戏元数据文件

**魔数头：** `INFO`（十六进制 `49 4E 46 4F`）

`.dat` 是每个 `.smf` 文件的配套元数据文件，由原机的前端启动器（`FHUI.smf`）读取，用于在游戏选择菜单中展示游戏信息。

#### 文件结构

```
┌─────────────────┐
│  INFO 魔数头      │  ← 4 字节: "INFO"
├─────────────────┤
│  版本/类型字段     │  ← 4 字节: 02 00 00 00
├─────────────────┤
│  游戏描述文本      │
├─────────────────┤
│  缩略图/预览图     │  ← YUV 格式图像数据
├─────────────────┤
│  游戏配置参数      │
└─────────────────┘
```

#### 与 .smf 的对应关系

每个 `.smf` 文件都有一个同名的 `.dat` 文件：

```
EACT/EBBLADE.smf  ←→  EACT/EBBLADE.dat
EACT/EPIRATE.smf  ←→  EACT/EPIRATE.dat
EPUZ/ESUDOKU.smf  ←→  EPUZ/ESUDOKU.dat
```

#### 模拟器中的状态

当前模拟器**不加载 `.dat` 文件**。模拟器通过命令行直接指定 `.smf` 或 `.SSL` 文件路径启动，跳过了原机的菜单选择流程。

原机的完整启动流程：
```
FHUI.smf（启动器）
  → 读取各 E* 目录下的 .dat 文件
  → 显示游戏列表（缩略图 + 描述）
  → 用户选择
  → 加载对应的 .smf 文件
  → 运行游戏
```

模拟器的启动流程：
```
命令行指定 .smf 或 .SSL 文件
  → 直接加载运行
```

---

### .mpg — MPEG 视频文件

**格式：** MPEG-1 system multiplex

标准 MPEG-1 视频文件，用于游戏中的全动态视频（FMV）播放。

#### 用途分类

| 类型 | 文件 | 说明 |
|------|------|------|
| Logo 视频 | `CHINESE/LOGO/NALOGO.mpg` | 中文版启动 Logo |
| Logo 视频 | `ENGLISH/LOGO/NALOGO.mpg` | 英文版启动 Logo |
| 过场动画 | `ENGLISH/METAL/MSCG1.mpg`, `MSCG2.mpg` | 金属风暴过场 |
| 游戏结束 | `ENGLISH/PIRATE/VIDEO/HDDEAD.mpg`, `HDOUT.mpg` | 海盗死亡/结束画面 |
| 开场动画 | `ENGLISH/STORM/STCG1-4.mpg`, `STSTART.mpg` | 风暴开场/过场 |

#### 模拟器中的状态

当前模拟器**不支持 `.mpg` 视频播放**。当游戏脚本引用视频文件时，模拟器会跳过该指令。

---

### .ssl_sav — 存档文件

**格式：** 纯文本数字字符串

由 `SSL_SaveSSLData` 指令自动生成的存档文件，用于保存玩家进度。

#### 命名规则

```
<原始SSL文件名>.ssl_sav
```

例如：
- `BBSTART.SSL` → `BBSTART.SSL.ssl_sav`
- `BBMENU.SSL` → `BBMENU.SSL.ssl_sav`
- `BBPLAY10.SSL` → `BBPLAY10.SSL.ssl_sav`

#### 文件内容示例

```
BBMENU.SSL.ssl_sav:    1109600000002
BBPLAY10.SSL.ssl_sav:  1106606900422
BBSTART.SSL.ssl_sav:   1109600000002
```

数字字符串可能编码了多个游戏状态标志（如关卡完成状态、音量设置等）。

#### 模拟器中的实现

模拟器通过 `SaveManager` 管理存档：

- `SSL_GetSSLData` → 从 `.ssl_sav` 文件读取数据，注入到 VM 变量
- `SSL_SaveSSLData` → 将 VM 变量中的数据写入 `.ssl_sav` 文件

存档文件保存在游戏文件同目录下。

---

### .nes — NES ROM 镜像

**位置：** `NESGAME/` 目录

标准 NES（Nintendo Entertainment System）ROM 文件，包含 iNES 格式的游戏数据。

#### 特点

- 共 1008 个 ROM 文件
- 文件头经过非标准/加密处理（标准 NES 魔数 `NES\x1a` 被修改）
- 文件大小范围：24 KB ~ 786 KB
- 涵盖经典 NES 游戏：Super Mario Bros、Contra、Zelda、Mega Man 等

#### 模拟器中的状态

NES ROM 的运行由原机平台的 NES 模拟器模块处理，不属于本 Rust 模拟器项目的实现范围。原机通过 `FHUI.smf` 中的 `StartGame` 指令启动 NES 游戏。

---

## 文件关联关系

### 完整的文件关系图

```
FHUI.smf（主启动器）
  │
  ├── 读取 .dat 文件获取游戏列表
  │     ├── EACT/EBBLADE.dat ──→ 对应 EBBLADE.smf
  │     ├── EACT/EPIRATE.dat ──→ 对应 EPIRATE.smf
  │     └── ...
  │
  ├── 加载 .smf 文件运行游戏
  │     │
  │     ├── [独立游戏] 如 EPUZ/ESUDOKU.smf
  │     │     └── 包含所有资源，单文件运行
  │     │
  │     └── [跳板文件] 如 EACT/EBBLADE.smf
  │           │
  │           ├── SSL_PlayNext ──→ NA32SSL/ENGLISH/LOGO/NALOGO.mpg
  │           └── SSL_PlayNext ──→ NA32SSL/ENGLISH/BBLADE/BBSTART.SSL
  │                                   │
  │                                   ├── SSL_PlayNext ──→ BBMENU.SSL
  │                                   │                      │
  │                                   │                      ├── SSL_PlayNext ──→ BBPLAY10.SSL
  │                                   │                      │                      │
  │                                   │                      │                      └── ...
  │                                   │                      │
  │                                   │                      └── SSL_GetSSLData ←── BBMENU.SSL.ssl_sav
  │                                   │
  │                                   └── SSL_SaveSSLData ──→ BBSTART.SSL.ssl_sav
  │
  └── StartGame ──→ NESGAME/*.nes（NES ROM 模拟）
```

### 以赤刃（BBLADE）为例的完整加载流程

```
1. 用户在 FHUI.smf 菜单中选择赤刃
      ↓
2. 加载 EACT/EBBLADE.smf（跳板文件，11 KB）
      ↓
3. 播放启动 Logo: /NA32SSL/ENGLISH/LOGO/NALOGO.mpg
      ↓
4. SSL_PlayNext 加载: /NA32SSL/ENGLISH/BBLADE/BBSTART.SSL（1.8 MB）
   - 显示标题画面
   - 读取存档: BBSTART.SSL.ssl_sav
   - 保存存档: BBSTART.SSL.ssl_sav
      ↓
5. SSL_PlayNext 加载: BBMENU.SSL（3.5 MB）
   - 显示主菜单
   - 读取/保存存档
      ↓
6. SSL_PlayNext 加载: BBPLAY10.SSL（2.6 MB）
   - 运行第 1 关
      ↓
7. 继续切换 BBPLAY20 → BBPLAY30 → ... → BBPLAY62
      ↓
8. 通关: BBFINISH.SSL / 游戏结束: BBOVER.SSL
```

---

## 模拟器支持情况

### 当前支持的格式

| 格式 | 支持状态 | 说明 |
|------|---------|------|
| `.smf` | ✅ 完全支持 | 通过 `Native32Reader` 解析加载 |
| `.SSL`/`.ssl` | ✅ 完全支持 | 与 `.smf` 共用同一解析器 |
| `.ssl_sav` | ✅ 完全支持 | 通过 `SaveManager` 读写存档 |

### 当前不支持的格式

| 格式 | 支持状态 | 说明 |
|------|---------|------|
| `.dat` | ❌ 不支持 | 模拟器跳过菜单流程，无需元数据 |
| `.mpg` | ❌ 不支持 | 视频播放未实现，相关指令被跳过 |
| `.nes` | ❌ 不在范围 | NES 模拟由原机平台模块处理 |

