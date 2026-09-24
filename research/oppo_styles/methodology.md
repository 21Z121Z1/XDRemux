# 原生消费者与低维校准实验

[English](methodology.en.md) | 简体中文

本文保存 solver 分支的**研究方法**，不是第二套转换器，也不声称重新完成了私有实验。先前引用的收敛台账不在当前目录树中。依赖退休引用的来源证明之前，需找回原始台账与源文件身份；本文的方法说明不能代替这些记录。未改动的公开模型与历史指标另见[模型卡](models/UniversalPhotographicStyleStateNet.model-card.md)。

## 数据边界与前瞻队列

历史原生数据集报告有 603 个样本、472 个拍摄会话；417/89/97 是按会话隔离后的 train/calibration/heldout **样本数**，不是三组会话数。私有 manifest 和原始字节不在 Git 中。复现需要这些输入及身份，公开模型本身不能证明历史拆分。

原生消费者 freezer 在每个既有 split/device 中按 source SHA-256 字典序选一张，不看图像质量或指标：四个设备各一张 calibration，加四个不同会话的 heldout。重新执行必须验证真实字节，拒绝缺失、重复或跨集合重叠会话，固定 dataset/model/checkpoint/code/OS 身份并写入全新的不可覆盖 manifest。旧脚本直接信任清单哈希并覆盖输出，这种实现不保留。

前瞻 OPPO freezer 排除四个历史 A/B 场景及旧审计出现过的全部 SHA-256，再按源哈希为每种设备选择一张存在的图片，历史报告包含五种设备。清单此前已经做过无标签观察：“前瞻消费者队列”不等于图像分布从未被观察。必须同时排除场景和字节身份，说明先前接触情况，不能看完输出再重新抽样。文件存在不代表其内容符合记录的哈希。

历史 A/B 图曾用于模型/尺度选择，必须标记为 **unlocked**。OOD 实验按 calibration p95 不确定度放行衡量的是覆盖率，不是原生渲染精度。DNG 内嵌预览不是解码后的 RAW。现在的 `training.py` / `inference.py` 以真实解码模态和字节身份为准，不恢复扩展名或字符串标记启发式。

## 原生响应矩阵与因果对照

渲染前冻结八个状态：`disabled`、`neutral`、`tone_+1`、`tone_-1`、`color_+1`、`color_-1`、`tc100_mid`、`tc100_plus`。保存原始/候选 carrier、请求、实际像素、源/候选/模型/checkpoint 哈希、框架/OS、命令、退出状态及错误。结构完整的 Styles 图、有限的代理响应与真正的私有渲染器响应是不同证据。

原生 alpha 网格为 `[0, .25, .5, .625, .75, 1]`，基线 `.625`；计划的 residual gain 为 `[.5, .75, 1, 1.25, 1.5]`。仅用 calibration 选择：改善至少 1%，任一设备退化不超过 10%，缺失/失败率和方向反转不增加。比较之前必须真正得到全部要求的状态与队列。旧指标是各状态 encoded-RGB RMSE 的均值，不是合并后的线性光误差。固定色彩空间、方向、尺寸和位深，不得静默转换不兼容的像素语义。

旧 alpha 汇总器存在错误泛化：即使选中了其他 alpha，也可能把 heldout 基线缓存标成 `chosenAlpha`；还会对不完整状态取均值，并用多个文件名回退。旧 consumer 汇总器的 Markdown 可能声称 calibration 完整，而 JSON 明确不完整。**这些报告生成器明确退役，不原样迁移。** 缺失候选/状态仍应缺失，候选结果必须来自该候选而非基线缓存；先持久化所选参数及来源，再生成 heldout 结果，机器可读与人类可读结论必须一致。

无缓存对照比较 alpha 0、.625、1：不同 key1 输入、不同 carrier，以及 `neutral` / `tone_plus` 的独立请求和输出路径。相同像素可能真实反映消费者不敏感，不能强制像素哈希不同；反过来，不同文件名也不能证明请求或执行不同。验证请求到 carrier 的完整哈希链和渲染输出身份。私有渲染不可用时不能替换成 identity 或语义代理。

Issue #32 记录 macOS 27.0 `26A5416b` / Xcode 27 `27A5218g` 的私有 ABI 漂移：旧 `PLPhotoEditSource` initializer 少了 image 参数，`_NUStyleTransferApplyProcessor` 在 color space 前新增 displacement。旧 Swift producer/oracle 不属于现在的 Rust 产品或窄 adapter。未来本地消费者 harness 必须先核对实际方法签名；超时、缺失 selector、ABI 不匹配都是阻塞证据，不是成功的 identity fallback。本次收敛不会在 CI 启用固件下载或私有渲染器。

