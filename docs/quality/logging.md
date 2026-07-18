# 日志规范

用途: 让 agent 能通过日志直接定位问题，并在 CI/本地稳定消费。

## 统一格式
Python 侧统一输出 JSON 日志。建议字段:
- `timestamp` (UTC ISO8601)
- `level` (`INFO`/`WARNING`/`ERROR`)
- `logger`
- `message`
- `trace_id` (默认可由环境变量 `HARNESS_TRACE_ID` 注入)
- `error_code` (错误时必填，未知可用 `UNSPECIFIED`)
- `context` (对象/命令/文件等上下文)

## 级别规范
- `INFO`: 正常阶段事件
- `WARNING`: 可降级但应关注
- `ERROR`: 阻断流程并返回非零退出

## 代码要求
1. 禁止关键路径只用 `print`。
2. 捕获异常时保留 traceback。
3. 新增日志字段应保持向后兼容。

## 查看方式
本地:
```bash
make verify 2>&1 | tee /tmp/proxdr_verify.log
```

过滤 ERROR:
```bash
cat /tmp/proxdr_verify.log | grep '"level": "ERROR"'
```

设备侧 (手动):
```bash
adb logcat | grep -E "ProXDR|Frida|ColorGallery"
```

## 相关实现
- `harness/logger.py`
- `scripts/smoke_test.py`
