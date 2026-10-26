# Texture Style 研究

[English](README.en.md) | 简体中文

本目录保存 Texture Styles 的历史观察、已否定方法和诊断探针。**这些探针仅用于研究。** 它们不写入 HEIF 文件，也不是另一套产品转换器。

当前 Rust 产品有独立的派生 Texture Styles 处理流程，见 [Apple 功能](../../docs/apple-features.md)和[运行时实现](../../crates/xdremux-runtime/src/apple_texture.rs)。该流程不声称恢复了完整的相机 FSINC 生成器。

先前引用的收敛台账不在当前目录树中。历史提交和归档源码必须与原始来源记录核对。本文摘要不能证明所有旧产物仍可获取。

## 历史载体实验的问题

载体实验分别改变了三个层次：2023 Styles 资源、2026 Texture 元数据项和 Apple MakerNote 标签 84。解析器接受其中一层，不能证明整个照片符合可编辑格式要求。

实验还比较了 `idat`/方法 1 与 `mdat`/方法 0 存放方式、项目名称、全局 `mdat` 布局和语义资源。某个候选的存放位置实验不能确立 HEIF 或原生读取器的通用要求。

| 资源 | 观察到或提出的表达 | 解释边界 |
| --- | --- | --- |
| 既有 Styles | `tag:apple.com,2023:photo:metadata:styles`、二进制属性列表，以及到主图/色调映射资源的 `cdsc` 关系 | 原生 v8、生成的 v15 和实验 v16 是不同格式。保留已验证的生成格式。 |
| Texture 元数据 | `tag:apple.com,2026:photo:metadata:texture_styles`；实验项目名为 `metadata` | URI 或名称匹配不等于语义有效。按准确身份和关系选择资源，不能选择最大载荷或首个相似 URI。 |
| Texture 信息 | `Preset`、`CaptureType`、`CaptureMode`、`PortType`、`HardwareModel`、`TextureStylePeopleDataVersion`、`FilmGrainSeed` 和可选的后处理人物数据 | 拍摄字段必须反映源数据事实。供体字段或 iPhone 默认值不能描述 OPPO 拍摄。无人候选省略人物数据。 |
| Apple MakerNote | 既有字节序、未知字段、源 `PhotoIdentifier`（标签 43）和有类型的二进制属性列表标签 84 | Boolean、integer、Float32 和 Float64 的编码不同，即使解码值相等也不能互换。不得伪造缺失的拍摄元数据。 |
| 语义资源 | 每种角色对应的辅助图像、描述符、`auxC` 身份、`ipma` 关联和 `auxl`/`cdsc` 关系 | 标签和可解析资源不能证明推理。单个人物的实例资源不是可互换的第十三类类别掩膜。 |

以下十二个类别 URI 后缀使用前缀 `tag:apple.com,2026:photo:aux:`。后缀拼写必须保持不变。

| 类别 URI 后缀 | 类别 URI 后缀 |
| --- | --- |
| `semanticnosematte` | `semanticskinmattev2` |
| `semanticnonfaceskinmatte` | `semanticlipsmatte` |
| `semanticteethmattev2` | `semanticpersonmatte` |
| `semanticglassesmattev2` | `semanticeyebrowsmatte` |
| `semantictattoomatte` | `semantichandsmatte` |
| `semanticearsmatte` | `semanticfaceskinmatte` |

要声称完成原生类别生成，必须明确每种角色与生成器的对应关系。私有 Vision 类别编号本身不足以确立该关系。当前产品的派生编辑语义更窄：部分保护区域来自原生关键点或手部姿态，耳部和文身保护则被禁用。这些限制由[当前功能约定](../../docs/apple-features.md)定义。

[`evidence/native-standard-matrix.json`](evidence/native-standard-matrix.json) 逐字节保留历史 B–E 候选定义。其中的营销名称、`iPhone19,2` 和 `iPhone19,3` 都是候选假设，不是任意输入的已验证拍摄事实。产品和写入器均不读取该文件。

已停用的校验器检查了解码值、单个数据区间和结构关系。它没有证明序列化类型准确、全部存放位置规则正确，或 Photos 支持编辑。

## 已否定的产品迁入方法

早期 Rust 载体生成 v16 Styles 属性列表，将同一张 2019 skin 图像及其 MIME 描述符别名化为全部十二个 2026 角色。它还生成包含 iPhone 拍摄声明的固定 216 字节 Texture 属性列表，以及固定 133 字节的标签 84 属性列表。

当 Apple MakerNote 缺失时，该载体会新建一个，并用源哈希派生 UUID 形状的标识符。标识符来自源字节，并不能让编造的拍摄字段成为事实。未使用供体图像字节，也不能证明语义标签正确。

