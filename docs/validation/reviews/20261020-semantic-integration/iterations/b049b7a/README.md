# 按目标文件系统构造回归 fixture

[English](README.en.md) | 简体中文

当前源码为 `b049b7a4bf41934bacf93514e5899567a54000ca`，tree `18f2e6a808ca4cbae66f8ac7e64e1824cce985f9`。[持久备份 manifest](recovery-manifest.json) 和[远端恢复/fsck 日志](remote-restore.log) 覆盖准确 HEAD。相对 [0a2c7c9](../0a2c7c9/README.md) 生产源码未变，只修改下面两处测试 fixture。原始 PR/source 和 main 未推进，没有合并或删除分支。

| 0a2c7c9 原生失败 | 根因与修正 | 保留的断言 |
| --- | --- | --- |
| [Intel 打包 job](https://github.com/21Z121Z1/XDRemux/actions/runs/39491609690/job/118865929006)、[原始错误摘录](package-path-native-red.log) | `bundled_reference` 按契约返回 resolve 后的物理路径；测试却期待 `/var` 字面路径，macOS 将它解析为 `/private/var`。现与 `library.resolve()` 精确比较。 | 缺失/外部/目录逃逸依赖仍拒绝；[五项边界测试本地通过](package-path-green.log)。首错发生在原生源码构建、解包转换前；upload 无文件是前置失败连带结果。 |
| [macOS portability job](https://github.com/21Z121Z1/XDRemux/actions/runs/39491609690/job/118865929335)、[原始错误摘录](i18n-filesystem-native-red.log) | APFS 在创建非 UTF8 文件名时返回 OS92/EILSEQ，尚未调用 CLI。Linux 支持该文件名的 runner 文件系统保留完整非法字节路径负例与数据保持测试；通用真实文件回归增强为 Unicode、空格、标点、引号和类似翻译目标的单词。 | 两种语言均实际读取文件、保留完整路径；JSON 成功且字节一致，input 字符串精确不变，源字节保持。Linux 继续检查非 UTF8 human 读取、既有 JSON 失败/状态/负载不变及源字节。[Linux 八项进程回归通过](i18n-filesystem-green.log)。macOS/Windows 有七项适用进程测试，不声称 APFS 接受非法 UTF8 文件名。 |

[准确提交 receipt](portable-receipt.json)、[计划](portable-plan.json)、[run/verify 日志](portable-gate.log) 在干净未变的 b049b7a 上八项全通过，包含 46+2 HEIF、28+8 CLI、全部 232 Python 和[真实 Samsung 双成员分别读回](real-cli-proof/manifest.json)。receipt SHA256 为 `bc45c8b72b82970a7563e00971e7c7b2c680d1a0969e487e0196fb06f6a10838`。中间的测试修正 c484046 也已持久备份、恢复：[manifest](intermediate-c484046-manifest.json)、[日志](intermediate-c484046-restore.log)；它未单独推进为 PR HEAD，immutable 备份仍保留。

新源码 CI：[产品/App](https://github.com/21Z121Z1/XDRemux/actions/runs/39493275887)、[exact-head](https://github.com/21Z121Z1/XDRemux/actions/runs/39493275810)、[portability 及双架构打包](https://github.com/21Z121Z1/XDRemux/actions/runs/39493276049)、[System HEIF/research](https://github.com/21Z121Z1/XDRemux/actions/runs/39493275803)、[性能](https://github.com/21Z121Z1/XDRemux/actions/runs/39493276015)、[真实 ProXDR](https://github.com/21Z121Z1/XDRemux/actions/runs/39493275833)。准备本附件时仍排队/运行，不新增原生通过声明。ba2 的产品/App 通过和 System/性能失败保留实际身份；必要源码推进取代旧失败 fixture，未手动重复 dispatch workflow，未放宽基线。最终合并/删除仍等待最终源码原生检查及父审核。
