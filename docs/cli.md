# XDRemux CLI 参考

[English](cli.en.md) | 简体中文

XDRemux 提供 Swift CLI 和 Python CLI。两套实现都覆盖 HDR、Motion Photo 和分类的主要目标，但命令细节并不完全相同。

## Swift CLI

Swift 可执行文件是 `xdremux`。

```bash
swift build
swift run xdremux --help
```

进行摄影风格工作时建议使用 Release 构建：

```bash
swift build -c release
.build/release/xdremux --help
```

### 命令

| 命令 | 功能 |
| --- | --- |
| `convert` | 转换单张 HDR 照片，或自动转换单张 Motion Photo。 |
| `batch` | 递归处理目录。默认链路可以同时处理普通 HDR 照片和 Motion Photo。 |
| `categorize` | 不做转换，只把照片资产复制到分类目录。 |
| `validate-apple` | 验证摄影风格输出并打印 JSON。 |
| `validate-portrait` | 验证 Apple 人像输出并打印 JSON。 |
| `portrait-self-test` | 运行人像核心自测并打印 JSON。 |

### `convert`

普通 ProXDR HEIC 或 HEIF：

```bash
xdremux convert --input IMG_001.heic --output IMG_001_hdr.heic
```

如果没有 `--output`，普通 HDR 转换会把输入路径作为目标路径。因此普通 HDR 链路可能替换输入文件。

支持的 Motion Photo：

```bash
xdremux convert --input IMG_001.jpg
```

对于支持的 `.jpg`、`.jpeg`、`.heic` 和 `.heif` 输入，Motion Photo 会被自动识别。

Motion Photo 不会原位转换。没有 `--output` 时，XDRemux 会在源文件旁保留一个新的 HEIC 文件名，并生成配套 MOV。如果 `IMG_001.heic` 或 `IMG_001.mov` 已经存在，会选择下一个可用名称，例如 `IMG_001 (2)`。

如果为 Motion Photo 指定 `--output`，静态输出必须是 `.heic` 或 `.heif`。指定的 HEIC/HEIF 或配套 MOV 已存在时，XDRemux 会失败，不会覆盖。

普通 Motion Photo 转换不能在同一遍处理中启用 Apple 人像、摄影风格或 OPPO 兼容输出。单文件 `convert` 另有一个显式启用的 Motion Photo + 摄影风格链路，见 [Apple 功能文档](apple-features.md)。

### `batch`

```bash
xdremux batch --input-dir photo_dump/ --output-dir converted/
```

Swift `batch` 会递归扫描。

没有显式 `--glob` 时，CLI 还会发现支持的 JPEG/JPG 和 HEIC/HEIF Motion Photo，并把它们路由到 Live Photo 转换器。已经生成的 Live Photo 静态 HEIC 不会再次进入普通 ProXDR 转换。

显式 `--glob` 会保留普通 batch 参数解析路径。如果希望自动发现 JPEG Motion Photo，不要使用只匹配 HEIC 的显式 glob。

Swift Motion Photo batch 使用持久状态。默认值：

- `--resume`：开启；
- `--skip-existing`：开启；
- `--jobs`：`min(cpu, 4)`；
- checkpoint：默认位于输出目录下的隐藏 JSONL 文件，除非用 `--checkpoint` 指定其他路径。

只有保存的源文件 provenance 和 Live Photo pair 身份都匹配时，已有资源对才会被复用。来源未知的有效 Live Photo pair 不会被静默归给另一个源文件。

### `categorize`

```bash
xdremux categorize \
  --input photo_dump/ \
  --output-dir categorized/
```

`--input` 可以重复。`--dry-run` 只打印计划，不复制文件。

分类时，通过验证的 Live Photo HEIC 和 MOV 始终作为一个资产一起移动。目录先区分静态照片和 Live Photo，再按主要拍摄模式分类。

### Swift 通用转换参数

