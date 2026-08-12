# 测试规范

[English](testing.en.md) | 简体中文

规定这个仓库里一次改动至少要过哪些检查。怎么跑测试见 [Tests/README.md](../../Tests/README.md)，验收流程见[验证说明](../validation/README.md)。

## 四层检查

| 层 | 跑什么 | 覆盖 |
| --- | --- | --- |
| 单元与契约 | `swift test` | 转换模型、HEIF 边界、错误文案、CLI 参数解析、Apple 功能契约 |
| 策略 | `python3 -m unittest discover -s Tests` | 架构边界、文档一致性、分类行为、Python 尾部策略 |
| 真实样本 | `Tests/validation/` 下的 harness | 拿真实 OPPO 照片跑完整转换并断言结果 |
| 验收 | `scripts/agent_completion_gate.py` | 把上面选出的检查绑定到具体提交，产出 receipt |

策略测试是纯静态的 —— 它们读源码和文档做断言，不跑转换。**不要把静态检查当成功能证据。**

## 通过标准

`swift test` 和 Python 套件必须全绿。涉及转换行为的改动，还必须有至少一条真实样本证据 —— 类型检查通过不算。

## 新增测试要求

1. 修 bug 必须补一条回归断言，且这条断言在修复前应该是失败的。
2. 改了用户能看见的文案（错误信息、help、命令输出），要有断言把它钉住。
3. 无法自动化的验证，在提交说明里写清楚验证不了的是什么、为什么。

## 已知空白

- 严格的 Samsung / Xiaomi / OPPO / vivo Motion Photo 真实样本已经版本化在 `fixtures/`，对应 CI 不再依赖私有压缩包或仓库 Secret。其他旧的 ProXDR / Apple 功能回归样本仍是独立的一套，可能继续保持私有。
- 没有真机 Photos 的自动化验收。Apple 摄影风格和人像的"导入、编辑、保存、重开"这一轮仍然只能手动做。
- OPPO 相册的实际显示行为无法在 CI 里验证。
