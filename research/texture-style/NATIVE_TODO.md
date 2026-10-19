# 原生后续与阻塞 — 2026-10-19

## 当前硬阻塞

- 动作：在 PSR `research/xdremux-texture-native-20261019` 的 xcode-27/macOS26 runner 上编译并执行原生探针。首两轮 run39282392876、39283079303 已提交，记录时均 queued。状态是**待执行**，不是已拒绝、失败或成功。尚无本轮原生日志/artifact、实际 OS、SDK 与 API 返回值。
- 动作：复现已知真实相机纹理正样、iPhone Photos Texture UI 和编辑后的效果。目标资源包含未重新授权的人像/原始拍摄上下文，设备/UI也不在当前容器中。状态是**未执行／缺少已授权且无隐私的正样和设备能力**。没有读取其他任务 secret 或上传敏感样本。
- 本地 `gh` 的 repo API 返回 Forbidden、认证检查失败；已连接 GitHub 工具可正常读写用户自有的两个仓库。没有尝试提取其他凭据或绕过限制。

## 按证据顺序继续

1. 先读回已排队 run 的实际 jobs、完整 build/probe logs、artifact 清单。下载后逐项校验 manifest SHA；把 run/head、runner OS/build、Xcode/SDK、probe源哈希和输入哈希写在同一条结果中。保留 unsupported、非零退出和错误字段，不能把 always-upload 成功当作 native step 成功。
2. 根据实际 `runtime.json` 选择**已经存在且 ABI 匹配**的 producer/consumer 方法。当前 `_NUTextureStyleProperties` 是消费/反序列化入口；不得改名为“任意图像计算 API”。查找 image/capture metadata 输出接口、完整 processor 输入配置，以及版本和调校选择。不要从类名、selector 名或 plist 键名猜算法。
3. 执行合成正负元数据矩阵：公开 Standard/Studio控制、缺 HardwareModel、非法版本、空字典。分别记录是否返回 Texture 类型、error、规范化值、persons 与 serialization；metadata-only 控制不能升级为可编辑相机正样。
4. 使用 `texture_synthetic_inputs` 的CC0 raw图案完成 JPEG/HEIC、横竖方向、SDR/PQ/Adaptive HDR 编码读回。检查实际颜色、Gain Map、headroom、方向是否与请求一致。固定输入 SHA，不使用仓库里的原始人像和其他私密图像。原生失败不得偷偷回填其他图案。
5. 在 PSR runner 上用 exact XDRemux SHA 编译运行 Rust/Apple 桥；对 `texture_synthetic_uhdr` 的纯合成 ProXDR 输入尝试真实 `convert` 与 `batch --apple-styles`，验证新版保留普通 Styles 并没有生成2026纹理字段。Vision/私有 semantic API 不支持时记录确切操作、错误与是否到达最终组装；不能用 stub、复制旧图或合成假的十二角色来使 gate 变绿。
6. 追踪真正的 Texture 编辑/渲染链：按 native input configuration 连接正确域、尺寸、方向的主图、LinearThumbnail、各角色资源和逐人上下文。分别测试 Standard、Filmic/Glowy 与人物相关效果。已知有些 processor 需要 person data 或 thumbnail，但具体预设/usage 条件必须取自本轮运行，不能统一断言全部需要12个角色。
7. 对每个参数做 identity、非零强度、grain正负对照，捕获真实渲染输出和数值差异。保持原生实现不打补丁；在新进程隔离 assertion/abort，记录原始信号和 stderr。CI 输出的 extended linear P3 不能推出内部 Texture 全链也是该域。
8. 最后才做 iPhone Photos 真机激活：同一系统/设备下真实原厂正样、纯结构仿制负样、缺依赖负样；记录实际 Texture 控件、preset/强度/grain、导出变化与重新打开后的编辑持久性。未到这一层时，claims中 Photos激活仍 false/unverified。

## 不得补称的拍摄信息

原始RAW、原始LinearThumbnail、原始逐人imageStats/instance masks、厂商CaptureType/Mode/Port/HardwareModel缺失时，不声称从成片恢复了它们。允许研究新推导上下文，但需标注 provenance、输入颜色与统计域，验证 Apple 实际接受的契约。LinearThumbnail保持Apple Log路线，不改成PQ中间体；新增PQ测试是独立HDR输入测试。

## 完成规则

离线 transport gate 可以完成而“通用 Texture producer/Apple Photos激活”仍被阻塞。源码或报告任何后续提交都会使旧 exact-HEAD receipt 失效，需要重新运行 completion gate。保留纯文本持久证据，artifact一律retention-days:1；不把短期artifact ID当唯一证据。不要清理或修改其他任务分支、secret、release。
