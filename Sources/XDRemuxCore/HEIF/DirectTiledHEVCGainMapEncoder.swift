import Foundation
import CoreGraphics
import ImageIO
import UniformTypeIdentifiers

package struct DirectTiledHEVCGainMap: Sendable {
    package let width: Int
    package let height: Int
    package let tileWidth: Int
    package let tileHeight: Int
    package let tilePayloads: [Data]
    package let tileSizes: [(width: Int, height: Int)]
    package let hvcC: Data
    package let channelCount: Int
}

package enum DirectTiledHEVCGainMapEncoder {
    private struct ProcessResult {
        let status: Int32
        let stdout: Data
        let stderr: Data
    }

    private static let compileLock = NSLock()

    package static func helperExecutable() throws -> URL {
        try encoderExecutable()
    }

    package static func encode(
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
        // The input is UUID-scoped scratch consumed immediately by the helper; durability and
        // atomic replacement are not part of this internal transport contract.
        try imageData.write(to: inputURL)
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
        // UUID-scoped scratch has no durability contract; a direct write avoids an otherwise
        // redundant safe-save/rename before the helper maps the bytes read-only.
        try raster.data.write(to: rawURL)
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
            throw CLIError.invalidContainer("private tile encoder failed: \(diagnostic)")
        }
        let tilePayloads = try idrTilePayloads(
            from: Data(contentsOf: annexBURL, options: [.mappedIfSafe])
        )
        let columns = (width + tileSize - 1) / tileSize
        let rows = (height + tileSize - 1) / tileSize
        guard tilePayloads.count == rows * columns else {
            throw CLIError.invalidContainer(
                "private tile encoder returned \(tilePayloads.count) samples; expected \(rows * columns)"
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

    private static func idrTilePayloads(from annexB: Data) throws -> [Data] {
        var starts: [(offset: Int, length: Int)] = []
        starts.reserveCapacity(64)
        var index = annexB.startIndex
        while index + 3 < annexB.endIndex {
            if annexB[index] == 0, annexB[index + 1] == 0,
               annexB[index + 2] == 0, annexB[index + 3] == 1 {
                starts.append((index, 4))
                index += 4
            } else if annexB[index] == 0, annexB[index + 1] == 0, annexB[index + 2] == 1 {
                starts.append((index, 3))
                index += 3
            } else {
                index += 1
            }
        }
        var payloads: [Data] = []
        payloads.reserveCapacity(starts.count)
        for position in starts.indices {
            let start = starts[position].offset + starts[position].length
            let end = position + 1 < starts.count ? starts[position + 1].offset : annexB.endIndex
            guard start < end else { continue }
            let type = (annexB[start] >> 1) & 0x3f
            guard type == 19 || type == 20 else { continue }
            var payload = Data()
            payload.reserveCapacity(4 + end - start)
            appendUInt32BE(end - start, to: &payload)
            payload.append(contentsOf: annexB[start..<end])
            payloads.append(payload)
        }
        guard !payloads.isEmpty else {
            throw CLIError.invalidContainer("private tile encoder emitted no HEVC IDR samples")
        }
        return payloads
    }

    private static func siblingURL(for base: URL, label: String, pathExtension: String) -> URL {
        base.deletingLastPathComponent().appendingPathComponent(
            ".xdremux-\(label)-\(UUID().uuidString).\(pathExtension)"
        )
    }

    private static func encoderExecutable() throws -> URL {
        let source = try resourceURL(name: "apple_vt_hevc_encoder.swift")
        let sourceData = try Data(contentsOf: source, options: [.mappedIfSafe])
        var cacheIdentity = sourceData
        cacheIdentity.append(contentsOf: "swiftc\u{0}-O\u{0}<source>".utf8)
        let sourceHash = sha256Hex(cacheIdentity)
        guard let caches = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask).first else {
            throw CLIError.invalidContainer("cannot resolve the user cache directory")
        }
        let directory = caches
            .appendingPathComponent("com.proxdr.XDRemux", isDirectory: true)
            .appendingPathComponent("AppleNativeTools", isDirectory: true)
            .appendingPathComponent(sourceHash, isDirectory: true)
        let executable = directory.appendingPathComponent("apple-vt-hevc-encoder")

        compileLock.lock()
        defer { compileLock.unlock() }
        if FileManager.default.isExecutableFile(atPath: executable.path) {
            return executable
        }
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let result = try run(
            URL(fileURLWithPath: "/usr/bin/xcrun"),
            arguments: ["swiftc", "-O", source.path, "-o", executable.path]
        )
        guard result.status == 0, FileManager.default.isExecutableFile(atPath: executable.path) else {
            let diagnostic = String(data: result.stderr, encoding: .utf8) ?? "unknown compiler error"
            throw CLIError.invalidContainer(
                "cannot build apple-vt-hevc-encoder: "
                    + diagnostic.trimmingCharacters(in: .whitespacesAndNewlines)
            )
        }
        return executable
    }

    private static func resourceURL(name: String) throws -> URL {
        let fileManager = FileManager.default
        var candidates: [URL] = []
        if let override = ProcessInfo.processInfo.environment["XDREMUX_CORE_NATIVE_ROOT"] {
            candidates.append(URL(fileURLWithPath: override, isDirectory: true).appendingPathComponent(name))
        }
        if let override = ProcessInfo.processInfo.environment["XDREMUX_APPLE_PLATFORM_ROOT"] {
            candidates.append(
                URL(fileURLWithPath: override, isDirectory: true)
                    .appendingPathComponent("Native", isDirectory: true)
                    .appendingPathComponent(name)
            )
        }
        if let resources = Bundle.module.resourceURL {
            candidates.append(
                resources.appendingPathComponent("Native", isDirectory: true).appendingPathComponent(name)
            )
        }
        if let resources = Bundle.main.resourceURL {
            candidates.append(
                resources.appendingPathComponent("Native", isDirectory: true).appendingPathComponent(name)
            )
        }
        if let match = candidates.first(where: { fileManager.fileExists(atPath: $0.path) }) {
            return match
        }
        throw CLIError.invalidContainer("missing XDRemux native Gain Map encoder resource \(name)")
    }

    private static func run(_ executableURL: URL, arguments: [String]) throws -> ProcessResult {
        let process = Process()
        process.executableURL = executableURL
        process.arguments = arguments
        let output = Pipe()
        let errors = Pipe()
        process.standardOutput = output
        process.standardError = errors
        do {
            try process.run()
        } catch {
            throw CLIError.invalidContainer(
                "cannot launch Gain Map helper \(executableURL.lastPathComponent): \(error)"
            )
        }
        let stdout = output.fileHandleForReading.readDataToEndOfFile()
        let stderr = errors.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()
        return ProcessResult(status: process.terminationStatus, stdout: stdout, stderr: stderr)
    }
}
