# Styles 性能回归定位与同 runner 对照

本轮先只读核对现有代码与 GitHub 日志，随后按主线程明确批准，仅新增同 runner 对照脚本、接入既有 performance workflow，并添加必要行为测试。没有改产品 producer、资源/验证语义、绝对预算或 baseline；没有启动/重跑 CI，没有 stage、commit、push、merge 或删分支。

## 已有观测

| 产品/原生负载 | f9 成功 run | ba2 失败 run | b049 失败 run | b049/f9 |
| --- | ---: | ---: | ---: | ---: |
| Styles median | 10.323 s | 15.140 s | 15.467 s | 1.498× |
| Portrait median | 3.220 s | 4.309 s | 4.840 s | 1.503× |
| UHDR RGB444 median | 6.823 s | 9.871 s | 10.615 s | 1.556× |
| batch-3 median | 16.021 s | 23.125 s | 24.174 s | 1.509× |
| 独立 VideoToolbox median | 0.213 s | 0.314 s | 0.311 s | 1.460× |

来源为现有 [f9 job](https://github.com/21Z121Z1/XDRemux/actions/runs/39481011670/job/118829264132)、[ba2 job](https://github.com/21Z121Z1/XDRemux/actions/runs/39486770093/job/118849106059)、[b049 job](https://github.com/21Z121Z1/XDRemux/actions/runs/39493276015/job/118871690275)。抽取结果保存在 `styles-performance-evidence.json`。

b049 的 Styles p95=18.216 s，仍超过现有 15.406 s；median 仍超过 11.732 s。RSS=1084.0 MiB 接近 reference 1089.1 MiB。三个 job 的 helper lifecycle 独立观测均为每次 Styles 转换 1 个 adapter 进程。

“其他 case 相对各自 committed reference 主要在预算内”和“相对 f9，多种独立 case 实际一起慢约 1.5 倍”可以同时成立。该 baseline 的其他 case 多继承 macOS 26 的历史 reference；它们不是 f9 的同次实测。故不能因为当前 Portrait≈其 reference 就丢掉 f9 的成功反证，也不能因共同变化就把 Styles 的绝对失败当成噪声豁免。初始 uptime 的高 load 只是一瞬间观测，不覆盖 9 分钟后的 Styles 采样窗口。

三次日志的可比依赖事实一致：xcode-27-arm64 image 20261012.0260.1、macOS 27.0.1 / 26A434、Rust 1.99.1、libheif 1.23.6、x265 4.3；独立 VT 均报告 5/5 hardware samples 和相同 paravirtualized HEVC encoder ID。版本一致不等于同一实际 CPU/GPU 服务吞吐。

## 按实际代码路径排除与保留疑问

`styles-performance-source-blobs.json` 记录 f9、ba2、b049 各源文件 Git blob ID。以下在三者之间逐字节相同：

- Sources/XDTextureNative 的 infer、statistics、readback；Swift adapter main 与 VideoToolboxMain10。
- runtime/apple_styles.rs、apple_texture.rs、apple_adapter.rs；heif/styles.rs。
- benchmark_rust_product.py、characterize_apple_adapter_launches.py、Cargo.lock。

runtime/apple_styles.rs 的三份 blob 均为 `7e1b27eb935b8e90d81a08c9084e0b41f37e7be7`。不能仅凭提交标题将 f9→ba2 的变化归到真实 Texture producer、solver、helper 生命周期或 benchmark 工作量。

唯一相关所有者变化是 heif/semantic.rs。Styles 确实调用其 `transplant_apple_semantic_auxiliary_heif`（runtime/apple_styles.rs:207）。两段变化应分开看：

1. f9→ba2 新增 `semantic_graph_error`，通过 `map_err` closure 懒执行；其 bounded graph 文本诊断在成功路径不构造。这不是每次成功 conversion 都格式化整个 graph。
2. ba2→b049 的 alpha preservation 确实新增成功路径工作：transplant 中先执行 `paired_alpha_images`，再执行 `semantic_items`，并对声明的 semantic auxC 做角色校验。`paired_alpha_images` 会扫描 source/scaffold 的 item/reference/property 图，若确有 alpha，再比较关系、几何/变换顺序及 grid payload。它没有启动额外 Apple producer 或解码/重编码整个 primary。仅凭图通常较小仍不能断言耗时为零；它也解释不了此前 ba2 已出现的 15.140 s。

真实 Styles 事务仍包含：portable HDR base → Vision semantic masks → ImageIO semantic scaffold → Rust graph transplant → full-resolution linear reconstruction/thumbnail/solver → VT thumbnail → scene classification/Styles parser → Texture prepare/infer → 每个人三种原生 effect statistics → Texture graph → native final readback → final MakerApple properties → atomic publication。f9 已运行此实际路径，不应把后来回归说成“首次启用真实 Texture”。

特别值得在需要时细分的既有原生工作是 texture-infer：infer.m 的 mask codec loop 对 13 个类别资源及每个人 instance 分别创建 HEIC destination，编码载体 primary、添加独立 mask、Finalize，再 native auxiliary readback。这是真实 producer 工作；当前没有数据证明它是主因，更不能以复用假 mask/metadata、跳过 readback 或删除 effect 来换速度。

现有 benchmark 为每个测量重新启动 CLI、全新输出目录，真实 adapter 直接进入 timed conversion，warmup=1、iterations=5。它保存 wall/RSS，丢弃逐次 /usr/bin/time 的 CPU 文本；现有 job logs 没有各 adapter operation 的耗时，无法把 5–7 秒精确归到某个阶段。

## 已实施的最小诊断

`scripts/performance/compare_apple_styles_revisions.py` 与既有 performance.yml 的新 step：

- PR 使用本次事件的准确 `pull_request.base.sha`；push/manual 使用 checkout 已 fetch 的 origin/main，立即解析并保存实际完整 SHA。candidate 必须等于 expected HEAD 且 tracked source 干净。没有硬编码未来 main=89d806c8。
- 仅在本仓库创建属于该次调用的 detached worktree。baseline Cargo target 与 Swift scratch 独立；candidate 使用本 job 已在准确 HEAD 构建的 release CLI/adapter。
- 校验两臂 fixture 与各自 SHA256SUMS 一致、两份 fixture 字节身份相同。计时始终共享同一输入路径和真实 adapter，不使用 relay/wrapper。
- 每臂 warmup 1，随后各 5 个样本，按 AB、BA、AB、BA、AB 轮换顺序。每个样本是新 CLI 进程、独立 owned output。
- 保存 SHA/dirty、binary SHA256/size、fixture SHA256、toolchain/host、顺序、每臂转换 stdout/stderr、wall/user/sys/RSS 和 output hash。构建日志独立保留。
- 全部 timed conversions 完成后才分别执行真实 `validate --apple-styles`，记录失败并检查 validation 没有改变 output bytes。验证不进入任何计时样本。
- 超时/异常只终止该命令新建的进程组，并等待直接子进程结束；cleanup 只移除本调用注册的临时 worktree，不 prune 共享 registry。
- report 逐样本原子保存。conversion、native validation、构建或 cleanup 失败都留下 failure receipt，不形成成功 summary。
- baseline==candidate 时明确 `distinct_revisions=false`，说明不能作代码变化归因。即使不同 SHA，main/candidate 也包含旧/新真实算法工作量差异，不能把该 ratio 直接当成 f9→ba2 跨 run 回归的根因。

新 step 位于**现有绝对 budget step 之后**，使用 `if: !cancelled()`。所以不会用 baseline warmup 干扰之前的正式 benchmark，budget 已失败时仍可留下对照报告；后续诊断成功不会覆盖之前的 GitHub step/job 失败。原来的 benchmark、reference、tolerance、结构 launch hard limit 和失败条件没有变化。没有增加新 workflow 或 continue-on-error。

额外成本主要是一次隔离 baseline release build、12 次真实 Styles conversion 和 timed 外 validation；目前不能把尚未在 macOS 跑过的诊断实现说成已得到同机结论。

## 验证与未完成证据

新增 9 项 portable behavior tests 通过：交错样本、Darwin time 形状/缺失收据、validation 排在全部 timed samples 之后、validation 失败收据、只清除 own worktree、同 SHA 不可归因、workflow 失败保留；还实际执行了本仓库临时 detached worktree add→故意 build failure→确认清理，并实际触发短超时后 kill/reap。没有编译 Apple framework。

其中真实 process-group/worktree lifecycle 两项显式限 POSIX；本次在 Linux 实际执行，Windows 应 skip，不能把 Windows 未执行描述为通过。其余纯逻辑检查保持可跨平台执行。

首轮新增 7 项与既有 performance/workflow 29 项一起通过（36）；补两项后新增 9 项独立复跑通过，共 38 个不同检查。本域 3 文件 `git diff --check` 通过。

尚缺同 runner 两臂的实际原生 conversion/validation 与计时收据，因此**根因仍未确定、原失败仍保留**。既有 GitHub job log 读取成功。尝试把下载工具返回的 b049 ZIP file reference 物化到本地时，临时文件域的网络代理返回 HTTP 403；没有继续重试或改变权限。故本报告使用公开 job log 中的汇总，不声称读取了每个原始测量样本。

若两臂结果仍不足，下一次应先只在已存在的 Rust `transplant_apple_semantic_auxiliary_heif` 调用边界计时，以直接量化唯一相关 successful-path owner 的成本；若该边界不足以解释差值，再选 `AppleAdapterClient::texture_primitive` 的单个既有调用边界记录 prepare/infer/person-effect/readback 的 round-trip。任何 profile 都应单独诊断，不纳入正式预算样本、不改变生产输入、资源和失败语义。本轮没有实施 profile wrapper 或这些产品计时点。
