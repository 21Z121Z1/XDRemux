#!/usr/bin/env python3
from pathlib import Path

root = Path(__file__).resolve().parents[1]
style_path = root / "Sources" / "XDRemuxAppleFeatures" / "PhotographicStyles" / "ConstrainedPolynomialStyleDataProducer.swift"
text = style_path.read_text(encoding="utf-8")

old = '''                    "metricsAgainstDisabled": Self.metrics(rendered, target).dictionary,
'''
new = '''                    "metricsAgainstDisabled": Self.sampledMetrics(
                        rendered,
                        target,
                        pixelStride: jacobian.pixelStride
                    ).dictionary,
                    "metricsAgainstDisabledSampling": "solver-sample-grid",
'''
if old in text:
    text = text.replace(old, new, 1)
elif new not in text:
    raise RuntimeError("Jacobian diagnostic metrics block not found")

anchor = '''    private static func metrics(_ left: Raster, _ right: Raster) -> Metrics {
        precondition(left.width == right.width && left.height == right.height)
        var squared = 0.0
        var absolute = 0.0
        var maximum = 0.0
        for index in left.rgb.indices {
            let difference = Double(left.rgb[index] - right.rgb[index])
            squared += difference * difference
            absolute += abs(difference)
            maximum = max(maximum, abs(difference))
        }
        let count = Double(max(1, left.rgb.count))
        return Metrics(
            rmse8: sqrt(squared / count),
            mae8: absolute / count,
            maximumAbsolute8: maximum
        )
    }

'''
addition = anchor + '''    private static func sampledMetrics(
        _ left: Raster,
        _ right: Raster,
        pixelStride: Int
    ) -> Metrics {
        precondition(left.width == right.width && left.height == right.height)
        precondition(left.rgb.count == right.rgb.count && left.rgb.count.isMultiple(of: 3))
        precondition(pixelStride > 0)
        var squared = 0.0
        var absolute = 0.0
        var maximum = 0.0
        var sampleCount = 0
        let pixelCount = left.rgb.count / 3
        for pixel in Swift.stride(from: 0, to: pixelCount, by: pixelStride) {
            let base = pixel * 3
            for channel in 0..<3 {
                let difference = Double(left.rgb[base + channel] - right.rgb[base + channel])
                squared += difference * difference
                absolute += abs(difference)
                maximum = max(maximum, abs(difference))
                sampleCount += 1
            }
        }
        let count = Double(max(1, sampleCount))
        return Metrics(
            rmse8: sqrt(squared / count),
            mae8: absolute / count,
            maximumAbsolute8: maximum
        )
    }

'''
if "private static func sampledMetrics(" not in text:
    if anchor not in text:
        raise RuntimeError("metrics helper insertion anchor not found")
    text = text.replace(anchor, addition, 1)

style_path.write_text(text, encoding="utf-8")

arch_path = root / "Tests" / "test_performance_design.py"
arch = arch_path.read_text(encoding="utf-8")
needle = '        self.assertNotIn("let lineSearchRasters = try Self.render", source)\n'
extra = needle + '        self.assertNotIn("Self.metrics(rendered, target).dictionary", source)\n'
if 'self.assertNotIn("Self.metrics(rendered, target).dictionary"' not in arch:
    if needle not in arch:
        raise RuntimeError("performance architecture insertion point missing")
    arch_path.write_text(arch.replace(needle, extra, 1), encoding="utf-8")

print("moved Jacobian-only diagnostics onto the solver sample grid")
