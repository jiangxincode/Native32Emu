# 在 iOS 上使用 Native32Emu

Native32Emu 以 libretro core 的形式运行在 RetroArch 上，因此可以在 iPhone / iPad 上玩 Native32 游戏。

> **已验证版本**：Nightly Build 20260701 (`d3bb0a8`) — 音频（MP3 背景音乐 + PCM 音效）已修复并正常工作。
>
> **推荐 RetroArch 版本**：1.17.0（文件结构最适合手动注入 core）。

## 前置条件

- iPhone 或 iPad（arm64，iOS 15+）
- 从 [Releases](https://github.com/jiangxincode/Native32Emu/releases) 下载以下文件：
  - `native32emu_libretro.dylib` — core 二进制文件（iOS arm64）
  - `native32emu_libretro.info` — core 元数据文件
- RetroArch 1.17.0 IPA（[官方下载](https://buildbot.libretro.com/stable/1.17.0/apple/ios-arm64/RetroArch.ipa)）
- 一款可以管理文件和重签 IPA 的 App（如 ESign、Documents by Readdle、SideStore 或 AltStore）
- Native32 游戏文件（`.smf`、`.sgm`、`.ssl` 或 `.zip`）
  - 游戏资源可从 [百度网盘](https://pan.baidu.com/s/1CuNeJe-RKXG_E-LhdI5ldg?pwd=aloy) 下载

## 方式一：手动注入 Core（推荐）

此方法通过修改 RetroArch 的 IPA 包，将 Native32 core 内嵌到应用中，安装后开箱即用。

### 步骤 1：获取 Core 文件

从 [Releases](https://github.com/jiangxincode/Native32Emu/releases) 下载最新版本的 iOS core 包，解压后得到：

- `native32emu_libretro.dylib`
- `native32emu_libretro.info`

### 步骤 2：将 IPA 改为 ZIP 并解压

1. 将下载的 `RetroArch.ipa` 文件重命名为 `RetroArch.zip`
2. 解压该 ZIP 文件，得到 `Payload/` 文件夹
3. 进入 `Payload/RetroArch.app/` 目录

### 步骤 3：注入 Core 二进制文件

将 `native32emu_libretro.dylib` 放入以下目录：

```
Payload/RetroArch.app/modules/
```

### 步骤 4：注入 Core 元数据

1. 找到 `RetroArch.app/` 目录下的 `assets.zip` 文件
2. 解压 `assets.zip`，进入解压后的 `assets/` 目录
3. 将 `native32emu_libretro.info` 放入 `assets/info/` 目录
4. 重新压缩整个 `assets/` 目录，确保压缩后的文件名为 `assets.zip`
5. 将新的 `assets.zip` 放回 `RetroArch.app/` 目录，替换原文件

### 步骤 5：重新签名

这是 iOS 上最关键的步骤。修改后的 IPA 必须重新签名才能安装。

**在 iOS 设备上使用 ESign：**

1. 安装 [ESign](https://www.e-sign.cn/)（或其他签名工具）
2. 将修改后的文件夹重新打包为 `.ipa` 文件
3. 在 ESign 中导入该 IPA
4. 选择 "签名" → 选择你的证书（个人证书或开发者证书）
5. 签名完成后安装

**在 Mac/PC 上使用 SideStore 或 AltStore：**

1. 安装 SideStore 或 AltStore 到你的 iOS 设备
2. 将修改后的 IPA 通过 SideStore/AltStore 安装

### 步骤 6：完成

安装后打开 RetroArch，Native32 core 应该会自动出现在核心列表中。

## 方式二：使用预构建 IPA（快速体验）

如果你不想手动修改 IPA，社区成员 `celiocasttro` 已经提供了集成好 Native32 core 的 RetroArch 1.17.0 IPA：

> ⚠️ **注意**：此文件由社区成员提供，非官方发布，请自行评估风险。

参考视频教程：https://youtu.be/MDoS2ir9cnI

## 首次启动配置

安装完成后，首次打开 RetroArch 时执行以下操作：

1. 打开 RetroArch
2. 进入 **Online Updater** 菜单
3. 依次运行以下更新：
   - Update Core Info Files
   - Update Assets
   - Update Controller Profiles
   - Update Cheats
   - Update Databases
   - Update Overlays
   - Update Slang Shaders
4. 更新完成后**重启** RetroArch

这确保所有必要的资源和配置正确加载，core 才能正常工作。

## 加载游戏

1. 打开 RetroArch
2. 选择 **Load Core** → **Native32 (Native32Emu)**
3. 选择 **Load Content** → 浏览到你的游戏文件所在目录
4. 选择 `.smf`、`.sgm`、`.ssl` 或 `.zip` 文件加载

> **ZIP 模式**：加载 `.zip` 文件时，模拟器会启动 FHUI 菜单。在菜单中选择游戏即可开始；游戏中按 **B** 键（RetroPad 的 A 按钮）返回菜单。

## 按键映射

| RetroPad 按钮 | Native32 按键码 | 功能 |
|---------------|----------------|------|
| 十字键 ↑ | `0x1c00` | 上 |
| 十字键 ↓ | `0x1e00` | 下 |
| 十字键 ← | `0x0200` | 左 |
| 十字键 → | `0x0400` | 右 |
| A（SNES 东侧） | `0x8800` | B / 菜单 |
| B（SNES 南侧） | `0x4000` | A |

> 💡 **提示**：在 RetroArch 的 *Quick Menu → Core Options* 中可以启用 **Swap A/B** 选项来交换 A/B 按钮映射。

## Core 选项

在 RetroArch 的 *Quick Menu → Core Options* 中可配置：

| 选项 | 说明 |
|------|------|
| Audio Volume | 音频音量调节 |
| Key Auto-repeat Timing | 按键自动重复延迟 |
| Swap A/B | 交换 A/B 按钮映射 |
| Auto-skip Cutscenes | 自动跳过过场动画 |

## 支持的功能

- ✅ 视频输出（XRGB8888 像素格式）
- ✅ 音频输出（MP3 背景音乐 + RAW PCM 音效，立体声）
- ✅ 输入处理（D-Pad + A/B 按钮）
- ✅ 游戏加载（`.smf`、`.sgm`、`.ssl`、`.zip` 文件）
- ✅ 存档功能（Save States）
- ✅ Core 选项配置

## 故障排除

### Core 在 RetroArch 中不显示

- 确认使用的是 RetroArch **1.17.0** 版本（更新版本的文件结构不同，注入更复杂）
- 确认 `native32emu_libretro.dylib` 放在了 `modules/` 目录
- 确认 `native32emu_libretro.info` 放在了 `assets/info/` 目录（在 `assets.zip` 内）
- 确认 IPA 已正确重新签名

### 音频问题

- 确保使用 Build 20260701 (`d3bb0a8`) 或更新版本——此版本修复了 iOS 上的 MP3 背景音乐问题
- 如果使用旧版本，部分游戏的音频可能在几秒后停止播放

### 安装 IPA 失败

- iOS 不允许直接安装未签名的 IPA 文件
- 必须通过 ESign、SideStore 或 AltStore 等工具重新签名后才能安装
- 参考视频教程：https://youtu.be/qoKI888e2P0

### 游戏文件无法加载

- 确认游戏文件格式为 `.smf`、`.sgm`、`.ssl` 或 `.zip`
- 确认文件完整无损坏
- 尝试在 RetroArch 的 *Settings → File Browser* 中添加游戏文件所在的目录

## 支持的游戏

全部 84 款 Native32 游戏均可运行。详细游戏列表见 [Game Compatibility](Game-Compatibility.md)。

| 分类 | 数量 | 状态 |
|------|------|------|
| 主菜单 | 1 | ✅ 通过 |
| EACT（动作） | 11 | ✅ 通过 |
| EELA（教育） | 32 | ✅ 通过 |
| EPOP（热门） | 9 | ✅ 通过 |
| EPUZ（益智） | 24 | ✅ 通过 |
| ESPG（运动） | 3 | ✅ 通过 |
| ETAB（棋盘） | 4 | ✅ 通过 |
| **合计** | **84** | **✅ 全部通过** |

## 参考链接

- [Native32Emu Releases](https://github.com/jiangxincode/Native32Emu/releases) — 下载 core 文件
- [RetroArch 1.17.0 iOS IPA](https://buildbot.libretro.com/stable/1.17.0/apple/ios-arm64/RetroArch.ipa) — 官方 RetroArch
- [游戏资源下载](https://pan.baidu.com/s/1CuNeJe-RKXG_E-LhdI5ldg?pwd=aloy) — 百度网盘
- [Issue #58](https://github.com/jiangxincode/Native32Emu/issues/58) — iOS 支持讨论
- [Issue #71](https://github.com/jiangxincode/Native32Emu/issues/71) — iOS 音频修复验证

## 致谢

- **celiocasttro** — iOS 注入方法的测试与文档编写
- **jiangxincode** — Native32Emu 项目作者
