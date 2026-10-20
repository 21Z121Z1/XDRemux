# 4b产品验收、System HEIF失败与hook竞态修复

[English](README.en.md) | 简体中文

远端PR source仍为 `4b04fb3168cdc9bf8f54d7755db94fd3677eebb6`，tree `a02a3e8fe5533af5aea10b7a10b56ab797b79022`。**System HEIF失败，未获merge/delete执行批准，也未执行。** 正单独补直接HVC1消费路径；未来新head须独立备份、干净receipt及原生门禁，不能继承4b绿灯冒充新源码验收。

用户于2026-10-20 22:58:19 UTC确认后续完整产品仅**arm64/macOS27**，取消新增Intel产品包门禁，保留跨平台Rust核心检查。本4b快照的Intel/macOS15编译下限仅为历史证据，不代表后续产品或SPI支持范围。构建/打包/App/能力/文档改动须在新最终head复验；直接HVC1失败仍是实际阻塞。

## 当前准确源码结果

| 证据 | 真实结果 |
| --- | --- |
| [产品/App job](https://github.com/21Z121Z1/XDRemux/actions/runs/39507240086/job/118919789946)、[原始日志](4b-product-job118919789946.log)、[摘要](4b-product-summary.json) | PASS。Rust workspace harness385过、0败、1项明确real-vendor矩阵ignore。独立CLI/Portrait/OPPO原生/ImageIO方向/Main10 Log/Styles/Texture与App转换真正执行；两App build和全部25项model调用（含新增6回归）完成，打印`XDRemuxAppModelTests passed`。mock CLI负例与实际native转换分开，不声称真实GPU故障或设备UI验收。 |
| [严格exact-head job](https://github.com/21Z121Z1/XDRemux/actions/runs/39507239977/job/118919303603)、[日志](4b-exact-job118919303603.log)、[14项摘要](4b-exact-summary.json) | PASS全部14项及runner内clean-head receipt verify，包含component conformance与明确实物Motion Photo gate。ZIP artifact12305292396存在，执行器未下载，不冒称独立复核其字节。 |
| [性能](../4b04fb3/4b-performance-summary.json) | PASS原绝对门限，Styles median9.142/p959.334秒；同机候选/main比值1.155591保留，不混为跨run因果证明。 |
| [System HEIF job](https://github.com/21Z121Z1/XDRemux/actions/runs/39507240004/job/118919132084)、[完整日志](4b-system-job118919132084.log) | **FAIL**。合成原生System HDR测试1/1通过，真实measurement越过旧alpha拒绝后在`gain-map item10 is hvc1, expected grid`失败。公共validator与Styles根识别仍假定GRID，是直接HVC1消费缺口；不能跳过验证或伪造GRID。本项仍阻止收敛。 |
| [检查状态快照](4b-checks-after-native.json) | 三portable hosts、arm64解包转换、CodeQL/Analyze、realProXDR、portable/CoreML/version/video/probe research通过；arm64及Intel固定依赖打包/checkout外解包转换均通过，[Intel原始日志](4b-intel-job118919419346.log)已保留；快照采集时tone仍运行；后来虽机械绿灯，[原始tone日志](4b-tone-job118919132030.log)显示三个产品extractor全return1、指标/原生render缺失，不能算产品Tone验收。此为新增阻塞，正定位首错及修复缺测绿报；Apple参考数据不能替代产品响应。 |

父已接受v7的22源ref/9分支恢复、fresh fsck、同clean4b磁盘失败/环境修复receipt及43字节比较来源。[v7远端恢复日志](evidence-v7-restore.log)补固定附件commit/tree/fsck。[规则/review/预合并树映射](4b04fb3-merge-rules-and-reviews.json)保留strict exact-head、never bypass、thread解决、自动删分支与相同源码tree；未改保护。

## 父指出的原hook竞态

v7先hash原hook，再exists/按路径exec；中间删除会跳过拒绝hook，替换会执行新字节。root接受该发现。旧16项只证明各自广告OID/remote atomic边界，不能证明原hook执行竞态安全；旧禁用launcher不得用于后续清理。

[冻结修复](hook-race-repair/README.md)移除通用执行/转发。只读核实实际repo core.hooksPath未设置、无pre-push、仅14个普通.sample。新prepare/launcher只支持此状态；所有观察到的真实/未知原hook拒绝，并重核scope配置、目录身份/权限。只执行已hash的guard字节，不执行原hook。保留实际无active原hook的行为，不声称兼容任意脚本原路径语义。

root已独立看四脚本、原始results，复核98项payload hash，并逐字节比较原`validate`，保持既有ref广告验证。[9项真实本地传输](hook-race-repair/evidence/results.json)包括两项旧漏洞RED；修后删除/替换负例拒绝、两ref保持、新旧hook marker均未执行。无active-hook的atomic正例只删两测试ref，随后[所有刻意删除的fixture ref均恢复](hook-race-repair/evidence/matching-empty-hooks/restore.json)。prepare遇active、配置/权限/目录替换负例也拒绝。真实命令、packet与初次注入行号错误纠正记录保留；错误初次尝试不计完整RED。

仅用两已备份tip的1314提交闭包，零新commit。[旧文件保留证明](hook-race-repair/v7-preservation-after.json)匹配113项v7文件；[FREEZE](hook-race-repair/FREEZE.json)/[hash表](hook-race-repair/SHA256SUMS)固定修复。生产模板仍**0644/NOT-APPROVED**，无GitHub ref、生产config/权限或全局安全改动。Git会跳过不可执行hook，禁用审查件不是已安装guard。

权限检查不能锁住同UID在最后检查后的任意写入。本修复只证明移除旧exists/path-exec竞态及拒绝观察到的不支持状态，不是OS级全竞态保证。将来独立批准的操作前须重核effective hooksPath、无active原hook及权限；不符返回父审。GitHub自动删除不经过client hook，仍须事前持久恢复备份；无force/lease/非atomic fallback。

## 审查和操作准备

[汇总review与9分支恢复表](4b04fb3-consolidated-review-and-operation.en.md)列真实缺陷、优先级、修复、不变量/检查与硬件限制。[expected-head操作JSON](4b04fb3-final-operation-review.NOT-APPROVED.json)只是**禁用4b快照**；HVC1修复推进PR后必须更新source字段再审核，不是最终批准。既定方向仍为门禁/父执行审核后普通merge#71，保留四原tip祖先关系，读实际间接merged状态，然后审核自动删除后的准确剩余ref。不开等价中间PR版本，不手工close冒充合并，不预测最终main SHA。

Photos Tone/Color激活、edit/save/reopen和原Gallery仍缺设备验收；全分辨率大内存是设计风险，不冒称测得OOM。源码、最终字节、native与设备证据分开。本纯文本附件无新生成原片、人像、像素buffer或可执行binary。

`hook-race-repair/v7-source-snapshot/`内旧checksum保留v7原root命名空间；完整旧payload位于兄弟`../4b04fb3/cleanup-guard/`，不能在只复制source的snapshot子目录运行旧root索引。外层附件SHA256SUMS覆盖每个复制文件。
