#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def write(path: str, text: str) -> None:
    (ROOT / path).write_text(text, encoding="utf-8")


def replace_once(path: str, old: str, new: str, marker: str | None = None) -> None:
    text = read(path)
    if marker is not None and marker in text:
        return
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{path}: expected one replacement, found {count}: {old[:80]!r}")
    write(path, text.replace(old, new, 1))


def regex_once(path: str, pattern: str, replacement: str, marker: str | None = None) -> None:
    text = read(path)
    if marker is not None and marker in text:
        return
    updated, count = re.subn(pattern, replacement, text, count=1, flags=re.S)
    if count != 1:
        raise RuntimeError(f"{path}: regex replacement count={count}: {pattern[:100]!r}")
    write(path, updated)


# 1. RAW TIFF parser: keep Data's COW/mapped storage instead of materializing the DNG as [UInt8].
RAW = "Sources/XDRemuxCore/RAW/CoreImageRAW.swift"
replace_once(RAW, "        let bytes: [UInt8]\n", "        let bytes: Data\n", marker="let bytes: Data")
replace_once(RAW, "        let bytes = [UInt8](data)\n", "        let bytes = data\n", marker="let bytes = data")

# 2. Gain-map reconstruction: borrow the Data bytes directly in the pixel loop.
HDR = "Sources/XDRemuxCore/HDR/HDRPipeline.swift"
old_gain = '''        let outputBytesPerRow = alignUp(mask.width, toMultipleOf: 256)
        var output = Data(count: outputBytesPerRow * mask.height)
        let maskBytes = [UInt8](mask.data)

        output.withUnsafeMutableBytes { rawBuffer in
            guard let outBase = rawBuffer.bindMemory(to: UInt8.self).baseAddress else { return }
            for y in 0..<mask.height {
                let inRow = y * mask.bytesPerRow
                let outRow = y * outputBytesPerRow
                for x in 0..<mask.width {
                    let maskValue = Double(maskBytes[inRow + x]) / 255.0
                    let idx0 = clamp(Int(maskValue * 1000.0), min: 0, max: 1000)
                    let linGray = lut0[idx0]

                    let boosted: Double
                    if linGray < params.knee {
                        boosted = 1.0
                    } else {
                        let t = (linGray - params.knee) / params.kneeRange
                        let idx1 = clamp(Int(t * 1000.0), min: 0, max: 1000)
                        let linear = lut1[idx1]
                        let idx2 = clamp(Int(linear * 1000.0), min: 0, max: 1000)
                        boosted = lut2[idx2]
                    }

                    let idx3: Int
                    if boosted < 1.0 {
                        idx3 = 1000
                    } else {
                        idx3 = clamp(Int(min(boosted, 8.0) * 1000.0), min: 0, max: 8000)
                    }

                    let logGain = clamp(Int(lut3[idx3]), min: 0, max: 255)
                    outBase[outRow + x] = UInt8(logGain)
                }
            }
        }
'''
new_gain = '''        let outputBytesPerRow = alignUp(mask.width, toMultipleOf: 256)
        var output = Data(count: outputBytesPerRow * mask.height)

        mask.data.withUnsafeBytes { inputRawBuffer in
            output.withUnsafeMutableBytes { outputRawBuffer in
                guard let inBase = inputRawBuffer.bindMemory(to: UInt8.self).baseAddress,
                      let outBase = outputRawBuffer.bindMemory(to: UInt8.self).baseAddress else {
                    return
                }
                for y in 0..<mask.height {
                    let inRow = y * mask.bytesPerRow
                    let outRow = y * outputBytesPerRow
                    for x in 0..<mask.width {
                        let maskValue = Double(inBase[inRow + x]) / 255.0
                        let idx0 = clamp(Int(maskValue * 1000.0), min: 0, max: 1000)
                        let linGray = lut0[idx0]

                        let boosted: Double
                        if linGray < params.knee {
                            boosted = 1.0
                        } else {
                            let t = (linGray - params.knee) / params.kneeRange
                            let idx1 = clamp(Int(t * 1000.0), min: 0, max: 1000)
                            let linear = lut1[idx1]
                            let idx2 = clamp(Int(linear * 1000.0), min: 0, max: 1000)
                            boosted = lut2[idx2]
                        }

                        let idx3: Int
                        if boosted < 1.0 {
                            idx3 = 1000
                        } else {
                            idx3 = clamp(Int(min(boosted, 8.0) * 1000.0), min: 0, max: 8000)
                        }

                        let logGain = clamp(Int(lut3[idx3]), min: 0, max: 255)
                        outBase[outRow + x] = UInt8(logGain)
                    }
                }
            }
        }
'''
replace_once(HDR, old_gain, new_gain, marker="mask.data.withUnsafeBytes { inputRawBuffer in")

