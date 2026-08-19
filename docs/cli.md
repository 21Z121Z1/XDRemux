# XDRemux CLI 参考

[English](cli.en.md) | 简体中文

本文档说明 `xdremux` 命令行工具。`xdremux --help` 列出命令；`xdremux <命令> --help` 显示该命令的参数和默认值。

## 构建与运行

```bash
swift build
swift run xdremux --help
swift run xdremux convert --help
```

也可以直接用构建产物：

```bash
.build/debug/xdremux convert --input IMG_001.heic
```

## 命令

| 命令 | 作用 |
| --- | --- |
| `convert` | 转换单张照片 |
| `batch` | 递归转换一个目录 |
| `categorize` | 只按拍摄模式归类文件，不做任何转换 |
| `validate-apple` | 检查一个文件的 Apple 摄影风格输出，向 stdout 打印 JSON |
| `validate-portrait` | 检查一个文件的 Apple 人像输出，向 stdout 打印 JSON |
| `portrait-self-test` | 运行人像流水线自检，向 stdout 打印 JSON |

```bash
xdremux convert --input IMG_001.heic --output IMG_001_hdr.heic
xdremux batch --input-dir ~/Pictures/ProXDR --output-dir ~/Pictures/HDR
xdremux categorize --input ~/Pictures/ProXDR --output-dir ~/Pictures/分类
```

## 结果写到哪里

- `convert` 省略 `--output` 时**原地覆盖输入文件**。
- `batch` 省略 `--output-dir` 时写回输入目录。
- `batch --categorize` 先按资产类型写进 `静态照片` / `实况照片`，再按主拍摄模式写进 `人像`、`夜景`、`大师模式` 等子目录；读不出模式的照片进入 `未分类`。
- `categorize` 只复制 HEIC/HEIF/JPEG 文件到这些目录，不修改也不转换任何东西。

## 参数

### 转换参数（`convert` 与 `batch` 通用）

| 参数 | 默认值 | 说明 |
| --- | --- | --- |
| `--input <文件>` | 必填（`convert`） | 输入照片，可以是 HEIC 或人像 JPEG |
| `--output <文件>` | 覆盖输入 | 输出照片 |
| `--oppo-compatible` | 关闭 | 输出 OPPO 相册能显示的 4:2:0 Gain Map，并保留完整 OPPO 私有尾部。不加这个参数时输出标准 ISO HDR，Gain Map 保持源通道结构（可能是 4:4:4）。已经是 4:2:0 的 Gain Map 无法升级回 4:4:4，丢掉的色度找不回来。 |
| `--discard-portrait-data` | 关闭 | 删除体积大的景深和后期编辑资源。水印、大师模式和其他非 HDR 厂商数据仍然保留。 |
| `--oppo-camera-tail <模式>` | `preserve-without-private-hdr` | 保留 OPPO 相机尾部的哪些部分，取值见下方。 |
| `--family auto\|x6\|x7` | `auto` | 源文件使用哪种 ProXDR 数据布局 |
| `--debug-dir <目录>` | 不写 | 保留本次运行的中间产物供检查 |

### 批量参数（`batch`）

| 参数 | 默认值 | 说明 |
| --- | --- | --- |
| `--input-dir <目录>` | 必填 | 输入目录，递归扫描 |
| `--output-dir <目录>` | 输入目录 | 输出目录 |
| `--glob <模式>` | `*.heic` | 挑选哪些文件 |
| `--jobs <数量>` | `min(cpu, 4)` | 同时转换几个文件 |
| `--categorize` | 关闭 | 按资产类型 + 主拍摄模式分目录写出 |
| `--resume` / `--no-resume` | `--resume` | 是否续跑上次的进度 |
| `--skip-existing` / `--no-skip-existing` | `--skip-existing` | 输出已经符合当前设置时是否跳过 |
| `--checkpoint <文件>` | 输出目录下的隐藏 JSONL | 进度记录位置 |

### 归类参数（`categorize`）

| 参数 | 默认值 | 说明 |
| --- | --- | --- |
| `--input <文件或目录>` | 必填，可重复 | 要归类的文件或目录 |
| `--output-dir <目录>` | 必填 | 分类根目录 |
| `--jobs <数量>` | `min(cpu, 4)` | 并发数 |
| `--dry-run` | 关闭 | 只打印计划，不复制文件 |

### Apple 功能（仅 macOS，研究性功能）

| 参数 | 默认值 | 说明 |
| --- | --- | --- |
| `--apple-photographic-styles` | 关闭 | 从照片自身生成 Apple 摄影风格数据，不读取任何 Apple donor 照片。`--apple-styles` 是旧写法。 |
| `--apple-portrait` | 关闭 | 生成 Apple 人像数据。需要照片本身带 `rear.depth`、`rear.depth.config` 和 `src.image`。 |
| `--apple-styles-raw-dng <文件>` | 无 | 配一张对应的 OPPO RAW MAX DNG。不匹配或方向不同的 DNG 会被拒绝，而不是将就使用。 |
| `--apple-style-data-producer <模式>` | `constrained-solver` | 可选 `constrained-solver`、`learn-node`、`identity-fallback`。后两者是诊断用对照。 |

