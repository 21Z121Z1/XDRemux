#!/usr/bin/env python3
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
STYLE = ROOT / "Sources/XDRemuxAppleFeatures/PhotographicStyles/ConstrainedPolynomialStyleDataProducer.swift"
text = STYLE.read_text(encoding="utf-8")


def sub_once(pattern: str, replacement: str, marker: str) -> None:
    global text
    if marker in text:
        return
    text, count = re.subn(pattern, replacement, text, count=1, flags=re.S)
    if count != 1:
        raise RuntimeError(f"phase2 replacement failed ({count}): {pattern[:120]}")


# Split request execution from raster decoding. The helper writes deterministic PNG outputs;
# callers that consume one candidate at a time no longer need to retain every decoded raster.
sub_once(
    r'''    private static func render\(\n        executable: URL,\n        requests: \[RenderRequest\]\n    \) throws -> \[Raster\] \{.*?\n    private static func decodeRGB8''',
    '''    private static func executeRenderRequests(
        executable: URL,
        requests: [RenderRequest]
    ) throws {
        guard !requests.isEmpty else { return }
        for request in requests {
            try FileManager.default.createDirectory(
                at: request.outputDirectory,
                withIntermediateDirectories: true
            )
        }
        let workerCount = min(renderConcurrency, requests.count)
        guard workerCount > 1 else {
            try executeRenderChunk(executable: executable, requests: requests)
            return
        }

        var chunks: [[RenderRequest]] = []
        chunks.reserveCapacity(workerCount)
        let baseSize = requests.count / workerCount
        let remainder = requests.count % workerCount
        var start = 0
        for index in 0..<workerCount {
            let size = baseSize + (index < remainder ? 1 : 0)
            chunks.append(Array(requests[start..<(start + size)]))
            start += size
        }

        let lock = NSLock()
        var firstError: (index: Int, error: Error)?
        DispatchQueue.concurrentPerform(iterations: chunks.count) { index in
            do {
                try executeRenderChunk(executable: executable, requests: chunks[index])
            } catch {
                lock.lock()
                if firstError == nil || index < firstError!.index {
                    firstError = (index, error)
                }
                lock.unlock()
            }
        }
        if let firstError {
            throw firstError.error
        }
    }

    private static func render(
        executable: URL,
        requests: [RenderRequest]
    ) throws -> [Raster] {
        try executeRenderRequests(executable: executable, requests: requests)
        return try requests.map { try decodeRGB8($0.pngURL) }
    }

    private static func executeRenderChunk(
        executable: URL,
        requests: [RenderRequest]
    ) throws {
        let planURL = FileManager.default.temporaryDirectory.appendingPathComponent(
            "xdremux-neutrino-style-render-batch-\\(UUID().uuidString).json"
        )
        defer { try? FileManager.default.removeItem(at: planURL) }
        let plan: [String: Any] = [
            "schema": "xdremux-neutrino-style-render-batch-v1",
            "requests": requests.map(\\.dictionary),
        ]
        let planData = try JSONSerialization.data(
            withJSONObject: plan,
            options: [.sortedKeys, .withoutEscapingSlashes]
        )
        try planData.write(to: planURL, options: .atomic)
        let process = try AppleNativeToolchain.run(
            executable,
            arguments: ["--render-style-batch", planURL.path],
            timeout: 180
        )
        let result = try? JSONSerialization.jsonObject(with: process.stdout) as? [String: Any]
        guard !process.timedOut,
              process.status == 0,
              result?["passed"] as? Bool == true else {
            let stderr = String(data: process.stderr, encoding: .utf8) ?? ""
            let stdout = String(data: process.stdout, encoding: .utf8) ?? ""
            throw CLIError.invalidContainer(
                "complete-Neutrino key-1 calibration render batch failed: "
                    + (process.timedOut ? "renderer exceeded 180 seconds; " : "")
                    + [stderr, stdout]
                        .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
                        .filter { !$0.isEmpty }
                        .joined(separator: " ")
            )
        }
    }

    private static func decodeRGB8''',
    "private static func executeRenderRequests("
)

# Initialization line-search candidates are executed concurrently but decoded/consumed one work item at a time.
sub_once(
    r'''        let initializationRasters = try Self\.render\(\n            executable: executable,\n            requests: initializationWork\.flatMap\(\\\.requests\)\n        \)\n(.*?)        var initializationCursor = 0\n        for work in initializationWork \{\n            guard initializationCursor \+ work\.requests\.count <= initializationRasters\.count else \{.*?\n            \}\n            let raster = initializationRasters\[initializationCursor\]\n            let candidateResponse: ResponseObjectiveState\? = responseActive\n                \? responseState\(\n                    mid: initializationRasters\[initializationCursor \+ 1\],\n                    plus: initializationRasters\[initializationCursor \+ 2\]\n                \)\n                : nil\n            initializationCursor \+= work\.requests\.count\n''',
    '''        try Self.executeRenderRequests(
            executable: executable,
            requests: initializationWork.flatMap(\\.requests)
        )
\\1        for work in initializationWork {
            let raster = try Self.decodeRGB8(work.requests[0].pngURL)
            let candidateResponse: ResponseObjectiveState?
            if responseActive {
                candidateResponse = responseState(
                    mid: try Self.decodeRGB8(work.requests[1].pngURL),
                    plus: try Self.decodeRGB8(work.requests[2].pngURL)
                )
            } else {
                candidateResponse = nil
            }
''',
    "requests: initializationWork.flatMap(\\.requests)\n        )\n"
)