# 3. Photographic Styles: sample Jacobian derivatives into one flat buffer.
STYLE = "Sources/XDRemuxAppleFeatures/PhotographicStyles/ConstrainedPolynomialStyleDataProducer.swift"
old_raster = '''    private struct Raster {
        let width: Int
        let height: Int
        let rgb: [Float]
    }

    private struct Metrics {
'''
new_raster = '''    private struct Raster {
        let width: Int
        let height: Int
        let rgb: [Float]
    }

    /// Parameter-major storage for exactly the samples consumed by the normal-equation solve.
    /// The former implementation retained a full RGB derivative raster for every parameter even
    /// though solveUpdate sampled at most roughly 50k pixels. Keeping the sampling contract here
    /// makes the data movement explicit and bounds Jacobian storage independently of image area.
    private struct SampledJacobian {
        let pixelStride: Int
        let sampleCount: Int
        let parameterCount: Int
        var values: [Float]

        init(rgbValueCount: Int, parameterCount: Int) {
            precondition(rgbValueCount >= 0 && rgbValueCount.isMultiple(of: 3))
            precondition(parameterCount >= 0)
            let pixelCount = rgbValueCount / 3
            pixelStride = max(1, rgbValueCount / (50_000 * 3))
            let sampledPixelCount = pixelCount == 0
                ? 0
                : ((pixelCount - 1) / pixelStride + 1)
            sampleCount = sampledPixelCount * 3
            self.parameterCount = parameterCount
            values = Array(repeating: 0, count: sampleCount * parameterCount)
        }

        mutating func populate(
            parameter: Int,
            rendered: [Float],
            current: [Float],
            step: Double
        ) throws -> Double {
            guard parameter >= 0, parameter < parameterCount,
                  rendered.count == current.count,
                  rendered.count.isMultiple(of: 3),
                  step.isFinite, step != 0 else {
                throw CLIError.invalidContainer("invalid constrained key-1 sampled Jacobian input")
            }
            let inverseStep = Float(1.0 / step)
            guard inverseStep.isFinite else {
                throw CLIError.invalidContainer("non-finite constrained key-1 Jacobian step")
            }
            let pixelCount = current.count / 3
            let parameterBase = parameter * sampleCount
            var sampled = 0
            var nextSamplePixel = 0
            var squared = 0.0

            // One streaming pass computes the full-raster diagnostic RMS while retaining only the
            // derivative samples used by solveUpdate. No temporary derivative raster is created.
            for pixel in 0..<pixelCount {
                let store = pixel == nextSamplePixel
                let base = pixel * 3
                for channel in 0..<3 {
                    let derivative = (rendered[base + channel] - current[base + channel]) * inverseStep
                    squared += Double(derivative * derivative)
                    if store {
                        values[parameterBase + sampled] = derivative
                        sampled += 1
                    }
                }
                if store {
                    nextSamplePixel += pixelStride
                }
            }
            guard sampled == sampleCount else {
                throw CLIError.invalidContainer("constrained key-1 sampled Jacobian count mismatch")
            }
            return sqrt(squared / Double(max(1, current.count)))
        }

        @inline(__always)
        func derivative(parameter: Int, sample: Int) -> Float {
            values[parameter * sampleCount + sample]
        }
    }

    private struct Metrics {
'''
replace_once(STYLE, old_raster, new_raster, marker="private struct SampledJacobian")

replace_once(
    STYLE,
    '''            var derivatives: [[Float]] = []
            var derivativeRows: [[String: Any]] = []
            let parameterCount = Self.solverRefinementParameterNames.count
            var hueDerivative = Array(repeating: 0.0, count: parameterCount)
''',
    '''            let parameterCount = Self.solverRefinementParameterNames.count
            var jacobian = SampledJacobian(
                rgbValueCount: currentRaster.rgb.count,
                parameterCount: parameterCount
            )
            var derivativeRows: [[String: Any]] = []
            var hueDerivative = Array(repeating: 0.0, count: parameterCount)
''',
    marker="var jacobian = SampledJacobian("
)

replace_once(
    STYLE,
    '''                let derivative = zip(rendered.rgb, currentRaster.rgb).map {
                    ($0 - $1) / Float(work.step)
                }
                derivatives.append(derivative)
                let derivativeRMS = sqrt(
                    derivative.reduce(0.0) { $0 + Double($1 * $1) }
                        / Double(max(1, derivative.count))
                )
''',
    '''                let derivativeRMS = try jacobian.populate(
                    parameter: work.refinementIndex,
                    rendered: rendered.rgb,
                    current: currentRaster.rgb,
                    step: work.step
                )
''',
    marker="let derivativeRMS = try jacobian.populate("
)

replace_once(
    STYLE,
    '''            let update = try Self.solveUpdate(
                current: currentRaster,
                target: target,
                derivatives: derivatives,
                scalarRows: scalarRows
            )
''',
    '''            let update = try Self.solveUpdate(
                current: currentRaster,
                target: target,
                jacobian: jacobian,
                scalarRows: scalarRows
            )
''',
    marker="jacobian: jacobian,"
)