两个功能相互独立，可以同时开启；组合运行时，非人像照片仍会得到摄影风格输出。Apple 输出与 `--oppo-compatible` 互斥。

这些功能**尚未达到可用于正式 Photos 输出的验收标准**。具体哪些结论已经验证、哪些还没有，见 [Apple 功能文档](apple-features.md)。

### 诊断参数

只在排查问题时才需要，正常使用可以忽略。

| 参数 | 默认值 | 说明 |
| --- | --- | --- |
| `--input-processing system\|system-decoded\|hybrid\|passthrough` | `hybrid` | base image 和 gain map 的重建方式 |
| `--tmap-format imageio\|strict` | `imageio` | `strict` 写 145 字节 ISO 形式，实测会让 Find X9 Ultra 相册的 Exif 解析和编辑异常 |
| `--oppo-compat [模式]` | `off` | 更细粒度的 HDR 路由位控制：`auto`、`iso`、`iso-no-local`、`iso-graph`、`on`、`tail`、`off`。只写裸 `--oppo-compat` 等同 `on`，`--no-oppo-compat` 等同 `off`。 |

### `--oppo-camera-tail` 取值

| 取值 | 说明 |
| --- | --- |
| `off` | 不追加任何 OPPO 相机私有尾部 |
| `watermark` | 只保留水印、大师模式预设和拍摄参数 |
| `compact` | 在水印基础上追加紧凑的人像/景深尾部 |
| `preserve` | 逐字节保留完整尾部 |
| `preserve-without-portrait` | 保留其他数据，只移除景深、蒙版、网格和恢复原图 |
| `preserve-without-portrait-or-private-hdr` | 同上，再移除全部私有 HDR 条目 |
| `preserve-without-private-uhdr` | 只物理移除 `local.uhdr.gainmap.data/info` |
| `preserve-without-private-hdr` | **默认**：物理移除全部私有 HDR 条目，保留人像、水印、大师模式等 |
| `preserve-no-uhdr` | 保留全部字节，只在 manifest 中等长改名停用私有 UHDR |
| `preserve-no-hdr` | 保留全部字节，等长停用全部私有 HDR key |

## 输出与退出码

CLI 输出人类可读的文本：进度写 stdout，错误写 stderr。目前没有 JSON 事件流，也没有 `--quiet`、`--verbose`、`--format`、`--language` 这些参数。

| 退出码 | 含义 |
| --- | --- |
| `0` | 成功，或正常显示帮助/版本信息 |
| `1` | 转换、验证、批量执行或其他运行时失败 |
| `64` | 命令行用法错误，例如缺少必填参数、未知参数或参数值无法解析 |

## 批量重跑

`batch` 会在输出目录下写一个隐藏的 JSONL 进度文件，只有整批零失败时才删除。重新运行同一条命令时：

1. 已经符合当前设置的输出直接跳过（`--skip-existing`，默认开启）。
2. 之前失败的文件重新尝试（`--resume`，默认开启）。
3. `--categorize` 写出的资产类型目录（以及旧版拍摄模式目录）不会被当成新输入重新扫描，所以重复运行是幂等的。

## 常见错误

| 提示 | 含义 |
| --- | --- |
| `not a ProXDR photo` | 这张照片没有 OPPO Local HDR 数据。可能是普通 HEIC，或者拍摄时没开 ProXDR。 |
| `already converted` | 这个文件已经带 ISO 21496-1 Gain Map，再转一次不会有任何变化。 |
| `not an OPPO portrait photo` | `--apple-portrait` 需要的景深数据不在这张照片里。 |
| `N file(s) failed to convert` | 批量存在失败项。再跑一次同样的命令只会重试失败的那些。 |

## Python CLI

Python 版本只做 HDR 转换，没有 Apple 摄影风格和 Apple 人像功能。

```bash
pip install -e .
xdremux-py convert --input IMG_001.heic
xdremux-py convert --oppo-compatible --input IMG_001.heic
```

不安装时，从仓库根目录用 `python3 -m xdremux_py` 调用同一套命令。

实现位于根目录的 `xdremux_py/` 包：`cli.py` 负责参数与输出，`pipeline.py` 负责转换，`commands.py` 是解析后的命令模型。安装后的控制台入口是 `xdremux-py`，仓库内直接运行的入口是 `python3 -m xdremux_py`。

需要 Python 3.11 或更高版本。新功能和自动化集成优先用 Swift CLI。
