# 原生后续与阻塞（2026-10-20）

## 已完成与实际失败

- 安全Rust传输与拒绝伪造：实现0a462134，旧base缺陷已复现，六项针对回归和不透明全字节功能检查通过。
- 21份真实job日志、18个manifest、147份原生JSON已读回并匹配哈希；不等于ZIP字节或Photos验证。
- macOS27的Texture解析控制4接受、空字典NUError3；macOS26类不可用。30个设置对象与效果依赖已观察。
- 12份CC0 JPEG/HEIC/SDR/PQ/Adaptive HDR原生I/O成功；不是Texture准入或HDR像素保真认证。
- 四个普通Styles convert在dimg/tmap条件失败，未到最终组装，batch未运行。不能说runtime功能完成。
- Bloom真实render已做强度0/0.5/1。先Missing arguments；加入同图像派生2023语义诊断参数后为Unexpected number of inputs。仍无有效像素。

## 当前阻塞，按动作定位

1. 动作：完整原生Texture图像计算、准入及渲染。目标：PITextureStyleProcessorKernel / PITextureStylePipelineProcessor / CMITextureStylesProcessor。原因：参数与固定输入布局未闭合，CMI初始化nil且_textureCache断言；单图控制触发输入数量断言。状态：**执行失败，原因未完全闭合**。不能称硬件必然不支持。需要追踪合法native graph与usage输入布局，而不是用自己的假角色填满数组。Bloom required为空不是整个kernel的充分条件。
2. 动作：普通Styles完整产品路径。目标：0a462134的四个合成ProXDR输入。原因：primary无dimg或scaffold缺tmap。状态：**实际exit1**。需单变量layout/方向对照修复，保留四个负例；独立检查媒体、颜色、HDR、orientation及发布状态。扩大图像尺寸或换图成功不能抹掉旧失败。
3. 动作：读回artifact ZIP字节。目标：GitHub工具返回的下载文件引用。前容器HTTP tunnel403，状态：**字节传输被拒绝**。已有允许的完整job日志及SHA核验JSON；未绕过网络，ZIP仍未核验。
4. 动作：真实相机正样与iPhone Photos纹理编辑。目标：已授权且无隐私的真实正样、设备/UI。原因：目前没有相应媒体授权与设备能力。状态：**未执行**。没有读取其他任务secret、使用其临时运输或上传敏感样本。
5. 动作：恢复本地待提交文件并重新核对。目标：/workspace/xdremux-texture-20261019等旧目录。新选容器starting，执行工具返回无可用环境；状态：**pending，文件是否仍可访问未知**。远端0a462134与PSR84f2c830已核对，后续证据用GitHub授权工具保存。不要将断连视为数据丢失，或reset/覆盖旧索引和文件。
6. 动作：为最终交付提交生成并读取exact-HEAD receipt。目标：PSR隔离offline-validation job。仅验离线传输、回归、文档、证据完整性；不得把此gate描述为Texture或普通Styles产品功能通过。收据绑定实际XD SHA后再报告结果。

## 接续顺序

1. 先核对实际run/head、OS/build、Xcode/SDK与样本SHA。各进程二进制plist字节可能不同，按其JSON绑定，不能假设Apple序列化确定。
2. 从已存在且ABI匹配的方法继续。Texture properties是反序列化，不是任意图像计算API。设置工厂无图像输入，也不是producer。
3. 取得NUVersion数值、规范化序列化和Preset真实getter。version999非空不等于版本兼容，persons0不等于没有人物。
4. 保留native effect配置：13个inputNames含hairMatte；Glow要求LinearThumbnail等，numeric1/2/3需personInstances/personData。其名称映射未知。区分effect required/optional与kernel固定输入布局。
5. 建立真正的生产图与新推导上下文。在正确颜色域与方向下计算新的统计、场景、语义、人物几何，Rust优先。缺原始拍摄信息不冒填。涉及派生LinearThumbnail时保持Apple Log，标注derived；PQ输入不是其替代。
6. 对真实编辑做identity/非零强度/grain对照并取得有效像素，Rust检查非有限值、变化范围、输入输出SHA。Apple原生实现不打补丁；调试器停止或exit0、非空CI图、空图和原图都不算成功。
7. 最后才在同一iPhone系统与设备上做原厂正样、结构仿制负样、缺依赖负样；验证Texture控件、实际参数效果、导出与重开持久性。没有这一层就保持Photos=false。

不合并、不发布、不force push、不向外部上游评论/提PR，不清理其他任务分支、secret或release。Artifact统一retention-days:1。后续改源码或报告后旧HEAD收据失效，应重新运行；阻塞不会因离线gate绿色而消失。