old_solve_sig = '''    private static func solveUpdate(
        current: Raster,
        target: Raster,
        derivatives: [[Float]],
        scalarRows: [(derivative: [Double], residual: Double, weight: Double)] = []
    ) throws -> [Double] {
        let count = solverRefinementParameterNames.count
        guard derivatives.count == count,
              derivatives.allSatisfy({ $0.count == current.rgb.count }),
              target.rgb.count == current.rgb.count,
'''
new_solve_sig = '''    private static func solveUpdate(
        current: Raster,
        target: Raster,
        jacobian: SampledJacobian,
        scalarRows: [(derivative: [Double], residual: Double, weight: Double)] = []
    ) throws -> [Double] {
        try solveUpdate(
            currentRGB: current.rgb,
            targetRGB: target.rgb,
            jacobian: jacobian,
            scalarRows: scalarRows
        )
    }

    private static func solveUpdate(
        currentRGB: [Float],
        targetRGB: [Float],
        jacobian: SampledJacobian,
        scalarRows: [(derivative: [Double], residual: Double, weight: Double)] = []
    ) throws -> [Double] {
        let count = solverRefinementParameterNames.count
        guard jacobian.parameterCount == count,
              targetRGB.count == currentRGB.count,
              currentRGB.count.isMultiple(of: 3),
'''
replace_once(STYLE, old_solve_sig, new_solve_sig, marker="currentRGB: [Float],")

# Rewrite the exact sampled accumulation loop while preserving its sample order and weighting.
old_solve_loop = '''        var normal = Array(repeating: Array(repeating: 0.0, count: count), count: count)
        var gradient = Array(repeating: 0.0, count: count)
        let stride = max(1, current.rgb.count / (50_000 * 3))
        var sampleCount = 0
        for pixel in Swift.stride(from: 0, to: current.rgb.count / 3, by: stride) {
            for channel in 0..<3 {
                let sample = pixel * 3 + channel
                let residual = Double(target.rgb[sample] - current.rgb[sample])
                let huberWeight = min(1.0, 12.0 / max(12.0, abs(residual)))
                sampleCount += 1
                for row in 0..<count {
                    let rowValue = Double(derivatives[row][sample])
                    gradient[row] += huberWeight * rowValue * residual
                    for column in row..<count {
                        normal[row][column] += huberWeight
                            * rowValue * Double(derivatives[column][sample])
                    }
                }
            }
        }
'''
new_solve_loop = '''        var normal = Array(repeating: Array(repeating: 0.0, count: count), count: count)
        var gradient = Array(repeating: 0.0, count: count)
        var sampleCount = 0
        var sampleOrdinal = 0
        for pixel in Swift.stride(
            from: 0,
            to: currentRGB.count / 3,
            by: jacobian.pixelStride
        ) {
            for channel in 0..<3 {
                let sample = pixel * 3 + channel
                let residual = Double(targetRGB[sample] - currentRGB[sample])
                let huberWeight = min(1.0, 12.0 / max(12.0, abs(residual)))
                sampleCount += 1
                for row in 0..<count {
                    let rowValue = Double(jacobian.derivative(parameter: row, sample: sampleOrdinal))
                    gradient[row] += huberWeight * rowValue * residual
                    for column in row..<count {
                        normal[row][column] += huberWeight
                            * rowValue
                            * Double(jacobian.derivative(parameter: column, sample: sampleOrdinal))
                    }
                }
                sampleOrdinal += 1
            }
        }
        guard sampleOrdinal == jacobian.sampleCount else {
            throw CLIError.invalidContainer("constrained key-1 sampled Jacobian solve count mismatch")
        }
'''
replace_once(STYLE, old_solve_loop, new_solve_loop, marker="sampleOrdinal == jacobian.sampleCount")

# Package-visible pure probes let regression tests prove numerical equivalence and the bounded storage shape.
probe_anchor = '''    private static func responseScore(rmse8: Double, state: ResponseObjectiveState?) -> Double {
        guard let state else { return rmse8 }
        return rmse8
            + responseScoreHueWeight * state.hueViolationDegrees
            + responseScoreRGWeight * state.rgViolation
    }

'''
probe_insert = probe_anchor + '''    package static func sampledJacobianStorageValueCount(
        rgbValueCount: Int,
        parameterCount: Int = 12
    ) -> Int {
        SampledJacobian(
            rgbValueCount: rgbValueCount,
            parameterCount: parameterCount
        ).values.count
    }

    package static func solveSampledUpdateForTesting(
        currentRGB: [Float],
        targetRGB: [Float],
        perturbedRGB: [[Float]],
        steps: [Double]
    ) throws -> [Double] {
        guard perturbedRGB.count == solverRefinementParameterNames.count,
              steps.count == perturbedRGB.count else {
            throw CLIError.invalidContainer("invalid sampled Jacobian regression fixture")
        }
        var jacobian = SampledJacobian(
            rgbValueCount: currentRGB.count,
            parameterCount: perturbedRGB.count
        )
        for parameter in perturbedRGB.indices {
            _ = try jacobian.populate(
                parameter: parameter,
                rendered: perturbedRGB[parameter],
                current: currentRGB,
                step: steps[parameter]
            )
        }
        return try solveUpdate(
            currentRGB: currentRGB,
            targetRGB: targetRGB,
            jacobian: jacobian
        )
    }

'''
replace_once(STYLE, probe_anchor, probe_insert, marker="sampledJacobianStorageValueCount")

