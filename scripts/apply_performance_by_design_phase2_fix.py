#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
STYLE = ROOT / "Sources/XDRemuxAppleFeatures/PhotographicStyles/ConstrainedPolynomialStyleDataProducer.swift"
TEST = ROOT / "Tests" / "test_performance_design.py"
text = STYLE.read_text(encoding="utf-8")


def replace_once(old: str, new: str, marker: str) -> None:
    global text
    if marker in text:
        return
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"expected exactly one replacement, found {count}: {old[:100]!r}")
    text = text.replace(old, new, 1)


# The phase-2 bootstrap introduced executeRenderRequests(), but the first script revision
# intentionally failed closed when these large caller blocks did not match its regex exactly.
# Finish those substitutions with literal source shapes so no ignored all-raster result remains.
old_initialization = '''        let initializationRasters = try Self.render(
            executable: executable,
            requests: initializationWork.flatMap(\\.requests)
        )
        // The renderer is a pure function of the materialized candidate, so the
        // raster rendered for the currently selected coefficients is carried
        // forward instead of re-materializing and re-rendering it per iteration.
        var latestRendered: (coefficients: [Double], raster: Raster) = (
            identityCoefficients,
            identityRender.raster
        )
        var latestResponse = identityResponse
        var initializationCursor = 0
        for work in initializationWork {
            guard initializationCursor + work.requests.count <= initializationRasters.count else {
                throw CLIError.invalidContainer(
                    "constrained key-1 renderer returned an incomplete initialization batch"
                )
            }
            let raster = initializationRasters[initializationCursor]
            let candidateResponse: ResponseObjectiveState? = responseActive
                ? responseState(
                    mid: initializationRasters[initializationCursor + 1],
                    plus: initializationRasters[initializationCursor + 2]
                )
                : nil
            initializationCursor += work.requests.count
'''
new_initialization = '''        try Self.executeRenderRequests(
            executable: executable,
            requests: initializationWork.flatMap(\\.requests)
        )
        // The renderer is a pure function of the materialized candidate, so the
        // raster rendered for the currently selected coefficients is carried
        // forward instead of re-materializing and re-rendering it per iteration.
        var latestRendered: (coefficients: [Double], raster: Raster) = (
            identityCoefficients,
            identityRender.raster
        )
        var latestResponse = identityResponse
        for work in initializationWork {
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
'''
replace_once(
    old_initialization,
    new_initialization,
    "let raster = try Self.decodeRGB8(work.requests[0].pngURL)\n            let candidateResponse: ResponseObjectiveState?\n            if responseActive"
)

old_derivative = '''            let derivativeRasters = try Self.render(
                executable: executable,
                requests: derivativeWork.flatMap(\\.requests)
            )
'''
new_derivative = '''            try Self.executeRenderRequests(
                executable: executable,
                requests: derivativeWork.flatMap(\\.requests)
            )
'''
replace_once(
    old_derivative,
    new_derivative,
    "try Self.executeRenderRequests(\n                executable: executable,\n                requests: derivativeWork.flatMap"
)

old_line_search = '''            let lineSearchRasters = try Self.render(
                executable: executable,
                requests: lineSearchWork.flatMap(\\.requests)
            )
            var proposedRaster: Raster?
            var lineSearchCursor = 0
            for work in lineSearchWork {
                guard lineSearchCursor + work.requests.count <= lineSearchRasters.count else {
                    throw CLIError.invalidContainer(
                        "constrained key-1 renderer returned an incomplete line-search batch"
                    )
                }
                let raster = lineSearchRasters[lineSearchCursor]
                let candidateResponse: ResponseObjectiveState? = responseActive
                    ? responseState(
                        mid: lineSearchRasters[lineSearchCursor + 1],
                        plus: lineSearchRasters[lineSearchCursor + 2]
                    )
                    : nil
                lineSearchCursor += work.requests.count
'''
new_line_search = '''            try Self.executeRenderRequests(
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
'''
replace_once(
    old_line_search,
    new_line_search,
    "requests: lineSearchWork.flatMap(\\.requests)\n            )\n            var proposedRaster: Raster?\n            for work in lineSearchWork {\n                let raster = try Self.decodeRGB8"
)

for forbidden in (
    "let initializationRasters = try Self.render",
    "let derivativeRasters = try Self.render",
    "let lineSearchRasters = try Self.render",
):
    if forbidden in text:
        raise RuntimeError(f"stale all-raster retention remains: {forbidden}")

STYLE.write_text(text, encoding="utf-8")

# Keep the static regression focused on the three production batches whose retained raster
# footprint scales with candidate count.
test = TEST.read_text(encoding="utf-8")
needle = '        self.assertNotIn("let derivativeRasters = try Self.render", source)\n'
addition = (
    needle
    + '        self.assertNotIn("let initializationRasters = try Self.render", source)\n'
    + '        self.assertNotIn("let lineSearchRasters = try Self.render", source)\n'
)
if 'self.assertNotIn("let initializationRasters = try Self.render"' not in test:
    if needle not in test:
        raise RuntimeError("architecture regression insertion point not found")
    test = test.replace(needle, addition, 1)
    TEST.write_text(test, encoding="utf-8")

print("completed bounded render decoding transformation")
