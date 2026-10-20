# 直接 Gain Map 校验、产品范围与 Tone 证据边界

本附录固定源码 `c379aa7c4d1f6fcad8d2893837ee20e4ae7dd1d5`，tree
`f618057269722e87be12636b46f5292c5e0d3baf`。这是观测中间 head，尚非最终验收。
合并与删除仍 **未批准**；未更改保护、安全、预算、发布或上游设置。

## 本轮修复与真实验证

| 已证明的问题 | 改动 | 证据边界 |
| --- | --- | --- |
| System HEIF 在通过此前 alpha 边界后，错误拒绝合法直接 hvc1 Gain Map。 | `eca4bf9` 为 GRID/直接 HVC1 统一完整图、codec、精度、几何和载荷校验；Styles 使用同一入口，保留资源、颜色及字节。 | 三组 RED；57 单元+2 fail-closed、clippy/fmt 通过。没有伪造 GRID 或重编码；新 head 原生 System HEIF 待 CI。 |
| 用户明确产品范围为 arm64/macOS27，旧配置仍有 Intel 包及较旧下限。 | `2ea21ed` 统一 Package/App/adapter host 检查、固定依赖构建、归档和文档。 | 配置与主机负例实际先红后绿，46 项近测通过；两个 native targets、闭包校验、离开 checkout 的真实转换保留，三平台 Rust 核心 CI 与固定依赖 manifest 未变。模拟 Mach-O 不算原生打包。 |
| 旧 Tone 绿色 job 实际三个产品 extractor 退出1，产品渲染/数值全缺。 | `c379aa7` 要求真实完整的进程、渲染与比较数据，保留首错及原日志哈希。 | 对不可变4b owner 的4个行为方法产生6次断言失败且无 setup errors；当前15 focused+19 workflow 通过。记录 SPI 状态整数而不猜其成功值域；Apple trusted 不再掩盖产品缺测。 |
| 原生 probe 日志会打印多行完整 facts 字典。 | 只导出字典前 stderr 与小型 typed exception，排除 extractor inventory 和最终 facts。 | secret-marker 负例验证无原始字典泄漏且保留首错；没有声称旧 artifacts 从未含公开 fixture 衍生物。 |

[作者审阅](c379aa7-author-review.json)、[HEIF 审阅](direct-gain-map-structure-review.txt)、
[打包审阅](arm64-macos27-distribution-review.md)、[Tone 审阅](tone-response-fail-closed-review.md)
列出来源与范围。测量完整不证明 Photos Tone/Color 编辑、保存重开或真机行为。
旧 extractor 内部首错没有被日志保存，本提交为下一原生运行补诊断，不能说该功能故障已修好。

## 准确提交与恢复

[7项可移植收据](c379aa7-portable-receipt.json) 在清洁 committed head 全部通过，
开始 PVT 实施前 positional verify 退出0。SHA256 为
`954db2154d21e7c724b3123bc8729e80e9630a4c3c4e5b54a42b01bf44ba177a`。
覆盖 metadata、fmt、workspace clippy、engine/HEIF/runtime all-target 回归、
全部272 Python、当前 CLI 构建和真实 Samsung 转换及两成员读回。Linux 不证明 Swift、
System HEIF/Tone/App、GPU、性能或 Photos。较早65项联合检查先于最后 Tone 隐私/状态修正；
最终7项收据和34项 Tone 日志对应修正后的字节。

自动新 head CI：[产品/App](https://github.com/21Z121Z1/XDRemux/actions/runs/39514720759)、
[exact-head](https://github.com/21Z121Z1/XDRemux/actions/runs/39514720814)、
[性能](https://github.com/21Z121Z1/XDRemux/actions/runs/39514720891)、
[研究](https://github.com/21Z121Z1/XDRemux/actions/runs/39514720871)、
[核心/arm64包](https://github.com/21Z121Z1/XDRemux/actions/runs/39514720882)。捕获时排队/运行中，
未算作通过、未重复 dispatch；后续源码还需自己的验收。

普通推进源分支前已建立持久 tag `backup/semantic-review-20261020/integration/c379aa7`；
[独立恢复](integration-restore-c379aa7.log) 证明远端取回、commit/tree 匹配和 fsck0。
[23ref manifest](all-23-backup-manifest.json) 与[完整恢复日志](all-23-backup-restore.log)
覆盖当前9分支 head；[v8证据自身恢复](evidence-v8-restore.log) 也保留。
没有删除任何源/研究分支。

收敛方向仍是最终功能门禁及备份经父审核后普通 merge #71 一次，保留四原 PR tip 祖先关系，
核对平台实际间接 merged 状态，再执行获审的准确 ref 清理。原 PR head CI 不会冒称全绿。
旧4b禁用操作 manifest 是历史材料，不授权本 head 或未来 PVT 的合并/删除；最终 snapshot、
graph、hook/config/权限和父审批仍需更新。

## 可选 PVT 的当前证据

[只读研究](pvt-output-research.public.md)、[历史盘点](pvt-history-scan.json)和
[七组 Linux 原型](pvt-prototype-manifest.json)保留。当前及此前22恢复备份未发现 PVT 实现。
本轮独立找到的[公开 makelive 固定版](https://github.com/RhetTbull/makelive/blob/40abfa4a5bc458609ddb9087d3a2743a826103a4/makelive/makelive.py#L315)
使用真实 .pvt 目录、照片、MOV、metadata.plist，版本值是字符串 `"1"`。
源码/test blob hash 已核对，未运行程序/安装依赖，不复用其覆盖式拷贝策略，也未在这里重新发布整份下载源码。

原型证明真实已验证 pair 包装往返全字节一致，以及 Linux 原子排他目录发布对已有文件/目录/
symlink 和并发竞争不覆盖；所用已有 CLI binary 明示未绑定 exact HEAD。
当前 CLI 仍拒绝包目录；此 head 不包含 PVT 产品实现。后续实现已在同一任务推进，
使用默认 pair、可选 `--live-photo-output pair|pvt`，CLI/App 一致，不额外重编码。
原 Snoopy 样本不可见、历史导入反馈、结构原型、原生 Photos 导入、动态墙纸分别列证据，
没有声称本轮原生导入或墙纸通过。
