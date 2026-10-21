# 拒绝借用对象源，补公开原始恢复证据

本轮只加一个对象存储不变量及一个真实借用源负例，不改产品e107、CI/预算、权限或分支；没有
原生编译、PVT/性能run、rerun、dispatch、产品head push或合并删除。完整证据见[英文说明](README.en.md)。

采用最小的“显式拒绝源”方案：固定owner执行前、clone前、clone后checkout/build前，核有效
objects目录；拒绝非空Git目录重定向环境及alternates/http-alternates元数据（含空文件/悬空link）。
不repack/dissociate/改写/删除源，若clone后检查失败也保留现场。--no-hardlinks只防hardlink，
不再据此推断没有借用。这是检查边界的事实，不是锁或新的shallow/promisor完整性承诺。

真实fixture先造lender再git clone --shared；借用者能读commit但本地没有该loose对象，真实
alternates存在。修改前exit1回执证明旧helper再次clone时继承alternates，并到达模拟build边界；
没有Cargo/Apple执行，lender/borrower/owned clone均保留。修改后原位22/22、搬到无Git且带
空格中文目录22/22通过，仅新增这一个场景。借用源在新clone/build或owner执行前就被拒绝。
原生Rust/Swift编译、macOS执行与阶段成本结果仍未验；v25释放时点/日志开销等解释边界保留。

v25原本只在本地的原始fetch/restore/fsck日志和publication JSON已纳入公开材料，准确ref为
refs/tags/review/semantic-integration-20261020-evidence-v25，commit64fb7e5c5193698e88e8ba035be3c809f7c38763，
tree9f5214d0e7ba50a4d939b4658133be79194cf5f8。旧原日志使用<restore>标签；当前实际目录为
/workspace/xdremux-audit/restore-test.git。另有新的只读原始命令/逐条stdout、stderr、exit：
核无alternates、准确commit/tree、非浅bare，full strict fsck0；没有重复网络fetch。

另一次从空bare完整fetch尝试在120.002秒超时、stderr空，leader -9但group未确认消失；
随后只读show-ref返回128/不存在，没有执行fsck。只观测该PGID见PID1下的Git僵尸，原清理回执
仍uncertain，不回填绿。该仓库及partial数据保留，无额外signal/删除/fetch重试。原始命令行及
流未改；顶层终态注明超时及后续只读核对，不能把这个失败尝试冒称成功恢复。
新fix发布后的准确恢复日志将用单独receipt-only证据提交公开，避免提交自引用hash。

当前performance.yml在integrate/e107上有无输入的workflow_dispatch、rust-performance/xcode-27，
但运行旧绝对benchmark及候选单端profile，不调用这个修正原型。dispatch evidence tag也不会
自动接入，二者都不是可用入口，本轮未触发。选定环境为Linux，无xcodebuild/Swift，工具未暴露
dispatch；这不表示服务器拒绝，旧gh只读资料仍不足以证明Actions write被拒，未重试或绕403。

未来需父审明确的仅性能入口，固定product=e107及harness SHA、显式diagnostic模式、免费标准
arm64/macOS27及真实宿主/Xcode事实、双clean/full-history checkout、fresh release build哈希。
原型输入为source/CLI/adapter/两个hash/新文本输出目录，保留媒体与clone不得进入artifact。
旧预算失败保留且不跑PVT。当前workflow没有这些mode/输入，本轮不实现或执行其它触发方式；
准确harness SHA在后续发布恢复附件中列明供父判断。此会话当前没有可执行的原生路径。
