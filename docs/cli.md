# XDRemux CLI 参考

[English](cli.en.md) | 简体中文

XDRemux 的产品入口是一个跨平台 Rust CLI：`xdremux`。输入类型、Motion Photo / ProXDR 路由以及 HDR / Gain Map 源结构都由程序自动识别；标准转换不要求用户选择格式、设备代际或底层处理策略。

Swift package 只作为 Apple 平台 capability adapter，剩余 Python package 只作为研究/训练工具。它们都不是第二套 XDRemux 实现，不再定义新的 CLI 产品语义或产品 policy。

## 命令

| 命令 | 功能 |
| --- | --- |
| `convert` | 转换一张 ProXDR 照片；支持的 Motion Photo 会自动转换为 Live Photo。 |
| `batch` | 批量发现并转换支持的照片资产。 |
| `categorize` | 不转换，只按资产类型和主要拍摄模式分类。 |
| `inspect` | 检查输入类型和关键结构。 |
| `validate` | 验证 ISO HDR HEIF 或 Live Photo 输出。 |

运行帮助：

```bash
xdremux --help
xdremux convert --help
```

## `convert`

标准转换不需要模式参数：

```bash
xdremux convert --input IMG_001.heic --output IMG_001_hdr.heic
```

对于普通 ProXDR，如果省略 `--output`，输出目标就是输入路径，并使用原子发布替换文件。

对于支持的 Motion Photo：

```bash
xdremux convert --input IMG_001.jpg
```

XDRemux 会自动识别 Motion Photo，并发布匹配的 HEIC + MOV Live Photo pair。Motion Photo 不做原位转换；未指定输出时会选择不会与现有 HEIC/MOV 冲突的新文件名。

### OPPO 相册兼容

需要输出继续面向 OPPO Gallery 时，只使用一个产品级开关：

```bash
xdremux convert \
  --input IMG_001.heic \
  --output IMG_001_oppo.heic \
  --oppo-compatible
```

`--oppo-compatible` 表示“生成 OPPO Gallery 兼容输出”。XDRemux 会根据输入和目标结果自动选择内部 Gain Map 编码、metadata routing 和平台能力。

OPPO-compatible 目前只适用于 ProXDR 静态照片，不能和 Motion Photo → Live Photo 转换组合。遇到这种组合时 CLI 会明确失败，而不会静默忽略参数。

## `batch`

可以重复提供文件或目录：

```bash
xdremux batch \
  --input-dir photo_dump/ \
  --recursive \
  --output-dir converted/
```

也可以重复 `--input FILE`。目录默认不递归，使用 `--recursive` 才进入子目录。隐藏文件和 XDRemux 自己生成的 `.xdremux` 输出不会重新进入发现流程。

常用参数：

| 参数 | 作用 |
| --- | --- |
| `--input FILE` | 添加一个输入文件，可重复。 |
| `--input-dir DIR` | 添加一个输入目录，可重复。 |
| `--recursive` | 递归扫描输入目录。 |
| `--output-dir DIR` | 指定输出目录。 |
| `--jobs N` | 最大并发转换数；必须大于 0。 |
| `--checkpoint FILE` | 指定持久 checkpoint。 |
| `--resume` | 依据 source provenance 恢复已完成工作。 |
| `--skip-existing` | 只在 provenance 与输出身份都匹配时复用已有结果。 |
| `--categorize` | 转换后直接按分类目录发布。 |
| `--oppo-compatible` | 对批次中的 ProXDR 静态照片请求 OPPO Gallery 兼容输出。 |
| `--json` | 输出稳定的机器可读 receipt。 |

batch 会在开始写文件前完成输出规划，避免源文件、HEIC 输出和 Live Photo MOV companion 之间发生路径碰撞。任务之间相互隔离；一个输入失败不会使已经成功发布的其他输入失去结果。

如果混合 batch 使用 `--oppo-compatible`，ProXDR 静态照片按该产品意图转换；Motion Photo 项会作为明确的逐项失败记录下来，不会假装已经应用兼容策略。

## `categorize`

```bash
xdremux categorize \
  --input photo_dump/ \
  --output-dir categorized/
```

`--input` 可以重复，目录会递归扫描。`--dry-run` 只生成计划，`--json` 输出机器可读 receipt。Live Photo 的 HEIC 与 MOV 作为同一个资产处理。

## `inspect`

```bash
xdremux inspect IMG_001.heic
xdremux inspect IMG_001.heic --json
```

`inspect` 用于观察从源文件自动解析出的事实，例如资产类型、HDR 模式、Gain Map 数据量、Motion Photo 视频范围和 presentation timestamp。它不是转换策略配置入口。

## `validate`

```bash
xdremux validate output.heic
xdremux validate output.heic --json
```

`validate` 自动识别并验证 ISO HDR HEIF 或 Live Photo pair，适合脚本和 CI 在转换后做独立检查。

## 产品意图与实现细节

普通 CLI 只暴露会改变用户所需结果的产品意图。源结构识别、重建算法、Gain Map layout、metadata routing、camera tail 和 codec/backend 选择由 engine/runtime 根据输入与平台能力决定，不要求最终用户配置。

需要排障时，应通过 `inspect`、结构化日志和开发测试观察自动决策，而不是把内部策略重新变成命令行参数。

## 退出状态

成功、帮助和版本输出使用 `0`。运行时转换或验证失败使用 `1`。命令行语法或参数错误使用 `2`。

## 机器可读输出

`inspect --json`、`batch --json`、`categorize --json` 和 `validate --json` 提供稳定的结构化输出。默认输出仍以人类可读性为主。

## Apple 功能所有权

Photographic Styles 和 Portrait 通过 `convert` 与 `batch` 的 intent 请求。Rust 负责 policy、orchestration、数据模型、metadata、assembly、validation 和 publication。`xdremux-apple-adapter` 只调用 ImageIO、Vision、Core Image、Core ML、VideoToolbox 等平台 API，并返回事实 observation 或 primitive operation 结果。

Rust 驱动的 macOS gate 会验证结构输出和已测试的 native consumer facts。视觉等价和 Photos 交互行为仍属于独立的真机结论，不能从 parser 或 adapter probe 成功推导出来。
