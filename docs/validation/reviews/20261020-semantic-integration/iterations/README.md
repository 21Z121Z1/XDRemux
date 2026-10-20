# 后续审查证据

[4b原生验收与hook竞态修复附件](4b04fb3-native-and-hook/README.md)记录App/exact-head完成、System直接HVC1新失败，以及收窄禁用guard的真实RED/GREEN/恢复。仍不放行merge/delete。

[4b04fb3 优化与上传约束附件](4b04fb3/README.md)记录真实1b原生profile/失败、保持顺序的producer原型、当前源码portable磁盘失败/恢复、22项源恢复及待完成原生验收；不放行merge/delete。

[1b2c07d 诊断/验收附件](1b2c07d/README.md)记录独立 profile、251测试干净增量回执、21个源恢复及新排队原生检查；[8dbe01d alpha 附件](8dbe01d/README.md)保留元数据与 RED/GREEN。父已选最终门禁后普通 merge #71 一次，仍须明确执行审核，未放行 merge/delete。



[fc3f04d 所有权与可比较性能附件](fc3f04d/README.md)补充干净的完整 workspace 可移植回执、既存 Live Photo 媒体损失 RED/GREEN、保守 subtype 诊断、同 runner 性能对照和重新核实的19个恢复 tag。b049 原生结果仍独立绑定，本附件不放行合并/删除。

[English](README.en.md) | 简体中文

[b049b7a 文件系统 fixture 附录](b049b7a/README.md) 记录后续两项真实 macOS 测试失败及对应修正、全新准确 HEAD receipt 和远端恢复证明。生产源码与 0a2c7c9 相同，新原生检查单独标明身份。

[0a2c7c9 候选附录](0a2c7c9/README.md) 补充分发/i18n 选择性移植、严格 alpha/资源保留、干净准确 HEAD portable receipt 及持久恢复证明，并记录 ba2 App 已完成和性能失败。下方历史观测保留原采集时间与范围。

父审核已接受第一份附件的九个备份恢复/receipt 准备，未放行合并或删除。本附录分别绑定后续两个源码身份并补齐旧 HEAD 的 CI 结果，不重标旧 receipt，也不声明最终整合全绿。

| 源码 | 实际证据 | 尚需门禁 |
| --- | --- | --- |
| `f9a419aa4eeb33214e2dfac2205719e6cb97d598` | [完整已完成检查](f9-final-checks.json)：exact-head、CodeQL、三平台 portability、真实 ProXDR、tone、版本对照、video、CoreML、probe 和性能通过。 | 原生产品诊断及完整 System HEIF 失败；诊断后 App build/model 被跳过，不能记为通过。 |
| `ba2b16918164b8700b57c0bdbaccec7fac5bf9c7` | [完整七项 receipt](ba2b169/portable-receipt.json)、[计划](ba2b169/portable-plan.json)、[run/verify](ba2b169/portable-gate.log)、[备份 manifest](ba2b169/recovery-manifest.json)、[恢复/fsck](ba2b169/remote-restore.log)。 | 新远端产品/App/System/性能检查另行记账，仍在运行。[System 原始错误](ba2b169/native-system-error.txt) 指出源及 scaffold 的额外未知角色辅助项；诊断不等于修复。 |
| #58 首阶段候选 `8263637fa9cb94c34c01892af80a4ecc97f48275` | [完整九项 receipt](pr58-8263637/portable-receipt.json)、[计划](pr58-8263637/portable-plan.json)、[run/verify](pr58-8263637/portable-gate.log)、[Rust 285 通过](pr58-8263637/full-rust.log)、[Python 215 通过](pr58-8263637/full-python.log)、[备份 manifest](pr58-8263637/recovery-manifest.json)、[恢复/fsck](pr58-8263637/remote-restore.log)。 | 未推进原 PR source，尚无该 SHA 的 native/App/exact-head GitHub 验收。原 PR58 仍为 `0688ea7d6270df52cc5968e2c36dd016f13bd31b`，不可仅凭 portable 结果合并。 |

#58 候选的具体链：现有 main+#58 本地 merge `96bb939`，随后 `e88d8e1`（视频/样本/HDR）、`84f34f6`（App 原片/管道/发布）、`9eb305c`（batch no-clobber/遗漏）、`f5dd2d0`（Texture failure/编辑状态）、`8263637`（复用同一 required-state validator）。旧 PR58 与已记录 main 均为祖先。持久 ref 是 `refs/tags/backup/semantic-review-20261020/candidate/pr58-8263637`，tree 为 `dc732f9c449536649e52b58d337c13c28092ffa2`。它只覆盖首阶段，不冒充后续 #69/#65/#68 HDR 行为；main/source 新写入需先协调。后续阶段 SHA 必须基于真实服务器 merge 和 strict 规则产生，不预报不存在的最终 head。

ba2b169 的产品诊断按 file/idat 声明读取真实 v15，不改载体版本。[旧负例](ba2b169/style-diagnostic-before.log) 证明错误拒绝两种 v15、错误接纳 v16，[修复检查](ba2b169/style-diagnostic-after.log) 通过。[图诊断红灯](ba2b169/graph-diagnostic-before.log)和[绿灯](ba2b169/graph-diagnostic-after.log)验证观察有界、不含媒体，且继续严格拒绝 descriptor 不一对一。执行器读取两个临时 artifact 链接时代理 CONNECT 403，未下载字节、未绕过或重试；不是自动审批拒绝。

[性能原始摘录](f9-performance-budget.txt)中 Styles median 10.323 秒，reference 8.691、budget 11.732。main 已有 `benchmarks/macos27-arm64-product-baseline.json` 覆盖联合 Styles/Texture；#68 旧 9.412 秒预算来自加入 Texture 前的工作负载。本任务未改 baseline/tolerance，也未归因为计时噪声。

父审核后来决定选择性移植 distribution/i18n，替代此前整分支暂缓提案。需保留 locale 优先级、JSON/退出码/用户路径、固定依赖、完整相对 dylib 闭包与可复现归档；保留根 Swift package/native targets；验证 checkout 外解包且不依赖 Homebrew 的真实转换。准备本附录时实现及原生证据仍待完成。未授权发布、签新协议、系统安全变更、删分支或合并。
