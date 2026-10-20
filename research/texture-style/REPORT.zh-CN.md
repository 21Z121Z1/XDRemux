# XDRemux Texture Styles 原生适配调查（2026-10-19—20）

## 交付结论

已实现的最小方案是：**拒绝凭空生成拍摄私有信息，逐字节保留已有不透明 Texture 载体**。实现提交为 XDRemux **0a462134a6082b048508cee89c85615606abfc7f**，专用分支 research/texture-native-20261019。

已实际执行 Apple Texture 专用元数据解析、设置工厂、依赖查询和 Bloom 渲染入口。macOS27 接受部分元数据控制，但三个强度值均未完成真实渲染。**任意图像的完整原生 Texture producer、完整图像的编辑资格和 iPhone Photos 纹理激活仍未证明。** 普通 Photographic Styles 的构建成功、ImageIO解码、非空properties、非空CI计算图都不算纹理激活。

原生研究仅在自有 PSR 分支 research/xdremux-texture-native-20261019 的 macOS Actions 执行，最后探针源码为 **84f2c8309c5d778d911148aacbeaae3aff3bdd4f**。没有修改 master 或其并行 research/gpt-styles-20261019，没有合并 XDRemux #65/#85，没有发布、force push、外部PR或评论。原生工作流 sparse checkout 自己的探针与固定XD源码，不读仓库相机照片、secret或其他任务的样本运输。

2026-10-20恢复会话时，新选容器一直 starting；执行工具实际返回 No usable execution environment is available。此前本地待提交证据的远端写入未发生，远端仍为0a462134。**未把断连等同于文件丢失，也未声称这些本地未提交文件已核对。** 本次从已连接GitHub读取真实历史job日志，在同一研究分支保存独立证据；后续本地恢复时应保留原索引与文件，再逐项比较。

## 审计起点与证据图

XD baseline：research/texture-style@f8fcf2067c506e87665e9ea113324db1a57d4cf7。PSR只读起点：research/gpt-styles-20261019@5fef06eddaff8a8d2fd42c59e33612a5d576b088。先读了AGENTS及既有Texture/semantic研究，没有从类名重新猜一套算法。另只读参考PSR consumer-model与macOS Photos probe分支的源文本，没有执行其修改Apple断言/residency策略的探针。

完整可检索图见 [evidence-graph.json](evidence-graph.json)。

~~~mermaid
flowchart LR
  Pixels[图像像素 / 颜色域 / 方向] --> Derived[新推导的图像事实]
  Capture[原始拍摄与逐人上下文] --> Producer[完整原生生产链：未闭合]
  Derived -. 输入契约待验证 .-> Producer
  Producer -. 输出尚未取得 .-> Texture[2026 Texture元数据 / MakerNote84 / 关联资源]
  Styles[2023 Photographic Styles v15或v16] --> Semantic[_NUSemanticStyleProperties]
  Texture --> Parser[_NUTextureStyleProperties：仅控制plist解析已执行]
  Parser --> Pipeline[PITextureStyle / CMITextureStylesProcessor]
  Pipeline --> Abort[真实Bloom render：断言终止]
  Pipeline -. 未验证 .-> Photos[iPhone Photos Texture编辑]
  Existing[已有完整载体] --> Transport[Rust不透明检查 / 原样传输]
~~~

|对象|已有证据|不能推出什么|
|---|---|---|
|2023 styles URI|v15与诊断v16均属于Photographic Styles；v16的l=false仍在这一URI内。|不是独立2026 Texture元数据，不是纹理激活。|
|2026 texture_styles URI|公开控制含Preset、CaptureType、CaptureMode、PortType、HardwareModel、people version、seed。|固定字典不是随图像重新计算；不能给任意成片冒填iPhone拍摄来源。|
|Apple MakerNote84 / 0x0054|已有typed bplist槽位0—12的静态契约；旧writer会补Apple iOS、UUID与84。|没有闭合每个槽位的物理含义、计算入口或Photos必要性。|
|2026 auxC资源|旧writer给一个2019 skin matte分配十二个新角色。原生输入清单实际包含十三项，含hairMatte。|角色标签不认证像素语义，十二项也不是通用充分条件。|
|逐人数据|原生properties在空aux控制下persons0；部分效果配置确实requiresPersonData。|persons0不是画面无人判断，不能还原instance mask、ROI、pose、landmarks、眼部/图像统计。|
|原生生产候选|PITextureStyleAutoCalculator、CMI处理器input/outputPersonImageStats、BW相机metadata节点等可观察。|名字或selector不是已验证的任意图像生产API；捕获节点可能依赖sampleBuffer与完整会话。|