# 4. Candidate HEICs: APFS clone (FileManager.copyItem) + bounded in-file key patch.
old_inject = '''    private static func injectStyleData(
        _ styleData: Data,
        into heicURL: URL,
        identityStylePropertyList: Data,
        outputURL: URL
    ) throws {
        let identity = try AppleStyleDataLayout.completeIdentity()
        guard let identityRange = uniqueRange(of: identity, in: identityStylePropertyList) else {
            throw CLIError.invalidContainer(
                "identity key 1 does not occur exactly once in the preliminary style plist"
            )
        }
        var replacementPropertyList = identityStylePropertyList
        replacementPropertyList.replaceSubrange(identityRange, with: styleData)
        let source = try Data(contentsOf: heicURL, options: [.mappedIfSafe])
        let range = try identityPlistRange(
            in: source,
            of: heicURL,
            identityStylePropertyList: identityStylePropertyList
        )
        var output = source
        output.replaceSubrange(range, with: replacementPropertyList)
        guard output.count == source.count else {
            throw CLIError.invalidContainer("key-1 injection changed HEIC byte length")
        }
        try output.write(to: outputURL, options: .atomic)
    }
'''
new_inject = '''    private static func injectStyleData(
        _ styleData: Data,
        into heicURL: URL,
        identityStylePropertyList: Data,
        outputURL: URL
    ) throws {
        let identity = try AppleStyleDataLayout.completeIdentity()
        guard styleData.count == identity.count,
              let identityRange = uniqueRange(of: identity, in: identityStylePropertyList),
              identityRange.count == styleData.count else {
            throw CLIError.invalidContainer(
                "identity key 1 does not occur exactly once in the preliminary style plist"
            )
        }
        let source = try Data(contentsOf: heicURL, options: [.mappedIfSafe])
        let plistRange = try identityPlistRange(
            in: source,
            of: heicURL,
            identityStylePropertyList: identityStylePropertyList
        )
        let styleOffset = plistRange.lowerBound + identityRange.lowerBound
        guard styleOffset >= source.startIndex,
              styleOffset <= source.endIndex,
              styleData.count <= source.endIndex - styleOffset else {
            throw CLIError.invalidContainer("key-1 injection range is outside the HEIC")
        }

        let fileManager = FileManager.default
        try? fileManager.removeItem(at: outputURL)
        do {
            // On APFS FileManager.copyItem creates a clone. Only the key-1 blocks touched below
            // become private; non-APFS volumes retain the same correctness with a normal copy.
            try fileManager.copyItem(at: heicURL, to: outputURL)
            let handle = try FileHandle(forWritingTo: outputURL)
            do {
                try handle.seek(toOffset: UInt64(styleOffset))
                try handle.write(contentsOf: styleData)
                try handle.synchronize()
                try handle.close()
            } catch {
                try? handle.close()
                throw error
            }

            let attributes = try fileManager.attributesOfItem(atPath: outputURL.path)
            let outputSize = (attributes[.size] as? NSNumber)?.intValue
            guard outputSize == source.count else {
                throw CLIError.invalidContainer("key-1 injection changed HEIC byte length")
            }
            let verification = try FileHandle(forReadingFrom: outputURL)
            defer { try? verification.close() }
            try verification.seek(toOffset: UInt64(styleOffset))
            let patched = try verification.read(upToCount: styleData.count) ?? Data()
            guard patched == styleData else {
                throw CLIError.invalidContainer("key-1 injection verification failed")
            }
        } catch {
            try? fileManager.removeItem(at: outputURL)
            throw error
        }
    }

    package static func injectStyleDataForTesting(
        _ styleData: Data,
        into heicURL: URL,
        identityStylePropertyList: Data,
        outputURL: URL
    ) throws {
        try injectStyleData(
            styleData,
            into: heicURL,
            identityStylePropertyList: identityStylePropertyList,
            outputURL: outputURL
        )
    }
'''
replace_once(STYLE, old_inject, new_inject, marker="injectStyleDataForTesting")

