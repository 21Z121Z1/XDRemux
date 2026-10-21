# e107 准确 head 的终态与有界性能归因原型

源码/harness e1072a90e31b12861dfe8b9cf25e07ce75c17c44，树209f1b5abc6faed76b65ebf8fcd093f9973465fa。
[20项检查已全部结束：18绿2红](e1072a9-ci-terminal.json)，取代v23当时仍运行的快照和待定计划。
完整证据、实际日志与边界见[英文说明](README.en.md)。exact14、App实际执行及32路径保护、
Tone真实数据、System HEIF和arm64离开checkout转换成功，不覆盖下面两项失败。产品范围仍
arm64/macOS27+，保留跨平台Rust。

PVT只执行独立控制：bootstatus152.202秒exit0并仅称already booted；唯一listapps在20.157秒
超时。shutdown/delete/清理验证也超时，cleanup_status=uncertain，不能称设备已清理。
产品探针调用0，尚未产品编译/生成媒体/安装/Photos或独立读回；没有本轮新媒体hash可报告。
不是Photos接收/拒收证据。artifact12330064787 API大小11302字节、SHA256在英文列出，
10月22日08:26:46UTC过期；本环境一次下载被代理Tunnel403拒绝，未取得ZIP字节，无本地校验
成功说法，也没有绕过。完整打印回执与job日志已封存。不重跑控制，须先有新健康隔离环境/
服务恢复证据。完整包导入、单个Live资产/原资源hash/配对ID/封面时间仍待验，不拆包pair替代；
macOS Photos、真机和墙纸另列。

性能attempt1真实失败：Styles median14.458>11.732秒，p9516.156>15.406秒，RSS在限内。
同宿主ABBA main12.434200333、candidate14.704374375，+18.2574993%；user11.70→13.29秒、
sys1.83→1.99秒。全部12行及两端build/hash保留，未刷绿或放宽阈值。旧失败、7a有理由的一次
重试以及+33.1235/+13.0422/+12.3091%等历史均保留，不跨runner判改善/根因。
候选profile的base3.628/semantic3.226/scene1.337/Texture4.003秒仅供排序；HDR0.961和缩小0.176
属于scene内层，不能重复相加，更不能据候选单端数据称已解释差距。完整HDR先重建后缩小的
语义不可为提速删掉，目前没有产品优化或完整因果结论。

封存了可执行的阶段对照原型，复用既有owner，固定main89/e107，只给一次性main副本加13个
可逐字节撤销的同边界timer。独立target、真实adapter、1预热+5轮交错，每轮回执完整且验证
输出hash；独立原生读回在计时外，整体90分钟和子进程均有界。只比较13个不重叠阶段，不把
阶段median相加当E2E归因。5项便携检查通过；没有failing-before产品复现。恢复容器缺cargo，
本地编译未开始且自有临时worktree已移除；模块仅macOS cfg，Linux check也不能证明编译。
原型尚未在macOS编译/执行，不能作原生阶段结果。工具未暴露dispatch，gh只读API也Forbidden，
未POST/绕过。未推新PR head，以免触发无新健康证据的PVT控制。下一步需仅性能的免费标准
arm64/macOS27分配环境，固定原型/产品hash及真实build收据，保留正式绝对门禁。

39refs覆盖9heads重新从远端核对、独立恢复全部commit/tree、non-shallow/fsck0。唯一旧dirty
文件和patch精确恢复且未应用。PR/规则/默认设置已刷新：5PR开放，PR71draft/e107，main89，
严格保护无bypass，merge自动删source仍开，未改设置。快照不能锁定其他写入者。
新计划相对路径均实际存在，operation_enabled=false，18/2及两项阻塞准确列明。合并/删除仍
待最终门禁、实时盘点/恢复及父审；4旧PR只保留祖先关系，不冒称独立绿或已合并。
没有合并/删除/force push/发布/上游写入/收费或全局安全操作；只新增审查证据tag。
