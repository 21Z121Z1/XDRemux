# ISO 有理数保真与替代 ICC 变换

[English](20261028-color-integrity.en.md) | 简体中文

本记录为 2026-10-28 的后续修复，替代[此前审计](20261028-iso-jpeg.md)中的最低 ICC 准入边界，不改写历史测量。测试和 CI 回执分别标识提交的源代码版本。原始 fixture 字节和清单均未修改。

## 复现的失败与修复

修改产品代码前，三项回归在原实现上失败：公共转换把 ISO headroom 1/1,000,000 改为零，并改写 gamma 6001/10 和偏移 1/64；公共转换发布了两个分数转为 f64 后相等、实际顺序非法的增益区间；替代 ICC 准入接受了只有 CICP、没有可用变换数据的头部。非法区间为 2147483646/2147483647 > 2147483645/2147483646，二者解码为 double 后却相同。

源 ISO 路径现在保留必需字段的有理数字节，并在转为浮点前精确比较分数。HEIF 封装和 C.4.7 应用空间选择不改写数值字段；受支持的可选尾部被忽略。转为同一 double 的合法不同 headroom 仍然有效。修改浮点观测值不能静默覆盖源分数。XMP／生成元数据采用按字段检查的有理数近似，投影误差不得超过 max(1e-12, abs(value) * 1e-9)，无法表示时明确失败。每份序列化 tmap 都重新执行数值校验。场景元数据使用同一写入器，不再使用饱和类型转换。兼容 XMP 从对数域数值生成，不再截断到六／八位小数。ISO headroom 文本保留最多 40 位小数并精确比较方向，包括转为 f64 后相同的数值。

替代 ICC 检查现在要求配置文件及变换标签、拒绝部分重叠，并以 f64 执行有界 CMM 变换，关闭 CICP 替代。实际为 sRGB、CICP 却声明 BT.2020/PQ 的完整配置文件在独立准入和公共转换中都失败。完整的合成 BT.2020 PQ、D65 P3 PQ、名义 HLG 和 sRGB 配置文件提供正向变换测试；它们是合成配置文件，不是真实相机 HDR 替代 ICC 或设备验收证据。带有匹配 CICP 的完整 sRGB ICC 在发布后原字节出现在 tmap 中，增益图 nclx 保持独立。

一致性网格、容差、尚不支持的限幅和部分未指定 LUT 情形，以及名义 HLG 参考，定义在[架构契约](../architecture.md#jpeg-增益图元数据选择)中。此策略可能拒绝超出验证模型的合法配置文件，不会改写 ICC 或猜测 HDR 色域。十份真实 JPEG 兼容样本均没有替代 ICC，因此新增检查不会排除其中任何一份。基线 ICC 和 XMP-only 配置文件选择保留现有行为。

## 增益精度测量

编码器回归使用 93×71 平滑 RGB 渐变、最近舍入的 8 位增益码值、产品质量 90 的编码器、补齐到 512 像素的图块和实际 HEVC 解码器。比较解码码值与原始连续渐变，包含量化和压缩。Linux x86_64、libheif 1.17.6、x265 3.5 的结果如下：

| 增益压缩 | 最大码值误差 | p99 码值误差 | 按 24 EV 换算的最大 EV 误差 | 按 24 EV 换算的 p99 EV 误差 |
| --- | ---: | ---: | ---: | ---: |
| 4:4:4 | 2.796296 | 1.537037 | 0.263181 | 0.144662 |
| 4:2:0 | 4.250000 | 3.130435 | 0.400000 | 0.294629 |

这是单个平滑合成图案上的观测。最大值／p99 的回归阈值分别为 4／3 和 6／4 码值，不是通用质量保证。gamma 为 1 时，EV 误差随增益跨度缩放；极端的 24 EV 因而会放大小码值误差。理论上仅量化的半步误差为 0.047059 EV，即约 3.32% 的增益倍率误差。上表实际压缩加量化的最大值对应更大的倍率误差；这些值都不是整幅图像的感知色差测量。本补丁不修改编码器代码点、子采样、位深或质量设置。

## 证据与限制

使用已配置的 Rust／libheif HEVC 工具链复现：

```bash
cargo test --locked -p xdremux-metadata --lib
cargo test --locked -p xdremux-runtime --test iso_jpeg_metadata
cargo test --locked -p xdremux-runtime --lib live_photo::tests
cargo test --locked -p xdremux-codec gain_ramp_quantization -- --nocapture
cargo test --locked -p xdremux-runtime --test live_photo_jpeg_fixtures \
  real_jpeg_motion_photo_vendor_matrix -- --ignored --nocapture
cargo clippy --locked --workspace --all-targets -- --deny warnings
cargo test --locked --workspace --all-targets
(cd fixtures && sha256sum -c SHA256SUMS)
```

公共回归检查实际发布的 tmap 字节、独立的 base／gain／tmap 色彩属性、源文件保留，以及失败时不发布新的配对文件。绑定 HEAD 的完成回执和原生 CI 证据必须来自当前提交，不能沿用此前 PR 结果。

公开样本仍缺真实相机 HDR 替代 ICC 正例；本次没有新的实体设备 Photos／Android 图库往返，也没有覆盖 RGBA32F 到最终原生显示的全阶段感知误差测量。ICC 网格检查不能证明每个 LUT 值或渲染意图。当前样本按回归覆盖选择，不是有代表性的随机抽样。在独立且有代表性的伯努利模型下，299 个零失败样本可给出至少 99% 成功率的单侧 95% 置信下界；十份定向样本和 CI 检查数不满足这些假设。不支持输入的拒绝率必须与成功且颜色正确的转换分别统计。

依据：[ISO 21496-1:2025](https://www.iso.org/standard/86775.html) §5.2.5.3、§5.2.7 和附录 C；[ICC.1:2022](https://www.color.org/specifications/ICC.1-2022-05.pdf) §9.2.17 及变换／标签要求；[H.273](https://www.itu.int/rec/T-REC-H.273-202407-I)；[moxcms](https://github.com/awxkee/moxcms)（0.9.1）。一致性阈值、近似预算和保守拒绝均为 XDRemux 产品策略，不是所引 ISO 或 ICC 的强制原文。
