# 两个组装 API 完整保留匹配的 alpha metadata

[English](README.en.md) | 简体中文

本日期化增量对应 `8dbe01daf0c3d4d44b670360bd9054ef54552da1`，tree `6ec4f69da7c2b84dc51e6da9789db8cda849a450`，包含 alpha 修复 `18fb51f` 和核验 skill 的三行改动。它承接 [fc3 证据](../fc3f04d/README.md)，不替换旧 SHA 失败的 System HEIF/性能结果。采集时已有持久备份，尚未推进 PR source。

真实原生诊断已测到唯一 source/scaffold alpha：同一已知 HEVC URI、相同16字节 subtype。旧逻辑拒绝全部非空 subtype，因而拒绝了这个可兼容的已识别资源。修复按源元数据原样透传；不把诊断外层封装标签当有效性证明。两个已知 alpha 角色、唯一性、完整记录、几何、变换顺序、premultiplication、GRID 关系、编码 payload 和 codec 要求全部保留。

两侧 subtype 都为空时保留既有 HEVC/MIAF alias 兼容。任一非空时，原始完整 auxC box 必须逐字节一致；version/flags 仍须全0。未知 URI、差异、重复属性或不完整图继续失败。transplant 复用既有 property writer 写入源 auxC，merge 保留源 alpha 图；两个 API 均独立回读最终 auxC 并要求与源完全一致，包括空 alias。source/scaffold 兼容与最终源字节保留是两个不同约束。

固定版本的 [libheif 角色消费者](https://github.com/strukturag/libheif/blob/364905fc9841bdda229e0ae2c5fda13b5fc98d1a/libheif/context.cc#L861) 和 [auxC parser/writer](https://github.com/strukturag/libheif/blob/364905fc9841bdda229e0ae2c5fda13b5fc98d1a/libheif/box.cc#L3376)提供真实透传契约。本改动不解码或验证 alpha SEI，不给猜测的16字节序列开白名单，不伪造元数据，不增加 backend。

## 审查与回归

- **Act on，P2 兼容：**旧公开 API 拒绝同 URI、相同合成16字节 subtype。[RED](semantic-alpha-subtype-passthrough-red.log)在原 unsupported-subtype guard 上失败。新正例覆盖空 alias、两种已知角色、opaque/合成封装、direct/GRID alpha、两个 API、源 payload/hvcC 及图/顺序保留。
- **Act on，P2 共享属性：**复制 alpha auxC 可能误改共享该属性的其他资源；[公开行为负例先红](semantic-alpha-subtype-passthrough-shared-property-red.log)。既有 guard 现按资源及 property kind 约束，并保留未复制 GRID root 的共享 codec 拒绝；不重写 association/reference 顺序掩盖共享。
- **Act on，P2 最终字节：**实际返回产物的 subtype 翻转一字节，box 长度、item/payload/codec 和图不变；独立 source/output 核对必须返回明确差异，两 API 重导入均拒绝。期望来自源契约。
- **Noted：**已识别角色的非空两侧一致证明透传兼容，不证明任意 alpha SEI 有效或设备编辑可用。新 SHA 仍须原生验收。

[最终 HEIF 日志](semantic-alpha-subtype-passthrough-green.log)52项单元测试、2项 fail-closed 全通过；clippy、rustfmt、diff 检查通过。[完整 patch](semantic-alpha-subtype-passthrough.patch)、[作者审查与命令](semantic-alpha-subtype-passthrough-review.txt)均保留。root 在提交前独立读了 diff、调用者、配对/输出边界与回归。冻结 semantic.rs SHA256 为 `5c49185d0f70471f2fb95668fb216aeb9fb9d33f2bd08d3083521a49bf071c4f`。

该 SHA 干净 committed-HEAD [7项增量回执](8dbe01d-portable-receipt.json)、[计划](8dbe01d-portable-plan.json)、[日志](8dbe01d-portable-gate.log)通过；回执 SHA256 为 `d7ee02eeb6dcf7c1d8464a9f657fc1e0446b83d1a210652dc4b4e3e4d858d0a7`。包括 workspace clippy、HEIF、全部241项 Python、当前 CLI build 和真实 Samsung pair 转换/读取；初始/结束 tracked 状态干净，HEAD 未变，下一 worker 编辑前也复核了 receipt。它是 fc3 之后的增量核验，不声称8dbe01d重跑完整 workspace/native。

核验 skill 现在覆盖继承的 `CARGO_TARGET_DIR`，将 CLI 绑定本 worktree target，并要求自定义 plan 每个 Cargo 命令明确绑定；直接回应[已作废的共享 target 事件](../fc3f04d/README.md)。不增加框架、不改产品断言。

## 恢复与剩余门禁

[远端备份 manifest](integration-backup-8dbe01d.json)与[独立恢复/full fsck](integration-restore-8dbe01d.log)证明 `backup/semantic-review-20261020/integration/8dbe01d` 的准确 commit/tree；这是第20个 source recovery tag，另有5个 evidence tag。[v5 恢复日志](evidence-v5-restore.log)确认已发布证据 commit `02c4a2ee161cd0d84788848c262bef50615c03ee`、tree `234fc7c93953ef49620f87eb4b7146a5f77653f9`。

未 merge/delete/release/force push/操作上游/更改保护、预算、支付或使用付费资源。最终原生 System HEIF、产品/App、exact-head、绝对性能、双架构包须跟随实际最终 PR head。Photos Tone/Color/edit/save/reopen 和原 Gallery 仍是设备缺口；父助手合并/删除审核仍必需，本附件不放行批次。

父审核随后认可 v5 恢复、产品哈希匹配的12项预提交 CLI 场景、fc3 干净回执及58/69隔离 target 新回执，并选定较简单收敛方向：最终候选全部门禁通过、最终 snapshot 获审后，优先用一次普通 merge commit 合并 #71，保留四个原 PR tip 的祖先关系，再观察 GitHub 实际间接 merged 状态。不 squash/rebase，不伪称旧 head CI 全绿。这替代继续构造等价中间 PR，不替代执行门禁；**当前仍未批准 merge/delete**。[原祖先/平台行为分析](../fc3f04d/merge-orchestration-review.md)保留。
