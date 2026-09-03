import Foundation

/// The macOS app is a presentation shell around the canonical Rust product.
///
/// This client deliberately owns only process transport and translation of
/// user-facing product intents. It must not recreate conversion, metadata, or
/// validation policy in Swift. The Rust executable is injected for tests and
/// deployments through `XDREMUX_CLI`; a bundled executable is preferred for a
/// shipped app.
enum RustCLIClient {
    enum ClientError: Error, CustomStringConvertible {
        case executableNotFound([URL])
        case unsupportedConfiguration(String)
        case launchFailed(String)
        case commandFailed(command: String, status: Int32, message: String)

        var description: String {
            switch self {
            case .executableNotFound(let candidates):
                let searched = candidates.map(\.path).joined(separator: ", ")
                return "Rust CLI 不可用：未找到 xdremux。已检查 \(searched)。"
            case .unsupportedConfiguration(let message):
                return "当前设置无法由 Rust CLI 执行：\(message)"
            case .launchFailed(let message):
                return "无法启动 Rust CLI：\(message)"
            case .commandFailed(_, let status, let message):
                if message.isEmpty {
                    return "Rust CLI 执行失败（退出码 \(status)）。"
                }
                return "Rust CLI 执行失败（退出码 \(status)）：\(message)"
            }
        }
    }

    private struct CommandResult {
        let status: Int32
        let standardOutput: String
        let standardError: String
    }

    private struct CategorizeReceipt: Decodable {
        let items: [CategorizeItem]
    }

    private struct CategorizeItem: Decodable {
        let source: String
        let destination: String
        let disposition: String
        let classification: ClassificationContract
        let error: String?
    }

    private struct ClassificationContract: Decodable {
        let assetType: String
        let primaryCaptureMode: String?
        let metadataStatus: String
        let unknownFlags: UInt64

        enum CodingKeys: String, CodingKey {
            case assetType = "asset_type"
            case primaryCaptureMode = "primary_capture_mode"
            case metadataStatus = "metadata_status"
            case unknownFlags = "unknown_flags"
        }
    }

    static func convert(
        inputURL: URL,
        outputURL: URL,
        config: ConversionConfig
    ) throws {
        var arguments = [
            "convert",
            "--input", inputURL.path,
            "--output", outputURL.path,
        ]
        arguments.append(contentsOf: try productArguments(for: config))
        _ = try run(arguments)
    }

    static func isValidOutput(_ outputURL: URL) -> Bool {
        guard FileManager.default.fileExists(atPath: outputURL.path) else { return false }
        do {
            _ = try run(["validate", outputURL.path])
            return true
        } catch {
            return false
        }
    }

    static func classify(at inputURL: URL) throws -> PhotoClassification {
        let items = try classify(inputs: [inputURL])
        guard let item = items.first(where: { $0.sourceURL.standardizedFileURL == inputURL.standardizedFileURL })
        else {
            throw ClientError.launchFailed("Rust CLI 没有返回输入文件的分类结果。")
        }
        return item.classification
    }

    static func classify(inputs: [URL]) throws -> [PhotoCategorizationItem] {
        var grouped: [String: [URL]] = [:]
        for input in inputs {
            let parent = input.deletingLastPathComponent().standardizedFileURL
            grouped[parent.path, default: []].append(input)
        }
        var items: [PhotoCategorizationItem] = []
        for key in grouped.keys.sorted() {
            guard let group = grouped[key] else { continue }
            items.append(contentsOf: try categorize(
                inputs: group,
                outputDirectory: URL(fileURLWithPath: key),
                dryRun: true
            ))
        }
        var seen = Set<String>()
        return items.filter { seen.insert($0.id).inserted }
    }

    static func categorize(
        inputs: [URL],
        outputDirectory: URL,
        dryRun: Bool
    ) throws -> [PhotoCategorizationItem] {
        guard !inputs.isEmpty else { return [] }
        var arguments = ["categorize"]
        for input in inputs {
            arguments.append(contentsOf: ["--input", input.path])
        }
        arguments.append(contentsOf: ["--output-dir", outputDirectory.path, "--json"])
        if dryRun {
            arguments.append("--dry-run")
        }

        let result = try run(arguments, allowNonZeroOutput: true)
        guard let data = result.standardOutput.data(using: .utf8) else {
            throw ClientError.launchFailed("Rust CLI 分类输出不是 UTF-8。")
        }
        let receipt: CategorizeReceipt
        do {
            receipt = try JSONDecoder().decode(CategorizeReceipt.self, from: data)
        } catch {
            let detail = result.standardError.trimmingCharacters(in: .whitespacesAndNewlines)
            throw ClientError.launchFailed(
                detail.isEmpty ? "无法解析 Rust CLI 分类回执：\(error)" : detail
            )
        }
        return try receipt.items.map(makeCategorizationItem(from:))
    }

    static func productArgumentsForTesting(config: ConversionConfig) throws -> [String] {
        try productArguments(for: config)
    }

    private static func productArguments(for config: ConversionConfig) throws -> [String] {
        if config.applePhotographicStyles && config.applePortrait {
            throw ClientError.unsupportedConfiguration(
                "Portrait 和 Photographic Styles 目前必须分别执行。"
            )
        }
        if config.oppoGalleryCompatibilityEnabled && config.appleFeaturesEnabled {
            throw ClientError.unsupportedConfiguration(
                "OPPO 相册兼容格式不能与 Apple 功能同时使用。"
            )
        }

        if config.applePhotographicStyles {
            return ["--apple-styles"]
        }
        if config.applePortrait {
            return ["--apple-portrait"]
        }
        if config.oppoGalleryCompatibilityEnabled {
            return ["--oppo-compatible"]
        }
        return []
    }

