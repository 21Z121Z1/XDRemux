# 输出规范

[English](logging.en.md) | 简体中文

规定 XDRemux 往终端写什么、写到哪里。用户视角的命令输出见 [CLI 参考](../cli.md)。

## 现状

CLI 只输出人类可读的文本，没有 JSON 事件流，也没有 `--quiet` / `--verbose` / `--format` 这类开关。

- 进度和结果写 **stdout**：`converted X -> Y`、`skipped X (output already up to date)`、`batch complete: N converted, N skipped, N failed -> <dir>`。
- 错误和诊断提示写 **stderr**，统一带 `error:` 前缀。
- 退出码只有 `0`（成功）和 `1`（任何错误）。

`validate-apple`、`validate-portrait` 和 `portrait-self-test` 例外：它们往 stdout 写 JSON，可以直接重定向到文件。

## 错误文案

`XDRemuxError` 有两种呈现：

- `description` 是完整形式，可以多行，用于单文件 `convert` —— 第一行说发生了什么，后面解释原因和下一步。
- `headline` 是单行形式，用于批量列表，保证一个文件一行。

写新的错误信息时：**先说用户能理解的事实，再给技术细节。** 不要把内部数据块名当成第一句话 —— `not a ProXDR photo` 是给人看的，`local.hdr.meta.data` 不是。

`XDRemuxCore` 的错误文案保持英文，因为它是对外的 Swift Package。中文本地化放在展示层：macOS App 里的 `AppStrings.failureReason`。

## 转换过程的诊断行

转换器会打印少量诊断，用于确认走了哪条路径，例如：

```text
[direct-gain] preserved compressed Base; encoded 15 Gain Map tiles once quality=0.90 tile=512x512
```

这类行是**被测试断言的**（`verify_swift_cli_sample.py --expect-direct-gain` 会数它出现的次数），改动措辞前先看有没有人在断言它。

## 排查失败

- `--debug-dir <目录>` 保留本次运行的中间产物。
- Apple 摄影风格在**失败时会保留**证据目录，并往 stderr 打印路径；成功才清理。
- 更细的调试开关见[开发文档](../development.md)的环境变量表。

## 代码要求

1. 关键路径不要只靠 `print` 吞掉异常。
2. 捕获异常时保留原始错误，不要压成一句话。
3. 批量场景一个文件一行，多行解释留给单文件路径。
