# 固定源码的手动 Styles 阶段诊断入口

入口仅在独立审查tag上扩展现有performance.yml，不是产品预算门禁。本轮不dispatch或运行
原生诊断，main及PR71的e107产品head不动。完整契约见[英文说明](styles-stage-diagnostic.en.md)。

未来仅在父审后，以review/styles-stage-diagnostic-20261021-v1及准确提交调用：
mode=styles-stage-attribution，reviewed_entrypoint_sha=该完整入口SHA。product固定
 e1072a90e31b12861dfe8b9cf25e07ce75c17c44，harness固定
1a1698e47e008b688cf5c3fc492dfc307b04ae1a，不允许输入覆盖。含准确SHA的请求另存静态审查材料，
本轮不提交。普通push/PR不能选新job；默认预算job步骤/阈值保留，审查tag即使standard也不能
误跑旧job。未知/缺失条件不能分配诊断job，运行时再核；skip不能作验收。

新job按run独立concurrency且不取消其它工作，无PVT、可复用workflow或跨job依赖。权限仅contents:
read，沿用标准xcode-27，实际要求arm64/macOS27、Xcode27、SDK27。以后执行前须再核公开仓库
及免费标准runner分类，不启用收费/自托管/预算/权限/保护改动。

三份full-history checkout分离且不持久化credentials。先有界核固定SHA、root/Git/objects身份、
clean及无alternates/目录重定向，并逐文件比实际字节与Git blob，再核两个受审Python SHA256；
之后才载入固定代码，受审原型不改。15分钟准备期限含验证、依赖及fresh release构建；
Cargo target/home、Swift scratch全新空目录，无Actions产品缓存或外供旧binary，拒绝wrapper/
自定义target/flags。收据包含命令、所选环境、tool版本/路径、feature graph及真实CLI/adapter
架构/hash，失败也保留。构建和诊断后再核产品字节，binary再hash；不声称SDK/宿主冷启动。

同进程调用原型CLI，保留原90分钟budget与进程清理语义，避免外层SIGKILL遗落其新session。
step/job为110/120分钟，末尾校验60秒，bootstrap/stop各20/2秒。失败/超时/中断/未知清理/缺失
回执均不通过。clone、媒体、build留在文本artifact外；SIGKILL/宿主丢失可能只剩最后checkpoint。

checkout前即保存初始收据；driver只认领全新artifact目录。上传仅平面json/log及初始单文件，
不递归raw/checkout/clone，不上传人像/电影/像素缓冲或converter stdout。仅列名收据、所选
构建/graph/计时stderr/validation/timer patch可导出；symlink、二进制、非UTF8、超限或疑似
credential文本留本地并记failed-export。单文件4MiB、总32MiB、保留1天，不dump完整env/event。
诊断完成不覆盖旧绝对预算失败或必需门禁；释放作用域及嵌套日志成本不同，不能把scene差值
当纯算法成本，也不把重叠span或阶段median相加当E2E归因，不预判根因。

本轮只做YAML/表达式矩阵、固定source、fresh目录/收据模型、文本边界和真实非macOS入口拒绝，
加原性能/workflow策略回归。模型输出不是实际build收据。没有macOS编译/测量、服务器dispatch
输入验证或artifact上传；当前无dispatch工具不代表Actions write被拒。审查ref推送前复核6个
push workflow均main-only，GitHub规则会排除tag；默认CodeQL仅默认/受保护分支。静态证明不等于
已发生服务器运行。
