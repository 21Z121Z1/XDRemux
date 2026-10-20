# 媒体所有权、有界原生事实与可比较的性能

[English](README.en.md) | 简体中文

本次日期化审查对应整合提交 `fc3f04db604e8d7f72a9dbe1305c5a9829472327`，tree 为 `30658a3a63fb3ebb1907b2555f9b0678e9d98711`。main 仍为 `89d806c8aec961a7991eb3e6d6e2b1370aed8791`，原四个 PR source head 未移动。未合并 PR、删除分支、发布、操作上游、force push、使用付费资源或修改预算、支付及保护设置。父助手的合并/删除门禁仍关闭。

## 发现与自动约束

| 证据或重复错误 | 优先级 | 所有者中的修正 | 可观察验证与边界 |
| --- | --- | --- | --- |
| Live Photo 按文件名清理未知文件/目录。真实成功转换删除了源目录；另一转换可删除活跃 staging 或其父目录。main 与修复前整合的 publisher 字节相同。 | P1 | 删除按名称推断所有权的清理与旧目录删除；runtime 使用自己创建的私有 `TempDir`；公共 reconcile 只检查并保留未知内容。 | [原始发现](livephoto-lifecycle-review.md)、[源目录删除原型](lifecycle-proof-m27chyyj/manifest.json)、runtime 回归和[修复后12项真实 CLI](lifecycle-green-proof-ywva836u/manifest.json)。不能据此认定普通 macOS batch ENOENT 的根因。 |
| 预检后出现的有效 pair 被覆盖；失败 writer 的 rollback 删除另一 writer 的有效 HEIC。 | P1 | 先以原子 no-clobber hard link 取得共享 MOV 名，再发布 still；不替换或删除最终路径。typed `partial_output()` 表示部分发布，runtime 保留完整 staging 供显式恢复。 | [两个真实 CLI 的失败复现](lifecycle-competition-proof-b22ddlu7/manifest.json)；修复后验证同输出、`.heic`/`.heif` 共享 MOV、late competitor、EIO 和非合作替换者。publication9、motion-photo16、runtime 负例5项通过。 |
| PID+时钟值与 `create_dir_all` 被当作独立目录租约；相同读数时一个测试的清理会删除另一个测试。历史 `a84ef27b798b8f6c37de8459f1c5b9d724579269` 已在另一 CLI 测试修过同类问题。 | P2 | batch/i18n fixture 改为原子 `TempDir` 分配和作用域清理，保留并行与路径观测。 | [允许相同时钟值的 RED](tempdir-ownership-legacy.log)、[GREEN](tempdir-ownership-fixed.log)、[原型源码](tempdir-ownership-probe.rs)、[两个并行 batch](owned-tempdir-parallel-batch.log)、[8项 i18n](owned-tempdir-i18n-green.log)。旧 macOS 日志没有路径，不能声称已证明那一次的因果。 |
| System HEIF 已定位唯一 HEVC alpha，但旧诊断没有 subtype 事实。 | P2，原生阻塞 | 拆分重复 alpha 与未知 subtype 错误；只报告有界长度、固定外层形状、两侧 URI/字节是否相等，不输出原始值。非空 subtype 继续拒绝。 | [RED](semantic-alpha-subtype-diagnostics-red.log)、[21项 semantic GREEN](semantic-alpha-subtype-diagnostics-green.log)、[完整 HEIF48+2](alpha-subtype-complete-heif.log)、[含固定官方来源的 diff](semantic-alpha-subtype-diagnostics.patch)。外层封装标签不等于有效 alpha payload 或原生通过。 |
| Styles median15.467s 超过11.732s；p9518.216s 超过15.406s。仅启动 load 不能判为噪声。 | P2，性能阻塞 | 在现有 performance owner 内，于原绝对门禁之后增加隔离的准确 main/候选交错对照；每臂 warmup1、真实 adapter 直接计时5次、轮换顺序，所有计时完成后独立验证。原门禁失败继续失败。 | [失败摘录](b049-performance-failure.txt)、[跨 run 与源码证据](styles-performance-evidence.json)、[调查](styles-performance-investigation.md)。诊断工具9项可移植测试通过；[fc3 原生同机对照](fc3-performance-failure-and-comparison.txt)候选 wall time 高15.07%，user CPU 也增加，原因仍在定位。 |

发布修复明确收窄恢复行为：未知 partial、无效 pair 和看似陈旧的 sidecar 被保留，不再自动删除。第二成员失败时，完整 staging 的 `capture.heic`/`capture.mov` 可独立验证；恢复前须检查两个最终路径。公共 helper 签名保留，覆盖/删除行为被禁止。仓库内唯一产品调用者是 runtime，其原有预检本就拒绝已有最终 pair。