# 5. Direct tiled HEVC: preserve compressed JPEG input for UHDR, but send reconstructed rasters
# to the helper as raw row-strided bytes instead of PNG encode -> disk -> PNG decode.
ENCODER = "Sources/XDRemuxCore/HEIF/DirectTiledHEVCGainMapEncoder.swift"
pattern = r'''    package static func encode\(\n        imageData: Data,.*?\n    private static func idrTilePayloads'''
replacement = '''    package static func encode(
        imageData: Data,
        width: Int,
        height: Int,
        channelCount: Int,
        scratchBaseURL: URL
    ) throws -> DirectTiledHEVCGainMap {
        let inputURL = siblingURL(
            for: scratchBaseURL,
            label: "direct-gain",
            pathExtension: channelCount == 1 ? "png" : "jpg"
        )
        defer { try? FileManager.default.removeItem(at: inputURL) }
        try imageData.write(to: inputURL, options: .atomic)
        return try encodeFile(
            inputURL: inputURL,
            width: width,
            height: height,
            bytesPerRow: nil,
            channelCount: channelCount,
            scratchBaseURL: scratchBaseURL
        )
    }

    package static func encode(
        raster: GainMapRaster,
        scratchBaseURL: URL
    ) throws -> DirectTiledHEVCGainMap {
        guard raster.width > 0, raster.height > 0,
              raster.channelCount == 1 || raster.channelCount == 3 else {
            throw CLIError.invalidContainer("direct Gain Map encoder received invalid raster geometry")
        }
        let minimumBytesPerRow = raster.width * (raster.channelCount == 1 ? 1 : 4)
        guard raster.bytesPerRow >= minimumBytesPerRow,
              raster.height <= Int.max / raster.bytesPerRow,
              raster.data.count >= raster.bytesPerRow * raster.height else {
            throw CLIError.invalidContainer("direct Gain Map encoder received invalid raster storage")
        }
        let rawURL = siblingURL(
            for: scratchBaseURL,
            label: "direct-gain",
            pathExtension: "raw"
        )
        defer { try? FileManager.default.removeItem(at: rawURL) }
        try raster.data.write(to: rawURL, options: .atomic)
        return try encodeFile(
            inputURL: rawURL,
            width: raster.width,
            height: raster.height,
            bytesPerRow: raster.bytesPerRow,
            channelCount: raster.channelCount,
            scratchBaseURL: scratchBaseURL
        )
    }

    private static func encodeFile(
        inputURL: URL,
        width: Int,
        height: Int,
        bytesPerRow: Int?,
        channelCount: Int,
        scratchBaseURL: URL
    ) throws -> DirectTiledHEVCGainMap {
        guard width > 0, height > 0, channelCount == 1 || channelCount == 3 else {
            throw CLIError.invalidContainer("direct Gain Map encoder received invalid raster geometry")
        }
        let tileSize = EncodingQualityPolicy.integer(
            environmentKey: "XDREMUX_GAIN_MAP_TILE_SIZE",
            defaultValue: 512,
            allowedValues: [256, 512, 1024]
        )
        let annexBURL = siblingURL(for: scratchBaseURL, label: "direct-gain", pathExtension: "hevc")
        let hvcCURL = siblingURL(for: scratchBaseURL, label: "direct-gain", pathExtension: "hvcc")
        defer {
            let environment = ProcessInfo.processInfo.environment
            if environment["XDREMUX_KEEP_GAIN_SCRATCH"] != "1"
                && environment["XDREMUX_KEEP_PORTRAIT_SCRATCH"] != "1" {
                for url in [annexBURL, hvcCURL] {
                    try? FileManager.default.removeItem(at: url)
                }
            }
        }
        let executable = try encoderExecutable()
        let mode = channelCount == 1 ? "mono8tile" : "rgb4448tile"
        let quality = EncodingQualityPolicy.value(
            environmentKey: "XDREMUX_GAIN_MAP_QUALITY",
            defaultValue: 0.9
        )
        var arguments = [
            inputURL.path,
            annexBURL.path,
            String(format: "%.6f", quality),
            mode,
            hvcCURL.path,
            String(tileSize),
        ]
        if let bytesPerRow {
            arguments += [String(width), String(height), String(bytesPerRow)]
        }
        let result = try run(executable, arguments: arguments)
        guard result.status == 0 else {
            let diagnostic = String(data: result.stderr + result.stdout, encoding: .utf8)?
                .trimmingCharacters(in: .whitespacesAndNewlines) ?? "unknown error"
            throw CLIError.invalidContainer("private tile encoder failed: \\(diagnostic)")
        }
        let tilePayloads = try idrTilePayloads(from: Data(contentsOf: annexBURL))
        let columns = (width + tileSize - 1) / tileSize
        let rows = (height + tileSize - 1) / tileSize
        guard tilePayloads.count == rows * columns else {
            throw CLIError.invalidContainer(
                "private tile encoder returned \\(tilePayloads.count) samples; expected \\(rows * columns)"
            )
        }
        return DirectTiledHEVCGainMap(
            width: width,
            height: height,
            tileWidth: tileSize,
            tileHeight: tileSize,
            tilePayloads: tilePayloads,
            tileSizes: Array(repeating: (tileSize, tileSize), count: tilePayloads.count),
            hvcC: try Data(contentsOf: hvcCURL),
            channelCount: channelCount
        )
    }

    private static func idrTilePayloads'''
regex_once(ENCODER, pattern, replacement, marker="private static func encodeFile(")

