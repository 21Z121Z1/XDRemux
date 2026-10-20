# 语义整合审查证据 — 2026-10-20

[English](README.en.md) | 简体中文

这是提交给 [PR #71](https://github.com/21Z121Z1/XDRemux/pull/71) 的有日期审查附件，通过独立 Git tag 持久保存，不推进 PR HEAD，也不取消正在运行的检查。它是历史审计快照，不是当前分支操作指令或已完成收敛的声明。父助手明确要求可公开的恢复证据；这里只包含公开 fixture 身份及文本日志，没有私有媒体或凭据。

被验证的产品提交为 `f9a419aa4eeb33214e2dfac2205719e6cb97d598`，tree 为 `02cda5ccb3daead965e35aba121bcc624a4cd63a`。记录的 main 为 `89d806c8aec961a7991eb3e6d6e2b1370aed8791`。附件提交只添加审查材料，SHA 不同；不得把产品提交的通过证据重新标成附件或后续产品 HEAD 的验收。

| 证据 | 可读取的完整文件 | 范围 |
| --- | --- | --- |
| 九个持久备份 ref、commit/tree | [恢复 manifest](recovery/manifest.json) | 八个原始 head 加整合 head。 |
| 原始 manifest，字节未改 | [原始分支](recovery/original-branches.json)、[整合分支](recovery/integration-f9a419a.json) | 保留先前报告的 SHA256 身份。 |
| 远端恢复与 fsck | [原始恢复日志](recovery/original-restore.log)、[整合恢复日志](recovery/integration-restore.log) | 从 GitHub 拉入新建 bare 恢复库，逐项匹配 commit/tree，fsck 返回 0。bare HEAD 尚无分支的提示符合预期。 |
| 九项准确 HEAD receipt | [完整 receipt](verification/portable-receipt-f9a419a.json)、[计划](verification/portable-plan.json)、[run/verify 日志](verification/portable-gate-f9a419a.log) | 九项均为必需项且通过，同时验证工作树干净及 HEAD 未变。仅限 Linux portable 范围。 |
| 完整 portable suites | [Rust 日志](verification/rust-workspace-f9a419a.log)、[Python 日志](verification/python-f9a419a.log) | Rust 318 通过、0 失败、1 项显式忽略；Python 219 通过。macOS cfg 排除后运行 0 个测试不算 Apple 执行。 |
| 真实公开 CLI 转换 | [十四例 manifest 及日志](verification/real-cli-matrix-f9a419a/manifest.json) | 14 个原始 fixture、每对两个成员分别验证。输出哈希保留，仅删除本次拥有的临时媒体。 |
| GitHub 进度快照 | [19:27:56 UTC 检查记录](verification/github-checks-f9a419a-20261020T192756Z.json) | 不可变历史状态，不是最终实时通过声明；后续结果见链接。 |
| 纠正、原型与自动约束 | [证据映射](evidence-map.md) | 风险、实际来源、不变量、收益、检查及证据限制。 |
| 合并与清理提案 | [合并计划](merge-plan.md) | 准确的已观测 head 和停止条件，不赋予合并或删除许可。 |

`SHA256SUMS` 标识除自身外的每个附件文件。在本目录运行 `sha256sum -c SHA256SUMS` 或 `shasum -a 256 -c SHA256SUMS`。八个原始 head 的 manifest 哈希为 `908b9c251b83c9f37ce54103530e944d9d8d1461fc819c8c946621bcced5bd69`；整合 manifest 为 `bcd02d0fe8d39e7864012c517e975e748269d7ff4ea47e2c31229d9056ac198e`；完整九项 receipt 为 `9be89f676ae7974105050c0570b17b0efe82072dd3ab2000326713b131ccbbf7`。

计划保留实际执行命令及当时绝对路径。[recipe 源码](verification/verify-current-cli.py.txt) 作为文本证据保存，没有另加可执行 runner。重新运行时应读当前项目 verification skill，使用现有 completion gate、当前路径/base/源码身份；旧 receipt 对后续 HEAD 应当失效。

## 原生与设备限制

产品 SHA 的现有运行：[产品/App/ImageIO](https://github.com/21Z121Z1/XDRemux/actions/runs/39481011626)、[必需 exact-head](https://github.com/21Z121Z1/XDRemux/actions/runs/39481011433)、[research/System HEIF](https://github.com/21Z121Z1/XDRemux/actions/runs/39481011596)、[性能](https://github.com/21Z121Z1/XDRemux/actions/runs/39481011670)、[跨平台](https://github.com/21Z121Z1/XDRemux/actions/runs/39481011556)、[真实 ProXDR](https://github.com/21Z121Z1/XDRemux/actions/runs/39481011714)。快照时 CodeQL、真实 ProXDR、portable research、tone response、Linux 及 macOS portability 通过；产品、exact-head、Windows 和其他原生 research/性能仍运行或排队。未为附件重复启动检查。

#65 作者曾报告生成文件无法在实际 iPhone Photos 激活 Styles。结构/原生读回或 tone sweep 不证明 Tone/Color 激活、编辑/保存/重开或 Gallery 行为。产物因此明确称为诊断；整合保留 v15 和 #70 按实际照片执行的原生 Texture 路径，没有推广假拍摄参数或仅修改版本号的激活结论。性能阈值和基线未放宽；未把模拟 Metal 故障、未实测 OOM 或设备行为当成真实原生结果。

## 实际采用的 pstack 方法

官方 pstack 固定在 `a66a8931c2dd71f1ed6abf6a52b1898a9f389ae4`；六项项目技能保留 MIT 及来源路径/blob/SHA256。实际阅读并用 `sharpen`、`in-your-own-words` 明确可观测媒体保留目标及证据类别；`correct` 将有证据的重复错误放入现有机器检查；`architect` 在改动前比较现有 owner 和调用形状；`create-verification-skill` 生成并执行真实 CLI recipe。TDD、工程原则、review 和 prose cleanup 用于实际修复及本次证据复核。初次既有 binary 演示和后续准确 HEAD gate 是不同记录。

未引入 Cursor 运行时、bootstrap/Bun、模型路由、外部聊天/工单、shipping/autopilot 或清理权限；无法访问 recall/Slack 历史，也未声称访问过。产品未增加泛化框架、策略引擎或第二套结果台账。官方 DeepSeek `dsh-find-simplifications` 也已实际阅读，保留真实调用者、失败、所有权和可恢复义务。
