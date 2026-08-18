#!/usr/bin/env python3
from pathlib import Path

root = Path(__file__).resolve().parents[1]
style_path = root / "Sources" / "XDRemuxAppleFeatures" / "PhotographicStyles" / "ConstrainedPolynomialStyleDataProducer.swift"
text = style_path.read_text(encoding="utf-8")
old = '''            try fileManager.copyItem(at: heicURL, to: outputURL)
            let handle = try FileHandle(forWritingTo: outputURL)
'''
new = '''            try fileManager.copyItem(at: heicURL, to: outputURL)
            // copyItem preserves POSIX mode bits. Inputs from read-only media may therefore clone
            // as 0444 even though the old Data.write candidate path could still create a writable
            // scratch output. Restore owner-write permission on the private candidate only.
            let copiedAttributes = try fileManager.attributesOfItem(atPath: outputURL.path)
            if let permissions = (copiedAttributes[.posixPermissions] as? NSNumber)?.intValue,
               permissions & 0o200 == 0 {
                try fileManager.setAttributes(
                    [.posixPermissions: permissions | 0o200],
                    ofItemAtPath: outputURL.path
                )
            }
            let handle = try FileHandle(forWritingTo: outputURL)
'''
if old in text:
    text = text.replace(old, new, 1)
elif new not in text:
    raise RuntimeError("candidate clone write-open block not found")
style_path.write_text(text, encoding="utf-8")

test_path = root / "Tests" / "XDRemuxAppleFeaturesTests" / "PerformanceDesignTests.swift"
test = test_path.read_text(encoding="utf-8")
old = '''        try base.write(to: input)

        let styleData = try Producer.styleData(parameters: [0.01, -0.01, 0.005, 0, 0, 0])
'''
new = '''        try base.write(to: input)
        try FileManager.default.setAttributes(
            [.posixPermissions: 0o444],
            ofItemAtPath: input.path
        )

        let styleData = try Producer.styleData(parameters: [0.01, -0.01, 0.005, 0, 0, 0])
'''
if old in test:
    test = test.replace(old, new, 1)
elif new not in test:
    raise RuntimeError("candidate patch fixture write block not found")
test_path.write_text(test, encoding="utf-8")

print("preserved candidate patch behavior for read-only source files")
