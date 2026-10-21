# 原生 Styles 阶段诊断，run192

父助手通过合法 GitHub Run workflow UI 一次提交已审请求。
[run39644162258/job119375231267](https://github.com/21Z121Z1/XDRemux/actions/runs/39644162258/job/119375231267)
于2026-10-21 17:37:44 UTC成功结束，attempt1，准确 tag/head33bbe860…。
普通性能 job 跳过；本会话没有 dispatch/rerun、新 PVT、产品修改、合并或删除。
终态见 [assessment](terminal-assessment.json)、[原始日志](job-terminal.log)、
[run](run-terminal.json)及[jobs](jobs-terminal.json)。初始 pending 快照属于历史，
已由终态覆盖结论；运行中的日志读取 BlobNotFound，终态读取成功。另一个 URL
在 connector 客户端 allowlist 被拒，不是 GitHub 权限拒绝。

已从日志完整解出[harness JSON](harness-report-from-job-log.json)：12次转换
及不改变输出 hash 的 CLI validate 全部成功，每端1次预热、5次测量；60条
命令全部exit0，所拥有进程组清理已确认。保留自有 clone，没有自动删除；
远端临时现场不当作持久备份，也不证明 Photos/设备接受。

实际环境macOS27.0.1/26A434、arm64、Xcode27.0/27A266a、rustc1.99.1；托管
镜像xcode-27-arm64，GitHub Actions runner group。候选固定e107，基线main89
仅插入可逆计时器。比较owner/Rust源码hash已与e107 Git blob核对；报告有clean
身份、无alternates clone及源码恢复证明。三个checkout的SHA亦在原始日志。
固定driver成功执行要求先验请求/源码/harness字节，但完整原始checkout/build/
export收据仍未本地读回，不能冒充独立核验已完成。

候选实际使用的新路径CLI hash为
`c4eab27ad5fa792f2ac5e9ebc82c3e312a69ae1923dcc61bc25102bc8db7a84b`，
adapter为`f50c7c11d2ddf5f36b394e273373252e87b0328b4d99d2a26db573bd11d06e04`。
报告含真实字节数/新Cargo与Swift路径、独立基线二进制与feature graph hash；
candidate-build.json、图文本、lipo及完整前后manifest还需artifact独立核实。

[测量分析](measured-stage-analysis.json)保留10条正式样本并重算中位数：同机
带计时器基线/候选10.13/12.22秒，慢20.6318%；user9.86/10.90、sys1.20/1.80秒。
scene_derivation为0.176646333/1.141381834秒，key1_fit为0.073629334/0.406472208；
semantic_production为2.877810500/3.284635333，texture_generation为3.584953583/
3.821498000。候选内层HDR重建0.815169583、线性缩减0.126567625秒，与scene
重叠，不相加。两端分辨率、raster释放时点、语义及嵌套日志不同，不相加阶段
median解释E2E，也不把scene差称纯算法CPU。数据支持优先做重建小原型，不证明
根因；保留local key1语义，旧安全累积优化已在e107，不能重复当新修复。

[artifact元数据](artifacts-terminal.json)：主文本12367047255，355356字节，
预期API digest为`46e9002136410ab4b0e79f636b702660cb1243a224fa1bdfd1bc9fff511768fd`，
2026-10-22T17:37:37Z到期；setup12367436245为337字节。connector已成功返回
两个新文件的合法引用；第一次本环境主文件materialize GET抛URLError，没有ZIP。
[收据](artifact-acquisition.json)遗漏底层reason，HTTP状态/代理是否拒绝未知，
不能称403。没有重试、换路由、访问旧artifact、改TLS/安全；setup也未另行GET。
不封存signedURL，API digest仅预期值，不冒称ZIP实测hash。独立取得的job日志
可以证明实际测量，不能代替未读artifact成员的核验。

完整fresh构建命令/环境、feature graph、准备耗时、tracked字节manifest、export
manifest及样本原始日志仍待合法读回，因此独立收据审计未完成。具体未执行的
下一步和重建假设见[bounded-next-plan.json](bounded-next-plan.json)。本轮没有
产品优化或新实验，先交父审。

已继承v28入口恢复证明，并新公开[v28原始恢复命令](v28-publication-raw/report.json)、
[准确ref/commit/tree收据](evidence-v28-publication.json)和[fsck日志](evidence-v28-restore.log)，
覆盖aeffd22及2128项清单，独立无alternates、非shallow恢复、fsck0。
[工作区盘点](workspace-before/report.json)含9heads/main89/PR71-e107/入口/v28及
未变脏文件/patch hash，没有套旧patch。

e107仍18绿2红：原绝对门禁14.458/16.156秒超过11.732/15.406，同机原ABBA慢
18.2574993%及所有旧attempt保留。本诊断不替换正式预算或exact-head门禁。
PVT仍为旧控制服务失败、产品调用0、清理不确定后的未验收；没有新Photos或
壁纸结论。operation_enabled=false，main/PR71、阈值、预算、保护、权限、runner
类别及媒体边界均未改。完整范围见[英文](README.en.md)。