Apple官方说明把Texture控制与Styles调色区分，并给出设备/拍摄条件：[相机指南](https://support.apple.com/guide/iphone/use-photographic-styles-iph629d2cd37/ios)、[Photos编辑](https://support.apple.com/en-lb/guide/iphone/iphb08064d57/ios)。这些资料不公开私有元数据生产算法。

## 已复现的旧缺陷与最小实现

在独立原base checkout上，用纯程序构造的HEIF item graph复现旧probe。primary/gain/matte是测试哨兵，**不是可解码照片或Apple正样**。输入不具备相机来源，但满足旧probe的style/skin/Exif/tmap前提。

旧augment仍输出3655B，新增Texture URI、HardwareModel=iPhone19,2、Apple iOS MakerNote、十二个角色别名。输入SHA256：
42cb3c77691fb6985a7ff9af71ce0dc3f7958a311ccd3ce1ae290e96085cfa43。
旧输出：
0ad11ab02be096811550113cbb40b4c74c57e5119bcbfb651211a442fd7c7ea4。
拒绝伪造回归在原base故意exit101，失败于 OLD_PROBE_FABRICATED_UNBACKED_TEXTURE_CARRIER。这是旧缺陷证据，不是新实现失败。

Rust最小方案：

1. inspect_texture_style_heif精确读取infe URI字段，检查唯一item ID、本地iloc、mdat/idat范围、主图cdsc关联和引用。返回的是不透明本地graph事实与字节SHA，不认证plist、HEVC或蒙版语义。
2. preserve_texture_style_heif返回**完整原字节**。compressed media、Gain Map、ICC、irot/imir、Exif、未知box与尾部因而相同。没有Texture载体时明确generation unsupported；越界、错误引用或不支持的位置失败。兼容augment入口遵循同一规则。
3. --apple-styles改用实际提交到semantic consumer的v15数据，停止自动追加固定2026 plist、MakerNote与十二个别名。它仍是普通Photographic Styles路径；本轮没有提供“任意图像纹理成功”标志。
4. 示例客户端采用create_new，不覆盖已有输出。引擎的v16序列化保留为2023 schema诊断工具，不称完整Texture producer。

离线功能样本输入与输出SHA均为
4f823272e3778e5b44605e20194f1e112e04f8c6ed875ec84ed4fd0713a263a3。
六项针对回归覆盖缺producer、URI名称混淆、越界idat、错误/悬空引用、JPEG/截断与原样传输。完整HEIF crate共27项测试。这里的哨兵不可解码，只支持传输/拒绝结论，不能自证Apple接受。

## 原生运行与精确绑定

所有原生job都在PSR；artifact retention-days为1，只上传json/txt/log。四个Rust CC0解析图案的清单SHA：
7967cb1ea86f962b2b43cc2fa9f4a9f61cc706498598a38e7652bc57d49f50ed。
尺寸384×256或256×384，orientation1/6/8，SDR与HDR阶跃/亮点。原始raw、原始LinearThumbnail、真实相机信息均明确为false。

|lane|实际OS / build|Xcode / build|SDK|
|---|---|---|---|
|xcode-27|27.0.1 / 26A434|27.0 / 27A266a|27.0|
|macos-26|26.6.2 / 25G83|26.6 / 17F113|26.5|

|run与源码|真实结果|
|---|---|
|[39282392876](https://github.com/21Z121Z1/Photographic_Style_Research/actions/runs/39282392876) / 6f420d751d0be1b2dc6f93ba7cccf62493affae9|Swift ObjC runtime类型编译失败，未调用消费者。|
|[39283079303](https://github.com/21Z121Z1/Photographic_Style_Research/actions/runs/39283079303) / 4e6630d50eab3626254249ec5563af5ab6db8b93|两lane相同编译缺陷，未调用消费者。|
|[39285550760](https://github.com/21Z121Z1/Photographic_Style_Research/actions/runs/39285550760) / 4857d1a3fc4871297b29a1313468fd677fb278a7|元数据probe在非NSObject根类桥接SIGTRAP。产品job编译成功；12个原生I/O用例成功；4个普通Styles转换失败。|
|[39286462456](https://github.com/21Z121Z1/Photographic_Style_Research/actions/runs/39286462456) / d105a967bab6d3f1c577670751c6fce7257dd171|Swift不能导入RTLD_DEFAULT宏，未消费Texture。|
|[39287928184](https://github.com/21Z121Z1/Photographic_Style_Research/actions/runs/39287928184) / e653f24219dd7bd152381ccfa172dd59121ac360|C ABI过滤类名成功，27发现60个相关类；消费者被探针NSError-out ABI guard拦住，该轮未调用。|
|[39289168902](https://github.com/21Z121Z1/Photographic_Style_Research/actions/runs/39289168902) / acb572f425728963d0ee375327016c52ab069c10|修正o^@ ABI，真实元数据控制、设置工厂与输入配置执行。|
|[39290912880](https://github.com/21Z121Z1/Photographic_Style_Research/actions/runs/39290912880) / 00f7b5eb1d496c7b0b8b69ff3e80290aa3a29bcb|Bloom三个强度实际render均SIGABRT；CMI三种初始化均nil。|
|[39292067603](https://github.com/21Z121Z1/Photographic_Style_Research/actions/runs/39292067603) / 098430d4f20d832b1a1a9ccda0b066bde0748490|原生诊断定位_textureCache与Missing arguments断言；调试器exit0不是渲染成功。|
|[39293075204](https://github.com/21Z121Z1/Photographic_Style_Research/actions/runs/39293075204) / 84f2c8309c5d778d911148aacbeaae3aff3bdd4f|补入同图像推导的2023语义诊断参数后，断言推进为Unexpected number of inputs；仍无像素输出。|

前五轮探针编译/桥接/ABI缺陷已区别记录，不能倒写成Apple拒绝数据或平台不支持。

### 文件编码与实际产品路径

run39285550760/job118170329181固定XD源码0a462134。12个原生I/O用例全部成功：4JPEG、4SDR HEIC、2PQ HEIF10、2Adaptive HDR HEIC。SDR方向1/6/8与请求一致；PQ报告10bit和ITUR2100PQ；两份Adaptive HDR的ImageIO ISO Gain Map为true。image-matrix SHA：
86512eaadf547aed09b0ef41b3080924e586acd024c199642f329c3d128842a2。
这支持原生编解码及所读字段，不认证HDR像素保真或Texture。

实际Rust/Swift产品编译成功，但四个程序生成的ProXDR输入在convert --apple-styles均exit1：两个横向输入为 source HDR item 1 has no dimg reference；两个竖向输入为 semantic scaffold has no tmap item。图案、尺寸、方向同时变化，不能单独归因于方向。均未到最终Styles组装，batch未运行，输出发布状态未独立检查。product-matrix SHA：
3c0c641710e32667952a73f2349f509906924fd4e50cc1dbb5e2b23bc5ae6fe8。
因此runtime路径只证明编译，**功能仍有阻塞**，不能由离线green gate抹除。

### Texture元数据消费者

真实selector为 _NUTextureStyleProperties.textureStylePropertiesFromImageMetadata:auxImageMetadata:error:，运行时编码 @40@0:8@16@24o^@32。输入为公开形状的合成二进制plist，aux字典为空，未写回照片。

run39289168902/job118184997247的结果：

|控制|27结果|该次二进制plist SHA256|
|---|---|---|
|Standard / version3 / seed0|Texture properties非空，NSError空，已读capture/hardware/port/seed，persons0。|f28bac185f7c081964905aac70e219e20cb1354ee855b86628c47ea6ae444d4e|
|Studio|同样非空；未取得用于核验Preset的getter。|39c4f3296fb6ee03076f775a6c04b46b1332dab846928e6bc823455311775f8c|
|缺HardwareModel|仍非空，未返回hardwareModel。|a07f8669cb4e56efa7e6b2c5f8822b52620d69da890385082f5e0fac4c2d6643|
|people version999|仍非空；version只读到NUVersion对象，未读数值。|a22168ec7e314a25f80a1bc8e81b916e89e1f2eb3af863135fe53b60e84c2563|
|空字典|nil，NUError code3。|9261ecceda608ef174256e5fdc774c1e6e3dcf533409c1bc393d490d01c713f1|

26的Texture类不可用，未调用。后续各run有同字段控制，但二进制plist每进程的实际SHA以各自JSON为准，不假设Apple序列化字节稳定。

非空仅证明这次元数据反序列化；version999非空不说明保留999或版本校验成功，persons0不说明画面无人，缺HardwareModel仍非空说明其不是完整性判据。没有执行完整HEIF/composition的canRenderTextureStyles资格判断。

FigCaptureTextureStyle和AVCaptureTextureStyle原生 styleWithPreset:intensity:grain: 共30个控制对象可构造：5预设×强度0/0.5/1，grain0.25。它们没有图像输入，是编辑设置工厂。不要混淆AV设置版本1、Photographic v15/v16、person metadata3与CMI APIVersion20260507。

### 效果依赖与原生渲染

27实测pipeline.isAvailable和isAvailableForTesting均true，没有使用override。inputNames为十三项，含hairMatte。依赖按效果变化：

|配置|required|optional|requiresPersonData|
|---|---|---|---|
|Bloom|空|空|false|
|Diffusion|空|skinMatteV2、personMatte|false|
|FilmGrain|空|personMatte、skinMatteV2、skyMatte、semanticStyleCast、semanticStyleColor|false|
|Glow|personMatte、linearThumbnail、semanticStyleCast、semanticStyleColor|空|false|
|Halation|空|skinMatteV2、personMatte、semanticStyleCast、semanticStyleColor|false|
|numeric1|faceSkinMatte、personInstances|hair、lips、ears、tattoo、glassesV2、nose|true|
|numeric2|skinMatteV2、personInstances|完整11项见JSON|true|
|numeric3|skinMatteV2、faceSkinMatte、personInstances|glassesV2、ears、nose、lips、tattoo|true|

numeric1/2/3的原生名称映射未取得，不猜。配置required为空不代表底层kernel接受一份图像。

Bloom输入是Rust HDR-edge raw，SHA：
0df50938ed0aade93fcc56d138cd6ba310677d8bb9564072dfc5280821ae9572。
输入和CIContext明确为analytical extended linear Display P3；内部域、相对GTM/LTM顺序仍未证明。使用Apple自己的Glowy调校查询和Bloom descriptor，iPhone19,2/LF/Still/Back只作为明确的调校目标控制，未写回输入或冒称相机来源。每例一份图像，没有语义角色填充或LinearThumbnail资源。

原生CIImageProcessorKernel.apply返回延迟计算图，但强度0/0.5/1全部在真实render SIGABRT(-6)。[官方apply API](https://developer.apple.com/documentation/coreimage/ciimageprocessorkernel/apply%28withextent%3Ainputs%3Aarguments%3A%29?language=objc)的计算图返回不能代替像素输出。default、nil queue、真实Apple Paravirtual device queue三种CMI初始化均nil。

未修改Apple方法或residency策略的子进程诊断先看到 CMITextureStylesProcessor.m:225 的_textureCache断言，再看到PhotoImaging Missing arguments。随后Rust以同图案的新统计、GTC、light maps和显式identity协议控制构造2023诊断plist，SHA：
ab2be255437bdd26285119966f796316a3fda576563a3b3279cd649355b0e00a。
它被原生_NUSemanticStyleProperties接受；加入semanticStyleProperties参数后，内核变为Unexpected number of inputs。该控制不是2026生产结果或相机原始统计，也没有赋予任意输入一个固定正样。

这定位了未闭合的参数/输入布局契约，**不能仅归因于GPU或硬件不支持**。没有用零图、重复蒙版、断言补丁或空输出将失败变绿；没有成功执行像素差异统计。

## 任意图像适用边界

|信息|可行范围与本轮结论|
|---|---|
|尺寸、编码色彩空间、方向、Gain Map、资源哈希|可观察。本轮12例原生I/O读回，transport保留原字节。|
|新图像统计、语义与脸部几何|可对正确方向/域重新分析，属于新上下文；尚未证明满足完整Texture producer。合成图案诊断不能推广到人像或任意场景。|
|Preset、强度、grain、新seed|可作为明确的新编辑选择，不是恢复拍摄选择。本轮只测设置工厂与控制参数。|
|HardwareModel、PortType、CaptureMode/Type|不能仅凭像素认证历史来源；缺失时unsupported或保持缺失。调校目标必须另作标注。|
|原始RAW、原始LinearThumbnail、逐人原始统计|普通成片不能认证这些历史信息。没有资源就不称已恢复。|
|LinearThumbnail|本轮未改原产品路径，也未在新native试验中生成该资源。后续新派生资源须沿Apple Log并明确derived，不改成PQ或冒称RAW。原路径的颜色域/保真仍需独立核验。|

[Apple HDR说明](https://developer.apple.com/videos/play/wwdc2024/10177/)与[HEIF10 API](https://developer.apple.com/documentation/coreimage/cicontext/writeheif10representation%28of%3Ato%3Acolorspace%3Aoptions%3A%29)支持HDR/Gain Map编解码路径，不提供Texture相机上下文生产算法。PQ测试是独立HDR输入测试，不是LinearThumbnail替代。

真实相机Texture正样的历史记录仅作既有研究来源，本轮没有使用或上传人像原件、私有媒体、密钥、厂商二进制，没有读取别的任务secret。没有真实正样或iPhone UI能力，就不宣称完成激活对照。

## 验证层级

|层|已证明|尚未证明|
|---|---|---|
|字段可读|静态字段、27的原生properties getter。|Preset getter、NUVersion数值、原始相机事实。|
|结构完整|限定本地opaque graph检查与全字节传输/拒绝。|通用ISO/HEVC认证和蒙版语义。|
|原生消费者接受|Texture专用解析器4接受/空字典拒绝；26类不可用。|完整文件准入；ImageIO通用解码另列。|
|Apple API认定完整图像为Texture可编辑|未验证。|composition资格与完整资源图。|
|实际参数/渲染有效|真实尝试已执行，全部断言失败。|有效输出、强度/grain差异。|
|真机Photos纹理控件及持久编辑|未执行。|已授权正样、设备/UI与重新打开后的持久性。|

## 证据完整性与复现

已重新读回**21份完整job log**；18份有完整manifest的job恢复并核验**147份原生JSON**，逐字节SHA/size匹配runner manifest。见 [native-readback-index.json](evidence/native-readback-index.json)。源码、run、OS/SDK、输入SHA均能沿图和manifest追踪。

ZIP下载引用在前容器遭HTTP tunnel403；**没有验证ZIP字节**。日志恢复是允许的文本读回，不声称完整artifact条目均已取回；上传器报告ZIP摘要也只作为报告值。所有脚本、日志和元数据是文本，没有提交生成图片或rawbuffer。为保留原始CR与时间戳尾随空白，.gitattributes仅对本次run目录下的job*.log设置-text与-whitespace；原始字节完整性继续由SHA回归与全量日志复核检查。[Git属性语义](https://git-scm.com/docs/gitattributes)适用于这一限定路径。原生Apple结果来自真实runner，不用自己的生成器和解析器自证整条链成功。

~~~bash
# 输出目录必须不存在；可选--baseline-repo引用另一个干净f8fcf206 checkout。
python3 scripts/research/verify_texture_transport.py --out /tmp/texture-offline-new

# 原生日志恢复与manifest核验；仅重验文本完整性，不重新执行Apple。
python3 scripts/research/read_texture_native_log.py JOB.log --out /tmp/native-log-new
python3 scripts/research/verify_texture_native_evidence.py

# 拒绝、传输与证据读回器的负例。
cargo test --locked -p xdremux-heif --test texture_transport
python3 -m unittest Tests.test_texture_native_evidence Tests.test_public_documentation Tests.test_rust_apple_semantic_policy

# 提交后按AGENTS运行，并verify其exact-HEAD receipt。
python3 scripts/agent_completion_gate.py run --base f8fcf2067c506e87665e9ea113324db1a57d4cf7 --plan research/texture-style/offline-verification-plan.json
python3 scripts/agent_completion_gate.py verify .codex/verification-receipts/$(git rev-parse HEAD).json
~~~

Swift只承载Apple API与探针；图案、统计、协议结构、像素差异统计都在Rust。PSR的native_images.swift、native_probe.swift、semantic_control.rs、run_metadata_matrix.py、run_bloom_matrix.py、run_product_matrix.py及隔离workflow提供真实命令。

前一容器在任务目录用官方Rustup安装Rust1.99.1，未改profile；使用已有libheif1.19.8及任务内匹配实际版本的pkg-config文件，未修改预加载libheif或环境配置。Linux运行macOS模块过滤实际0项，不算原生通过。后续exact-HEAD gate的范围仅为已测试的离线传输和证据校验；四个普通Styles功能失败与完整Texture/Photos阻塞不会被gate覆盖。最终交付SHA的收据由PSR隔离offline-validation job输出并读回，不把源码中的预期状态当作收据。

机器可读结论见 [claims.json](claims.json)，具体下一步和阻塞见 [NATIVE_TODO.md](NATIVE_TODO.md)。