# 6. Native encoder helper: mmap raw row-strided input when geometry is supplied.
HELPER = "Sources/XDRemuxCore/Resources/Native/apple_vt_hevc_encoder.swift"
source_frame = '''struct SourceFrame {
    let pixelBuffer: CVPixelBuffer
    let width: Int
    let height: Int
}
'''
raw_struct = source_frame + '''
struct RawRasterDescriptor {
    let width: Int
    let height: Int
    let bytesPerRow: Int
}
'''
replace_once(HELPER, source_frame, raw_struct, marker="struct RawRasterDescriptor")

load_anchor = '''func loadImage(_ path: String) -> CGImage {
    let url = URL(fileURLWithPath: path)
    guard let source = CGImageSourceCreateWithURL(url as CFURL, nil),
          let image = CGImageSourceCreateImageAtIndex(source, 0, nil) else {
        fail("Could not load image: \\(path)")
    }
    return image
}
'''
load_raw = load_anchor + '''
func loadRawImage(
    _ path: String,
    mode: PixelMode,
    descriptor: RawRasterDescriptor
) -> CGImage {
    let isMono = mode == .mono8 || mode == .mono8tile
    let bytesPerPixel = isMono ? 1 : 4
    guard descriptor.width > 0, descriptor.height > 0,
          descriptor.width <= Int.max / bytesPerPixel,
          descriptor.bytesPerRow >= descriptor.width * bytesPerPixel,
          descriptor.height <= Int.max / descriptor.bytesPerRow else {
        fail("Invalid raw raster geometry")
    }
    let expectedCount = descriptor.bytesPerRow * descriptor.height
    let url = URL(fileURLWithPath: path)
    guard let data = try? Data(contentsOf: url, options: [.mappedIfSafe]),
          data.count == expectedCount,
          let provider = CGDataProvider(data: data as CFData) else {
        fail("Could not map raw raster: \\(path)")
    }
    let colorSpace = isMono ? CGColorSpaceCreateDeviceGray() : CGColorSpaceCreateDeviceRGB()
    let bitmapInfo = isMono
        ? CGBitmapInfo(rawValue: CGImageAlphaInfo.none.rawValue)
        : CGBitmapInfo(
            rawValue: CGBitmapInfo.byteOrder32Little.rawValue
                | CGImageAlphaInfo.noneSkipFirst.rawValue
        )
    guard let image = CGImage(
        width: descriptor.width,
        height: descriptor.height,
        bitsPerComponent: 8,
        bitsPerPixel: bytesPerPixel * 8,
        bytesPerRow: descriptor.bytesPerRow,
        space: colorSpace,
        bitmapInfo: bitmapInfo,
        provider: provider,
        decode: nil,
        shouldInterpolate: false,
        intent: .defaultIntent
    ) else {
        fail("Could not create CGImage from raw raster")
    }
    return image
}
'''
replace_once(HELPER, load_anchor, load_raw, marker="func loadRawImage(")

replace_once(
    HELPER,
    '''func makeSourceFrame(path: String, mode: PixelMode, tileSize: Int = 512) -> SourceFrame {
    let image = loadImage(path)
''',
    '''func makeSourceFrame(
    path: String,
    mode: PixelMode,
    tileSize: Int = 512,
    rawDescriptor: RawRasterDescriptor? = nil
) -> SourceFrame {
    let image = rawDescriptor.map {
        loadRawImage(path, mode: mode, descriptor: $0)
    } ?? loadImage(path)
''',
    marker="rawDescriptor: RawRasterDescriptor? = nil"
)

old_args = '''let args = CommandLine.arguments
if args.count < 3 || args.count > 7 {
    fail(
        "usage: apple_vt_hevc_encoder.swift input output.hevc "
            + "[quality] [rgb10|rgb4448|rgb4448tile|mono8|mono8tile] [output.hvcc] [tile-size]"
    )
}
'''
new_args = '''let args = CommandLine.arguments
if args.count < 3 || (args.count > 7 && args.count != 10) {
    fail(
        "usage: apple_vt_hevc_encoder.swift input output.hevc "
            + "[quality] [rgb10|rgb4448|rgb4448tile|mono8|mono8tile] [output.hvcc] [tile-size] "
            + "[raw-width raw-height raw-bytes-per-row]"
    )
}
'''
replace_once(HELPER, old_args, new_args, marker="args.count != 10")

old_source_call = '''let source = makeSourceFrame(path: args[1], mode: mode, tileSize: tileSize)
let pixelBuffer = source.pixelBuffer
'''
new_source_call = '''let rawDescriptor: RawRasterDescriptor?
if args.count == 10 {
    guard mode == .mono8tile || mode == .rgb4448tile,
          let rawWidth = Int(args[7]),
          let rawHeight = Int(args[8]),
          let rawBytesPerRow = Int(args[9]) else {
        fail("raw geometry is valid only for mono8tile/rgb4448tile and must contain integers")
    }
    rawDescriptor = RawRasterDescriptor(
        width: rawWidth,
        height: rawHeight,
        bytesPerRow: rawBytesPerRow
    )
} else {
    rawDescriptor = nil
}

let source = makeSourceFrame(
    path: args[1],
    mode: mode,
    tileSize: tileSize,
    rawDescriptor: rawDescriptor
)
let pixelBuffer = source.pixelBuffer
'''
replace_once(HELPER, old_source_call, new_source_call, marker="let rawDescriptor: RawRasterDescriptor?")

