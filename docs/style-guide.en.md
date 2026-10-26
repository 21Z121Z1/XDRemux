# Technical writing guide

English | [简体中文](style-guide.md)

This guide defines how XDRemux documents product behavior, research results, and maintenance procedures. English is the canonical source for current bilingual documentation.

## References and scope

Use [ASD-STE100 Issue 9](https://www.asd-ste100.org/assets/files/ASD-STE100_ISSUE9.pdf) for English technical writing. The issue date is 2025-01-15. Use the [official download page](https://www.asd-ste100.org/STE_downloads.html) if the PDF address changes.

For Chinese, apply Fenng's [Tech-Doc-Style-Chinese Skill](https://github.com/Fenng/Tech-Doc-Style-Chinese/blob/b119d01c9bb7cd132b3c62fafdff34227c4410d1/SKILL.md) and its [controlled technical Chinese rules](https://github.com/Fenng/Tech-Doc-Style-Chinese/blob/b119d01c9bb7cd132b3c62fafdff34227c4410d1/references/controlled-technical-chinese.md). These rules adapt the method to Chinese. They do not transfer the English dictionary or word-count rules to Chinese sentences.

XDRemux does not claim formal ASD-STE100 compliance. A style check cannot certify terminology, technical truth, or complete compliance with the standard.

The repository's [pstack workflow](../.agents/skills/pstack-workflow/SKILL.md) coordinates source review, scoped changes, verification, and independent review. These project skills retain their recorded upstream provenance. They do not require Cursor or grant permission for external actions.

## Preserve meaning before changing style

Apply these priorities in order:

1. Preserve facts, requirements, legal meaning, warnings, and evidence limits.
2. Follow the requested scope and the repository's existing contracts.
3. Preserve identifiers, values, units, commands, and source quotations.
4. Improve the order, sentences, and terminology.
5. Correct punctuation, spacing, and Markdown layout.

Check each behavioral statement against its code owner, tests, or recorded evidence. An old document or a proposed PR does not establish current behavior. Keep a hypothesis, a historical observation, and a supported product feature distinct.

## Choose the document's purpose

| Purpose | Required content |
| --- | --- |
| Entry page | Supported tasks, prerequisites, material limits, and links to the next step. |
| Procedure | Prerequisites, risk before the affected step, ordered actions, expected results, and failure handling. |
| Reference | Exact commands, types, values, defaults, units, conflicts, outputs, and errors. |
| Explanation | One bounded subject, its implementation owner, and the reason for relevant design choices. |
| Research record | Inputs, method, measured results, uncertainty, and the conditions needed to establish a stronger claim. |

Separate reference material from a long procedure with headings or a link. Do not repeat the complete command contract in every document.

## Write English instructions and descriptions

Use the applicable rules from ASD-STE100 Part 1:

- Use the same technical noun for the same object. Use approved words with their specified meaning and part of speech.
- Keep necessary technical nouns and technical verbs from the software and imaging domain. Do not disguise jargon as a technical term.
- Use active voice and direct verbs. In procedures, write commands.
- Keep one instruction in each sentence, except for actions that must occur together.
- Put a condition before the action that depends on it. Put a data-loss caution before an overwrite command.
- Limit a procedure sentence to 20 words and a descriptive sentence to 25 words, using the standard's counting rules.
- Keep one topic in each paragraph. Use no more than six sentences in a descriptive paragraph.
- Keep articles and other words needed for a complete sentence. Rewrite a dense sentence instead of deleting its conditions.
- Avoid long noun groups and ambiguous pronouns. Name the object again when necessary.
- Use American English spelling. Avoid semicolons, idioms, promotional claims, and unsupported claims about quality or speed.

Use `must` for a requirement, `can` for a capability, and `may` for a possibility. Do not turn a qualified observation into a requirement or guarantee.

## Write Chinese explanations

Write natural Chinese after the English technical content is settled. Preserve the same requirements, defaults, exceptions, and uncertainty.

- Translate explanatory prose. Keep API names, framework names, command options, metadata keys, paths, and other identifiers exact.
- Use one preferred Chinese term for each concept. Explain a necessary English term at first use.
- Give each sentence a clear subject or action. Split unrelated actions, but keep a condition with the action it controls.
- Use Chinese punctuation and `「」` for authored quotations. Preserve fixed quotations and code as written.
- Put one half-width space between Chinese and Latin text, Arabic numerals, or inline code. Do not add spaces around Chinese punctuation.
- Keep each prose paragraph on one source line. Preserve code blocks, tables, explicit line breaks, and quoted records.
- Use sentence headings that name the task or subject. Remove vague wording such as 「闭环」 when the actual checks can be named.

Do not translate `Motion Photo` and `Live Photo` into one interchangeable format name. Both can be described as motion-enabled photos, but their containers and validation contracts differ.

## Project terminology

| English term | Preferred Chinese explanation | Meaning in XDRemux |
| --- | --- | --- |
| ProXDR | ProXDR | The supported vendor HDR source format. |
| Gain Map | 增益图（Gain Map） | Auxiliary pixels and metadata used to reconstruct HDR from a base image. |
| Motion Photo | Motion Photo | An Android asset with a still image and motion-video data. |
| Live Photo | Live Photo | An Apple still-image and MOV pair with matching identity and timing. |
| PVT package | PVT 目录包 | A directory with the validated Live Photo members and its metadata plist. |
| cover-frame PTS | 封面帧显示时间戳（PTS） | The presentation timestamp of the source cover frame. |
| `still-image-time` | `still-image-time` | The Apple timed-metadata value that marks the still frame. |
| compressed-sample passthrough | 压缩样本原样复制 | Copy encoded media samples without decoding and encoding them again. |
| Photographic Styles | 摄影风格（Photographic Styles） | Apple style-editing resources generated by the feature pipeline. |
| Texture Styles | 纹理风格（Texture Styles） | The separate Apple texture-editing resources. |
| Apple Portrait | Apple 人像 | Portrait resources generated from compatible source depth data. |
| fixture | 测试样本 | A versioned input with a known identity for a test. |
| checkpoint | 检查点 | Persisted progress and source identity used to decide whether output can be reused. |
| receipt | 验证回执或执行回执 | A structured record of the operation and its observed results. |
| historical record | 历史记录 | A dated result for a particular source state, method, and environment. |

Translate prose about ownership as responsibility, not as a new product component. Preserve crate names when the code owner matters.

## Keep document classes distinct

Current user guides, developer guides, quality policies, model cards, and maintenance instructions describe the maintained code. Update their English and Chinese versions together. Agent skills can remain English when they have no Chinese counterpart.

Historical records retain their measurements, original commands, paths, and conclusions. Correct a current summary when the implementation changes. Add a dated scope note or an erratum without replacing the evidence body. Release notes describe the named release, not the current branch.

Generated output and machine-readable text retain their exact spelling. Preserve JSON keys, metadata fields, filenames, fixture hashes, copyright notices, and license text. Code examples can change only when source evidence shows that the documented command needs correction.

## Check a documentation change

1. Confirm the changed files and their document classes.
2. Compare each changed claim with the relevant code, test, or evidence record.
3. Compare both languages for the same scope, defaults, warnings, and limits.
4. Check local links, anchors, command names, and options.
5. Run the existing documentation and repository-layout checks.
6. Review the final diff for lost conditions and altered machine-readable text.

The Chinese Skill's `lint_copy_rules.py` provides additional wording and formatting suggestions. Review its warnings in context. A style warning does not justify changing a code identifier or a historical record.
