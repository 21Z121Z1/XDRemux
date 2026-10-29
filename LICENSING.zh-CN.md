# 授权说明

XDRemux 中由 `21Z121Z1` 持有版权的原创软件代码，采用 **GNU GPL 第 3 版且仅限此版本（GPL-3.0-only）** 授权。完整法律文本见 [LICENSE](LICENSE)。英文 [LICENSING.md](LICENSING.md) 为本授权范围说明的原始版本。

## 授权范围

GPLv3 涵盖本人拥有版权的 Rust、Swift、Objective-C、Python、构建脚本、研究代码，以及未另行声明授权的原创项目文档。

以下内容**不属于 GPLv3 授权范围**：

- `fixtures/` 中的原始照片和视频：遵循[技术研究样张许可](fixtures/LICENSE.md)，允许必要的格式解析、元数据研究与测试，不开放一般摄影素材使用权。
- `research/oppo_styles/models/*.mlpackage/` 中的 Core ML 模型结构及权重：保留独立版权，见[模型授权说明](research/oppo_styles/models/LICENSING.md)。原创模型卡及训练脚本仍按代码/文档范围处理。
- `.agents/skills/`：保留第三方 MIT 许可证及原作者声明。
- LibLivePhoto（Apache-2.0）、x265、libheif、libde265、libwebp 等第三方组件：维持各自原有授权，不因共同分发而被重新授权。

软件授权不自动授予本人没有的专利权、HEVC 专利池许可、Apple SDK 权利、照片中人物的相关权利或第三方商标使用权。

## 历史版本的追加授权

**自 2026 年 10 月 29 日起**，版权人 `21Z121Z1` 额外允许公众依据 `GPL-3.0-only` 使用以下原有发布标签中本人持有版权的 XDRemux 原创软件代码：

- [v1.0](https://github.com/21Z121Z1/XDRemux/tree/v1.0)
- [v1.1](https://github.com/21Z121Z1/XDRemux/tree/v1.1)
- [v1.2](https://github.com/21Z121Z1/XDRemux/tree/v1.2)
- [v1.3](https://github.com/21Z121Z1/XDRemux/tree/v1.3)
- [v1.3.1](https://github.com/21Z121Z1/XDRemux/tree/v1.3.1)
- [v1.4](https://github.com/21Z121Z1/XDRemux/tree/v1.4)

这是现在提供的**追加使用许可**：不声称旧版发布时已有 GPL 文本，不追溯限制过去的行为，不撤销此前合法取得的其他许可，也不重新授权第三方代码。旧版 README 中“仅供技术研究”的表述是用途及风险提示，不限制本次追加的 GPL 授权。历史版本中存在的照片、模型同样适用上述排除条款。

保留历史标签原貌。分发旧版二进制时，应对应提供准确版本的源码及第三方许可证信息。

## 分发

当前 macOS 包的 `manifest.json` 记录精确提交，`CORRESPONDING_SOURCE.md` 提供源码地址和构建入口。原生依赖源码及许可文本随包提供；Rust 依赖声明从 Cargo 元数据汇总。打包测试通过不等于全部法律合规问题已自动解决。

Copyright © 2026 21Z121Z1（仅限本人持有版权的作品）。
