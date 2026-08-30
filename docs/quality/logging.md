# CLI 输出政策

[English](logging.en.md) | 简体中文

本文档定义当前命令输出契约。

命令参数见 [CLI 参考](../cli.md)。

## 面向人的输出

普通 Swift 和 Python 转换命令输出面向人的进度信息。

正常进度和结果写到 stdout。

错误写到 stderr。

除非 CLI 契约明确增加新的协议，否则不要再添加第二套通用日志协议。

## 机器可读命令

以下 Swift 命令把 JSON 写到 stdout：

- `validate-apple`
- `validate-portrait`
- `portrait-self-test`

不要在这些命令的 JSON stdout 中混入无关进度文本。

## 错误文本

错误信息首先说明用户可见的问题。

只有在有助于诊断时，再加入内部 metadata key 或容器术语。

batch UI 依赖逐行输出时，每个文件失败保持一行。

上层增加上下文时要保留原始错误。

不要把关键路径的 thrown error 替换成无结构 `print`，然后继续按成功处理。

## 退出状态

Swift 和 Python 使用不同的参数解析实现。

Swift：

- 正常成功使用 `0`；
- 运行时失败使用 `1`；
- parser-level usage exit 由 Swift Argument Parser 处理；
- `main.swift` 捕获的 Motion Photo 预路由失败使用 `1`。

Python：

- 正常成功使用 `0`；
- 命令运行失败使用 `1`；
- `argparse` parser-level usage error 使用 `2`。

除非两套实现确实共享相同行为，不要发布一张统一数字退出码表。

## 诊断输出

部分转换链路会打印用于标识实际实现路径的诊断行。

测试可能依赖准确诊断文本。修改诊断字符串前先搜索对应断言。

支持的转换链路可以使用 `--debug-dir` 保留诊断产物。

Apple feature 实现需要时，也可能在失败时保留证据或 helper 输出。

## Library 边界

`XDRemuxCore` 是 library。它应该暴露结构化结果、warning 和 error，不应该依赖终端格式。

终端格式和 localization 属于 CLI 或 App presentation layer。

当前技术写作和用户可见错误规则遵循[技术写作规范](../style-guide.md)。