| 参数 | 默认值 | 功能 |
| --- | --- | --- |
| `--family auto|x6|x7` | `auto` | 选择源 ProXDR family。 |
| `--oppo-compatible` | 关闭 | 请求自动 OPPO 相册兼容输出。 |
| `--oppo-compat [mode]` | 关闭 | 选择细粒度 OPPO 兼容模式。裸参数表示 `on`。 |
| `--no-oppo-compat` | 关闭 | 强制使用标准非 OPPO 兼容输出。 |
| `--oppo-camera-tail <mode>` | `preserve-without-private-hdr` | 选择 OPPO 私有 tail 保留策略。 |
| `--discard-portrait-data` | 关闭 | 在 tail 策略允许时移除较大的 OPPO 人像/景深编辑资源。 |
| `--input-processing system|system-decoded|hybrid|passthrough` | `hybrid` | 选择 HDR 输入处理分支。 |
| `--tmap-format imageio|strict` | `imageio` | 选择 tone-map metadata 写入方式。 |
| `--debug-dir <dir>` | 无 | 保留诊断产物。 |
| `--apple-photographic-styles` | 关闭 | 启用摄影风格生成。`--apple-styles` 是旧写法。 |
| `--apple-portrait` | 关闭 | 启用 Apple 人像生成。 |
| `--apple-styles-raw-dng <file>` | 无 | 为摄影风格分析提供匹配的 RAW DNG。 |
| `--apple-style-data-producer <mode>` | 启用 Styles 时为 `constrained-solver` | 选择 `constrained-solver`、`learn-node` 或 `identity-fallback`。 |

`--apple-styles-raw-dng` 和 `--apple-style-data-producer` 都要求同时启用 `--apple-photographic-styles`。

Apple 功能和 OPPO 兼容输出互斥。

### `--oppo-camera-tail`

参数解析器接受：

| 值 | 含义 |
| --- | --- |
| `off` | 不附加 OPPO 私有 tail。 |
| `watermark` | 保留水印、大师模式预设和拍摄参数。 |
| `compact` | 保留水印数据和精简的人像/景深 tail。 |
| `preserve` | 保留完整 tail。 |
| `preserve-without-portrait` | 移除景深、mask、mesh 和 restore-original 资源。 |
| `preserve-without-portrait-or-private-hdr` | 移除人像资源和私有 HDR 条目。 |
| `preserve-without-private-uhdr` | 移除私有 UHDR Gain Map 条目。 |
| `preserve-without-private-hdr` | 默认非 OPPO 兼容策略；移除私有 HDR 条目并保留其他支持的厂商数据。 |
| `preserve-no-uhdr` | 保留字节，但原位禁用私有 UHDR manifest key。 |
| `preserve-no-hdr` | 保留字节，但原位禁用私有 HDR manifest key。 |

### Swift 退出行为

正常成功和帮助输出使用退出状态 `0`。

运行时转换失败使用退出状态 `1`。

Swift Argument Parser 自己处理 parser-level usage error。Motion Photo 预路由错误由 XDRemux 入口捕获，并使用退出状态 `1`。编写脚本时，不要假设所有无效命令都属于同一种错误类型。

## Python CLI

Python 可执行文件是 `xdremux-py`。需要 Python 3.11 或更高版本。

```bash
pip install -e .
xdremux-py --help
```

也可以直接从仓库运行：

```bash
python3 -m xdremux_py --help
```

### Python 命令

Python CLI 提供 `convert`、`batch` 和 `categorize`。

它支持标准 HDR 转换、Motion Photo → Live Photo 转换和分类。它不生成 Apple 摄影风格或 Apple 人像数据。

### Python `convert`

```bash
xdremux-py convert --input IMG_001.heic --output IMG_001_hdr.heic
xdremux-py convert --input IMG_001.jpg
```

对于普通 ProXDR 输入，没有 `--output` 时目标路径就是输入路径。

对于 Motion Photo 输入，没有 `--output` 时始终创建新的 HEIC + MOV，并保留源文件。显式指定的 Motion Photo 输出已经存在时会失败。

Python Motion Photo 转换只接受默认转换配置。不要把它和 `--oppo-compatible`、`--reencode` 或 `--debug-dir` 一起使用。

### Python `batch`

没有 `--glob` 时，Python 默认 batch discovery **不递归**，只检查 `--input-dir` 直接包含的文件。它会识别 HEIC/HEIF 和支持的 JPEG/JPG Motion Photo。

Python CLI 中的 `--skip-existing` 和 `--resume` 都需要显式开启。两者都依赖持久 source provenance，只有匹配时才复用 Live Photo pair。

`--checkpoint` 用于指定 Motion Photo 状态文件。

### Python `categorize`

```bash
python3 -m xdremux_py categorize \
  --input photo_dump/ \
  --output-dir categorized/
```

`--input` 可以重复。`--jobs` 默认是 `min(cpu, 4)`。`--dry-run` 不复制文件。

### Python 退出行为

成功使用退出状态 `0`。

命令运行失败使用退出状态 `1`。

Python `argparse` 的 parser-level usage error 使用退出状态 `2`。

## 机器可读输出

Swift 的 `validate-apple`、`validate-portrait` 和 `portrait-self-test` 会把 JSON 写到 stdout。

普通 Swift 和 Python 转换命令输出面向人的进度信息。目前没有通用 JSON event-stream 模式。
