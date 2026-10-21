# PVT、原生 Texture carrier 与 Tone 消费路径整合

源码 **000e74b329d7f4f37a60da3523ccec1887e29469**，tree
`f738fcbdd67254195e3933d749a53c6cec57b47b`。可移植验证已通过，原生最终验收尚未完成；
merge/delete 仍 **未批准**。

## 改动与审查

| 已证明的问题 | 修复 | 真实证据及边界 |
| --- | --- | --- |
| 缺少可选实况封装；递归扫描可能拆散已有包或向包内写入。 | `587f253` 增加默认 pair、可选 PVT 强类型意图，复用已验证 pair，以真实三成员目录及字符串版本 plist 排他发布。CLI/App 一致，包、成员、alias、CWD 和 checkpoint 均受保护。 | [整合审查](pvt-parent-review.md)、[Rust冻结](pvt-final-manifest.json)、[App冻结](app-pvt-FREEZE.json)。真实 CLI 竞态五组通过，但该较早 dirty build 不冒充最终 HEAD。Photos 导入和墙纸未验证。 |
| 临时原生 codec 可能同时带 skin mask 和 alpha；旧逻辑还会接受 alpha-only。 | `711213c` 按 auxC/auxl 选唯一严格 skin carrier，独立校验临时 primary/已知 alpha 背景图，保留未知/歧义拒绝及原 source 全资源。 | [HEIF审查](texture-native-skin-carrier-review.txt)，真实RED、61单元+2负例通过；源 alpha/gain/tmap 载荷、属性、标志/顺序及引用保全。原失败日志未记录额外项身份，alpha 仍是等待新原生运行证明的推断。 |
| c379 暴露 Tone 首错：固定研究 parser 将合法 iinf v1 的 u32 计数按 u16 读，丢失条目声明。 | `6d24577` 校验完整源码及补丁 hash，只在自有 launcher 修补该 parser 函数；原 sweep/native/MSL 及缺测门禁保留，不写上游或改产品 metadata。 | [真实Rust writer与完整parser原型](tone-iinf-probe/REVIEW.md)、[字节/role事实](tone-iinf-probe/complete-parser-proof.json)，37近测通过；载荷和引用未丢，恢复3条目及唯一Styles。原生渲染仍待执行。 |
| c379 超过 Styles 绝对预算，粗粒度 scene span 无法区分重建和缩小。 | `000e74b` 增加两项 opt-in 嵌套计时，明确与 coarse 重叠；正式样本、数学及预算不变。 | [性能审查](000e74b-performance-profile-review.md)、[旧失败数值](c379aa7-performance-summary.json)、[跨job/source对照](c379-performance-readonly-review.md)。没有声称性能已修复或已有新原生计时。 |

旧 Snoopy 样本仍不可访问；独立找到的 makelive 格式来源见[前一附录](../c379aa7/pvt-output-research.public.md)，
不能称为字节级复用。PVT 包装已验证 pair，不增加 codec/remux；此前转换仍可能重编码照片或报告厂商资源遗漏。

## 准确提交验证

[7项clean-head收据](000e74b-portable-receipt.json) 全通过，positional verify 退出0。
[计划](pvt-texture-tone-portable-plan.json) 覆盖 metadata、fmt、workspace all-target clippy、
workspace all-target 回归、全部275 Python、当前CLI构建及真实pair/PVT转换、结构读回、解包字节一致和重复拒绝。
[最终CLI证明](final-head-cli-proof/manifest.json) 记录清洁源码、binary/fixture hash、实际命令、
原片保全及scratch清理后证明仍在。recipe 对已有binary保守标为未绑定；紧邻的当前HEAD构建与completion收据另列绑定证据。

Linux明确过滤两项重复JPEG CLI编码：`live_photo_batch_reports_direct_conversion_omissions_and_preserves_source_media`
与`convert_command_executes_full_rust_jpeg_ultrahdr_live_photo_pipeline`，不能称无过滤整套通过。
原生产品CI保留完整套件、vendor CLI、App构建和30项模型调用。较早主动终止的广跑及失败recipe时序均保留，不算通过。
Darwin发布、Windows cfg、GPU、System HEIF/Tone/App、性能及arm64离checkout包均需新head实际运行；Photos导入/墙纸另列。

## 原生状态与恢复

[c379完成快照](c379aa7-completed-native-observation.json) 及原日志保留：产品/App、exact-head通过，
两个App构建与25模型调用实际执行；arm64/macOS27离checkout包及三平台核心通过。
小HDR roundtrip通过，但真实System HEIF Styles fixture仍在旧mask计数处失败；Tone提取及Styles绝对预算失败。
这些不能替代新head验收。

新000e自动运行在[捕获时](000e74b-initial-workflows.json)排队/运行中：
[产品/App](https://github.com/21Z121Z1/XDRemux/actions/runs/39521326147)、
[exact-head](https://github.com/21Z121Z1/XDRemux/actions/runs/39521326263)、
[System HEIF/Tone](https://github.com/21Z121Z1/XDRemux/actions/runs/39521326078)、
[性能](https://github.com/21Z121Z1/XDRemux/actions/runs/39521326241)、
[核心/arm64包](https://github.com/21Z121Z1/XDRemux/actions/runs/39521326082)、
[ProXDR真fixture](https://github.com/21Z121Z1/XDRemux/actions/runs/39521326105)。未重复dispatch，也未取消前一轮尚未完成的诊断。

发布前[追加观察](000e74b-followup-observations.json)保留三个不同首错：macOS PVT resume将同一目录的/var与/private/var字符串比较；真实Linux symlink alias已复现RED，本地7c7773a改为目录身份及全部成员字节一致，8项PVT通过，但不是新的原生通过。产品Rust测试也在此断言失败，两个App步骤明确跳过，30项新模型调用仍待修后执行。

实际System fixture已转换越过Texture，但旧测量shell绕过共用适配，计分时缺tone-1 render；该旧路径未导出内部extractor stderr，不能捏造具体内部异常。主Tone另有确定launcher回归：runpy未保留原脚本目录的模块搜索路径，五个Apple参考均无法导入heif_standalone，未到产品sweep；此前stub测试漏掉此集成。两项消费修复正在同一边界用固定原extractor/helper推进，缺测继续失败、不改上游或合法产品容器。追加捕获时性能仍运行。

711和最终000e均有持久tag，独立取回commit/tree匹配及fsck0；000e在普通推进源分支前备份。
[25ref manifest](all-25-backup-manifest.json) 与[完整恢复日志](all-25-backup-restore.log)证明非浅仓库恢复，覆盖当前9head；
[push记录](integration-source-push-000e74b.json)显示其余8head未变。旧证据及原PR heads仍可从持久ref恢复。

方向仍为最终新head功能门禁通过、更新规则/reviews/heads并获父执行审核后，用expected head000e普通merge#71一次，
保留四原PR tip祖先关系。核对平台实际间接merged及自动删除后，再批准准确剩余ref清理集；不可复用旧禁用hook/操作manifest。
独有研究提交保留tag，不把原head失败CI说成绿。没有删源分支、改保护/安全/预算、写上游或正式发布。
此前PR metadata写入返回未保留的tool error，readback确认未更新，原因未知，未走替代写入路径。
