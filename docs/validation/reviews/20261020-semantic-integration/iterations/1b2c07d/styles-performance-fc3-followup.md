# fc3 Styles 同机对照与阶段诊断

## 已确认的数据及边界

本次读到 [run 39498974839 / job 118891282005](https://github.com/21Z121Z1/XDRemux/actions/runs/39498974839/job/118891282005) 的完整 job log。未取到 artifact 内的逐样本报告；此前临时下载域返回 403，本轮没有重试，也没有把汇总反推为逐样本数据。

正式 budget 窗口的 Styles median=17.005 s、p95=21.875 s，分别超过保留的 11.732 s / 15.406 s 门限。Styles 的转换在 21:30:50–21:32:40 测量。它没有因为后续诊断成功而恢复绿色。

后续同机交错测量在 21:33:12–21:37:17（含独立 baseline 构建）完成：

| 臂 | median wall | median user | median sys | peak RSS |
| --- | ---: | ---: | ---: | ---: |
| main 89d806c8 | 10.38127925 s | 9.54 s | 1.05 s | 1,141,325,824 B |
| candidate fc3f04d | 11.94621825 s | 11.32 s | 1.29 s | 1,135,869,952 B |

candidate/baseline median wall=1.15074625798。该窗口给出了候选总体成本增量的证据；独立 user 中位数也增加 1.78 s。各字段的中位数不保证来自同一个样本，不能将它们组合成精确的配对 CPU 分解。

同一个 candidate 在正式窗口 17.005 s、后续窗口 11.946 s，这个窗口间差别尚未定位。两套脚本都用同一个 public portrait fixture、真实 adapter、全新 CLI 进程和 owned output；正式测量没有 timed 内额外 standalone validate。不能将差别直接归因于验证位置。后续脚本的 Popen timeout wait 有轮询调度差别，亦没有证据表明它解释数秒差值。初始 uptime/load 不覆盖实际测量窗口，不能当成根因或门禁豁免。

## 按实际调用路径确认的新增工作

下面是 `89d806c8..fc3f04d` 的工作量差异，尚不是阶段耗时实测。

1. **全分辨率线性 HDR 重建及两次 area resize。** main 的 `apple_styles.rs:223–224` 先缩小 encoded SDR，再调用 scene reconstruction。fc3 的 `apple_styles.rs:657–710` 对 full RGB8 每个分量做 sRGB decode、Gain Map sampling/application 并保存完整 SDR/HDR f32 数组，随后 `:758–761` 分别 area-resize 两图。`scene_linear.rs:197–218` 是逐源像素/分量 reconstruction，`:265–318` 是 area footprint 累加。fixture primary ispe 为 3064×4080（12,501,120 像素），fit-even thumbnail 为 770×1024（788,480 像素），源像素规模约 15.85 倍。主图的完整 HEIF decode 本来就在 main 路径中，不能把共同 decode 成本也算作新增工作。
2. **局部 key1 拟合。** main `apple_styles.rs:788–810` 生成全局 source/target pair，调用 sampled global polynomial，再把同组系数扩展到 key1。fc3 的 `source_derived_style_data` 调用 `apple_style_revert_local_data`。后者 `apple_photographic_styles.rs:1759–1828` 逐 thumbnail 像素计算 8 个亮度平面权重、最多 4 个空间邻域，向相应 normal/rhs 累加；`:1830–1872` 对 12×9×8 单元、3 个通道分别解 10×10 线性方程（2592 次）。累加部分每邻域/平面包含 55 个 normal 与 30 个 rhs 项，最多约 2.14×10^9 个标量累加项。此上界用于指出应测的循环，不是执行时间估计。顶部的 `APPLE_STYLE_REFINEMENT_MAX_PIXELS=50_000` 属于另一个 refinement/Jacobian 路径，并没有限制这个 local producer 的像素循环。
3. **线性 thumbnail 的精度与原生转换。** main 使用 RGB8/BGRA staging；当前保留 RGB16，Swift `VideoToolboxMain10.swift:289–399` 构造 64RGBALE buffer，再用 VTPixelTransfer 转 10-bit bi-planar YCbCr 后交给 VT。1024 边 thumbnail 的实际转换成本待测，不能从独立 VT 成绩直接扣除整个场景构建的成本。
4. **Texture Vision 请求调度。** `infer.m` 的 heterogeneous segmentation batch 改为逐 request handler；全部角色与失败传播保留，并把 instance mask consumer 绑定到原来的 instance handler。它可能改变原生共享工作/排队，需要真实耗时，不能改回已知崩溃的 compound batch 来试图消除预算失败。statistics receipt 增补保留失败记录、终态 readback，没有新增 effect/setup/GPU 阶段。

系统 HDR producer 是明确的 `XDREMUX_SYSTEM_GAIN_MAP=1` opt-in，当前 performance workflow 没有设置它。不能把默认未执行的 CoreImage producer 当成本次热点。tag84 standalone `validate_existing` 的修复也不等于 convert 增加了一次相同的外部校验；convert 本来就读取最终 MakerApple 字典。

**f9 的反证仍有效。** `git diff f9a419a..fc3f04d -- Sources runtime/apple_styles.rs runtime/apple_adapter.rs runtime/apple_texture.rs engine/apple_photographic_styles.rs hdr/scene_linear.rs heif/styles.rs benchmark_rust_product.py` 对这些准确仓库路径的输出为空。上面四种工作在 f9 已存在，不能据此解释 f9 10.323 s 到 ba2 15.140 s 的跨运行变化。相关成功路径仍有 semantic owner 的 alpha graph 检查差异；该域由独立 agent 检查。f9→ba2 的 bounded error text 通过 map_err 懒构造，成功转换不会生成这些失败诊断。

首层历史对照可选择 main→6f637ae→083b9d3，但不能将其直接当等价输出的 A/B：6f 的 `derive_style_scene_at_resolution` 每个目标像素取 2×2 stratified source samples；083 才进入全分辨率 reconstruction/area 与 local fit/RGB16 producer。中间提交还没有后续 geometry/scene 修复。若确需历史臂，先真实转换及独立验证，记录实际语义差异；不能为计时删验证或回退 producer。

## 获准并实施的诊断增量

仅修改 integration 中三个文件，没有 commit、push、dispatch 或改 workflow/budget：

- `crates/xdremux-runtime/src/apple_styles.rs`：`XDREMUX_STYLES_PROFILE=1` 时在现有边界输出固定标签与 elapsed seconds。默认只做一次环境检查，checkpoint 不读时钟、不写日志。stderr 不输出媒体、路径或输入事实；不碰 stdout/adapter 协议。
- `scripts/performance/compare_apple_styles_revisions.py`：baseline release 构建改为和 workflow candidate 一样的 `cargo build --locked --workspace --all-targets --release`。记录两臂 build 命令和 Cargo feature graph，保留原始日志/hash，另记录只替换各 worktree source prefix 的归一化 hash，避免路径不同伪称 features 不同。归一化按最长优先同时替换 source 与 source.resolve()，覆盖 Darwin /var→/private/var 的实际目录别名边界。原 fc3 构建 scope 不同是可比性限制，当前没有证据认定它已造成实际性能差异。
- `tests/test_styles_revision_comparison.py`：添加行为测试，保留原有 timeout/process-group/worktree 负例。

原绝对门禁不变，5+5 未 profile 交错样本及各臂 warmup 1 不变，正常样本环境显式删除 profile env。全部样本及各自真实 native validation 完成后，才在独立 owned output/logs 中额外运行一次 **candidate-only** profile conversion，随后用真实 candidate CLI/adapter validate，复核 output hash 不变。这个单次 profile 不参与 budget、不参与两臂中位数，也不是 baseline 阶段对照；main baseline 不打补丁、不伪称脏树为 exact SHA。

固定阶段标签及覆盖范围：

| 标签 | 覆盖现有操作 |
| --- | --- |
| source_analysis | convert_file 内源分析及 parent check |
| base_conversion | staging、portable HDR base 及 base file write |
| semantic_production | adapter capability、ImageIO facts、Vision roles、matte encoding、ImageIO scaffold、Rust transplant |
| resource_decode | primary/gain graph checks、primary/gain raster decode（默认路径） |
| scene_derivation | full-resolution linearization/reconstruction、两次 area resize、linear metadata/GTC |
| thumbnail_encode | RGB16 codes、coded orientation undo、VT encode |
| key1_fit | source-derived local key1 producer |
| style_metadata | statistics、light maps、Vision scene scores、plist construction |
| native_style_properties | exact key1/plist 原生 consumer |
| style_assembly | neutral delta verification、HEIF Styles graph/staging |
| texture_generation | prepare/infer/全部 per-person effects/Texture graph/native readback |
| final_validation | final Gain Map structure、final ImageIO MakerApple facts |
| publication | atomic publisher |

阶段是 wall spans，整体 user/sys 仍来自真实 `/usr/bin/time -l`。没有引入 unsafe CPU syscall 或 adapter wrapper。CLI 进入 convert_file 前的读入/dispatch、函数退出后的析构等不全在阶段和中，故不能强制阶段和等于整体 wall。单次额外 profile 也不能重现并解释较早正式窗口的所有环境状态。

joblog 现在直接打印两臂 SHA、CLI/adapter hashes、fixture hash、构建命令/feature hashes，以及每个 warmup/计时样本的 wall/real/user/sys/RSS，避免关键证据只存在不可下载 ZIP。最终打印 profile 阶段、整体 CPU 与 validation status。转换失败保留已完成阶段、可解析的 CPU、错误和 stderr；缺失/重复/乱序/非有限阶段收据失败，验证失败或改写输出失败。整体流程不会因 profile 失败打印 completed success summary；已经有效的 interleaved 数据保留在失败报告中。

下一步先看 `scene_derivation`、`key1_fit`、`semantic_production`、`texture_generation` 的相对规模。只有首层实测仍不能区分时才建议细分单个现有边界；当前不改算法、采样量、真实 Texture inputs、GPU receipt、native readback 或门限。

## 本地验证与冻结

- 新增首轮四项在旧实现上失败：三个因 profile owner 缺失，一个因 baseline build 仍使用 CLI-only scope。随后实现通过。
- `python3 -m unittest tests.test_styles_revision_comparison tests.test_performance_design tests.test_performance_budget -v`：31 checks PASS（19 comparison / 8 design / 4 budget）。包含真实 Linux owned worktree cleanup、POSIX 子进程超时回收及真实 symlink cwd/child 观测。后者证明相同 features 的 canonical/alias 路径 hash 归一后相同，而真正 feature 文本变化仍不同；没有把 Windows skip 算通过。
- `rustfmt --check --edition 2021 crates/xdremux-runtime/src/apple_styles.rs`：PASS。
- `git diff --check`：PASS。
- Linux `cargo tree --locked --offline --workspace --edges features --prefix none --depth 0`：命令可运行，仅验证 graph 命令接口；没有冒充 Darwin feature graph 或原生 producer。

冻结的 SHA256：

```text
d63104091cdd605f4842750eced0f8132dae0ad8a5ded76efa8bb5db698c353c  crates/xdremux-runtime/src/apple_styles.rs
894827a1f94e88fcb59d064439f87da3ab80cc94504ad9787dd8a4855d8f8932  scripts/performance/compare_apple_styles_revisions.py
d4ed6f3c48abbce2d29e29d08d5c330e9192d297113393a7cc4cf34e16a5d233  tests/test_styles_revision_comparison.py
```

Apple code编译、真实额外 conversion/validation、stage wall/整体 CPU 收据仍待 macOS CI。此环境没有该原生能力；本轮没有再次启动 CI，也没有声称根因已经确定。