这些自动调用入口、元数据模板和构造辅助函数均已停用。恢复历史源码需要原始归档记录。仍然有效的结构要求由当前 Rust 格式、HEIF 和运行时模块负责：

- 隔离实验变量时，保留无关载荷、未知数据盒和布局。
- 检查每个数据区间的边界与溢出。
- 保留既有标识符。
- 验证运行时实际原子发布的同一份最终字节。

独立编码的空掩膜、同源别名和十三类资源存在性扫描都属于负对照，不能证明原生推理。相同哈希也不一定错误：两个经真实推理确认缺席的类别都可能为空。

证据必须说明生成器身份和角色含义。哈希不同本身不足以证明正确。MakerNote 细化和最终发布测试没有消除这个语义缺口。当前生命周期测试保留了发布要求，不依赖历史局部变量名。

## 人物输入与感兴趣区域探针

[`PersonInputProbe.m`](PersonInputProbe.m) 定义九种合成输入，比较 CGRect 字典与 NSValue、可选 yaw/pitch/roll、四种关键点键名，以及使用全部别名的对照。合成人脸的归一化矩形为 `(0.20, 0.25, 0.40, 0.40)`，人脸 ID 为 17，76 个椭圆关键点均为人工构造。

可选的裁剪实验单独测试 `normalizePersonInputDataArray:toCropRect:`。它不能证明人脸感兴趣区域（ROI）等于皮肤 ROI，也没有恢复图像推理模型。

探针动态解析 `kFigCaptureStreamMetadata_*` 键，报告中区分真实解析的符号与回退字符串假设。它观察 `faceROI`、`faceSkinROI`、`faceROIAndLandmarksROIRelativeScalingROI`、姿态/关键点字段和实例引用。

在 `NSInvocation` 访问参数或返回值存储前，探针检查参数数量、object/CGRect 类型和返回类型。未知 ABI、缺少所请求的归一化方法、异常、nil/非数组输出或缺失人物都会导致失败。每项测试使用独立且有限时的子进程。单个进程中止不会抹掉其他测试结果。

[`InputTrace.m`](InputTrace.m) 观察以下方法：

- `personInputDataArrayFromDetectedFaces:`
- `personInputDataFromStillProperties:`
- `setInputPersonData:`
- `setInputSkinSmoothingFaceDetections:`

跟踪器只替换目标类自身定义且 ABI 匹配的方法。它始终转发到原实现，日志记录失败也不会阻止原调用。跟踪器仅供主动启用的自有本地研究测试程序使用。CI 不会将其注入 Photos 或系统进程。

### 运行探针

需要 macOS 和 Xcode 命令行工具。每条命令都要求输出目录尚不存在，且父目录已存在。

```sh
python -m research.texture_styles.probe self-test --output /tmp/texture-abi-check
python -m research.texture_styles.probe person --output /tmp/texture-person-matrix \
  --normalize 0.03 0.04 0.94 0.92
python -m research.texture_styles.probe runtime --output /tmp/texture-runtime-inventory
```

`self-test` 使用公开 SDK 和合成 NSObject 类检查 ABI 防护，不调用私有选择器。研究工作流运行此模式。

`person` 和 `runtime` 是明确的本地实验。框架或方法不可用属于实验限制，不是产品失败。报告保留源码哈希、编译器/SDK、操作系统/架构、命令、退出码、超时和日志哈希。矩阵未完成时会保留 incomplete 标记并以非零状态退出。包装器不下载固件，也不接受私有照片。

没有强化运行时限制的自有测试程序可以通过 `dlopen` 加载跟踪器。另一种显式方式是使用 `DYLD_INSERT_LIBRARIES`、`XDREMUX_TEXTURE_TRACE=1` 和 `XDREMUX_TEXTURE_TRACE_FILE`。不要禁用 SIP、削弱签名应用的校验或上传私有跟踪记录/媒体。CI 仅编译跟踪器，不进行注入。

## FSINC 与 E5 观察

历史探针研究了 ANSTKit 的 FSINC 配置、描述符、E5 网络包装器，以及实例/类别掩膜 API。候选参数如下：

| 参数 | 历史候选值 |
| --- | --- |
| 版本 | `0x2012c`（2.3）；`0x20190`（2.4） |
| 分辨率 | 0 = 768×576；1 = 576×768；2 = 256×256 |
| 计算设备值 | 整数 0–7 |
| 图像输入 | 居中、保持比例填满的 BGRA/sRGB 图像 |

