# Texture Style 研究

[English](README.en.md) | 简体中文

**仅限研究。** Rust 产品不会伪造 Texture Style 支持。本目录保存原生契约观察、已否定的捷径、合成探针和明确的晋升门槛，不是第二套转换器，也不会写入 HEIF。[有日期的收敛台账](../../docs/convergence/2026-09-23/README.md)记录历史提交及各项退休资产；其中的归档引用用于追溯，不是开发分支。

## Carrier 实验证明了什么，又没有证明什么

历史矩阵分别改变三个层次：2023 Styles 资源、2026 Texture 元数据 item，以及 Apple MakerNote tag 84。把任一层的解析成功当作完整可编辑照片契约是错误的。实验也改变了 `idat`/method 1 与 `mdat`/method 0 存放方式、元数据 item 名称、全局 `mdat` 布局和语义资源。某一 carrier 的读取结果，不是 HEIF 或所有原生 reader 的通用要求。

| 资源 | 观察到或假设的契约 | 限制 |
| --- | --- | --- |
| 既有 Styles | `tag:apple.com,2023:photo:metadata:styles`、binary plist、到 primary/tone-map 资源的 `cdsc` 关系 | 原生 v8、生成的 v15 和实验 v16 不可混为一谈；产品保留已验证的 producer 格式。 |
| Texture 元数据 | `tag:apple.com,2026:photo:metadata:texture_styles`；实验使用 item 名 `metadata` | URI 或名称匹配不等于语义有效。必须精确识别资源及关系，而不是选择最大 payload 或首个相似 URI。 |
| Texture 信息 | `Preset`、`CaptureType`、`CaptureMode`、`PortType`、`HardwareModel`、`TextureStylePeopleDataVersion`、`FilmGrainSeed`、可选后处理 people data | 其中包含拍摄事实，不能从 donor 复制，也不能给 OPPO 来源填入 iPhone 默认值。无人场景候选省略 people data，而非伪造人物。 |
| Apple MakerNote | 保留字节序、未知字段和源 `PhotoIdentifier`（tag 43）；tag 84 是有类型的 binary plist | Boolean、integer、Float32、Float64 的线格式不因 Python 解码值相等而等价。缺少 Apple MakerNote 不代表可以编造。 |
| 语义资源 | 每个 role 对应的辅助图像、descriptor、`auxC` 身份、`ipma` 关联和 `auxl`/`cdsc` 边 | 重复标签或可被解析的资源不能证明按 role 推理。Per-person instance 也不是可与 category mask 互换的第十三类。 |

十二个新增 category URI 的前缀是 `tag:apple.com,2026:photo:aux:`，后缀分别为 `semanticnosematte`、`semanticskinmattev2`、`semanticnonfaceskinmatte`、`semanticlipsmatte`、`semanticteethmattev2`、`semanticpersonmatte`、`semanticglassesmattev2`、`semanticeyebrowsmatte`、`semantictattoomatte`、`semantichandsmatte`、`semanticearsmatte`、`semanticfaceskinmatte`。准确的 role-to-producer 映射仍是晋升要求；private Vision 结果中的 category 编号本身不是这种映射。

`evidence/native-standard-matrix.json` 逐字节保留历史 B–E 候选定义。其中营销名称、`iPhone19,2`、`iPhone19,3` 是**候选假设**，不是任意输入的已验证拍摄事实。产品或 writer 均不读取该文件。旧 validator 检查解码值、单 extent 和结构关系，并未证明线格式类型忠实度、所有 location 语义或 Photos 可编辑性。

## 被否定的 carrier 产品晋升

历史 Rust carrier 构造 v16 Styles plist，把同一张 2019 skin 图像及 MIME descriptor 别名化为全部十二个 2026 role；它还合成固定 216 字节 Texture plist（包含 iPhone 硬件/拍摄声明）、133 字节 tag-84 plist，并在缺失时创建 Apple MakerNote，用源哈希生成 UUID 形状的标识符。从源字节派生标识符，并不能把编造的 Apple 拍摄元数据变成源事实；“没有使用 donor 图像字节”也不能证明语义标签正确。