    private static func run(
        _ arguments: [String],
        allowNonZeroOutput: Bool = false
    ) throws -> CommandResult {
        let executable = try resolveExecutable()
        let process = Process()
        let stdout = Pipe()
        let stderr = Pipe()
        process.executableURL = executable
        process.arguments = arguments
        process.standardOutput = stdout
        process.standardError = stderr

        var environment = ProcessInfo.processInfo.environment
        if environment["XDREMUX_APPLE_ADAPTER"] == nil,
           let adapter = resolveBundledAdapter() {
            environment["XDREMUX_APPLE_ADAPTER"] = adapter.path
        }
        process.environment = environment

        do {
            try process.run()
        } catch {
            throw ClientError.launchFailed(error.localizedDescription)
        }
        process.waitUntilExit()

        let standardOutput = String(
            data: stdout.fileHandleForReading.readDataToEndOfFile(),
            encoding: .utf8
        ) ?? ""
        let standardError = String(
            data: stderr.fileHandleForReading.readDataToEndOfFile(),
            encoding: .utf8
        ) ?? ""
        let result = CommandResult(
            status: process.terminationStatus,
            standardOutput: standardOutput,
            standardError: standardError
        )
        guard result.status == 0 || allowNonZeroOutput else {
            let message = result.standardError
                .trimmingCharacters(in: .whitespacesAndNewlines)
            throw ClientError.commandFailed(
                command: arguments.joined(separator: " "),
                status: result.status,
                message: message
            )
        }
        return result
    }

    private static func resolveExecutable() throws -> URL {
        var candidates: [URL] = []
        if let override = ProcessInfo.processInfo.environment["XDREMUX_CLI"], !override.isEmpty {
            candidates.append(URL(fileURLWithPath: override))
        }

        if let bundled = Bundle.main.url(forResource: "xdremux", withExtension: nil) {
            candidates.append(bundled)
        }
        if let executable = Bundle.main.executableURL {
            let contents = executable.deletingLastPathComponent()
            candidates.append(contentsOf: [
                contents.appendingPathComponent("xdremux"),
                contents.appendingPathComponent("../Helpers/xdremux").standardized,
                contents.appendingPathComponent("../Resources/xdremux").standardized,
            ])
        }

        let workingDirectory = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
        candidates.append(contentsOf: [
            workingDirectory.appendingPathComponent("target/debug/xdremux"),
            workingDirectory.appendingPathComponent("target/release/xdremux"),
        ])

        let pathEntries = (ProcessInfo.processInfo.environment["PATH"] ?? "")
            .split(separator: ":")
            .map(String.init)
        candidates.append(contentsOf: pathEntries.map { URL(fileURLWithPath: $0).appendingPathComponent("xdremux") })
        candidates.append(contentsOf: [
            URL(fileURLWithPath: "/opt/homebrew/bin/xdremux"),
            URL(fileURLWithPath: "/usr/local/bin/xdremux"),
        ])

        var seen = Set<String>()
        let unique = candidates.filter { seen.insert($0.standardizedFileURL.path).inserted }
        if let executable = unique.first(where: { FileManager.default.isExecutableFile(atPath: $0.path) }) {
            return executable
        }
        throw ClientError.executableNotFound(unique)
    }

    private static func makeCategorizationItem(
        from item: CategorizeItem
    ) throws -> PhotoCategorizationItem {
        guard let assetType = PhotoAssetType(rawValue: item.classification.assetType) else {
            throw ClientError.launchFailed(
                "Rust CLI 返回了未知资源类型：\(item.classification.assetType)"
            )
        }
        let mode = item.classification.primaryCaptureMode.flatMap(OppoCaptureMode.init(rawValue:))
        let status: OppoPhotoClassificationStatus
        switch item.classification.metadataStatus {
        case "missing-user-comment":
            status = .missingUserComment
        case "malformed-user-comment":
            status = .malformedUserComment
        case "unreadable-image":
            status = .unreadableImage
        case "ok":
            status = mode != nil || item.classification.unknownFlags == 0
                ? .categorized
                : .unknownFlags
        default:
            throw ClientError.launchFailed(
                "Rust CLI 返回了未知 metadata 状态：\(item.classification.metadataStatus)"
            )
        }
        guard let disposition = PhotoCategorizationDisposition(rawValue: item.disposition) else {
            throw ClientError.launchFailed(
                "Rust CLI 返回了未知分类结果：\(item.disposition)"
            )
        }
        return PhotoCategorizationItem(
            sourceURL: URL(fileURLWithPath: item.source),
            destinationURL: URL(fileURLWithPath: item.destination),
            classification: PhotoClassification(
                assetType: assetType,
                mode: mode,
                status: status
            ),
            disposition: disposition,
            errorDescription: item.error
        )
    }

    private static func resolveBundledAdapter() -> URL? {
        if let override = ProcessInfo.processInfo.environment["XDREMUX_APPLE_ADAPTER"],
           FileManager.default.isExecutableFile(atPath: override) {
            return URL(fileURLWithPath: override)
        }
        if let resource = Bundle.main.url(forResource: "xdremux-apple-adapter", withExtension: nil),
           FileManager.default.isExecutableFile(atPath: resource.path) {
            return resource
        }
        guard let executable = Bundle.main.executableURL else { return nil }
        let contents = executable.deletingLastPathComponent()
        let candidates = [
            contents.appendingPathComponent("xdremux-apple-adapter"),
            contents.appendingPathComponent("../Helpers/xdremux-apple-adapter").standardized,
            contents.appendingPathComponent("../Resources/xdremux-apple-adapter").standardized,
        ]
        return candidates.first(where: { FileManager.default.isExecutableFile(atPath: $0.path) })
    }
}