调用顺序为类选择、配置、算法创建、`prepareWithError:`、输入绑定、推理，以及实例/类别掩膜获取。尝试某个计算设备整数不能证明支持 Apple 神经网络引擎（ANE）执行。类别编号或非 nil 包装器也不能确立十二种语义角色。

后续实验涉及 macOS 26/Xcode 27、描述符工厂、E5 准备、MIL 版本/算子集、`constexpr_*` 稀疏算子、Core ML 加载，以及模型/数据块读取器。这些实验验证不同的假设。重命名算子集或文件头无法补齐编译器缺失的算子、解码专有打包数据或证明模型等价。

部分旧工作流允许诊断失败，并生成二进制转储。因此，任务显示绿色不能证明推理成功。当前目录不恢复这些采集、内存转储或一次性工作流。[`RuntimeInventory.m`](RuntimeInventory.m) 只读取类声明、方法编码和符号可用性，不读取任意原始内存，也不按猜测的原生函数 ABI 发起调用。

## 正确的稀疏运算

历史 r10 等式在偏移非零时不成立：

```text
wrong: sparse_shift_scale_then_dense(mask, q, scale, offset)
       == dense_shift_scale(sparse_to_dense(q, mask), scale, offset)

correct: materialized[index] = scale[block] * (q[next] - offset[block])
         only where mask[index] == 1; every other entry remains zero.
```

也就是说，只对 `mask[index] == 1` 的位置计算上述值，其余位置保持零。对于 mask `[[1,0],[0,1]]`、q `[4,7]`、scale `0.5` 和 offset `2`，正确结果为 `[[1,0],[0,2.5]]`。`[[1,-1],[-1,2.5]]` 是错误结果。

[`sparse.py`](sparse.py) 使用显式解包的输入，实现有边界检查的运算。测试覆盖块坐标、两种浮点宽度、可选/非零偏移、C 顺序遍历、无效输入和溢出。

[`oracle.py`](oracle.py) 将十二种合成输入与 **coremltools 9.0** 的公开算子进行比较。这只检查算子等价性，不转换原生 FSINC 模型，也不证明原生推理。历史展开方案保留了已否定等式，作为错误记录。

## 后续产品修改的要求

产品修改必须说明拍摄字段的来源，以及每种声称支持的语义角色由哪个生成器产生。还必须明确尺寸、方向、ROI 变换和资源关系，检查输出是否为有限值，拒绝无效输入，并保留主图和 Gain Map 媒体。

验证最终 HEIF 字节，以及适用操作系统/设备上的原生读取行为。随后验证 Photos 控件、编辑、保存、重新打开、渲染和可逆性。发布必须经过当前 Rust 运行时，并在失败时保留既有输出。缺少原生输入时，应保持缺失或明确失败。合成对照不能替代原生证据。

历史设备正结果只支持其中记录的准入行为，不能独立证明通用可编辑性或角色真实性。本文没有重建或重新运行私有样本字节，以及已删除或过期的 Actions 输出。

重新验证某个假设时，应记录新的产物哈希、操作系统/设备/软件身份、精确命令、预期结果、实际结果和验收要求。当前 CI 与报告只证明其实际执行的检查。

## 历史原始证据

[`evidence/historical-contracts.json`](evidence/historical-contracts.json) 保留七份小型 RC/数字键/候选记录，以及原始提交、Git 对象和 SHA-256 身份。其中的原始声明与合成值仍是历史假设，不是当前拍摄事实。

这些记录保留实验定义，无需保留多套相互竞争的修改工具，也无需重新获取私有框架和固件。`native-standard-matrix.json` 保持逐字节一致。来源索引中的副本是历史证据，不是另一份运行时配置。

## 一手参考资料

- [NSMethodSignature 参数数量](https://developer.apple.com/documentation/foundation/nsmethodsignature/numberofarguments)：包括 `self` 和 `_cmd`。
- [NSInvocation 参数存储](https://developer.apple.com/documentation/foundation/nsinvocation/setargument:atindex:)和[返回值所有权](https://developer.apple.com/documentation/foundation/nsinvocation/getreturnvalue:)：调用前检查大小和类型。
- [Objective-C 方法替换](https://developer.apple.com/documentation/objectivec/method_setimplementation(_:_:))：保留原 IMP，并验证选择器 ABI。
- [Core ML 稀疏算子源码](https://apple.github.io/coremltools/_modules/coremltools/converters/mil/mil/ops/defs/iOS18/compression.html#constexpr_sparse_blockwise_shift_scale)：只对选中的位置执行反量化。
- [固定版本的公开对照实现](https://pypi.org/project/coremltools/9.0/)：可选研究依赖，不是产品依赖。