# Jacobian renders are the largest batch (12 parameter directions, up to three renders each).
# Execute all GPU work first, then decode one direction at a time into the sampled flat Jacobian.
sub_once(
    r'''            let derivativeRasters = try Self\.render\(\n                executable: executable,\n                requests: derivativeWork\.flatMap\(\\\.requests\)\n            \)''',
    '''            try Self.executeRenderRequests(
                executable: executable,
                requests: derivativeWork.flatMap(\\.requests)
            )''',
    "requests: derivativeWork.flatMap(\\.requests)\n            )"
)
sub_once(
    r'''            var derivativeCursor = 0\n            for work in derivativeWork \{\n                guard derivativeCursor \+ work\.requests\.count <= derivativeRasters\.count else \{.*?\n                \}\n                let rendered = derivativeRasters\[derivativeCursor\]\n                let perturbedResponse: ResponseObjectiveState\? = responseActive\n                    \? responseState\(\n                        mid: derivativeRasters\[derivativeCursor \+ 1\],\n                        plus: derivativeRasters\[derivativeCursor \+ 2\]\n                    \)\n                    : nil\n                derivativeCursor \+= work\.requests\.count\n''',
    '''            for work in derivativeWork {
                let rendered = try Self.decodeRGB8(work.requests[0].pngURL)
                let perturbedResponse: ResponseObjectiveState?
                if responseActive {
                    perturbedResponse = responseState(
                        mid: try Self.decodeRGB8(work.requests[1].pngURL),
                        plus: try Self.decodeRGB8(work.requests[2].pngURL)
                    )
                } else {
                    perturbedResponse = nil
                }
''',
    "let rendered = try Self.decodeRGB8(work.requests[0].pngURL)"
)

# Per-iteration line search follows the same bounded-decoding shape.
sub_once(
    r'''            let lineSearchRasters = try Self\.render\(\n                executable: executable,\n                requests: lineSearchWork\.flatMap\(\\\.requests\)\n            \)\n            var proposedRaster: Raster\?\n            var lineSearchCursor = 0\n            for work in lineSearchWork \{\n                guard lineSearchCursor \+ work\.requests\.count <= lineSearchRasters\.count else \{.*?\n                \}\n                let raster = lineSearchRasters\[lineSearchCursor\]\n                let candidateResponse: ResponseObjectiveState\? = responseActive\n                    \? responseState\(\n                        mid: lineSearchRasters\[lineSearchCursor \+ 1\],\n                        plus: lineSearchRasters\[lineSearchCursor \+ 2\]\n                    \)\n                    : nil\n                lineSearchCursor \+= work\.requests\.count\n''',
    '''            try Self.executeRenderRequests(
                executable: executable,
                requests: lineSearchWork.flatMap(\\.requests)
            )
            var proposedRaster: Raster?
            for work in lineSearchWork {
                let raster = try Self.decodeRGB8(work.requests[0].pngURL)
                let candidateResponse: ResponseObjectiveState?
                if responseActive {
                    candidateResponse = responseState(
                        mid: try Self.decodeRGB8(work.requests[1].pngURL),
                        plus: try Self.decodeRGB8(work.requests[2].pngURL)
                    )
                } else {
                    candidateResponse = nil
                }
''',
    "requests: lineSearchWork.flatMap(\\.requests)\n            )\n            var proposedRaster"
)

# Final response-envelope validation keeps only file URLs in the cache and decodes the six rasters
# needed for one pair. This bounds retained RGB storage instead of keeping every response raster.
if "var renderCache: [String: Raster]" in text:
    text = text.replace("        var renderCache: [String: Raster] = [:]\n", "        var renderURLs: [String: URL] = [:]\n", 1)
    old = '''        let responseRasters = try render(
            executable: executable,
            requests: renderWork.map(\\.request)
        )
        for (work, raster) in zip(renderWork, responseRasters) {
            renderCache[work.cacheKey] = raster
        }
'''
    new = '''        try executeRenderRequests(
            executable: executable,
            requests: renderWork.map(\\.request)
        )
        for work in renderWork {
            renderURLs[work.cacheKey] = work.request.pngURL
        }
'''
    if old not in text:
        raise RuntimeError("response render cache population block not found")
    text = text.replace(old, new, 1)
    old = '''            guard let cached = renderCache[key] else {
                throw CLIError.invalidContainer(
                    "missing batched native response render \\(owner)/\\(pairName)-\\(side) for \\(heicURL.lastPathComponent)"
                )
            }
            return cached
'''
    new = '''            guard let url = renderURLs[key] else {
                throw CLIError.invalidContainer(
                    "missing batched native response render \\(owner)/\\(pairName)-\\(side) for \\(heicURL.lastPathComponent)"
                )
            }
            return try decodeRGB8(url)
'''
    if old not in text:
        raise RuntimeError("response rendered() cache block not found")
    text = text.replace(old, new, 1)
elif "var renderURLs: [String: URL]" not in text:
    raise RuntimeError("response render cache marker not found")

STYLE.write_text(text, encoding="utf-8")

# Strengthen the permanent source-shape regression so the largest raster-retention pattern cannot return.
test_path = ROOT / "Tests" / "test_performance_design.py"
test = test_path.read_text(encoding="utf-8")
anchor = '''        self.assertIn("private struct SampledJacobian", source)
        self.assertIn("var values: [Float]", source)
'''
replacement = anchor + '''        self.assertNotIn("let derivativeRasters = try Self.render", source)
        self.assertIn("try Self.executeRenderRequests", source)
        self.assertNotIn("var renderCache: [String: Raster]", source)
'''
if "self.assertNotIn(\"let derivativeRasters = try Self.render\"" not in test:
    if anchor not in test:
        raise RuntimeError("performance test anchor not found")
    test = test.replace(anchor, replacement, 1)
    test_path.write_text(test, encoding="utf-8")

print("performance-by-design phase2 render-storage transformation applied")
