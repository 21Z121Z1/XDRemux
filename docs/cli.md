# XDRemux CLI 参考

[English](cli.en.md) | 简体中文

本文档说明正式用户命令 `xdremux`。实验参数、validator 和内部诊断命令见[开发文档](development.md)。

## 构建与运行

```bash
swift build
swift run xdremux --help
```

仓库仍保留旧脚本入口，便于已有自动化逐步迁移：

```bash
swift xdremux/swift-cli/XDRemux.swift convert --input IMG_001.heic
```

新旧入口调用同一个 package executable，命令、默认值和退出码相同。

## 命令

```text
xdremux convert --input <文件> [--output <文件>] [选项]
xdremux batch --input-dir <目录> [--output-dir <目录>] [选项]
```

`convert` 转换单个文件。`batch` 递归处理目录，并在指定输出目录中保留输入的相对目录结构。

## 公开参数

| 参数 | 适用范围 | 说明 |
| --- | --- | --- |
| `--input <文件>` | `convert` | 输入照片 |
| `--output <文件>` | `convert` | 输出照片；省略时覆写输入 |
| `--input-dir <目录>` | `batch` | 输入目录 |
| `--output-dir <目录>` | `batch` | 输出目录；省略时在输入目录中原位写入 |
| `--glob <模式>` | `batch` | 文件匹配模式 |
| `--jobs <数量>` | `batch` | 最大并发任务数 |
| `--overwrite` | 两者 | 即使已有输出有效也重新生成 |
| `--discard-portrait-data` | 两者 | 不保留原始厂商人像编辑数据 |
| `--oppo-compatible` | 两者 | 生成 OPPO 相册兼容输出 |
| `--apple-photographic-styles` | 两者 | 生成 Apple 摄影风格资源 |
| `--apple-portrait` | 两者 | 生成 Apple 人像资源 |
| `--quiet` | 两者 | 只显示错误和最终结果 |
| `--verbose` | 两者 | 增加逐文件结果、主要路径和跳过原因 |
| `--debug` | 两者 | 增加内部配置、临时路径和完整诊断 |
| `--format text\|json\|jsonl` | 两者 | 选择人类文本或机器输出 |
| `--language auto\|zh-Hans\|en` | 两者 | 选择人类文本语言 |

Apple 摄影风格和 Apple 人像可以组合。`--oppo-compatible` 与任一 Apple 模式互斥，CLI 会在开始转换前返回参数错误。

## 输出模式

默认文本模式显示任务概况、当前进度、警告、失败和最终总结。批量成功时不会为每个文件新增一行。

| 模式 | 输出内容 |
| --- | --- |
| 默认 | 概况、进度、警告、失败、总结 |
| `--quiet` | 错误和最终结果 |
| `--verbose` | 默认内容，加逐文件完成、跳过原因和警告代码 |
| `--debug` | verbose 内容，加内部配置、helper 活动、临时路径和底层错误链 |

`--quiet`、`--verbose` 和 `--debug` 互斥。

## stdout、stderr 与终端行为

- `--help` 写入 stdout。
- 人类可读的进度、警告、错误和总结写入 stderr。
- `--format json` 和 `--format jsonl` 的机器数据写入 stdout。
- stderr 连接交互式终端时使用单个原地进度区域。
- 管道、重定向和 CI 自动使用无 ANSI 控制字符的逐行输出。

批量默认输出不会为每个成功文件打印一行。警告和失败会暂时清除动态进度区域，打印消息后再恢复。

## JSON 与 JSONL

`--format json` 输出一个包含 `events` 数组的 JSON 文档。`--format jsonl` 每行输出一个独立 JSON 对象。

所有机器记录都包含 `schema_version: 1`。字段名、事件名、阶段名、warning code 和 error code 始终使用稳定英文；只有 `message` 可能本地化。

```json
{"schema_version":1,"event":"conversion_failed","error_code":"source_gain_map_missing","input":"IMG_001.heic","message":"输入照片没有可用的 HDR Gain Map。"}
```

当前事件名包括：

- `conversion_started`
- `conversion_progress`
- `conversion_warning`
- `conversion_completed`
- `conversion_skipped`
- `conversion_failed`
- `batch_started`
- `batch_progress`
- `batch_completed`

## 稳定错误代码

| 错误代码 | 含义 |
| --- | --- |
| `source_not_found` | 输入路径不存在 |
| `source_not_supported` | 不支持的输入照片 |
| `source_gain_map_missing` | 没有可用的 HDR Gain Map |
| `source_gain_map_corrupt` | Gain Map 不完整或损坏 |
| `portrait_data_unavailable` | 缺少 Apple 人像所需资源 |
| `apple_runtime_unavailable` | 当前系统缺少 Apple 处理能力 |
| `output_not_writable` | 无法创建或替换输出 |
| `output_verification_failed` | 写入后的输出未通过验证 |
| `internal_container_error` | 不支持的内部容器状态 |
| `invalid_arguments` | 命令或参数无效 |
| `batch_incomplete` | 批量任务存在失败项 |

默认文本只显示用户可理解的原因和恢复建议。`--verbose` 增加错误代码，`--debug` 才显示底层容器诊断和完整错误链。

## 语言

语言选择顺序：

1. `--language`
2. `XDREMUX_LANGUAGE`
3. 系统首选语言
4. 英文回退

支持简体中文标识 `zh-Hans`、`zh-CN`，以及英文标识 `en`、`en-US`、`en-GB`。其他语言暂时回退英文。

JSON 字段、事件名、错误代码、参数名、环境变量、文件名和退出码不会本地化。

## 批量重跑与失败报告

批量输出保留输入相对 `--input-dir` 的目录结构，因此不同相册中的同名文件不会互相覆盖。

重新运行时：

1. 已有且通过轻量验证的输出直接跳过。
2. 无效或不完整输出重新转换。
3. `--overwrite` 强制重新转换。
4. 每个文件先写入同目录临时文件，验证后再原子安装。
5. 单文件失败不终止剩余任务。

失败项写入 `<output-dir>/xdremux-failures.json`。干净重跑成功后，旧失败报告会被删除。批量恢复不使用 checkpoint journal、配置 hash 或 mtime 状态机。

## 退出码

| 退出码 | 含义 |
| --- | --- |
| `0` | 成功 |
| `1` | 内部容器错误 |
| `2` | 命令或参数错误 |
| `3` | 输入缺失、不支持或无效 |
| `4` | 输出或 Apple runtime 错误 |
| `5` | 批量完成，但存在失败项 |
| `130` | 被 Ctrl+C 中断 |

## Python CLI

Python CLI 保留原有 HDR 转换能力，不提供 Apple 摄影风格或 Apple 人像功能。

```bash
pip install pillow-heif Pillow numpy
python3 xdremux/python/XDRemux.py convert --input IMG_001.heic
python3 xdremux/python/XDRemux.py batch --input-dir photo_dump/
python3 xdremux/python/XDRemux.py convert --oppo-compatible --input IMG_001.heic
```

正式 Swift CLI 是新功能和自动化集成的首选入口。
