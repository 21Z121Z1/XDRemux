# 将 Styles 阶段诊断与验收计时隔离

[English](README.en.md) | 简体中文

当前 PR source 为 `1b2c07da2cb2b0c89c64ca546148fe5d68d75a97`，tree `53fc1b202711f2554bc6af0f70b880920c932392`。包括[完整 alpha 保留增量](../8dbe01d/README.md)及本轮诊断；8dbe01d 回执仍独立绑定。main 和四个原 PR head 未变，本证据不批准 merge/delete。

[fc3 原生对照](../fc3f04d/fc3-performance-failure-and-comparison.txt)测得 main/候选 wall median10.381/11.946s、user CPU9.54/11.32s；此前正式 Styles17.005s/p9521.875s仍未通过原绝对预算。不同字段中位数不一定来自同一样本，不能拼成精确分阶段 CPU。相对 main，现有代码增加全分辨率重建/area reduction、局部逆拟合、RGB16→Main10 和安全 singleton Vision 调度；它们在早期 f9 已存在，不能解释之后跨 run 跳变。[实际调用链/源码审查](styles-performance-fc3-followup.md)保留这些区别。

## 诊断范围

仅改既有 Styles runtime、同 runner 比较工具和其测试三文件。`XDREMUX_STYLES_PROFILE=1` 在 stderr 输出固定阶段标签和 elapsed seconds；默认不读 profile 时钟、不写日志。阶段记录无媒体、输入事实或路径，stdout/adapter 协议不变。

原绝对性能 owner/reference/tolerance 和正式 benchmark 不变。每臂 warmup1、5样本、AB/BA 交错不变，正常样本明确删除继承的 profile env。baseline 与候选 workflow 统一 release scope（`--workspace --all-targets`），两臂记录构建命令及 feature graph 原始/源路径归一 hash；canonical/symlink 别名归一不掩盖实际 feature。未修改 main 源码冒充 exact baseline。

全部样本及各自真实 native validation 后，才在独立 owned 目录额外执行一次 candidate-only profile，再独立验证并要求输出 hash 不变。13阶段覆盖 source、base、semantic、decode、scene、thumbnail、local key1、metadata、native properties、Styles assembly、Texture、final validation、publication。它是单次 wall span，不是 baseline 阶段对照、正式预算样本或 CPU 分解；整体 user/sys 仍由真实 `/usr/bin/time` 获取，边界外 dispatch/析构等也可能耗时。

缺失、重复、乱序或非有限阶段收据失败；转换/验证/改写输出失败保留已有阶段、CPU、stderr 和原比较数据，不打印 completed 成功。joblog 直接打印两臂 binary/fixture/build/features 身份及逐样本 CPU/wall/RSS，不依赖 ZIP 才能看关键信息。不削弱算法、采样、真实 Texture 资源、GPU receipt、readback 或门限。

## 验证与恢复

19项 comparison、8项 design、4项 budget 共31通过；新例在缺少 profile/旧构建 scope 上先红。覆盖真实 owned 进程/worktree 清理、symlink cwd child、失败收据、环境隔离、profile validation 及输出改写。Linux 证据不冒充 Darwin；未执行平台项不算通过。

干净 HEAD [7项增量回执](1b2c07d-portable-receipt.json)、[计划](1b2c07d-portable-plan.json)、[日志](1b2c07d-portable-gate.log)通过：metadata、格式、workspace clippy、全部 HEIF/runtime、251项 Python、当前 CLI build 和[真实 Samsung pair 转换/读取](real-cli-proof/manifest.json)。回执 SHA256 `895198be9ef7a493d146ba8997f956108d561a461a69ba93151f24daf7542f4a` 已复核。它是 fc3 之后的增量证据，不是新完整 workspace/native 验收；Styles Rust 为 macOS 条件模块，Linux rustfmt 不能证明原生编译。

[全部21个源恢复 ref](all-21-backup-manifest.json)重新远端 fetch、逐项匹配 commit/tree、[full fsck](all-21-backup-restore.log)通过；9个当前远端分支 head 均有匹配恢复备份。[当前 head manifest](integration-backup-1b2c07d.json)和[恢复日志](integration-restore-1b2c07d.log)先于正常快进 PR source 推送，manifest 保留采集时状态。未删分支、force push、操作上游、发布、使用付费资源或修改预算/保护/系统安全。

[新源码初始检查快照](1b2c07d-checks-initial.json)仅为排队/运行，不是通过：[产品/App](https://github.com/21Z121Z1/XDRemux/actions/runs/39504421318)、[System HEIF/research](https://github.com/21Z121Z1/XDRemux/actions/runs/39504421379)、[performance/profile](https://github.com/21Z121Z1/XDRemux/actions/runs/39504421273)、[portability/packages](https://github.com/21Z121Z1/XDRemux/actions/runs/39504421267)、[exact-head](https://github.com/21Z121Z1/XDRemux/actions/runs/39504421369)、[real ProXDR](https://github.com/21Z121Z1/XDRemux/actions/runs/39504421278)。均由必要 source 更新触发，未手动重复 dispatch。fc3 exact-head 推送前仍运行，若现有 concurrency 取消也不算绿。

父助手已接受 v5 证据并选普通合并 #71 一次的方向：最终 head 全部门禁、最终恢复 snapshot 及执行审核后，保留四个原 PR tip 祖先关系，观察实际间接 merged 状态。不再构造等价中间候选。当前 alpha 原生、绝对性能及新源码门禁仍阻止执行；Photos Tone/Color/edit/save/reopen 和原 Gallery 仍未设备验收。
