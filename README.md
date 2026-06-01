# Native32 Emulator

[![CI](https://github.com/jiangxincode/Native32Emu/actions/workflows/ci.yml/badge.svg)](https://github.com/jiangxincode/Native32Emu/actions/workflows/ci.yml)
[![Quality Gate Status](https://sonarcloud.io/api/project_badges/measure?project=jiangxincode_Native32Emu&metric=alert_status)](https://sonarcloud.io/dashboard?id=jiangxincode_Native32Emu)
[![License: BSD 3-Clause](https://img.shields.io/badge/License-BSD%203--Clause-blue.svg)](LICENSE)

A Native32 game emulator written in Rust.

Native32 is a game format developed by Sunplus for DVD player and TV chipsets (circa 2005–2011). Games use `.smf`, `.sgm`, or `.ssl` file extensions and feature a stack-based, ActionScript-like virtual machine with raster graphics.

## Features

- **Full Native32 format support** — file loading, header decryption, resource table parsing
- **YUV & ARGB image decoding** — with packbits/RLE decompression and color space conversion
- **Action bytecode VM** — 36 opcodes covering arithmetic, logic, string ops, control flow, sprites, and I/O
- **Sprite/movie system** — animation, cloning, visibility control, depth-sorted rendering
- **Audio playback** — MP3 music and raw 16-bit PCM sound effects
- **Keyboard input** — configurable key remapping
- **Save system** — `.ssl_sav` file persistence
- **SSL multi-file content** — seamless switching between game levels/files
- **CLI controls** — scaling, fullscreen, volume adjustment

## Building

Requires [Rust](https://www.rust-lang.org/tools/install) (stable).

```bash
cargo build --release
```

## Usage

```bash
# Basic usage
cargo run -- path/to/game.smf

# With options
cargo run -- --scale 2 --volume 80 path/to/game.smf

# Release build
cargo run --release -- path/to/game.smf
```

### Command-line Options

| Option | Description | Default |
|---|---|---|
| `<GAME_PATH>` | Path to the game file (`.smf`, `.sgm`, or `.ssl`) | *required* |
| `-s, --scale <1-16>` | Integer scaling factor | `1` |
| `-f, --fullscreen` | Run in fullscreen mode | off |
| `-v, --volume <0-100>` | Volume level (0 = mute, 100 = original) | `100` |
| `-S, --screenshot <PATH>` | Take a screenshot and exit (saves as PNG) | — |
| `--screenshot-frames <N>` | Frames to run before screenshot | `30` |
| `--debug` | Enable debug/development mode | off |
| `--remap <keycode:key>` | Remap a Native32 keycode to a physical key | — |

### Default Key Mappings

| Native32 Keycode | Physical Key | Action |
|---|---|---|
| `0x0200` | ← Left | Left |
| `0x0400` | → Right | Right |
| `0x1c00` | ↑ Up | Up |
| `0x1e00` | ↓ Down | Down |
| `0x4000` | Z | A |
| `0x8800` | X | B / Menu |

### Examples

```bash
# 2x scaling with 50% volume
cargo run -- --scale 2 --volume 50 native32_game/FHUI.smf

# Remap A button to Space
cargo run -- --remap "0x4000:space" native32_game/FHUI.smf

# Fullscreen mode
cargo run -- --fullscreen native32_game/EACT/EBBLADE.smf

# Take a screenshot after 30 frames and exit
cargo run -- --screenshot screenshot.png --screenshot-frames 30 native32_game/EACT/EBBLADE.smf
```

## Architecture

```
src/
├── main.rs              # Emulation loop and VmHost implementation
├── cli.rs               # Command-line argument parsing
├── actions.rs           # Action opcode enum (36 opcodes)
├── action_vm.rs         # Stack-based virtual machine
├── audio_engine.rs      # MP3/PCM audio playback (rodio)
├── content_loader.rs    # SSL multi-file content switching
├── des_constants.rs     # DES permutation tables and S-boxes
├── error.rs             # Error types
├── file_loader.rs       # File I/O, header parsing, resource tables
├── frame_player.rs      # Main timeline frame playback (30fps)
├── header_decryptor.rs  # Custom DES ECB header decryption
├── image_decoder.rs     # YUV 4:2:0 and ARGB1555 image decoders
├── input_handler.rs     # Keyboard input to keycode mapping
├── renderer.rs          # Frame rendering with depth sorting
├── save_manager.rs      # Save data persistence (.ssl_sav)
└── sprite_system.rs     # Movie/sprite instance management
```

## Dependencies

| Crate | Purpose |
|---|---|
| `clap` | Command-line argument parsing |
| `minifb` | Window creation and pixel rendering |
| `rodio` | Audio playback (MP3 + PCM) |
| `anyhow` / `thiserror` | Error handling |
| `log` / `env_logger` | Logging |
| `rand` | Random number generation (for VM `RandomNumber` opcode) |

## Game Compatibility

All 84 Native32 games in the test suite load and run without fatal errors. Each game was tested by launching the emulator, loading the ROM, running for 5 seconds, and checking for panics or crashes.

### Main Menu (1 game)

| # | 游戏名 | 英文名 | 文件名 | 图片 | 游戏介绍 | Status |
|---|--------|--------|--------|------|----------|--------|
| 1 | 主菜单 | FHUI | FHUI.smf | ![主菜单](images/root/FHUI.png) | — | ✅ Pass |

### EACT — Action Games (11 games)

| # | 游戏名 | 英文名 | 文件名 | 图片 | 游戏介绍 | Status |
|---|--------|--------|--------|------|----------|--------|
| 1 | 赤刃 | Bloody Blade | EBBLADE.smf | ![赤刃](images/EACT/EBBLADE.png) | 经典的横版动作过关（ACT）游戏。赤刃一族被灭族，20 年后幸存的后裔女刺客开始复仇之旅。 | ✅ Pass |
| 2 | 枪火 | Gun Fire | EGUNFIRE.smf | ![枪火](images/EACT/EGUNFIRE.png) | 横版射击（STG）游戏。骑上心爱的机车，拿起火枪，在枪火纷飞的世界去打击黑社会，对抗独裁者，消灭生化怪兽。 | ✅ Pass |
| 3 | 钢铁风暴 | Metal Storm | EMETAL.smf | ![钢铁风暴](images/EACT/EMETAL.png) | 四向自由卷轴射击游戏。游戏虚构了一个近未来的世界，军事科技以现实世界科技为基础有一定发展，武器与现实世界相似。 | ✅ Pass |
| 4 | 海盗 | Pirate | Epirate.smf | ![海盗](images/EACT/Epirate.png) | 以"炸弹人"为原型的休闲桌面（PUZ）游戏。小海盗雷克获得藏宝图后开启冒险。 | ✅ Pass |
| 5 | 风暴之翼 | Storm | ESTORM.smf | ![风暴之翼](images/EACT/ESTORM.png) | 纵版射击（STG）游戏。王牌飞行员 Jason 和 Blanche 深入 239 区侦察，遭遇敌方终极战机及隐藏秘密武器。 | ✅ Pass |
| 6 | 小小我 | Little Me | LittleMe.smf | ![小小我](images/EACT/LittleMe.png) | — | ✅ Pass |
| 7 | 失落之剑 | Lost Sword | LostSwor.smf | ![失落之剑](images/EACT/LostSwor.png) | — | ✅ Pass |
| 8 | 鼓舞 | Music Game | MusicGam.smf | ![鼓舞](images/EACT/MusicGam.png) | 音乐游戏，相当于劲舞团一样的玩法，但操作较其简单，跟随节奏提示按下方向键。 | ✅ Pass |
| 9 | 泡泡精灵 | PoPo Fun | PoPoFun.smf | ![泡泡精灵](images/EACT/PoPoFun.png) | 经典的踩气球游戏，控制主角用脚踩爆敌人头顶气球来消灭对手。 | ✅ Pass |
| 10 | 少林功夫 | ShaoLin Kung Fu | ShaoLinK.smf | ![少林功夫](images/EACT/ShaoLinK.png) | — | ✅ Pass |
| 11 | 三只小猪 | Three Pigs | ThreePig.smf | ![三只小猪](images/EACT/ThreePig.png) | — | ✅ Pass |

### EELA — Educational Games (32 games)

| # | 游戏名 | 英文名 | 文件名 | 图片 | 游戏介绍 | Status |
|---|--------|--------|--------|------|----------|--------|
| 1 | 加 21 | Adding 21 | Adding21.smf | ![加21](images/EELA/Adding21.png) | — | ✅ Pass |
| 2 | 字母排序 | Alphabetical Order | AlpOrder.smf | ![字母排序](images/EELA/AlpOrder.png) | — | ✅ Pass |
| 3 | 动物朋友 | Animal Friends | AnimalFr.smf | ![动物朋友](images/EELA/AnimalFr.png) | — | ✅ Pass |
| 4 | 动物一 | Animals 1 | Animals1.smf | ![动物一](images/EELA/Animals1.png) | — | ✅ Pass |
| 5 | 动物二 | Animals 2 | Animals2.smf | ![动物二](images/EELA/Animals2.png) | — | ✅ Pass |
| 6 | 小鸡回家 | Chicklin | Chicklin.smf | ![小鸡回家](images/EELA/Chicklin.png) | 关于简单加减乘除运算正误判断的练习，适合多个年龄层幼儿，判断数学等式是否正确。 | ✅ Pass |
| 7 | 颜色魔法 | Colors Magic | ColorsMa.smf | ![颜色魔法](images/EELA/ColorsMa.png) | — | ✅ Pass |
| 8 | 阅读理解 | Comprehension | Comprehe.smf | ![阅读理解](images/EELA/Comprehe.png) | — | ✅ Pass |
| 9 | 数字猎人 | Digital Hunt | DigiHunt.smf | ![数字猎人](images/EELA/DigiHunt.png) | — | ✅ Pass |
| 10 | 找匹配 | Find The Match | FindTheM.smf | ![找匹配](images/EELA/FindTheM.png) | — | ✅ Pass |
| 11 | 水果一 | Fruits 1 | Fruits1.smf | ![水果一](images/EELA/Fruits1.png) | — | ✅ Pass |
| 12 | 水果二 | Fruits 2 | Fruits2.smf | ![水果二](images/EELA/Fruits2.png) | — | ✅ Pass |
| 13 | 地理 | Geography | Geograph.smf | ![地理](images/EELA/Geograph.png) | — | ✅ Pass |
| 14 | 生物 | Living Things | LivingTi.smf | ![生物](images/EELA/LivingTi.png) | — | ✅ Pass |
| 15 | 魔法链 | Magic Chain | MagicCha.smf | ![魔法链](images/EELA/MagicCha.png) | — | ✅ Pass |
| 16 | 魔法 A | Magical A | MagicalA.smf | ![魔法A](images/EELA/MagicalA.png) | — | ✅ Pass |
| 17 | 数学讨价还价 | Math Bargain | MathBarg.smf | ![数学讨价还价](images/EELA/MathBarg.png) | — | ✅ Pass |
| 18 | 猴子军队 | Monkey Army | MonkeyAr.smf | ![猴子军队](images/EELA/MonkeyAr.png) | — | ✅ Pass |
| 19 | 比大小 | More Or Less | MoreOrLe.smf | ![比大小](images/EELA/MoreOrLe.png) | 关于比较大小的游戏，适合较低年龄幼儿，判断两边宝石数量并填上 `<、=、>` 连接符。 | ✅ Pass |
| 20 | 音乐基础 | Music Basic | MusicBas.smf | ![音乐基础](images/EELA/MusicBas.png) | — | ✅ Pass |
| 21 | 单词拼块 | Ordered Blocks | OrdBlock.smf | ![单词拼块](images/EELA/OrdBlock.png) | 关于英语单词识别和拼写的游戏，游戏会读出一个单词，控制字母块完成拼写。 | ✅ Pass |
| 22 | 重新拼字 | Re-Letter | ReLetter.smf | ![重新拼字](images/EELA/ReLetter.png) | — | ✅ Pass |
| 23 | 看图说话 | Reads Picture | ReadsPic.smf | ![看图说话](images/EELA/ReadsPic.png) | — | ✅ Pass |
| 24 | 汉语学堂一 | School 1 | SCHOOL1.smf | ![汉语学堂一](images/EELA/SCHOOL1.png) | 中英文双语学习（ELA）软件，全 3D 图画，涵盖服装、水果、室内、身体、头部、野餐六大类。 | ✅ Pass |
| 25 | 汉语学堂二 | School 2 | SCHOOL2.smf | ![汉语学堂二](images/EELA/SCHOOL2.png) | 中英文双语学习（ELA）软件，全 3D 图画，配以中英文双重语音，涵盖动物、快餐、汽车、数字、颜色、自然六大类。 | ✅ Pass |
| 26 | 搜索者 | Seeker | Seeker.smf | ![搜索者](images/EELA/Seeker.png) | — | ✅ Pass |
| 27 | 简单算术 | Simple Arithmetic | SimpleAr.smf | ![简单算术](images/EELA/SimpleAr.png) | 讲述 Raidy 和 Annie 训练 Borry 跑步跳高的故事，数学基础加减乘除运算练习，从泡泡中选择缺少的数字。 | ✅ Pass |
| 28 | 快速算术 | Speed Arithmetic | SpeedAri.smf | ![快速算术](images/EELA/SpeedAri.png) | — | ✅ Pass |
| 29 | 超级加法 | Super Add | SuperAdd.smf | ![超级加法](images/EELA/SuperAdd.png) | — | ✅ Pass |
| 30 | 我们的时间 | Us Time | UsTime.smf | ![我们的时间](images/EELA/UsTime.png) | — | ✅ Pass |
| 31 | 单词选择 | Word Choice | WordChoi.smf | ![单词选择](images/EELA/WordChoi.png) | — | ✅ Pass |
| 32 | 单词车间 | Workshop | Workshop.smf | ![单词车间](images/EELA/Workshop.png) | 关于英语单词拼写的练习，适合较低年龄层幼儿，选择字母将单词补充完整，难度逐渐增加。 | ✅ Pass |

### EPOP — Hot/Featured Games (9 games)

| # | 游戏名 | 英文名 | 文件名 | 图片 | 游戏介绍 | Status |
|---|--------|--------|--------|------|----------|--------|
| 1 | 赤刃 | Bloody Blade | EBBLADE.smf | ![赤刃](images/EPOP/EBBLADE.png) | 经典的横版动作过关（ACT）游戏。赤刃一族被灭族，20 年后幸存的后裔女刺客开始复仇之旅。 | ✅ Pass |
| 2 | 极速任务 | Express | EExpress.smf | ![极速任务](images/EPOP/EExpress.png) | 类赛车多角色游戏，含城市、高速、荒野、F1 赛道场景，有障碍物和道具。 | ✅ Pass |
| 3 | 枪火 | Gun Fire | EGUNFIRE.smf | ![枪火](images/EPOP/EGUNFIRE.png) | 横版射击（STG）游戏。骑上心爱的机车，拿起火枪，在枪火纷飞的世界去打击黑社会，对抗独裁者，消灭生化怪兽。 | ✅ Pass |
| 4 | 钢铁风暴 | Metal Storm | EMETAL.smf | ![钢铁风暴](images/EPOP/EMETAL.png) | 四向自由卷轴射击游戏。游戏虚构了一个近未来的世界，军事科技以现实世界科技为基础有一定发展，武器与现实世界相似。 | ✅ Pass |
| 5 | 海盗 | Pirate | Epirate.smf | ![海盗](images/EPOP/Epirate.png) | 以"炸弹人"为原型的休闲桌面（PUZ）游戏。小海盗雷克获得藏宝图后开启冒险。 | ✅ Pass |
| 6 | 符文之语 | Rune Word | ERuneWod.smf | ![符文之语](images/EPOP/ERuneWod.png) | 类似于对对碰的游戏。年轻的魔法学徒艾莉进入炼金房修炼，在大魔导师雷斯林的指导下学习一个又一个的魔法。各种符文代表不同属性的魔力。 | ✅ Pass |
| 7 | 风暴之翼 | Storm | ESTORM.smf | ![风暴之翼](images/EPOP/ESTORM.png) | 纵版射击（STG）游戏。王牌飞行员 Jason 和 Blanche 深入 239 区侦察，遭遇敌方终极战机及隐藏秘密武器。 | ✅ Pass |
| 8 | 汉语学堂一 | School 1 | SCHOOL1.smf | ![汉语学堂一](images/EPOP/SCHOOL1.png) | 中英文双语学习（ELA）软件，全 3D 图画，涵盖服装、水果、室内、身体、头部、野餐六大类。 | ✅ Pass |
| 9 | 汉语学堂二 | School 2 | SCHOOL2.smf | ![汉语学堂二](images/EPOP/SCHOOL2.png) | 中英文双语学习（ELA）软件，全 3D 图画，配以中英文双重语音，涵盖动物、快餐、汽车、数字、颜色、自然六大类。 | ✅ Pass |

### EPUZ — Puzzle Games (24 games)

| # | 游戏名 | 英文名 | 文件名 | 图片 | 游戏介绍 | Status |
|---|--------|--------|--------|------|----------|--------|
| 1 | 坏男孩 | Bad Boy | Bad Boy.smf | ![坏男孩](images/EPUZ/BadBoy.png) | — | ✅ Pass |
| 2 | 铃铛女孩 | Bell Girls | BellGirl.smf | ![铃铛女孩](images/EPUZ/BellGirl.png) | — | ✅ Pass |
| 3 | 小猫快跑 | Cat Run | Cat Run.smf | ![小猫快跑](images/EPUZ/CatRun.png) | 选择喜欢的小猫下注，控制它参加跑步比赛。不同名次获得不同奖金，取决于玩家操作。 | ✅ Pass |
| 4 | CE 城堡 | CE Castle | CeCastle.smf | ![CE城堡](images/EPUZ/CeCastle.png) | — | ✅ Pass |
| 5 | 龙 | Dragon | Dragon.smf | ![龙](images/EPUZ/Dragon.png) | — | ✅ Pass |
| 6 | 仙女博士 | Dr. Fairy | DrFairy.smf | ![仙女博士](images/EPUZ/DrFairy.png) | — | ✅ Pass |
| 7 | 元素大冒险 | Element | Element.smf | ![元素大冒险](images/EPUZ/Element.png) | 俄罗斯方块原型游戏。不同形状的方块需要合理组合，连成横行被收集消除。红色方块增加特殊效果。 | ✅ Pass |
| 8 | 符文之语 | Rune Word | ERuneWod.smf | ![符文之语](images/EPUZ/ERuneWod.png) | 类似于对对碰的游戏。年轻的魔法学徒艾莉进入炼金房修炼，在大魔导师雷斯林的指导下学习一个又一个的魔法。各种符文代表不同属性的魔力。 | ✅ Pass |
| 9 | 食物雨 | Food Rain | FoodRain.smf | ![食物雨](images/EPUZ/FoodRain.png) | — | ✅ Pass |
| 10 | 青蛙 | Frog | Frog.smf | ![青蛙](images/EPUZ/Frog.png) | — | ✅ Pass |
| 11 | 水果派对 | Fruit Party | FruParty.smf | ![水果派对](images/EPUZ/FruParty.png) | — | ✅ Pass |
| 12 | 宝石森林 | Gem Woods | GemWoods.smf | ![宝石森林](images/EPUZ/GemWoods.png) | — | ✅ Pass |
| 13 | 翻翻乐 | Guess | Guess.smf | ![翻翻乐](images/EPUZ/Guess.png) | 古老纸牌翻牌游戏，挑战记忆力。牌面有卡瓦伊小动物。 | ✅ Pass |
| 14 | 躲猫猫 | Hide & Seek | HideSeek.smf | ![躲猫猫](images/EPUZ/HideSeek.png) | — | ✅ Pass |
| 15 | 幸运兔 | Lucky Rabbits | LRabbits.smf | ![幸运兔](images/EPUZ/LRabbits.png) | — | ✅ Pass |
| 16 | 打地鼠 | Mouse | Mouse.smf | ![打地鼠](images/EPUZ/Mouse.png) | 经典打地鼠游戏。地鼠破坏庄稼，玩家拿起武器教训它们。 | ✅ Pass |
| 17 | 木偶 | Mushu Mus | MushuMus.smf | ![木偶](images/EPUZ/MushuMus.png) | — | ✅ Pass |
| 18 | 猴子 | Nau Orang | NauOrang.smf | ![猴子](images/EPUZ/NauOrang.png) | — | ✅ Pass |
| 19 | 果园 | Orchard | Orchard.smf | ![果园](images/EPUZ/Orchard.png) | — | ✅ Pass |
| 20 | 海盗 C | Pirate C | PirateC.smf | ![海盗C](images/EPUZ/PirateC.png) | — | ✅ Pass |
| 21 | 小熊拼图 | Puzzle | Puzzle.smf | ![小熊拼图](images/EPUZ/Puzzle.png) | 经典拼图游戏。游戏开始时打乱图片，经过努力成功后带来成就感和喜悦。 | ✅ Pass |
| 22 | 星空吞吞 | Snake Mania | SnakeMa.smf | ![星空吞吞](images/EPUZ/SnakeMa.png) | 贪吃蛇原型游戏。角色自动前进，吃到食物身体加长，速度加快，碰到边缘或自身则失败。 | ✅ Pass |
| 23 | 数独 | Sudoku | Sudoku.smf | ![数独](images/EPUZ/Sudoku.png) | — | ✅ Pass |
| 24 | 零猎人 | Zero Hunt | ZeroHunt.smf | ![零猎人](images/EPUZ/ZeroHunt.png) | — | ✅ Pass |

### ESPG — Sport Games (3 games)

| # | 游戏名 | 英文名 | 文件名 | 图片 | 游戏介绍 | Status |
|---|--------|--------|--------|------|----------|--------|
| 1 | 篮球 | Basketball | Basketba.smf | ![篮球](images/ESPG/Basketba.png) | — | ✅ Pass |
| 2 | 保龄球 | Bowling | Bowling.smf | ![保龄球](images/ESPG/Bowling.png) | 保龄球游戏，传统规则上增加障碍赛概念，选好位置和力量击倒全部球瓶。 | ✅ Pass |
| 3 | 极速任务 | Express | EExpress.smf | ![极速任务](images/ESPG/EExpress.png) | 类赛车多角色游戏，含城市、高速、荒野、F1 赛道场景，有障碍物和道具。 | ✅ Pass |

### ETAB — Chess/Board Games (4 games)

| # | 游戏名 | 英文名 | 文件名 | 图片 | 游戏介绍 | Status |
|---|--------|--------|--------|------|----------|--------|
| 1 | 幸运 21 | Lucky 21 | Lucky 21.smf | ![幸运21](images/ETAB/Lucky21.png) | 以标准的 21 点赌博游戏为背景。K、Q、J 和 10 牌都算作 10 点，A 牌既可算作 1 点也可算作 11 点，其余所有 2 至 9 牌均按其原面值计算。 | ✅ Pass |
| 2 | 幸运宝盒 | Lucky Box | LuckyBox.smf | ![幸运宝盒](images/ETAB/LuckyBox.png) | 由苹果机、单人梭哈、赌骰子 3 种赌博小游戏组成的棋牌类游戏。苹果机中对水果下注，不同水果有不同赔率。 | ✅ Pass |
| 3 | 天堂 777 | Paradise 777 | Parad777.smf | ![天堂777](images/ETAB/Parad777.png) | 模拟老虎机的游戏，游戏获胜机率从开始高到后来很低。在游戏期间玩家可获得一定的道具，并具有特殊效果。 | ✅ Pass |
| 4 | 长空斗士 | Sky Fighter | SkyFight.smf | ![长空斗士](images/ETAB/SkyFight.png) | 类似于飞行棋的游戏，游戏中需要掷出骰子，得到点数，选择行动的飞机，飞机按点数行走。只有掷出 6 点，才能将一架新的飞机由机场出发。 | ✅ Pass |

### Summary

| Category | Count | Passed | Failed |
|----------|-------|--------|--------|
| Main Menu | 1 | 1 | 0 |
| EACT (Action) | 11 | 11 | 0 |
| EELA (Educational) | 32 | 32 | 0 |
| EPOP (Hot/Featured) | 9 | 9 | 0 |
| EPUZ (Puzzle) | 24 | 24 | 0 |
| ESPG (Sport) | 3 | 3 | 0 |
| ETAB (Chess/Board) | 4 | 4 | 0 |
| **Total** | **84** | **84** | **0** |

## Acknowledgments

- [n32emu](https://github.com/gatecat/n32emu) by Myrtle Shah — the Python reference implementation
- [BootlegGames Wiki](https://bootleggames.fandom.com/wiki/Native_32) — hardware documentation and game catalog

## License

This project is licensed under the [BSD 3-Clause License](LICENSE).
