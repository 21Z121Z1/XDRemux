# 测试规范

用途: 规定仓库的最小验证链路和通过标准。

## 测试金字塔 (当前)
1. 结构检查: `make lint`
2. 回归检查: `make test`
3. 冒烟检查: `make smoke`
4. 全链路收口: `make verify`

## 命令与覆盖范围

| 命令 | 覆盖内容 | 阻断级别 |
| --- | --- | --- |
| `make lint` | Python 语法、目录边界、工作流文档完整性、可选 flake8 | Hard fail |
| `make test` | `evals/` 回归 + `oracle-dump/tests` 单元测试 | Hard fail |
| `make smoke` | 关键目录与核心文件存在性、基础入口可执行 | Hard fail |
| `make verify` | 上述全部 + `scripts/check_architecture.py` | Hard fail |

## 通过标准
仅当 `make verify` 返回 0，才允许视为本次改动完成。

## 新增测试要求
1. 修 bug 必须新增至少一个回归断言。
2. 新增脚本至少提供一个失败路径测试。
3. 无法自动化的验证，必须登记技术债并说明阻塞条件。

## 已知空白
- 设备依赖的动态探针链路未纳入默认 CI。
- fixture 覆盖仍有限，需要持续补样本。

关联文档: `docs/quality/evals.md`, `docs/runbooks/local-dev.md`。