## 冻结低维校正

`evaluate_selfpair_lowdim_adapter.py` 的有效契约现在由 `calibration.py` 实现：仅拟合一次、在全部空间与 plane 位置共享的三十个归一化 polynomial/output-channel bias；序列化后只应用这些存储值。拟合使用有效观测块与正 ridge；应用接口没有 target 或 mask 参数，均不修改输入，并检查有限性、形状、正尺度、容量上限和算术溢出。

```python
from research.oppo_styles.calibration import ChannelBias, fit_channel_bias, apply_channel_bias

# 从指定训练队列拟合；calibration 选择时不重新拟合这些系数。
fitted = fit_channel_bias(train_prediction, train_target, train_mask, scales, ridge=10.0)
frozen_record = fitted.to_dict()  # 同时绑定 code/cohort/model 哈希
# 选择已经持久化之后，恢复冻结参数并应用到 heldout。
restored = ChannelBias.from_dict(frozen_record)
heldout_prediction = apply_channel_bias(heldout_base_prediction, scales, restored)
```

旧脚本保存了拟合 bias，却在 heldout 的 channel-ridge 路径忽略它，再调用读取 heldout target 并重新拟合的函数，违背自身“heldout 不再拟合”的注释。新应用接口无法这样做。测试覆盖冻结后修改标签、空间/plane 维度变化、记录往返、非有限数与溢出；这证明冻结契约，不证明原生精度。

其他历史候选仍是研究假设：以 .05 递增的 blend alpha；.85–1.15 的 global residual gain 和 −.02–.02 的归一化 bias；按设备/profile 或亮度混合；共享/channel bias；ridge 为 1/3/10/30/100/300 的设备单参数 bias；rank 1/2/3 的会话均值残差 PCA，输入 RGB 均值/标准差、ridge 100；kNN 3/5 配 .25/.5/1 shrink。train 会话负责拟合，calibration 负责选择/晋级，未知 profile 固定回退 `.625`。置信区间应按会话聚类 bootstrap（旧实验为 20,000 次、seed 260819），不能把独立像素当独立样本。heldout 不能拟合系数或选择模型族。

17 Pro 脚本无条件输出 rejection 字符串；其中宣称“30 参数 term/channel gain”的实现实际是 identity no-op，“bias”只用了一个标量；部分 profile 恢复路径仅实现 device-alpha。因此这些标签不能证明全部声明候选真的被正确拟合和评估。保留为受限/负面历史尝试；重新研究必须提供准确数学实现、明确拟合状态、样本顺序/会话核查和真正执行的晋级规则，不能复用硬编码结论。

## 训练尝试与导出一致性

当前研究训练包使用显式 `TrainingInputMode` 区分真实 paired 与单图 self-pair；self-pair 只改变观测，不改变原生标签或缓存字节。历史 heads-only 混合实验用很小学习率组合 true/self-pair Huber loss 与 detached true-prediction 一致性目标，但使用宽松 checkpoint 加载和无保护的可复用输出。旧包装脚本退役；再次实验必须使用校验来源的受限加载、全新目录、有限优化、冻结拆分和真正由 calibration 选择的 checkpoint。

旧模型卡报告短程 self-pair 候选 heldout normalized MAE 为 .80226、.78751，均差于 .78220 基线，未晋级。Universal → 冻结 paired cascade 报告 .82674，差于独立 universal .82233；cascade 输入 universal 预测的 unstyled 缩略图，oracle 输入真实 native disabled 缩略图。不能混淆输入或把 oracle 当运行时能力。这些是私有历史指标，不是本次 CI 重测结果。

旧 Core ML parity harness 对 Python 输出使用 key1/light-map .02、GTC 最多 2 字节误差、scalar .1 的容差。复现需要相同训练 checkpoint、归一化/布局、源字节、Core ML 包、compute 配置和方向相关序列化；仅公开模型无法恢复这些私有输入。当前 `CoreMLProbe.swift` 只对公开模型做合成输入编译/执行及有限形状检查，不声称 Python/原生渲染 parity 或实际 Neural Engine 放置。

## 晋级门槛

研究推进需要冻结身份/拆分、完整响应矩阵、没有隐藏 fallback 或标签泄漏、有意义的 identity/shuffle/paired/消融对照、明确设备 guard 与可复现的源相关资源。产品晋级还需整合进 Rust intent/capability/publication 所有者、角色正确的元数据、原生消费者检查，以及在声称可编辑时提供真实 Photos 导入/编辑/保存/重开证据。保留假设不等于产品缺陷或已交付功能。
