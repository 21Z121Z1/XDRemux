# 以真实服务状态取代冗余 help 前置

候选86e810dfb52cbe2516d816bc0ac11b94de84d0e7，树0997860e6554abedc9999d5ea63ca7936df2e144。
仅删除两项不必要help调用并补外部调用策略回归；产品及workflow字节不变。完整证据见
[英文说明](README.en.md)。

392新的首错是作者引入的mandatory simctl help install15秒超时，发生在真实协调器之前；
inventory、安装、授权/基线/三路径UI均未运行。源/包hash不变、清理成功，不是Apple媒体拒绝。
现删除install/listapps两个help前置，不加时或重跑旧head；真实有界inventory回答才是就绪
观察。一次120秒安装、注册/app/data/bundle/二进制身份、迟到结果读回和失败日志保持，未知
安装不重复。策略回归旧源真实失败，修后46重点通过；它不是原生安装实测。准确head四项
门禁通过，含308Python及真实CLI pair/PVT往返/不覆盖，未重跑未变完整Rust/clippy。

已启动392性能/App/Tone自然完成后才推进分支，原始日志完整保留。release位等价及原绝对
性能通过9.196/9.472秒，但同机main7.920269/候选10.543739秒仍慢33.1235%，CPU7.89→9.58、
sys0.53→1.04；比af223的17.8593%更大，不能以绝对绿或宿主噪声掩盖。未重跑/改阈值，未称
ARM64/端到端提速或完整回归归因。App/Tone准确执行结果另列，不回填为86验收。

完整包/保存回调/单新增Live资产/原资源/ID/封面/hash/清理断言均未削弱，无解包pair替代。
本head真实inventory/安装/Photos共享与读回仍待原生CI，真机/墙纸未验。37持久ref逐一
恢复commit/tree、非浅库完整严格fsck0，覆盖9head，其余8未变、main仍89d806c8。合并/清理
计划禁用，待最终门禁及父审；未合并/删除/force push/发布/付费/写上游/改保护或全局安全。