[fc3 原生 subtype 诊断](fc3-system-alpha-facts.txt)现已实测：两侧 alpha 都是同一已知 HEVC URI，长度16，符合记录的外层 SEI 封装，完整 subtype 字节相同。该 head 仍拒绝非空 subtype，因此 System HEIF 仍失败。这是此前缺失的兼容事实，不是原生修复；后续支持须保留完整源 metadata、alpha 编码字节和资源关系，不能归入 fc3，须重新验证新 head。

需要同一文件系统支持 hard link。注入 `EOPNOTSUPP` 证明带原始 I/O 错误的安全失败，不是实际 FAT/exFAT 挂载测试，也不宣称支持这些文件系统。两个路径不是一个系统原子事务；不宣称断电后的目录项持久性，也不能阻止其他 writer 自行主动替换。上述取舍已写入当前中英文架构契约。

## 证据身份

修复后 CLI 原型在记录的 dirty source 上真实构建，再复制快照；manifest 记录产品源文件、二进制、fixture 前后哈希及清理结果。两个产品源文件的哈希与最终提交一致。随后 runtime 负例加强为检查唯一 staging 内恰好两个指定成员，并说明 Unix 文件名能力前提，已再跑5/5。[最终文件哈希](lifecycle-final-files.sha256)记录此变化，不把旧 manifest 改称最终 HEAD 回执。Linux fault 调度和注入 EIO/EOPNOTSUPP 与真实磁盘故障、Apple 原生验证分别报告。

fc3f04d 的干净 committed-HEAD [7项回执](portable-receipt.json)、[计划](portable-plan.json)、[run/verify日志](portable-gate.log)已全部通过：metadata、格式、workspace clippy、完整 Rust workspace（一个既有 ignored 测试）、全部241项 Python 回归、当前 CLI build 和[真实 Samsung pair 双成员独立读取](real-cli-proof/manifest.json)。回执 SHA256 为 `2410bca91802e1b1d7129f0aed2b903e248cd288c90a59f9e5650fa89f34f392`，初始和结束 tracked 状态干净。预提交 targeted 测试仍单列；此可移植回执不证明原生、性能或设备验收。

这里只保留文本原型源码、stdout/stderr，不包含媒体、凭据、可执行 shim 库或明文私人肖像。原型使用仓库公开 Samsung fixture，记录了执行器路径，重放时需适配；它们是审查证据，不是新产品测试框架。

## 已完成的上一 head 检查

[b049 完整检查记录](b049-completed-checks.json)只对应 `b049b7a4bf41934bacf93514e5899567a54000ca`：