这些自动产品 hook、元数据模板和构造 helper 被有意退休，历史源码通过台账中的 annotated archive tag 恢复。有效的结构性教训保留在此：隔离变量时保留无关 payload、未知 box 和布局；检查所有 extent 的边界及溢出；保留已有标识符；验证**实际原子发布的同一份最终字节**。这些不变量归当前 Rust format/HEIF/runtime owner 所有，而不是复活 carrier backend。

独立编码的空 mask、同源别名及十三类资源存在性扫描是有用的负对照，不是原生推理。哈希相同也不一定错误：两个实际推理出的缺席类别可能都是空白。因此晋升要求 producer 身份和 role 含义，而不只是哈希互异。原生形态 MakerNote 的细化和最终发布测试没有消除语义阻塞。当前 lifecycle 测试保留最终发布不变量，不依赖历史局部变量名。

## Person 输入和 ROI 实验

`PersonInputProbe.m` 保留九种合成输入：CGRect dictionary/NSValue、可选 yaw/pitch/roll、四种 landmark key 及 all-aliases 对照。合成人脸的归一化矩形是 `(0.20, 0.25, 0.40, 0.40)`，face ID 为 17，76 个椭圆 landmark 刻意不是实测数据。可选 crop 矩阵隔离 `normalizePersonInputDataArray:toCropRect:` 的变换，不声称 face ROI 等于 skin ROI，也没有恢复原生图像推理模型。

探针动态解析 `kFigCaptureStreamMetadata_*`，记录哪些是实际符号、哪些仍是 fallback-string 假设；观察 `faceROI`、`faceSkinROI`、`faceROIAndLandmarksROIRelativeScalingROI`、pose/landmark 字段及 instance 引用。在 `NSInvocation` 写入或读取参数存储**之前**检查参数数量、object/CGRect 类型和返回类型。未知 ABI、缺失所请求的 normalization 方法、异常、nil/非数组结果或缺失人物均是失败，不是成功的空报告。每种 case 使用独立、有限时的子进程，一次 abort 不会抹掉后续矩阵。

`InputTrace.m` 观察 `personInputDataArrayFromDetectedFaces:`、`personInputDataFromStillProperties:`、`setInputPersonData:`、`setInputSkinSmoothingFaceDetections:` 的输入/输出。只 hook 自身定义且 ABI 匹配的方法，始终转发到原实现；日志失败不会吞掉原调用。它只供主动启用的自有本地研究 harness 使用，CI 不向 Photos 或系统进程注入。

```sh
# 每个命令要求一个尚不存在、但父目录已存在的输出目录。
python -m research.texture_styles.probe self-test --output /tmp/texture-abi-check
python -m research.texture_styles.probe person --output /tmp/texture-person-matrix \
  --normalize 0.03 0.04 0.94 0.92
python -m research.texture_styles.probe runtime --output /tmp/texture-runtime-inventory
```

`self-test` 使用公开 macOS SDK 编译，在合成 NSObject 类上验证 ABI guard，不调用 private selector；研究 CI 执行此模式。`person` 和 `runtime` 是显式本地研究操作，framework/方法不可用不算产品缺陷。报告保留源码哈希、编译器/SDK、OS/架构、命令、退出码、timeout 和日志哈希。矩阵未完成时退出码非零并标为 incomplete。wrapper 不下载固件，也不接收私人照片。

自有、未 hardened 的 harness 可使用 `dlopen`，或显式设置 `DYLD_INSERT_LIBRARIES`、`XDREMUX_TEXTURE_TRACE=1`、`XDREMUX_TEXTURE_TRACE_FILE` 加载 trace。不要禁用 SIP、削弱签名应用校验或上传私人 trace/媒体。CI 只编译 trace，不注入。

## FSINC / E5 探索与修正后的稀疏 oracle

历史本地探针探索 ANSTKit 的 FSINC configuration、descriptor、E5 network wrapper、instance/category mask API。版本候选为 `0x2012c`（2.3）和 `0x20190`（2.4）；分辨率候选为 0 = 768×576、1 = 576×768、2 = 256×256。旧图像探针使用居中 aspect-fill BGRA/sRGB 输入，顺序是具体类选择 → configuration → algorithm → `prepareWithError:` → input binding → inference → instance/category masks。尝试 compute-device 整数 0–7、导出 category 编号或打印非 nil wrapper，不等于证明 ANE 执行受支持，也不能证明十二个 role 正确。

