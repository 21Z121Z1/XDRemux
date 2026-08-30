# Apple 功能

[English](apple-features.en.md) | 简体中文

XDRemux 提供 Apple 特有的摄影风格、Apple 人像和 Apple Live Photo metadata 转换链路。

这些链路与标准 ISO HDR 链路分离。部分组合受到支持，其他组合会在写入输出前被拒绝。

## 平台边界

Swift package 需要 macOS 15 或更高版本。

Apple 特有的分析和渲染会使用 Apple 平台框架和辅助进程。普通跨平台 Python 转换器不生成摄影风格或 Apple 人像数据。

OPPO 兼容 HDR 输出和 Apple 特有编辑输出互斥。

## 摄影风格

启用摄影风格：

```bash
xdremux convert \
  --apple-photographic-styles \
  --input IMG_001.heic \
  --output IMG_001_styles.heic
```

启用摄影风格且没有指定 producer 时，CLI 使用 `constrained-solver`。

参数解析器还接受：

- `--apple-style-data-producer constrained-solver`
- `--apple-style-data-producer learn-node`
- `--apple-style-data-producer identity-fallback`
- `--apple-styles-raw-dng <file>`

producer 参数和 RAW DNG 参数都要求同时启用 `--apple-photographic-styles`。

`learn-node` 和 `identity-fallback` 是诊断或研究控制，不是正常产品默认路径。

提供 RAW DNG 时，它必须满足链路对源照片匹配关系的要求。可选 RAW 输入不可用时，转换会拒绝它，不会静默使用不相关 RAW 数据。

### 验证边界

摄影风格链路会通过仓库验证器检查生成的 HEIC 结构和 Apple style 资源，并在宿主系统支持时使用原生 Apple 组件验证。

Apple 私有接口可能随 macOS 版本变化。仓库会在运行时检查 style-response 工具使用的私有 selector ABI。ABI 形状不受支持时会返回兼容性错误，不会按假定的函数签名调用。

离线结构验证不等于在所有 Apple Photos 版本上完成导入、编辑、保存、退出和重新打开。涉及真机编辑行为的结论必须使用对应的真机证据。

## Apple 人像

启用 Apple 人像：

```bash
xdremux convert \
  --apple-portrait \
  --input IMG_001.heic \
  --output IMG_001_portrait.heic
```

源文件必须包含转换链路要求的人像资源。普通非人像照片不会自动变成人像照片。

存在且有效时，转换可以使用源景深、焦点、光圈、语义和 restore-original 资源。

支持的人像 JPEG 只能通过 Apple 人像链路输入，输出仍然是 HEIC。

成功的人像转换可以在输出旁生成 portrait manifest。manifest 记录转换使用的资源和决策，只用于诊断，不需要导入 Apple Photos。

## 摄影风格 + Apple 人像

静态照片可以同时启用两个参数：

```bash
xdremux convert \
  --apple-photographic-styles \
  --apple-portrait \
  --input IMG_001.heic \
  --output IMG_001_apple.heic
```

启用 Styles 时，Apple feature engine 进入摄影风格链路，由该链路负责 Styles + Portrait 的组合输出契约。

源文件缺少必要人像数据时，不要假设组合请求一定能生成有效的人像编辑数据。

## Motion Photo + 摄影风格

当前 Swift CLI 为这个组合提供单文件独立桥接：

```bash
xdremux convert \
  --apple-photographic-styles \
  --input IMG_001.jpg \
  --output IMG_001_apple.heic
```

这条链路先把 Motion Photo 转换为 Apple Live Photo pair，再给 Live Photo 静态照片生成摄影风格，并验证 Live Photo asset identifier 仍然有效。

该组合不支持在同一遍处理中加入 Apple 人像。

这条组合链路不同于普通 Motion Photo 转换。部分 hosted macOS 版本不会为外部 style-rich HEIC 完成 PhotoKit display-object 请求，因此 hosted style-rich 路径不把 PhotoKit load 当成写入 gate。发布前仍然必须通过确定性的 Live Photo validator 和摄影风格 validator。

普通 Motion Photo 转换继续使用自己的 Live Photo 验证链路。

## 不支持的组合

CLI 会拒绝：

- Apple 功能 + OPPO 兼容输出；
- 普通 Motion Photo + Apple 人像；
- Motion Photo + 摄影风格 + Apple 人像；
- 未启用 `--apple-photographic-styles` 时选择 style producer；
- 未启用 `--apple-photographic-styles` 时提供 Styles RAW DNG。

## 研究控制

仓库包含用于摄影风格研究的环境变量和可选模型路径。

研究控制可能改变 solver 行为或验证范围。只有默认代码路径使用相同配置时，才能把研究结果描述为默认产品结果。

可选 `ReverseKey1Ensemble` 模型见[模型卡](../Models/ReverseKey1Ensemble.model-card.md)。

## 验收

把证据分为三类：

1. 结构证据证明 HEIF 资源和 metadata 存在且可解析。
2. 原生框架证据证明被测试的 macOS 框架接受生成资源。
3. 真机证据证明特定真实设备上的 Apple Photos 版本行为。

如果产品结论涉及 Apple Photos 交互式编辑，不要用结构证据替代真机证据。
