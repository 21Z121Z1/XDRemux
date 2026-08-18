#!/usr/bin/env python3
from pathlib import Path

root = Path(__file__).resolve().parents[1]
path = root / "Sources/XDRemuxAppleFeatures/PhotographicStyles/ConstrainedPolynomialStyleDataProducer.swift"
text = path.read_text(encoding="utf-8")
old = '''            let inverseStep = Float(1.0 / step)
            guard inverseStep.isFinite else {
                throw CLIError.invalidContainer("non-finite constrained key-1 Jacobian step")
            }
'''
new = '''            let floatStep = Float(step)
            guard floatStep.isFinite, floatStep != 0 else {
                throw CLIError.invalidContainer("non-finite constrained key-1 Jacobian step")
            }
'''
if old in text:
    text = text.replace(old, new, 1)
elif new not in text:
    raise RuntimeError("sampled Jacobian step block not found")
old_expr = "                    let derivative = (rendered[base + channel] - current[base + channel]) * inverseStep\n"
new_expr = "                    let derivative = (rendered[base + channel] - current[base + channel]) / floatStep\n"
if old_expr in text:
    text = text.replace(old_expr, new_expr, 1)
elif new_expr not in text:
    raise RuntimeError("sampled Jacobian derivative expression not found")
path.write_text(text, encoding="utf-8")
print("preserved former Float division semantics")
