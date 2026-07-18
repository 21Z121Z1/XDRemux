# Eval 与回归机制

用途: 定义 agent 可消费的最小评估系统, 避免“改完只靠人眼看一遍”。

## 评估对象
1. Repo-level 回归: `evals/fixtures/repo_regression_cases.json`
2. 工作流结构回归: `scripts/check_workflows.py`
3. 架构边界回归: `scripts/check_architecture.py`
4. Harness 状态回归: ADB `device/offline/unauthorized` 输出解析
5. Repo policy 回归: `evals/repo_policy_eval.py`, 保护 canonical evidence 可见性与 eval 自包含性
6. Swift XDRemux 回归: `evals/swift_xdremux_eval.py`, 覆盖 typecheck、UHDR smoke 与 batch failure exit
7. ImageIO-native tmap 回归: `evals/imageio_tmap_eval.py`, 区分 142B 兼容载荷与 strict ISO tmap
8. 单元回归: `oracle-dump/tests`

说明: 在 Codex 的 `CODEX_SANDBOX=seatbelt` 嵌套环境里, Python 子进程再启动 Swift/ImageIO 可能无法完成 UHDR runtime smoke。该场景由 `evals/swift_xdremux_eval.py` 识别为环境 skip, 需要用同一条 `swift ... XDRemux.swift convert` 命令从 shell 直接验证 ImageIO 行为。

## 执行入口
- `make test`: 运行 repo-level + oracle-dump pytest
- `make verify`: lint + test + smoke + 架构边界

## 通过标准
- `make verify` 返回码为 0
- 无新的架构越界
- 无 workflow 结构缺失

## 已知空白
1. 设备依赖流程 (ADB/Frida) 仍为手动 gate
2. 真实样本 fixture 密度不足, 目前以仓库结构回归为主
3. API36 framework/native 绝对等价仍需外部证据
4. ImageIO-native 142B `tmap` 是兼容性证据, 不是 strict ISO 145B payload 通过证据
5. Homebrew Python 3.14 在当前机器上可能卡在 Pillow/bz2 导入；`evals/test_eval.py` 已将 image-stack 相关回归隔离到有超时的 worker, 本机完整回归建议使用健康 Python runtime 或 `.venv`

## TODO 路线
1. 扩充 `evals/fixtures/` golden 样本
2. 将关键 failure mode 映射为独立 case ID
3. 对动态探针链路补充可复现 mock 合同测试

关联文档: `docs/quality/testing.md`, `docs/debt/tech-debt-tracker.md`。
