import CryptoKit
import Foundation
import XDRemuxCore

private func sha256(_ data: Data) -> String {
    SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
}

private func fittedSize(width: Int, height: Int, maximum: Int) -> (Int, Int) {
    let scale = min(1.0, Double(maximum) / Double(max(width, height)))
    return (
        max(2, Int((Double(width) * scale / 2).rounded()) * 2),
        max(2, Int((Double(height) * scale / 2).rounded()) * 2)
    )
}

private func jsonData(_ object: Any) throws -> Data {
    try JSONSerialization.data(
        withJSONObject: object,
        options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
    )
}

private func rawData<T>(_ values: [T]) -> Data {
    values.withUnsafeBytes { Data($0) }
}

private func sha256File(_ url: URL) throws -> String {
    sha256(try Data(contentsOf: url, options: [.mappedIfSafe]))
}

private func main() throws {
    let arguments = CommandLine.arguments
    guard arguments.count >= 3 else {
        throw NSError(
            domain: "CoreImageRAWDiagnostics",
            code: 2,
            userInfo: [NSLocalizedDescriptionKey: "usage: coreimage-raw-diagnostics DNG_DIRECTORY OUTPUT_DIRECTORY [MAX_SIZE]"]
        )
    }
    let root = URL(fileURLWithPath: arguments[1]).standardizedFileURL
    let output = URL(fileURLWithPath: arguments[2]).standardizedFileURL
    let maximum = arguments.count >= 4 ? max(32, min(1024, Int(arguments[3]) ?? 256)) : 256
    let paths = (FileManager.default.enumerator(
        at: root,
        includingPropertiesForKeys: [.isRegularFileKey],
        options: [.skipsHiddenFiles]
    )?.compactMap { $0 as? URL } ?? [])
        .filter { $0.pathExtension.lowercased() == "dng" }
        .sorted { $0.path < $1.path }
    guard !paths.isEmpty else {
        throw NSError(
            domain: "CoreImageRAWDiagnostics",
            code: 3,
            userInfo: [NSLocalizedDescriptionKey: "no DNG files under \(root.path)"]
        )
    }
    try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
    var rows: [[String: Any]] = []
    for path in paths {
        let started = CFAbsoluteTimeGetCurrent()
        let before = try Data(contentsOf: path, options: [.mappedIfSafe])
        let metadata = try CoreImageRAW.extractDNGMetadata(from: path)
        let size = fittedSize(width: metadata.rawWidth, height: metadata.rawHeight, maximum: maximum)
        let result = try CoreImageRAW.decode(
            dngURL: path,
            targetWidth: size.0,
            targetHeight: size.1
        )
        let after = try Data(contentsOf: path, options: [.mappedIfSafe])
        let sampleOutput = output.appendingPathComponent(path.deletingPathExtension().lastPathComponent)
        try FileManager.default.createDirectory(at: sampleOutput, withIntermediateDirectories: true)
        let previewURL = sampleOutput.appendingPathComponent("embedded-preview.jpg")
        let float32URL = sampleOutput.appendingPathComponent("ciraw-neutral.rgba32f.bin")
        let float16URL = sampleOutput.appendingPathComponent("ciraw-neutral.rgba16f.bin")
        let normalizedURL = sampleOutput.appendingPathComponent("ciraw-neutral-calibrated.rgba16unorm.bin")
        try result.embeddedPreview.data.write(to: previewURL, options: .atomic)
        try rawData(result.rgbaFloat32).write(to: float32URL, options: .atomic)
        try result.rgbaFloat16.write(to: float16URL, options: .atomic)
        try result.normalizedRGBA16.write(to: normalizedURL, options: .atomic)
        let provenanceData = try jsonData(result.provenance)
        try provenanceData.write(to: sampleOutput.appendingPathComponent("provenance.json"), options: .atomic)
        let unchanged = sha256(before) == sha256(after)
        rows.append([
            "fileName": path.lastPathComponent,
            "dngPath": path.path,
            "dngSHA256": result.dngSHA256,
            "targetSize": [size.0, size.1],
            "elapsedSeconds": CFAbsoluteTimeGetCurrent() - started,
            "rawFileUnchanged": unchanged,
            "rawStatistics": result.rawStatistics.dictionary,
            "previewStatistics": result.previewStatistics.dictionary,
            "pairValidation": result.pairValidation?.dictionary as Any? ?? NSNull(),
            "dngMetadata": metadata.dictionary,
            "cirawFilterDefaults": result.filterDefaults,
            "cirawFilterEffectiveValues": result.filterEffectiveValues,
            "calibration": [
                "model": result.calibrationModel,
                "channelGains": result.calibrationChannelGains,
                "sampleCount": result.calibrationSampleCount,
                "confidence": result.calibrationConfidence,
            ],
            "outputs": [
                "embeddedPreviewJPEG": [
                    "path": previewURL.path,
                    "byteCount": result.embeddedPreview.data.count,
                    "sha256": result.embeddedPreview.sha256,
                ],
                "rgbaFloat32": [
                    "path": float32URL.path,
                    "byteCount": result.rgbaFloat32.count * MemoryLayout<Float>.size,
                    "sha256": try sha256File(float32URL),
                    "colorSpace": "extended-linear-Display-P3",
                    "byteOrder": "little-endian",
                ],
                "rgbaFloat16": [
                    "path": float16URL.path,
                    "byteCount": result.rgbaFloat16.count,
                    "sha256": try sha256File(float16URL),
                    "colorSpace": "extended-linear-Display-P3",
                    "byteOrder": "little-endian",
                ],
                "calibratedRGBA16Unorm": [
                    "path": normalizedURL.path,
                    "byteCount": result.normalizedRGBA16.count,
                    "sha256": try sha256File(normalizedURL),
                    "colorSpace": "extended-linear-Display-P3",
                    "consumerInput": true,
                    "byteOrder": "little-endian",
                ],
            ],
            "provenancePath": sampleOutput.appendingPathComponent("provenance.json").path,
        ])
    }
    let summary: [String: Any] = [
        "schema": "xdremux-coreimage-raw-dng-formal-batch-v1",
        "sourceDirectory": root.path,
        "sourceReadOnly": true,
        "rawDecoder": "CIRAWFilter only",
        "algorithmVersion": CoreImageRAW.algorithmVersion,
        "calibrationVersion": CoreImageRAW.calibrationVersion,
        "maximumAnalysisSize": maximum,
        "sampleCount": rows.count,
        "rawFileUnchangedCount": rows.filter { $0["rawFileUnchanged"] as? Bool == true }.count,
        "pairValidatedCount": rows.filter {
            ($0["pairValidation"] as? [String: Any])?["validated"] as? Bool == true
        }.count,
        "rows": rows,
        "productionEligible": false,
        "claimBoundary": "formal CoreImageRAW offline evidence only; no Apple producer-exact or Photos acceptance claim",
    ]
    try jsonData(summary).write(to: output.appendingPathComponent("formal_batch_summary.json"), options: .atomic)
    print(String(data: try jsonData([
        "sampleCount": rows.count,
        "rawFileUnchangedCount": summary["rawFileUnchangedCount"]!,
        "pairValidatedCount": summary["pairValidatedCount"]!,
        "output": output.path,
    ]), encoding: .utf8)!)
}

do {
    try main()
} catch {
    FileHandle.standardError.write(Data(("\(error)\n").utf8))
    exit(1)
}