- 产品/App 通过，实际 App build/model executable 及新增6项 Swift 回归已执行：[摘录](b049-product-app-evidence.txt)、[run39493275887](https://github.com/21Z121Z1/XDRemux/actions/runs/39493275887)。
- arm64 和 Intel 均通过离开 checkout、仅系统环境下的解包真实转换：[Intel 摘录](b049-intel-package-evidence.txt)、[run39493276049](https://github.com/21Z121Z1/XDRemux/actions/runs/39493276049)。Linux/Windows portability 通过，macOS batch 失败。
- CodeQL、三项 Analyze、real ProXDR、portable research、tone/version/video/CoreML/probe 通过；System HEIF 与性能失败。exact-head 在必要新提交推送时仍运行，被现有 concurrency 设置取消，没有算通过。

相对 f9 的成功 run，Styles、Portrait、UHDR、batch 和独立 VideoToolbox 均约慢1.5倍；非 Styles 仍在各自历史预算内。其参考值来自不同 run/平台代际。这两个事实均应保留，均不证明原因。f9→ba2 的 producer/native/benchmark blob 未改变；后续 alpha graph 检查确实进入 Styles 成功路径，不能未经测量就断言无成本或有因果。新同机对照保留所有样本及准确二进制、fixture、toolchain 身份。main/候选差异包含预期 HDR 改动，不能单凭其比例解释 f9→ba2 跳变。

## 本轮原生结果

[fc3 检查快照](fc3-checks-snapshot.json)记录产品/App、双架构离开 checkout 后真实打包转换、三平台 portability、real ProXDR、CodeQL 与三项 Analyze 均通过。[App model 实际可执行文件通过](fc3-product-app-evidence.txt)，未因前置 Styles 诊断跳过。除 System HEIF 外所有 research 任务通过。快照时 exact-head 仍运行，未算通过。

[绝对性能仍失败](fc3-performance-failure-and-comparison.txt)：Styles median17.005s 超过11.732s，p9521.875s 超过15.406s。之后隔离、交错的同 runner 对照测得 main median10.381279s，候选11.946218s，比例1.150746；user CPU 中位数9.54s→11.32s，system CPU1.05s→1.29s。这是两个独立实验，后者不替代原验收值。已可比较的相对差异要求进一步定位阶段/CPU，宿主 load 不足以归因。未改阈值，失败记录保持失败。

## 持久恢复与临时候选

[全部19个远端恢复 ref](all-backup-manifest.json)已重新独立 fetch，逐一比对 commit/tree，并完成[完整 fsck](all-backup-restore.log)。这些是 GitHub 持久 tags；本任务不移动、删除或改变其服务器保护。远端仍有9个分支。本地原始 `work` 由原 main tag 覆盖，整合分支由当前整合 tag 覆盖。

| 源状态 | 准确 head / tree | 恢复与验收 |
| --- | --- | --- |
| 当前整合 | `fc3f04db604e8d7f72a9dbe1305c5a9829472327` / `30658a3a63fb3ebb1907b2555f9b0678e9d98711` | [Manifest](integration-backup-fc3f04d.json)、[restore](integration-restore-fc3f04d.log)，产品/App 与双架构包通过；System HEIF、绝对性能失败，快照时 exact-head 待完成。 |
| 诊断/fixture 中间提交 | `ab3756746b6482da704f921de207fb12c5bc86a6` / `a486a44c54b8f84bcc6e39e1d5f5a42bcfa817f6` | [Manifest](integration-backup-ab37567.json)、[restore](integration-restore-ab37567.log)，没有作为单独 PR head 推进。 |
| 新 #58 候选 | `6ae70b2b2ffcdc258808cceb5ae8263b2150073c` / `f46af7f3edc11a275b29b745af4328e7e7ebcb5e` | [Manifest](pr58-6ae70b2-backup.json)、[restore](candidates-6ae70b2-6458e3d-restore.log)、[全新独立 target 的10项回执](pr58-6ae70b2-portable-receipt.json)；在旧8263637后加入发布修复。原生/实际服务器祖先关系仍待验收。 |
| 新 #69 候选 | `6458e3d2c7a6da3ae378b867377047bd8a59595e` / `07772237825199fb852ca889fde53799879ce880` | [Manifest](pr69-6458e3d-backup.json)、同一 restore、[全新独立 target 的11项回执](pr69-6458e3d-portable-receipt.json)；以新 #58 为基，合并原 #69，再补 RGB8 mirror/order。原生/实际服务器祖先关系仍待验收。 |
| 旧 #69 候选 | `01cceceb3c94636df7d7857707970c3888635bd4` / `caa6df8f539fe1b2a39f76b81808b9de519506ee` | [Manifest](pr69-01ccece-backup.json)、[restore](pr69-01ccece-restore.log)、[旧9项回执](pr69-01ccece-portable-receipt.json)，不移用给新6458e3d。 |

#58 cherry-pick 仅英文文档上下文冲突：保留原 runtime 段落，只插入发布契约。Git patch ID 不同；[7文件新增/删除行比较](pr58-6ae70b2-delta-equivalence.json)证明安全修复 delta 相同，不声称整树等价或原生通过。推进前必须复查原 PR/main 和并行 writer，不猜测后续服务器 merge SHA。

root 首轮让两个候选 worktree 复用了 Cargo target。新 #69 门禁报源码明确导出的接口不存在；[依赖回执](candidate-shared-target-dependency-evidence.json)显示旧 non-test HEIF 产物未包含 orientation 模块。源码不变，改全新 worktree 专属 target 后11项全部通过。[共享 target 的失败回执](pr69-6458e3d-shared-target-failed-receipt.json)、计划、日志保留；此前[共享 target 的 #58 PASS 回执](pr58-6ae70b2-shared-target-quarantined-receipt.json)明确隔离，不采信。两候选均已独立重建，#58 新计划在每个 Cargo 检查和真实 CLI 路径中固定私有 target。整合分支的完整门禁一直使用自己的 target，没有通过修改产品源码或降低断言遮掩此缓存问题。

原提议顺序仍为 #58→#69→#65→#68→剩余整合，须经父审核并以届时 main 为准。本附件不放行任何合并/删除。保留 strict exact-head、PR、resolved-thread 和 CodeQL 规则。auto-delete-on-merge 已开启，因此每次获准合并前，准确 source head 也必须先有持久备份与恢复证明。最终 SHA 的 System HEIF subtype 支持、绝对性能、产品/App、双架构打包及父恢复审核仍阻止最终收敛。

[独立只读编排审查](merge-orchestration-review.md)另提供需父明确考虑的条件备选：全部最终 source 门禁通过后，仅普通 merge #71，再观察原四 PR 的实际 indirect `merged` 状态。[精确祖先证据](merge-orchestration-ancestry.json)证明四原 tip 均已包含，[当前规则/PR/test-merge 快照](merge-orchestration-github.json)保留身份。GitHub 明确间接 merged 不要求旧 PR 各自门禁通过，因此该方案须明确以最终整合验收替换逐次验收；没有被批准，不代表旧 head 变绿，不改保护或预算，也不是已执行合并。未获父批准则保留原顺序。自动删分支的后果仍须先满足全部准确 head 的恢复条件。
