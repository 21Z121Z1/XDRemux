# 手动 Styles 诊断入口：静态审查

入口固定在 **33bbe860ad6b1777839753a738ec17d3f9229b61**，tree
**bfd8a388308c0e3b6e3e59a6065abb54247e37cc**，独立持久 tag 为
`review/styles-stage-diagnostic-20261021-v1`。它是产品 e107 的独立子提交，
仅改六个文件；main 与 PR71 的 e107 源 head 均未改。本轮没有原生执行、
dispatch、rerun、新 PR、合并或删除。

[准确请求](dispatch-request.json)仅已准备，未提交。沿用默认分支已有的
`performance.yml` 注册；新 job 必须同时满足 workflow_dispatch、显式诊断
模式、准确 tag、匹配的完整入口 SHA 和本用户公开仓库元数据。普通 push/PR
不能启动它，诊断模式不能启动旧性能 job；此 tag 即使选择 standard 也不能
误启旧 job。旧 job 的步骤、runner、阈值及期限与 e107 fixture 完全一致，
原 concurrency group 移到 job 层；新诊断有独立、按 run 唯一且不取消其他
工作的 group，没有 PVT job 或依赖。

父审后若执行，仅使用既有免费标准公开 xcode-27 arm64 配置、contents: read，
运行时核实实际 arm64/macOS27、Xcode27/SDK27。没有新增收费或自托管 runner，
未改预算、权限、安全或保护设置。免费标准分类沿用此前已授权配置；不能仅凭
标签名推断，最终执行前仍须复核。

三个分离 checkout 固定 control、产品 e107 和 harness
1a1698e47e008b688cf5c3fc492dfc307b04ae1a。先核 clean、准确 SHA、无 alternates、
独立存储、实际 tracked 字节/模式及两个已审 Python SHA256，再执行原封不动
的 harness。Cargo home/target 和 Swift scratch 为新建空目录，没有 Actions
产品缓存；记录构建命令、选定环境、工具链/路径、feature graph、arm64 架构和
真实新 CLI/adapter hash，结束后复核源码及二进制。

准备阶段 15 分钟，已审 harness 保留自身 90 分钟及清理行为；step/job 为
110/120 分钟，最后源码核对 60 秒，bootstrap/终止各 20/2 秒。保留失败、部分
收据；checkout 前已有初始文本收据。仅显式白名单中的有界 UTF-8 JSON/log
可平铺上传，拒绝 symlink、二进制、越界、大文件和疑似凭据。clone、媒体、
converter stdout 与二进制不进入 artifact。上传尚未在 GitHub 实测；宿主丢失
最多留下最后 checkpoint，缺日志不能证明清理成功。

准确入口提交上的 **65 项便携测试通过**（14 新增、51 既有）。见
[命令/源 hash/日志 hash](styles-dispatch-committed-test-receipt.json)、
[原始日志](styles-dispatch-committed-tests.log)和
[14 项事件矩阵](static/event-mode-matrix.json)。矩阵使用实际 job 表达式的
静态子集，不等于 GitHub 执行；编译器/架构收据测试明确使用模型。Linux 入口
实际在 checkout/build/harness 前拒绝。初次开发测试的目录准备错误日志仍保留，
已在最终提交前修复。[固定 blob 证明](styles-dispatch-fixed-blob-proof.json)
核对 1a169 原文和 e107 workflow fixture，没有改已验收原型。

[发布前原始核对](preflight-raw/report.json)覆盖默认 main、入口和旧证据树的
workflow、9 个远端 head、工作区及原有 dirty 文件/patch。没有套用旧 patch。
[PR71 状态](styles-dispatch-pr71-state.json)仍为 draft/open/e107。三个树的六个
push workflow 均仅 branches: [main]，无 tags/create/workflow_run；根据
[GitHub branch/tag 规则](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#onpushbranchestagsbranches-ignoretags-ignore)，
未定义的 tag 类型不触发，未把 path filter 当作 tag 防护。
[默认 CodeQL](https://docs.github.com/en/code-security/concepts/code-scanning/setup-types)
只扫描默认/受保护分支。先完成这些核对，才普通新增审查 tag；
[发布与恢复原始日志](publication-restore-raw/report.json)含准确 ref/commit/tree、
独立非 shallow、无 alternates 恢复和 full strict fsck 0。使用既有独立裸库，
没有隐去旧 fresh-fetch 超时。[全事件 Actions 查询](styles-dispatch-server-runs-initial.json)
在发布后该 SHA 下为 0 次，属于带时间的观测，不保证未来永远无运行。

本次为按项目 pstack workflow、工程原则和 review rubric 进行的作者审查，
没有独立 reviewer 或替代模型会话。边界纪律用于固定源码、新输出、收据与
导出所有权；直接验证区分模型、Linux 拒绝及未执行原生行为。最终便携审查
未发现新增可证明的范围内阻塞，不推导原生通过。

当前会话没有 callable workflow-dispatch 工具，未尝试 POST，不能称 Actions
write 被拒。客户端不支持的只读 URL 和旧 gh 缺完整收据的 Forbidden 都不能
证明 dispatch 权限。服务端输入校验、macOS 编译、阶段测量及 artifact 上传
均未执行。父审后才由合适工具/操作方提交准确请求，并先复核免费标准资源。

e107 仍为 **18 绿、2 红**，所有旧失败保留：Styles median14.458/p9516.156 秒
超过原 11.732/15.406 秒门槛，同机 ABBA main12.434200333/candidate14.704374375
慢 18.2574993%。没有阶段结果或优化结论；raster/gain_raster 释放时点和嵌套
日志开销不同，不相加重叠 spans 或阶段 median 当 E2E 归因。PVT 仍停在模拟器
服务，产品调用 0、清理不确定；新入口没有 PVT，不给出 Photos 接收/拒绝结论。
整包保存/独立 readback 和壁纸仍各自未验收。main/PR heads、预算、严格规则与
operation_enabled=false 均未改。完整英文说明见 [README.en.md](README.en.md)。
