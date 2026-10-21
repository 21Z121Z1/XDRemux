# 已验证参数 HDR kernel 原型

独立候选为 **baa763e4cea14ee79953912b50c90d877ab9d352**，tree
**c0f0b24f3cd77336049da50a499a0867f46153fa**，持久tag
`review/hdr-validated-kernel-20261021-v1`。它只改e107的四个HDR代码/测试文件，
未推进PR71/main、未在macOS运行，**速度收益未知**。

[生产补丁](production-source.patch)增加一个私有数值kernel和小型样本有限性
validator。公开scalar仍按样本→channel→weight检查；整图已在像素循环前验证
所有不变参数，因此改用private kernel，逐分量不再重复参数检查。每次sample/
gain及f64结果非有限检查、原f32溢出拒绝均保留。公开scalar为保留错误优先级，
在kernel内再次检查有限样本。没有改公共签名、采样、精度、像素顺序、HDR-before-
reduction、runtime或Apple SPI。[源码不变量](source-invariants.json)证明数值公式
和f64结果guard原字节不变；整图函数仅换callee和加两行注释，其余恢复为e107。
这不证明编译后更少指令或Mac速度改善。

冻结oracle为e107源文件的16,424字节非测试前缀，SHA256
`a795cd6d0f9f85ce9d8bff9f3911926d455fa4b316cc361d28f16edfb1843053`；
完整来源/blob/hash/cut point见[映射](prototype-map.json)及候选provenance.json。
旧校验、插值、数学及整图循环均独立保留，不调用新kernel，仅共享未改的错误/
结果/scale表示类型。最终与Git原文逐字节核对，rustfmt跳过冻结模块。

生产改动前，新增差分测试及16项原单测已在e107通过；属于行为基线，不伪造
failing-before。准确候选上，实际Linux Rust1.99.1 **debug/release各17单测+6
integration通过**，doc tests为0，rustfmt/diff检查通过。见[最终命令收据](rust-tests/final/report.json)、
[debug](rust-tests/final/debug.log)、[release](rust-tests/final/release.log)。
覆盖1600组scalar、720组整图RGB/mono/padding/缩放/参数组合，有限值按f64/f32
bits比较，无效值按e107错误精确比较，含公开入口优先级、几何、非有限、signed
zero/subnormal、f64非有限及f32溢出。私有kernel单测直接保护逐样本/结果检查。
两个隔离故意错误（gamma、公开校验顺序）均被oracle测试以预期exit101拒绝，
实际候选未改；原始日志和patch保留。

workspace离线命令在无关LibLivePhoto Git依赖解析前置失败，见[原始错误](rust-tests/before/cargo-test.log)，
未改依赖或联网checkout。HDR crate没有依赖，故使用保持edition2021及forbid
unsafe设置、直接指向真实lib/tests的[独立Cargo manifest](rust-tests/isolated-Cargo.toml)。
是真Rust执行，不是全workspace通过。生成收据中的绝对路径描述本次执行；维护
的Rust测试/oracle使用相对路径。容器原无Rust，官方组件按实际manifest hash
验证后仅放工作区自有前缀，未执行远端installer/bootstrap或改系统配置。
[工具链收据](toolchain/preparation-receipt.json)及manifest、formatter证明已留，
不公开归档/二进制；来源为[官方独立安装文档](https://forge.rust-lang.org/infra/other-installation-methods.html)。

本次按项目pstack原则作作者审查，没有独立agent/model。未见额外可证明行为
回归；保留scalar重复有限检查、编译器可能hoist、workspace范围及Mac未验边界，
无SIMD、近似、删功能或泛化框架。[原型发布/恢复原文](prototype-publication-restore-raw/report.json)
核main/候选六套workflow只匹配main且无tag/create/workflow_run，再普通新增tag，
独立准确commit/tree、非shallow/无alternates恢复及strict fsck0。
[全事件查询](hdr-kernel-actions-none.json)该SHA为0次Actions；9heads、PR71/main及
原有脏文件hash未变，没有触发普通或原生CI。

补齐父审要求的准确v29恢复原文：[命令](v29-publication-restore-raw/report.json)、
[恢复/fsck日志](evidence-v29-restore.log)、[ref/tree/index收据](evidence-v29-publication.json)
确实对应 **6e8e77d0db1b318726007447460da619cf3551ee**、tree
**2f364eb0831a9fd2c9675099099d4fbfe925bfd9**、2189项及c919c2d1…清单，
不是旧v28附件。

[未执行的同机计划](same-host-native-comparison-plan.json)固定e107对baa763e4，
一次免费标准macOS27 arm64分配环境，分离fresh CLI builds、同一e107 adapter
hash、feature/源码字节收据、arm64 oracle测试及12次无profile ABBA转换；最多另
两次profile只作定位，不混入E2E。所有输出独立验证，原始hash保全，仅上传文本。
原33bbe86/1a169入口硬编码e107/main89，不能原样用来比较原型；下一轮须先实现、
固定并父审新control/harness/ref，本轮未创建或dispatch。预算/门禁不放宽，失败
保留partial，收益不明显则不能称性能修复。

artifact12367047255仍未独立读回；旧URLError未留底层reason，不称403或权限拒绝，
本轮未重试、换路由或重跑原生。v29已有真实hash/结果继续保留，完整feature graph/
tracked字节/export/build收据仍是独立缺口，不阻塞本次代码原型，也不使Linux
测试变成Mac验收。e107仍18绿2红，全部旧失败保留；operation_enabled=false，
没有merge/delete/force push/上游/发布/安全或预算变更。详见[英文](README.en.md)。