后续实验比较 macOS 26/Xcode 27 环境、descriptor factory、E5 preparation、MIL version/opset namespace、`constexpr_*` sparse operator、Core ML loading 及 model/blob reader；这些是假设不同的实验。仅修改 opset/header 不能提供缺失的 compiler operator、解码专有打包 blob 或证明模型等价。旧 workflow 包括宽松 diagnostic step 和 binary dump；job 绿色不能被解释为 inference 成功。不恢复这些采集、内存 dump 和一次性 runner workflow。`RuntimeInventory.m` 用类声明、method encoding、符号可用性的只读清单替代无边界的 raw-memory/ivar 读取，不按猜测的 native function ABI 调用函数。

历史 r10 中的等式对非零 offset 明确错误：

```text
错误：sparse_shift_scale_then_dense(mask, q, scale, offset)
      == dense_shift_scale(sparse_to_dense(q, mask), scale, offset)

正确：仅当 mask[index] == 1 时，
      materialized[index] = scale[block] * (q[next] - offset[block])；其余仍为零。
```

例如 mask `[[1,0],[0,1]]`、q `[4,7]`、scale `0.5`、offset `2`，正确结果为 `[[1,0],[0,2.5]]`，不是 `[[1,-1],[-1,2.5]]`。`sparse.py` 独立实现有边界、显式解包的运算。测试覆盖 block 坐标、两种浮点宽度、可选/非零 offset、C-order 遍历、畸形输入和溢出。`oracle.py` 将十二种合成输入与 **coremltools 9.0** 的公开 operator 对比。这是算子等价性测试，不是原生 FSINC 模型转换或 inference 声明。旧 materialization recipe 连同已否定等式的说明一起归档，不把错误结论默默保留为有效。

## 晋升阶梯与证据边界

产品变化首先要求来源可信的 capture 字段及所有声称的 semantic role、明确的尺寸/方向/ROI 变换和资源关系、有限的模型输出、畸形输入拒绝，以及 primary/Gain Map 媒体保真。随后需要最终字节 HEIF 校验、适用 OS/device 的原生 framework 消费、Photos 控件和 edit/save/reopen/re-render/reversible-edit 证据，以及经当前 Rust runtime 的失败安全发布。缺失原生输入必须维持缺失或显式失败；合成对照不满足晋升门槛。

历史 device-positive carrier 报告只能支撑其报告的 admission 观察，不能独立证明通用可编辑性或 role 真值。这里不重建私人样本字节、已删除或过期的 Actions 输出，也不声称重跑了它们。归档保存受 Git 跟踪的源码和方法；当前 CI/receipt 只证明其实际执行的检查。重启一个假设时，需要新的 artifact 哈希、OS/device/software 身份、精确命令、预期与实际观察，以及明确的晋升门槛。

## 一手参考

- [NSMethodSignature 参数数量](https://developer.apple.com/documentation/foundation/nsmethodsignature/numberofarguments)：包括隐藏的 `self` 和 `_cmd`。
- [NSInvocation 参数存储](https://developer.apple.com/documentation/foundation/nsinvocation/setargument:atindex:)及[返回值所有权](https://developer.apple.com/documentation/foundation/nsinvocation/getreturnvalue:)：先检查大小/类型，再调用。
- [Objective-C 方法替换](https://developer.apple.com/documentation/objectivec/method_setimplementation(_:_:))：保留原 IMP，不假定 selector ABI。
- [Core ML sparse operator 源码](https://apple.github.io/coremltools/_modules/coremltools/converters/mil/mil/ops/defs/iOS18/compression.html#constexpr_sparse_blockwise_shift_scale)：只对 mask 选中的元素反量化。
- [固定版本的公开 oracle](https://pypi.org/project/coremltools/9.0/)：可选、隔离的研究依赖，不是产品依赖。
