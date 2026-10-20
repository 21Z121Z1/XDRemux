# 辅助资源保留与分发功能选择性移植

[English](README.en.md) | 简体中文

源码 `0a2c7c94765a6fd52a83331d66b74f26259d1141`、tree `f8d3a07de6745a3279517bb83e84b5fb6bd3eba2` 是新的整合候选。[持久备份](recovery-manifest.json) 已从 GitHub 拉入独立 bare 恢复库，[日志](remote-restore.log) 逐项匹配 commit/tree，fsck 返回 0。原始 PR source 分支未推进，也未合并或删除分支。新候选仍需自己的原生 CI，旧 SHA 的通过结果不能算作它的验收。

| 提交 | 已证明的问题及形成的约束 | 证据 |
| --- | --- | --- |
| `0facc0a` | 共享 storage resolver 的导入破坏既有研究 CLI 的直接文件入口。现在直接文件和 module 两种入口均保留原有单字节版本探针行为。 | [修复前](style-probe-entrypoint-red.log)、[修复后](style-probe-entrypoint-green.log)。 |
| `422781d` | 分发代码和依赖 JSON 原可获得非产品 receipt。现有门禁现要求回归及功能证据；包内普通说明文档仍按文档处理。性能环境记录补 CPU/核心/内存及负载，预算未改。 | [修复前](distribution-gate-red.log)、[15 项门禁回归通过](distribution-gate-green.log)。 |
| `45d7bac` | 真实 System HEIF scaffold 多出一个尚未在原生环境确认身份的辅助图。组装器只处理精确 HEVC/MIAF alpha 角色和唯一 source/scaffold 配对，保留源压缩 alpha、hvcC、tile 顺序及兼容的几何/变换。未知、歧义、仅源端存在的辅助资源明确拒绝；共享 codec 会影响未替换项时也拒绝。 | [alpha 初始失败](semantic-alpha-red.log)、[身份/关系失败](semantic-alpha-identity-red.log)、[源未知资源丢失](semantic-alpha-source-unknown-red.log)、[19 项 semantic 回归](semantic-alpha-green.log)、[clippy](semantic-alpha-clippy.log)。原生身份仍待确认，merge helper 通过不能代替 runtime transplant 验收。 |
| `0a2c7c9` | 按父助手决定移植旧分发分支的有效 CLI 语言和打包功能。语言只在 CLI 展示层，机器 schema、退出码、现有 feature 参数和用户路径保持契约。保留当前根 Swift native targets。打包采用固定源码、相对 Mach-O 闭包、原始源码/notice、规范化归档，并要求在 checkout 外以隔离系统环境解包真实转换。 | [六提交取舍及完整审计](distribution-i18n-port.en.md)、[20 文件哈希](distribution-i18n-files.json)、[依赖观测](distribution-source-review.json)、[43 项策略/打包/文档检查](distribution-staged-policy.log)、[8 项进程回归](i18n-process-green.log)。arm64/Intel 包真实执行仍待新 CI。 |

[八项完整 receipt](portable-receipt.json)、[计划](portable-plan.json) 和 [run/verify 日志](portable-gate.log) 均绑定干净且未改变的 `0a2c7c9`。receipt SHA256 为 `fb354850ab625a27111f75fae20e2d92a5f9c9973a4f9d51c9bd897a6439a66f`。覆盖 metadata、格式、整个 workspace clippy、46 项 HEIF library 加 2 项边界回归、28 项 CLI unit 加 8 项进程回归、全部 232 项 Python、当前 CLI build，以及真实 Samsung 转换和两个成员分别验证。[真实 CLI 证明](real-cli-proof/manifest.json) 保留源/输出/binary 哈希、命令、日志及清理。recipe 独立标记 binary 为既有文件；准确 HEAD 计划中紧邻其前的 build 建立该 receipt 的源码绑定。完整 workspace 及 Apple 原生 suites 仍是单独 CI 义务。

首次本地计划遗漏 `XDREMUX_VERIFY_CLI`，保留了[失败 receipt](first-failed-receipt.json) 与[日志](first-failed-gate.log)。它在转换前抛出 `KeyError`，不是产品错误；修正计划显式传入 binary 路径后完整重跑八项。[补充的 CLI 全套 49 项通过](i18n-full-cli-supplemental.log) 起于旧 dirty 工作树，跨过其他 root 提交，不改标为最终 HEAD receipt。Unix 路径测试也纠正了过强假设：既有 JSON 序列化拒绝非 Unicode `PathBuf`，human inspect 仍能读取；没有为了测试改生产 JSON 编码。

在 [20:16:17 UTC 快照](ba2-checks-snapshot.json)，`ba2b169` 产品/App 已完整成功，包含 Styles 诊断、原生 Texture 及 App bridge、App build，以及包含新增六项 Swift 行为回归的模型 executable。[真实日志摘录](ba2-product-excerpt.txt) 最后为 `XDRemuxAppModelTests passed`；[完整运行](https://github.com/21Z121Z1/XDRemux/actions/runs/39486770103)。exact-head 和部分 research 当时仍在运行；System HEIF 和性能仍失败。

[性能对比](ba2-performance-comparison.json) 保留两个源码身份、run 链接及全部 workload 汇总。Styles median 15.140 秒超过 11.732 秒预算，p95 15.859 秒超过 15.406 秒；多个独立 portable workload 相比 f9 同步变慢。宿主吞吐差异只是待验证假设，不是根因证明或豁免。未改基线/容差、未排除失败样本、未重复启动运行中的检查。

本执行器的 libwebp archive 和两个临时 artifact 下载因 HTTPS proxy CONNECT 403 未取得字节；未绕过或重试同一路径，也未声称校验了缺失源码。这是网络路径失败，不是自动审批拒绝。新 CI 必须校验固定依赖，在两种标准 macOS 架构上真实构建/启动/解包/转换。通用包和 adapter smoke 不证明 Apple 私有 feature 或设备 Photos 编辑。新源码原生/产品/性能检查及最终恢复审核完成前，父助手的合并/删除门禁仍关闭。
