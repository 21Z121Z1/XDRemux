# 纠正与自动约束

[English](evidence-map.en.md) | 简体中文

依据是实际读取的提交、PR、源码、回归和执行原型，不声称有不可访问的用户历史或外部 recall。优先级来自可达后果，不能用审查者一致或代码行数代替证据。

| 来源与失败模式 | 风险 | 已有 owner、不变量或原型 | 收益与自动检查 |
| --- | --- | --- | --- |
| #70 报告内部 GPU failure 却只检查显式 command/fence；转换和复用对 tag84 的要求不同。 | P1 失败原生工作可能发布；P2 不完整输出可能被复用。 | Rust Texture 策略要求完整数字回执和空 failure 数组；原生 producer 累积至 terminal finish；复用调用同一个 required-state owner。 | 单一接纳边界。[旧策略](regressions/texture-policy-before.log) 三项新增回归失败，[修复后](regressions/texture-policy-after.log) 真实生产源码 harness 五项通过。不是实际 GPU 故障证据。 |
| 历史提交 [de9558f](https://github.com/21Z121Z1/XDRemux/commit/de9558f53ac1684305c0c64d05a73adb2b54fa8a) 隔离 Vision compound 调度；#70 在另一个 producer 重引入。 | P1 必经路径重引入已知崩溃形状。 | Texture 异构请求逐一提交，保留真正执行 instances 请求的 handler。 | 角色及首个失败传播均保留；`tests.test_rust_apple_semantic_policy` 检查相关调度形状，仍需 macOS 运行证据。 |
| 输出预检后竞争发布；App 输出可别名到原片；单文件替换可能破坏实况文件对。 | P1 媒体覆盖或不完整发布。 | batch 新 ProXDR 只有一个私有 staging 后 `persist_noclobber` 发布入口；App 检查 canonical path、device/inode，拒绝以单文件路径替换实况输出。 | 实际发布竞争、失败和清理负例；Swift 原片别名、目录、双管道、文件对回归待 native CI。失败时保留现有字节。 |
| #68 System HEIF 被计为 4 images / 3 descriptors；同数量坏图也可含重复/缺失 descriptor。 | P2 合法 HVC1 拒绝或歧义关系被接纳。 | HEIF owner 根据唯一 `tmap dimg` 找 Gain Map，支持 HVC1/GRID，每个 semantic image 要唯一 descriptor。 | 明确一对一图约束，没有泛化图框架。[坏图红灯](regressions/semantic-graph-before.log)、[source 红灯](regressions/semantic-source-before.log)、[36 unit + 2 integration 绿灯](regressions/semantic-heif-after.log)。不冒充原生读回。 |
| #69 只撤销旋转，保留已解码镜像；writer 改变不交换的变换顺序。 | P2 缩略图/Gain Map 镜像或方向错。 | typed coded geometry 保存 mirror/order；RGB16 inverse 整体复制六字节像素，三条 writer 保留关联顺序。 | 独立排列覆盖全部旋转、镜像轴、顺序和低位；真实 Samsung portable 相关性 luma 0.9976、edge 0.9978，故意旋转为 0.0923。ImageIO 待原生 CI。 |
| 已知封面时间跳过视频/样本验证；未映射有效 JPEG/R 可能静默 SDR。 | P2 实况无可用视频或损失 HDR。 | runtime 始终验证主视频/样本，发布前拒绝未映射 HDR，已知有理时间保持精确。 | 三个公开 fixture 变异负例先失败后修复；真实 SDR SEF 仍支持；最终 receipt 含 14 例真实 CLI 和双成员验证，保留 movie payload/精确时间断言。 |
| System encoder 换 Gain Map 后仍用原 OPPO 参数；测量先覆盖输出再验证。 | P2 系数错；P1 失败测量破坏已有输出。 | transaction 保存实际量化 SDR/half HDR 提交对，Styles 直接消费；测量验证后使用已有 atomic publisher。 | 无陈旧第二推导路径或假元数据 fallback；原生门控回归验证 pair 身份和失败保留输出。macOS Clippy 已编译模块，执行通过只在 native job 完成后记账。 |
| [f89eea8](https://github.com/21Z121Z1/XDRemux/commit/f89eea8c1714eb5d1e3da1ff8ba83e61655181d5) 曾对齐 workflow/exact-head；三个新 wrapper 再次违反当前 owner/allowlist。 | P2 门禁失败或验证错误源码。 | 现有 `ci.yml` 管产品 ImageIO，`research.yml` 管实验，workflow tests 约束 owner/read-only source/exact head。 | 删除三个 wrapper，保留实质检查/产物和性能限制。219 portable Python 通过，布局通过不等于功能通过。 |
| 怀疑越界精确封面时间仍可发布。 | 执行原型后排除。 | [实际 CLI 原型](verification/time-range-prototype.json) 仅将临时 Samsung fixture 时间改为 9.999999 秒。 | 退出 1、`motion asset presentation time is invalid`、不发布双成员且源字节未变；已有 library timeline 约束生效，未加重复 guard。 |

剩余设计压力：最大公开 ProXDR 为 58,360,320 像素，两份完整 RGB f32 每 worker 1.304 GiB，四 worker 5.218 GiB，尚未包含 codec/Metal。未实测 OOM。精确 streaming area accumulation 是需等价及性能证据的候选，未用稀疏四点采样替代。#65 Photos 激活失败仍是独立设备缺口。