# Permanent architecture-regression tests: these fail on the original allocation/materialization shapes.
TEST = ROOT / "Tests" / "test_performance_design.py"
TEST.write_text('''from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class PerformanceDesignArchitectureTests(unittest.TestCase):
    def source(self, relative: str) -> str:
        return (ROOT / relative).read_text(encoding="utf-8")

    def test_gain_map_hot_loop_borrows_mask_storage(self) -> None:
        source = self.source("Sources/XDRemuxCore/HDR/HDRPipeline.swift")
        self.assertNotIn("let maskBytes = [UInt8](mask.data)", source)
        self.assertIn("mask.data.withUnsafeBytes { inputRawBuffer in", source)

    def test_raw_tiff_reader_keeps_data_storage(self) -> None:
        source = self.source("Sources/XDRemuxCore/RAW/CoreImageRAW.swift")
        self.assertNotIn("let bytes = [UInt8](data)", source)
        self.assertIn("let bytes: Data", source)

    def test_style_jacobian_is_sampled_and_flat(self) -> None:
        source = self.source(
            "Sources/XDRemuxAppleFeatures/PhotographicStyles/ConstrainedPolynomialStyleDataProducer.swift"
        )
        self.assertNotIn("var derivatives: [[Float]]", source)
        self.assertNotIn("zip(rendered.rgb, currentRaster.rgb).map", source)
        self.assertIn("private struct SampledJacobian", source)
        self.assertIn("var values: [Float]", source)

    def test_style_candidate_does_not_copy_whole_heic_in_swift(self) -> None:
        source = self.source(
            "Sources/XDRemuxAppleFeatures/PhotographicStyles/ConstrainedPolynomialStyleDataProducer.swift"
        )
        self.assertNotIn("var output = source", source)
        self.assertIn("fileManager.copyItem(at: heicURL, to: outputURL)", source)
        self.assertIn("try handle.seek(toOffset: UInt64(styleOffset))", source)

    def test_direct_raster_encoder_uses_raw_transport(self) -> None:
        wrapper = self.source("Sources/XDRemuxCore/HEIF/DirectTiledHEVCGainMapEncoder.swift")
        helper = self.source("Sources/XDRemuxCore/Resources/Native/apple_vt_hevc_encoder.swift")
        self.assertNotIn("CGImageDestinationCreateWithData", wrapper)
        self.assertIn('pathExtension: "raw"', wrapper)
        self.assertIn("RawRasterDescriptor", helper)
        self.assertIn("Data(contentsOf: url, options: [.mappedIfSafe])", helper)


if __name__ == "__main__":
    unittest.main()
''', encoding="utf-8")

