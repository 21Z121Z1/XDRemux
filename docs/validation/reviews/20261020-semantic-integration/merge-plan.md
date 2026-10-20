# 供父审核的合并与恢复提案

[English](merge-plan.en.md) | 简体中文

本文不授权 PR 合并、分支删除、保护设置变更、force push、上游写入或发布。父审核尚未放行收敛；所有身份为快照，动作前必须重取。产品审查面为 PR71，`f9a419aa4eeb33214e2dfac2205719e6cb97d598`。

## 准确的已观测 head 与拟顺序

| 顺序 | PR | source branch | 准确 head | 正常合并前的语义准备 |
| --- | --- | --- | --- | --- |
| 1 | #58 | codex/liblivephoto-consumer-20261004 | 0688ea7d6270df52cc5968e2c36dd016f13bd31b | 更新 main，保留 pinned consumer，带入主视频/样本、精确时间、未映射 HDR 及适用 App/batch 发布修复。 |
| 2 | #69 | cursor/portrait-heic-orientation-4765 | 7952d0b19c6b47738cc0722acae57465a243186f | 基于实际新 main，保留 coded geometry、完整 mirror/order 和 ImageIO 产品证据。 |
| 3 | #65 | feat/styles-hdr-first-linear-thumbnail-20261015 | bc4d54753f18905dbe73f595a06a6eaf21aaa014 | 基于实际新 main，保留重建后缩小及逐 fixture 诊断、v15、#70 native Texture；排除仅版本号推广。 |
| 4 | #68 | cursor/full-res-linear-hdr-e174 | 1f29e3cdc1b8fd49d56718ceecf96b8de80ec817 | 基于实际新 main，保留完整重建/面积缩小、倒数 gamma、RGB16/Main10、实际 system pair 和图修复，解决真实 native/性能失败。 |
| 5 | #71 | integrate/semantic-review-20261020 | f9a419aa4eeb33214e2dfac2205719e6cb97d598 | 原 PR 处理后核对剩余整合差异与 ancestry，验证实际最终 HEAD；整合通过不能让旧失败 head 变绿。 |

#85 API 404，无开放 PR 可操作。#58/#65 目前 draft，仅在具体可接受 candidate 完成后标为可审查。

这些原始 head **不是可直接合并候选**。准备修复会改变 SHA；strict main 要求意味着后续最终 head 也不能在前一个服务器 merge commit 尚不存在时诚实预报。每一步记录实际新 SHA、新不可变备份/ref/恢复证明、exact-head/native 检查与实际 base。若改变本提案应把具体阶段交回父审核；不可用旧 SHA、PR merge ref 或未验证冲突解决替代。

## 每阶段执行门

1. 只 fetch 用户仓库，比较 remote main/source 与记录，检查所有 worktree 未提交及并行写入；变化需先协调，不覆盖。
2. 独立 worktree 准备最小语义完整 candidate，保留旧 source 为 ancestor 以便 fast-forward；按当前行为解冲突，不整树选旧版本。运行适用 regression/functional/native 并审查。
3. 为准确 candidate 建立全新持久 tag，推送后从远端恢复到新 bare 库，匹配 commit/tree、fsck 并保存日志。绝不移动原始备份；必须早于可能自动删源分支的合并。
4. 重查旧 source 后发布 candidate，再读实际新 SHA；读取现有运行、不重复启动。准确 SHA 的 required/applicable 门禁、strict exact-head、CodeQL、native product 和原性能限制全部完成。
5. 重读默认规则、review threads、main/source、auto-delete 和 writer 状态，解决 threads。只有父审核放行后才以准确 candidate 为 `expected_head_sha` 正常合并。不得绕过/改规则；核实 merge 返回 SHA 和实际远端结果。
6. 下个 PR 使用真实新 main；HEAD 变化使旧 receipt 失效，旧/新证据分别保留。

当前 ruleset 21932863 要求 PR、strict exact-head、threads resolved、CodeQL high/errors，禁止默认分支删除/非快进，不能 bypass。auto-delete-on-merge 为 true，本任务未改任何设置。legacy branch-protection 管理读取被 connector 403 限制；active ruleset 和仓库/分支元数据已通过授权只读 API 获取，不假设有管理权限。

## 最终审核后的清理候选

[九行恢复 manifest](recovery/manifest.json) 保存原始备份。七个非 main 原始分支及整合分支，仅在语义取舍、最终 head 备份和父审核放行后进入删除候选。保留 main 及全部备份 tag。删除前重列分支、比较准确 head；新提交需新备份及审核。未记录分支不在本提案授权内。

| 其他分支 | 准确 head | 独有内容及提案 |
| --- | --- | --- |
| feat/rust-cli-distribution-i18n-20260924 | c6d307af662ed516f6e3994c039513359cb98610 | 六个独有提交含真实 CLI 双语及 macOS 包装。父审核需选选择性移植或不可变备份后明确暂缓；整树导入会丢当前 native targets，且缺 relocation/unpack 后真实转换证据。不是已合并/等价。 |
| research/texture-style | f8fcf2067c506e87665e9ea113324db1a57d4cf7 | 旧固定/别名 carrier 语义已替代，非 ancestry 等价。main 保留原始契约；native-standard matrix SHA256 为 `2bb45a9273a270802da6148b21431666a03dd8d0296915532e396bf197277d4d`。保留备份，不恢复假拍摄/资源 hook。 |
| research/texture-native-20261019 | 18340989590d451f5676924feb03c70baa503e1a | opaque inspect/preserve 仅有诊断/研究消费者，未证明缺少产品消费者。保留独有 147 JSON 和 21 native failure 日志；失败证据不是设备验收。 |

恢复不改 main、不重写历史：

```sh
git fetch origin refs/tags/backup/semantic-review-20261020/cursor/full-res-linear-hdr-e174
git switch -c recovered-hdr FETCH_HEAD
```

准备附件时尚未合并/关闭原 PR，也未删分支。最终 main SHA 和单分支状态仍待实际合并、原生验证及父审核。
