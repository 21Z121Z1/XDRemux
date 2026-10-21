# 同一 job 的一次就绪对照：固定双 SHA，明确预热

harness e1072a90e31b12861dfe8b9cf25e07ce75c17c44，树209f1b5abc6faed76b65ebf8fcd093f9973465fa；产品/原探针固定8d6c2823f015f06ca28d35ef80e4b3e7e2a4971a且不改。
完整协议、限制和证据见[英文说明](README.en.md)。新head原生/适用CI仍待结果，旧19绿不回填。

原research单job内双checkout核固定SHA、7产品tree及原探针所有文件一致后，先创建控制UUID，
bootstatus最多180秒，只一次20秒listapps且须bundle dictionary。只清理该控制UUID；退出码、
一次device-list不含UUID且目录消失才确认。失败或清理未知即停止，不猜UUID、不重试/重装/
全局重置。控制成功清理后才一次执行原8d6完整探针，其自身就绪/一次安装/三个整包入口/
独立单Live资产/原资源/ID/封面/hash/清理全部保留；不拆包创建pair替代。

同一Python调度步骤直接顺序调用，不以needs跨job当同宿主。控制可能预热CoreSimulator，
作为实验条件记录；通过仅证明该分配环境当时响应，不称冷启动修复/新VM或Photos成功。
公开仓库API及GitHub官方标准runner表已核免费compute；沿用xcode-27/25分钟且加公开元数据
guard，不增付费配置/预算/支付方式。准确head局部门禁5/5通过，含320Python、真实CLI及双
checkout绑定/未选原生时0调用；58重点通过。这些模型/CLI证据不是Apple验收。

v21引用/终态问题已在v22固定；本版计划指向同目录最新实际文件，operation_enabled=false。
39持久refs覆盖9head并完整恢复/fsck0，原4PR祖先保留、main仍89d806c8、其他8head未动；
既有未提交文件/补丁可重建且未覆盖。合并/删除仍待父审。全部历史性能失败和同机差距保留，
不跨runner推断新实验改善原因。Photos整包导入/真机/墙纸尚未验收。