# Numerical regression tests for the sampled Jacobian and bounded in-file candidate patch.
APPLE_TEST = ROOT / "Tests" / "XDRemuxAppleFeaturesTests" / "PerformanceDesignTests.swift"
APPLE_TEST.write_text(r'''import Foundation
import XCTest
import XDRemuxCore
@testable import XDRemuxAppleFeatures

final class PerformanceDesignTests: XCTestCase {
    private typealias Producer = ConstrainedPolynomialStyleDataProducer

    func testSampledJacobianStorageIsBoundedAtAnalysisResolution() {
        let rgbValueCount = 1024 * 1024 * 3
        let sampled = Producer.sampledJacobianStorageValueCount(
            rgbValueCount: rgbValueCount,
            parameterCount: 12
        )
        let formerFullDerivativeStorage = rgbValueCount * 12
        XCTAssertLessThan(sampled, 2_000_000)
        XCTAssertLessThan(sampled * 15, formerFullDerivativeStorage)
    }

    func testSampledJacobianMatchesFormerFullDerivativeSolve() throws {
        let pixelCount = 100_002 // forces the production sampling stride above one
        let valueCount = pixelCount * 3
        var current = [Float](repeating: 0, count: valueCount)
        var target = [Float](repeating: 0, count: valueCount)
        for index in 0..<valueCount {
            let base = Float((index * 17 + 13) % 251) / 10
            current[index] = base
            target[index] = base + Float((index * 7 + 3) % 19 - 9) * 0.05
        }
        let steps = (0..<12).map { 0.01 + Double($0) * 0.0005 }
        var perturbed: [[Float]] = []
        perturbed.reserveCapacity(12)
        for parameter in 0..<12 {
            let step = Float(steps[parameter])
            var raster = current
            for index in raster.indices {
                let feature = Float(((index + 1) * (parameter + 3)) % 23 - 11) / 11
                raster[index] += step * feature
            }
            perturbed.append(raster)
        }

        let sampled = try Producer.solveSampledUpdateForTesting(
            currentRGB: current,
            targetRGB: target,
            perturbedRGB: perturbed,
            steps: steps
        )
        let legacy = try legacyFullDerivativeSolve(
            current: current,
            target: target,
            perturbed: perturbed,
            steps: steps
        )
        XCTAssertEqual(sampled.count, legacy.count)
        for index in sampled.indices {
            XCTAssertEqual(sampled[index], legacy[index], accuracy: 1e-10, "parameter \\(index)")
        }
    }

    func testCandidatePatchChangesOnlyStyleDataBytes() throws {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-style-patch-\\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }

        let identity = try AppleStyleDataLayout.completeIdentity()
        var plist = Data("plist-prefix".utf8)
        let identityOffset = plist.count
        plist.append(identity)
        plist.append(Data("plist-suffix".utf8))
        var base = Data(repeating: 0xA5, count: 4096)
        let plistOffset = base.count
        base.append(plist)
        base.append(Data(repeating: 0x5A, count: 4096))
        let input = root.appendingPathComponent("input.heic")
        let output = root.appendingPathComponent("output.heic")
        try base.write(to: input)

        let styleData = try Producer.styleData(parameters: [0.01, -0.01, 0.005, 0, 0, 0])
        XCTAssertEqual(styleData.count, identity.count)
        try Producer.injectStyleDataForTesting(
            styleData,
            into: input,
            identityStylePropertyList: plist,
            outputURL: output
        )
        let patched = try Data(contentsOf: output)
        XCTAssertEqual(patched.count, base.count)
        let absoluteStyleOffset = plistOffset + identityOffset
        XCTAssertEqual(
            patched.subdata(in: absoluteStyleOffset..<(absoluteStyleOffset + styleData.count)),
            styleData
        )
        XCTAssertEqual(patched.prefix(absoluteStyleOffset), base.prefix(absoluteStyleOffset))
        XCTAssertEqual(
            patched.suffix(from: absoluteStyleOffset + styleData.count),
            base.suffix(from: absoluteStyleOffset + styleData.count)
        )
    }

    private func legacyFullDerivativeSolve(
        current: [Float],
        target: [Float],
        perturbed: [[Float]],
        steps: [Double]
    ) throws -> [Double] {
        let count = 12
        let derivatives = zip(perturbed, steps).map { raster, step in
            zip(raster, current).map { ($0 - $1) / Float(step) }
        }
        var normal = Array(repeating: Array(repeating: 0.0, count: count), count: count)
        var gradient = Array(repeating: 0.0, count: count)
        let stride = max(1, current.count / (50_000 * 3))
        var sampleCount = 0
        for pixel in Swift.stride(from: 0, to: current.count / 3, by: stride) {
            for channel in 0..<3 {
                let sample = pixel * 3 + channel
                let residual = Double(target[sample] - current[sample])
                let huberWeight = min(1.0, 12.0 / max(12.0, abs(residual)))
                sampleCount += 1
                for row in 0..<count {
                    let rowValue = Double(derivatives[row][sample])
                    gradient[row] += huberWeight * rowValue * residual
                    for column in row..<count {
                        normal[row][column] += huberWeight
                            * rowValue * Double(derivatives[column][sample])
                    }
                }
            }
        }
        if sampleCount > 0 {
            let normalization = 1.0 / Double(sampleCount)
            for row in 0..<count {
                gradient[row] *= normalization
                for column in row..<count {
                    normal[row][column] *= normalization
                }
            }
        }
        for row in 0..<count {
            for column in 0..<row { normal[row][column] = normal[column][row] }
        }
        let trace = (0..<count).reduce(0.0) { $0 + normal[$1][$1] }
        let ridge = max(trace / Double(count) * 1e-6, 1e-9)
        for index in 0..<count { normal[index][index] += ridge }
        var solution = try solveLinearSystem(normal, gradient)
        let epsilon = 1.0 / 32.0
        for index in solution.indices {
            solution[index] = min(epsilon, max(-epsilon, solution[index]))
        }
        return solution
    }

    private func solveLinearSystem(_ matrix: [[Double]], _ vector: [Double]) throws -> [Double] {
        let count = vector.count
        var augmented = zip(matrix, vector).map { $0 + [$1] }
        for pivot in 0..<count {
            let best = (pivot..<count).max {
                abs(augmented[$0][pivot]) < abs(augmented[$1][pivot])
            }!
            guard abs(augmented[best][pivot]) > 1e-12 else {
                throw NSError(domain: "PerformanceDesignTests", code: 1)
            }
            if best != pivot { augmented.swapAt(best, pivot) }
            let divisor = augmented[pivot][pivot]
            for column in pivot...count { augmented[pivot][column] /= divisor }
            for row in 0..<count where row != pivot {
                let factor = augmented[row][pivot]
                for column in pivot...count {
                    augmented[row][column] -= factor * augmented[pivot][column]
                }
            }
        }
        return augmented.map { $0[count] }
    }
}
''', encoding="utf-8")

print("performance-by-design source transformation applied")
