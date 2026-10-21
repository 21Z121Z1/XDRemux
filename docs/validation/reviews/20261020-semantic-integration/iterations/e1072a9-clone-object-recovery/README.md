# 最终clone对象修复的公开原始恢复附件

修复代码固定于1a1698e47e008b688cf5c3fc492dfc307b04ae1a，树ab487dafe6f6567726b9a2166172052cf8fe347f，
ref为refs/tags/review/semantic-integration-20261020-evidence-v26。[修复及22+22便携证据](../e1072a9-clone-object-independence/README.md)未改；本附件只加原始回执，不改代码/增测试。

v26原始fetch/restore/fsck日志、publication JSON、push输出均已可公开读回。旧日志<restore>
对应/workspace/xdremux-audit/restore-test.git，另有实际argv/cwd/逐条流/退出码/期限的完整原始
核验：远端ref、commit/tree精确一致，无alternates、非浅、full strict fsck0。通过后单独保存
日志，避免声称提交可包含自身未知SHA的恢复证明；没有分支/main写入或删除。

v25成功原日志和本次只读fsck仍保留；另一个空bare的120秒fetch失败也未抹去：ref不存在、
未fsck、cleanup uncertain，随后只观测到僵尸。未重试，所有仓库及本次owned fixture clone保留。

原生输入清单已固定harness=1a1698e47e008b688cf5c3fc492dfc307b04ae1a、product=e1072a90e31b12861dfe8b9cf25e07ce75c17c44，
CLI需source/新CLI和adapter路径/各自build hash/新文本输出路径。现有performance.yml@integrate/e107
的dispatch没有输入，也不调用修正原型；所需显式仅性能mode/product SHA/harness SHA输入目前
缺失，未实现/更改workflow或trigger。选定环境Linux无Xcode/Swift，没有dispatch工具，未请求
服务器，不代表Actions write被拒。仍需父审明确macOS27 arm64免费标准执行路径，不能用推产品
head、rerun旧run、PVT或其它入口替代。本轮原生编译/macOS执行/阶段成本结果均未做，旧失败
门禁保持；没有改权限、预算、安全或资源设置。
