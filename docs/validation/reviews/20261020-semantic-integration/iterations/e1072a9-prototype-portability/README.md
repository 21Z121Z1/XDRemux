# 原型的可搬迁、固定源码与有界命令修复

本版只替代v24原型的可搬迁/生命周期设计，不改已接受的e107终态封存。产品head仍
`e1072a90e31b12861dfe8b9cf25e07ce75c17c44`。未改产品源码、预算、workflow、权限、保护、
main或分支；没有原生实验、dispatch、rerun或source push，operation_enabled=false。
完整实现、证据和限制见[英文说明](README.en.md)。

先用各自有期限的Git命令核实际checkout根目录、固定SHA、Git目录身份及tracked/untracked
clean，再取固定提交的owner Python与候选Rust字节，复核身份、先落盘hash后执行固定代码，
不导入漂移工作文件。ROOT固定为已验证目录。候选Rust也来自固定blob；迟到的工作树写入不会
替换已冻结字节，末尾仍需拒绝漂移，clean检查不能充当锁。测试使用相邻固定文本fixture及
隔离Git仓库，不再依赖原/workspace目录。错误SHA/根目录、脏树、读取期间写入均不会执行owner；
迟到写入测试证明执行/使用固定字节，不宣称变化后的树仍clean。

取消原型对owner linked-worktree finally路径的调用，改用一次性独立本地clone，无hardlink/
alternates、不登记共享worktree，Cargo/Swift目录独立且在文本证据外。成功、失败、中断都
按设计保留clone、target、样本；不运行worktree list/remove、force删除、reset或递归删除。
canonical comparison owner及产品源码未改，不能说其所有调用者都已修复。计时改写先落盘恢复
位置，将原inode留存后核字节，独占创建新叶；并发改动保留，新叶已存在即拒绝，不覆盖。
rename后中断可能使clone内叶缺失，恢复文件位置在回执中，不自动恢复以免覆盖其他写入者。

每条Git观察30秒，clone/checkout120秒，原构建/原生命令预算保留，整体90分钟只是额外上限。
超时/中断只SIGKILL本次新进程组，另有独立2秒reap预算，期间延后处理相关信号。不使用Popen
隐式无界wait。leader已reap就不向可能复用的group发信号；残留组、reap未知、Popen未返回句柄
均记uncertain，不猜PID、不全局reset、不自动重试。回执保留命令阶段、预算、流路径、退出/
异常、耗时和独立终止结果，main明确interrupted/timed-out/failed。没有句柄不等于没有执行。
workspace保留与进程停止是两件事，不宣称workspace已清理。SIGKILL/宿主丢失可能只有最后一次
checkpoint；命令期限不保证不可返回的内核/文件系统已恢复，也不能将uncertain写成通过。
只允许归档明确的文本清单，不上传含保留媒体/clone的上层目录。

真实验证：Linux Python3.12上21项测试原地通过；复制到无Git、带空格和中文的独立目录，清除
source/PYTHONPATH覆盖后同21项再次通过。故障回执含真实超时、exit7和stderr、SIGINT、Popen后
注入KeyboardInterrupt、主alarm耗尽及独立stop；模拟用户字节仍在。reap不确定的回执在测试
专用drain后仍保持uncertain；未知启动不猜PID。实际独立clone的模拟构建失败保留用户数据和
clone，源worktree清单未动；并发改写负例保留原inode并拒绝新叶。这些Git/进程fixture不运行
Cargo或Apple代码。平台特有测试明确POSIX skip；真实非macOS入口拒绝在Darwin上skip。
未捕获修改前失败执行，旧无界路径依据源码审查；新增故障注入是本实现行为，不冒称原生复现。

便携21+21通过；Rust/Swift原生编译未做；macOS原型未执行；没有阶段成本数据。PVT/性能CI没有
新run，v24的e10718绿2红保持。两端raster/gain_raster释放作用域与嵌套日志开销不同，阶段只代表
大操作，scene差值不能称纯重建/缩小算法成本。内层span不重复相加，阶段median不相加作E2E归因。

能力表述已修正：无workflow dispatch工具不等于服务器拒绝dispatch。旧gh只读Forbidden缺完整
endpoint/退出码/stderr持久回执，不能据此认定Actions write被拒；本轮未重试、POST或换路径下载
403artifact，也未rerun/推head绕触发新实验。便携检查命令为`python3 verify-portability.py`；
此文不授予原生运行许可。后续原生执行仍须固定原型/helper hash、准确源码和真实build收据审查。
