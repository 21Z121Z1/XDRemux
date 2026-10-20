# 保持数值顺序的局部拟合优化与诊断上传约束

[English](README.en.md) | 简体中文

PR source 为 `4b04fb3168cdc9bf8f54d7755db94fd3677eebb6`，tree `a02a3e8fe5533af5aea10b7a10b56ab797b79022`。包括此前 alpha 保留与阶段诊断、`9ddd2c8` 上传范围修正及 `4b04fb3` 局部拟合优化。原 main 和四个原 PR tip 未变。本附件不放行 merge/delete，不宣称最终原生门禁完成。

## 性能失败与有证据的局部改进

优化前 [1b2 原生 job](https://github.com/21Z121Z1/XDRemux/actions/runs/39504421273/job/118909654219)实际失败：Styles median/p95 **17.221/17.955 秒**，原门限 **11.732/15.406 秒**。同构建 scope、同 runner AB/BA 的 main/候选 median **12.742538/15.380543 秒**，比值 **1.207024**，user median11.6/13.9 秒。未剔样本、调门限或判为宿主噪声。[原始日志](local-fit-prototype/native-profile-1b2-job118909654219.log)和[解析摘要](local-fit-prototype/native-profile-1b2-summary.json)保留样本身份。

额外 candidate-only profile 已独立原生验证，wall/user/sys15.346095/14.36/1.87 秒。semantic4.027638、Texture3.926049、base3.327722、local key1 1.486920、scene1.340215 秒。这说明拟合值得优化，但不是基线阶段差，也不能解释早期 f9 与后续 run 的差异；全分辨率/局部拟合/RGB16/安全 Vision 调度在 f9 已存在。即去掉该单次 profile 全部 key1 仍约13.859 秒，不可据此预测其他正式窗口。

独立双 worktree 原型调用现有公开 producer 并读取 release 机器码。最终方案借用单 cell 有界100/30元素 normal/RHS 切片，按固定10/3元素行遍历，每行只算一次 `weight * basis[row]`。乘法左结合、每个累加器更新顺序、f64、输入、权重、全部采样、求解/ridge/clamp 和错误返回均保留，无 unchecked access 或泛化 solver 层。LLVM 已提取 residual，因此删除原型中多余的 residual-array 改写。

Linux x86_64 v2 七对 AB/BA 均更快，median **1.863121→1.143775 秒**，比值 **0.613903**；**43次完整字节直接比较通过**，覆盖26种有效/退化/极端与NaN/无穷/溢出/短缓冲负例。真实公开主图的恢复 target 是合成数据，不是 Apple HDR 目标。[原型报告](local-fit-prototype/REPORT.md)、[v2比较](local-fit-prototype/v2-no-explicit-residual/comparison-report.json)、[构建身份](local-fit-prototype/v2-no-explicit-residual/build-receipt.json)保留独立target/lock/binary及被弃v1的噪声窗口。[产品diff](local-fit-ordered-accumulation.patch)的engine SHA256与冻结v2相同：`b1f58170b7bc6504eb7e1957c481bb8cd6d32e91ccfbbbca5ee0213cb615cc81`。

这些portable producer检查不证明ARM64字节等价。[4b原生job](https://github.com/21Z121Z1/XDRemux/actions/runs/39507239989/job/118919126778)已完整**通过**原绝对门禁：Styles median **9.142秒**、p95 **9.334秒**、RSS1083.9MiB、一次adapter launch。[原始日志](4b-performance-job118919126778.log)/[比较与profile摘要](4b-performance-summary.json)保留结果。同机main/候选median **7.912647/9.143782秒**、比值 **1.155591**、user7.88/8.99秒。额外已验证profile wall9.289624秒，key1 .366361、base2.326812、semantic2.031062、Texture2.818307秒。main与候选都比旧1b窗口快，不能把跨run全部收益归于patch。它证明真实ARM64执行及native输出验证，但不是旧/新ARM64 key1直接字节比较；相对main仍慢15.6%，没有抹去这项差异。

## 诊断 artifact 所有权修正

审查发现旧/当前候选 workflow 的宽目录上传可能包含公开 fixture 转换人像、原始像素字典或缓冲。此前 run 可能已上传此类公开 fixture 派生物。本任务未用私人肖像，也未删除历史 artifact 或声称其不存在。

[workflow改动](ci-media-upload-scope.patch)改用明确报告/日志路径：指定 root/nested log、orientation JSON、类型明确的Texture result、linear-thumbnail回读、评分tone/asymmetry报告及SystemHDR/scalar tone-shape。禁止泛用 `**/*.json`，因为像素字典也可能是JSON。Photos-import媒体与生成HEIF/PNG/人像/raw缓冲留在runner本地；实际生成、前置检查、ImageIO/Styles/readback/native验证、App build/model及断言全部保留。

旧配置[12项RED](ci-media-uploads-red.log)，修后[配置/文档检查通过](ci-media-uploads-green.log)，[40项相关检查通过](ci-media-uploads-nearby.log)。测试按实际配置路径验证允许报告与排除像素/媒体样本。此约束仅适用于未来上传，不追溯修改历史。公开原型仅源码、patch、汇编、hash、统计及文本日志；请求/像素/payload、可执行文件和媒体均未纳入。

## 准确源码验证与磁盘失败

首次干净HEAD[七项回执](4b04fb3-portable-receipt.json)、[计划](4b04fb3-portable-plan.json)、[日志](4b04fb3-portable-gate.log)保留为**失败**。metadata、fmt、workspace clippy、选定engine/HEIF/runtime全部targets、CLI build及真实pair通过；253项Python有一失败：真实 `git worktree add` 在/tmp未能checkout，尚未进入测试预期的build失败场景。

[诊断](4b04fb3-storage-diagnosis.json)记录/tmp99%满、仅115MB可用，源码checkout需278,827,277字节；[独立复现](tmp-full-manual-worktree-reproduction.log)exit128、unable-to-write。源码/断言不变，指定owned workspace TMPDIR后[原测试通过](4b04fb3-worktree-tmpdir-regression.log)。任务所有的污染PR58缓存完整移入workspace隔离区，未删除，旧无效结果仍不计验收。

独立[恢复六项回执](4b04fb3-recovered-portable-receipt.json)、[计划](4b04fb3-recovered-portable-plan.json)、[日志](4b04fb3-recovered-portable-gate.log)通过并verify同一干净4b：metadata、fmt、workspace clippy、全部253项Python、当前CLI build及[真实Samsung pair转换/回读](real-cli-proof/manifest.json)。回执SHA256 `937257e24e8cef9e310675a2610d1741b4bb80d2807c2e02ccd8ebe9599cef73`。Rust engine/HEIF/runtime在首次已通过，未重复昂贵检查；不将两份记录拼成虚构七项全绿。fc3完整workspace证据仍单独绑定，Linux cfg零测试不冒充Apple执行。

## 恢复与原生门禁

[22个源恢复ref](all-22-backup-manifest.json)均从GitHub fetch至独立bare仓库，逐项commit/tree匹配，[full fsck](all-22-backup-restore.log)exit0；当前9分支每个head均有持久备份。[4b manifest](integration-backup-4b04fb3.json)/[恢复日志](integration-restore-4b04fb3.log)先于正常快进PR source。证据tag另计，操作前仍须重核并行写入与准确head。

[清理guard准备](cleanup-guard/README.md)提供明确**NOT-APPROVED**的8非main分支清单与固定hash临时pre-push链。root已看源码、真实前后refs、packet日志并复核110项组件hash。16项真实本地bare传输检查验证广告head不符、广告后server old-ID竞态与atomic拒绝、精确批次删除、main/tag排除、原hook传播及无atomic拒绝。仅用两已备份tip的1314提交闭包，零新commit，无GitHub ref/config/hook改动。这只证明本地机制，不是GitHub能力或删除许可；合并自动删除不经过client hook，仍依赖事前验证备份。最终merge后剩余ref清单与具体命令须重新交父执行审核，禁用模板不能执行。

新source运行：[产品/App](https://github.com/21Z121Z1/XDRemux/actions/runs/39507240086)、[SystemHEIF/research](https://github.com/21Z121Z1/XDRemux/actions/runs/39507240004)、[performance](https://github.com/21Z121Z1/XDRemux/actions/runs/39507239989)、[portability/packages](https://github.com/21Z121Z1/XDRemux/actions/runs/39507239964)、[exact-head](https://github.com/21Z121Z1/XDRemux/actions/runs/39507239977)、[realProXDR](https://github.com/21Z121Z1/XDRemux/actions/runs/39507239974)。[检查快照](4b04fb3-checks-snapshot.json)保留真实时间/状态，待完成原生/App/System/Intel/对照不能由别的绿灯代替。必要代码更新触发新run，未重复dispatch。

父已接受v5旧恢复并选最终门禁、最终snapshot及执行审核后普通merge#71一次。四原PR tip仍为祖先，之后读取实际间接merged状态。未合并/删分支、force push、操作上游、发布或改支付/预算/保护。Photos Tone/Color激活、edit/save/reopen和原Gallery仍缺设备验收。
