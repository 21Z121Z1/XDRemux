import Foundation
import CryptoKit
import XDRemuxCore
import XDRemuxAppleFeatures

/// Runtime behavior for parsed CLI commands. Argument declaration, command
/// routing, validation command options, and generated help live in
/// `XDRemuxRootCommand`; this type owns execution only.
enum XDRemuxCommand {
    private static let fileManager = FileManager.default

    static func runAppleValidation(
        inputURL: URL,
        reportURL: URL?,
        expectsPortrait: Bool
    ) throws {
        let report = try AppleFeatureConversionEngine.validationReport(
            for: inputURL.standardizedFileURL,
            expectsPortrait: expectsPortrait
        )
        let data = try JSONSerialization.data(
            withJSONObject: report,
            options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        )
        if let reportURL {
            let standardizedReportURL = reportURL.standardizedFileURL
            try ensureDirectory(
                standardizedReportURL.deletingLastPathComponent(),
                fileManager: fileManager
            )
            try data.write(to: standardizedReportURL, options: .atomic)
        }
        FileHandle.standardOutput.write(data)
        FileHandle.standardOutput.write(Data("\n".utf8))
    }

    static func runPortraitValidation(
        inputURL: URL,
        reportURL: URL?
    ) throws {
        let report = try AppleFeatureConversionEngine.portraitValidationReport(
            for: inputURL.standardizedFileURL
        )
        let data = try JSONSerialization.data(
            withJSONObject: report,
            options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        )
        if let reportURL {
            let standardizedReportURL = reportURL.standardizedFileURL
            try ensureDirectory(
                standardizedReportURL.deletingLastPathComponent(),
                fileManager: fileManager
            )
            try data.write(to: standardizedReportURL, options: .atomic)
        }
        FileHandle.standardOutput.write(data)
        FileHandle.standardOutput.write(Data("\n".utf8))
    }

    static func runPortraitSelfTest() throws {
        let report = try AppleFeatureConversionEngine.portraitSelfTestReport()
        let data = try JSONSerialization.data(
            withJSONObject: report,
            options: [.prettyPrinted, .sortedKeys]
        )
        FileHandle.standardOutput.write(data)
        FileHandle.standardOutput.write(Data("\n".utf8))
    }

    static func runConvert(_ cmd: ConvertCommand) throws {
        try validateInputType(cmd.inputURL, appleFeatures: cmd.appleFeatures)
        if cmd.appleFeatures.photographicStyles {
            try AppleFeatureConversionEngine.convert(
                inputURL: cmd.inputURL,
                outputURL: cmd.outputURL,
                configuration: cmd.configuration
            )
            print("converted \(cmd.inputURL.lastPathComponent) -> \(cmd.outputURL.path) [\(cmd.appleFeatures.stableDescription)]")
            return
        }
        if cmd.appleFeatures.portrait {
            try AppleFeatureConversionEngine.convert(
                inputURL: cmd.inputURL,
                outputURL: cmd.outputURL,
                configuration: cmd.configuration
            )
            print("converted OPPO portrait \(cmd.inputURL.lastPathComponent) -> \(cmd.outputURL.path)")
            return
        }
        try ConversionEngine.convert(
            inputURL: cmd.inputURL,
            outputURL: cmd.outputURL,
            config: cmd.configuration
        )
        print("converted \(cmd.inputURL.lastPathComponent) -> \(cmd.outputURL.path)")
    }

    /// Reduces an error to the one line that belongs in a per-file list entry.
    private static func singleLine(_ error: Error) -> String {
        if let cli = error as? CLIError { return cli.headline }
        return String(describing: error)
            .split(whereSeparator: \.isNewline)
            .map { $0.trimmingCharacters(in: .whitespaces) }
            .filter { !$0.isEmpty }
            .joined(separator: " ")
    }

    static func runBatch(_ cmd: BatchCommand) throws {
        try ensureDirectory(cmd.outputDirURL, fileManager: fileManager)
        let discovered = try enumerateInputs(
            root: cmd.inputDirURL,
            glob: cmd.glob,
            excluding: cmd.outputDirURL,
            categorized: cmd.categorizeOutput
        )
        let matched = discovered
        guard !matched.isEmpty else {
            throw CLIError.noFilesMatched(cmd.inputDirURL, cmd.glob)
        }
        for inputURL in matched {
            try validateInputType(inputURL, appleFeatures: cmd.appleFeatures)
        }

        let jobs = max(1, cmd.jobs)
        let configHash = batchConfigHash(cmd)
        let checkpointURL = resolvedCheckpointURL(cmd: cmd, configHash: configHash)

        // Precompute outputs and fail fast on collisions.
        var reservedOutputPaths = Set<String>()
        let workItems = matched.map { inputURL -> BatchWorkItem in
            let stem = inputURL.deletingPathExtension().lastPathComponent
            let directory: URL
            if cmd.categorizeOutput,
               let folderName = PhotoCategorizationEngine.categorizedDirectory(for: inputURL) {
                directory = cmd.outputDirURL.appendingPathComponent(folderName, isDirectory: true)
            } else {
                directory = cmd.outputDirURL
            }
            var sequence = 1
            var outputURL = directory.appendingPathComponent("\(stem).heic")
            while reservedOutputPaths.contains(outputURL.standardizedFileURL.path) {
                sequence += 1
                outputURL = directory.appendingPathComponent("\(stem) (\(sequence)).heic")
            }
            reservedOutputPaths.insert(outputURL.standardizedFileURL.path)
            return BatchWorkItem(inputURL: inputURL, outputURL: outputURL)
        }
        try assertNoOutputCollisions(workItems)

        var checkpointState: [String: BatchCheckpointItem] = [:]
        if cmd.resume {
            checkpointState = try loadCheckpointStateIfPresent(url: checkpointURL, expectedConfigHash: configHash)
        } else {
            // Fresh run: truncate any existing checkpoint so future resumes are consistent.
            if fileManager.fileExists(atPath: checkpointURL.path) {
                do {
                    try fileManager.removeItem(at: checkpointURL)
                } catch {
                    // If removal fails, best-effort truncate.
                    if let handle = try? FileHandle(forWritingTo: checkpointURL) {
                        try? handle.truncate(atOffset: 0)
                        try? handle.close()
                    }
                }
            }
        }

        let checkpointWriter = try BatchCheckpointWriter(url: checkpointURL, fileManager: fileManager)
        defer {
            try? checkpointWriter.close()
        }
        try checkpointWriter.appendHeader(configHash: configHash, jobs: jobs)

        let logLock = NSLock()
        func log(_ message: String) {
            logLock.lock()
            defer { logLock.unlock() }
            print(message)
        }

        let statsLock = NSLock()
        var convertedCount = 0
        var skippedExistingCount = 0
        var failureCount = 0

        let queue = OperationQueue()
        queue.maxConcurrentOperationCount = jobs
        queue.qualityOfService = .userInitiated

        for item in workItems {
            queue.addOperation {
                autoreleasepool {
                    let inputKey = item.inputURL.standardizedFileURL.path
                    let outputKey = item.outputURL.standardizedFileURL.path
                    let signature = (try? fileSignature(for: item.inputURL, fileManager: fileManager))

                    func record(status: BatchCheckpointStatus, error: String? = nil) {
                        do {
                            try checkpointWriter.appendItem(
                                inputPath: inputKey,
                                outputPath: outputKey,
                                status: status,
                                inputSize: signature?.size,
                                inputMtimeNs: signature?.mtimeNs,
                                error: error
                            )
                        } catch {
                            // Checkpoint failure should be visible, but do not abort in-flight conversions.
                            log("checkpoint write failed: \(error)")
                        }
                    }

                    func isOutputValid() -> Bool {
                        guard fileManager.fileExists(atPath: item.outputURL.path) else { return false }
                        if cmd.appleFeatures.isEnabled {
                            if cmd.appleFeatures.photographicStyles {
                                return AppleFeatureConversionEngine.isValidOutput(
                                    item.outputURL,
                                    options: AppleFeatureOptions(
                                        photographicStyles: true,
                                        portrait: cmd.appleFeatures.portrait
                                            && AppleFeatureConversionEngine.isConvertiblePortraitInput(item.inputURL)
                                    )
                                )
                            }
                            return AppleFeatureConversionEngine.isValidOutput(
                                item.outputURL,
                                options: cmd.appleFeatures
                            )
                        }
                        return ConversionEngine.isValidOutput(
                            item.outputURL,
                            config: cmd.conversionConfiguration
                        )
                    }

                    // Resume: only treat checkpoint success/skipped as done. Failures always retry.
                    if cmd.resume, let prior = checkpointState[inputKey], prior.matchesSignature(signature) {
                        if (prior.status == .success || prior.status == .skippedExisting), prior.outputPath == outputKey {
                            if isOutputValid() {
                                statsLock.lock(); skippedExistingCount += 1; statsLock.unlock()
                                record(status: .skippedExisting)
                                log("skipped \(item.inputURL.lastPathComponent) (output already up to date)")
                                return
                            }
                        }

                        if prior.status == .failure {
                            // fallthrough: retry conversion even if an output exists.
                        }
                    }

                    // Skip-existing: filesystem-based fast path, unless resume explicitly says we must retry.
                    if cmd.skipExisting {
                        let prior = cmd.resume ? checkpointState[inputKey] : nil
                        let signatureMatchesCheckpoint = prior?.matchesSignature(signature) == true
                        let mustRetryFromCheckpoint = signatureMatchesCheckpoint && (prior?.status == .failure)
                        let inputChangedSinceCheckpoint = (prior != nil) && !signatureMatchesCheckpoint
                        if !mustRetryFromCheckpoint && !inputChangedSinceCheckpoint {
                            if isOutputValid() {
                                statsLock.lock(); skippedExistingCount += 1; statsLock.unlock()
                                record(status: .skippedExisting)
                                log("skipped \(item.inputURL.lastPathComponent) (output already up to date)")
                                return
                            }
                        }
                    }

                    // If we are going to write to a different output path and it exists, remove it to avoid ImageIO failures.
                    if item.outputURL.standardizedFileURL.path != item.inputURL.standardizedFileURL.path,
                       fileManager.fileExists(atPath: item.outputURL.path) {
                        try? fileManager.removeItem(at: item.outputURL)
                    }

                    do {
                        try ensureDirectory(item.outputURL.deletingLastPathComponent(), fileManager: fileManager)
                        if cmd.appleFeatures.photographicStyles {
                            try AppleFeatureConversionEngine.convert(
                                inputURL: item.inputURL,
                                outputURL: item.outputURL,
                                configuration: cmd.conversionConfiguration
                            )
                        } else if cmd.appleFeatures.portrait {
                            try AppleFeatureConversionEngine.convert(
                                inputURL: item.inputURL,
                                outputURL: item.outputURL,
                                configuration: cmd.conversionConfiguration
                            )
                        } else {
                            try ConversionEngine.convert(
                                inputURL: item.inputURL,
                                outputURL: item.outputURL,
                                config: cmd.conversionConfiguration
                            )
                        }
                        statsLock.lock(); convertedCount += 1; statsLock.unlock()
                        record(status: .success)
                        log("converted \(item.inputURL.lastPathComponent)")
                    } catch {
                        statsLock.lock(); failureCount += 1; statsLock.unlock()
                        record(status: .failure, error: String(describing: error))
                        // A batch listing stays one line per file; the richer
                        // multi-line form belongs to single-file `convert`.
                        log("failed \(item.inputURL.lastPathComponent): \(singleLine(error))")
                    }
                }
            }
        }

        queue.waitUntilAllOperationsAreFinished()
        try checkpointWriter.close()

        log(
            "batch complete: \(convertedCount) converted, \(skippedExistingCount) skipped, "
                + "\(failureCount) failed -> \(cmd.outputDirURL.path)"
        )
        if failureCount == 0 {
            try? fileManager.removeItem(at: checkpointURL)
        } else {
            log("run the same command again to retry only the \(failureCount) failed file(s)")
            throw CLIError.batchFailed(failures: failureCount, checkpoint: checkpointURL)
        }
    }

    static func runCategorize(_ cmd: CategorizeCommand) throws {
        let plan = try PhotoCategorizationEngine.makePlan(
            inputs: cmd.inputURLs,
            outputDirectory: cmd.outputDirURL,
            livePhotoPairValidator: { imageURL, videoURL in
                AppleLivePhotoValidator.isValidPair(imageURL: imageURL, videoURL: videoURL)
            },
            fileManager: fileManager
        )
        let result = PhotoCategorizationEngine.execute(
            plan,
            jobs: cmd.jobs,
            dryRun: cmd.dryRun,
            fileManager: fileManager
        )
        for item in result.items {
            let category = PhotoFolderProjection.relativeDirectory(for: item.classification)
            let detail = item.errorDescription.map { " error=\($0)" } ?? ""
            print("\(item.disposition.rawValue) [\(category)] \(item.sourceURL.path) -> \(item.destinationURL.path)\(detail)")
        }
        print(
            "categorize complete: \(result.categorizedCount) categorized, "
                + "\(result.unclassifiedCount) unclassified, \(result.copiedCount) copied, "
                + "\(result.dryRunCount) dry-run, \(result.duplicateCount) duplicate, "
                + "\(result.issueCount) failed"
        )
        if result.issueCount > 0 {
            throw CLIError.categorizationFailed(failures: result.issueCount)
        }
    }

    private static func validateInputType(
        _ inputURL: URL,
        appleFeatures: AppleFeatureOptions
    ) throws {
        let pathExtension = inputURL.pathExtension.lowercased()
        guard pathExtension == "jpg" || pathExtension == "jpeg" else { return }
        guard appleFeatures.portrait else {
            throw CLIError.invalidValue(
                option: "--input",
                value: "JPEG input is supported only with --apple-portrait"
            )
        }
        guard AppleFeatureConversionEngine.isConvertiblePortraitInput(inputURL) else {
            throw CLIError.invalidContainer(
                "JPEG Apple Portrait input requires src.image + rear.depth + "
                    + "rear.depth.config and an ImageIO-readable src.image Gain Map"
            )
        }
    }

    private struct BatchWorkItem {
        let inputURL: URL
        let outputURL: URL
    }

    private enum BatchCheckpointStatus: String {
        case success = "success"
        case failure = "failure"
        case skippedExisting = "skipped_existing"
    }

    private struct BatchCheckpointItem {
        let inputPath: String
        let outputPath: String
        let status: BatchCheckpointStatus
        let inputSize: Int64?
        let inputMtimeNs: Int64?

        func matchesSignature(_ signature: FileSignature?) -> Bool {
            guard let signature else { return true }
            if let inputSize, inputSize != signature.size { return false }
            if let inputMtimeNs, inputMtimeNs != signature.mtimeNs { return false }
            return true
        }

        func isDone(for expectedOutputPath: String, signature: FileSignature?) -> Bool {
            guard status == .success || status == .skippedExisting else { return false }
            guard outputPath == expectedOutputPath else { return false }
            return matchesSignature(signature)
        }
    }

    private struct FileSignature {
        let size: Int64
        let mtimeNs: Int64
    }

    private static func fileSignature(for url: URL, fileManager: FileManager) throws -> FileSignature {
        let attrs = try fileManager.attributesOfItem(atPath: url.path)
        let sizeValue = (attrs[.size] as? NSNumber)?.int64Value ?? 0
        let mtime = (attrs[.modificationDate] as? Date)?.timeIntervalSince1970 ?? 0
        let mtimeNs = Int64(mtime * 1_000_000_000)
        return FileSignature(size: sizeValue, mtimeNs: mtimeNs)
    }

    private final class BatchCheckpointWriter {
        private let url: URL
        private let queue = DispatchQueue(label: "xdremux.checkpoint")
        private var fileHandle: FileHandle?
        private var isClosed = false

        init(url: URL, fileManager: FileManager) throws {
            self.url = url
            let parent = url.deletingLastPathComponent()
            try ensureDirectory(parent, fileManager: fileManager)
            if !fileManager.fileExists(atPath: url.path) {
                let ok = fileManager.createFile(atPath: url.path, contents: nil)
                guard ok else { throw CLIError.unableToWriteCheckpoint(url) }
            }
            do {
                let handle = try FileHandle(forWritingTo: url)
                try handle.seekToEnd()
                self.fileHandle = handle
            } catch {
                throw CLIError.unableToWriteCheckpoint(url)
            }
        }

        func appendHeader(configHash: String, jobs: Int) throws {
            let record: [String: Any] = [
                "kind": "header",
                "schema": 1,
                "configHash": configHash,
                "jobs": jobs,
                "startedAtMs": Int64(Date().timeIntervalSince1970 * 1000)
            ]
            try appendJSONLine(record)
        }

        func appendItem(
            inputPath: String,
            outputPath: String,
            status: BatchCheckpointStatus,
            inputSize: Int64?,
            inputMtimeNs: Int64?,
            error: String?
        ) throws {
            var record: [String: Any] = [
                "kind": "item",
                "schema": 1,
                "inputPath": inputPath,
                "outputPath": outputPath,
                "status": status.rawValue,
                "finishedAtMs": Int64(Date().timeIntervalSince1970 * 1000)
            ]
            if let inputSize { record["inputSize"] = inputSize }
            if let inputMtimeNs { record["inputMtimeNs"] = inputMtimeNs }
            if let error { record["error"] = error }
            try appendJSONLine(record)
        }

        func close() throws {
            var thrown: Error?
            queue.sync {
                if isClosed { return }
                isClosed = true
                do {
                    try fileHandle?.close()
                } catch {
                    thrown = error
                }
                fileHandle = nil
            }
            if let thrown {
                throw thrown
            }
        }

        private func appendJSONLine(_ record: [String: Any]) throws {
            let data: Data
            do {
                data = try JSONSerialization.data(withJSONObject: record, options: [])
            } catch {
                throw CLIError.unableToWriteCheckpoint(url)
            }
            var line = data
            line.append(UInt8(ascii: "\n"))

            var thrown: Error?
            queue.sync {
                guard !isClosed, let fileHandle else {
                    thrown = CLIError.unableToWriteCheckpoint(url)
                    return
                }
                do {
                    try fileHandle.write(contentsOf: line)
                    try? fileHandle.synchronize()
                } catch {
                    thrown = CLIError.unableToWriteCheckpoint(url)
                }
            }
            if let thrown {
                throw thrown
            }
        }
    }

    private static func loadCheckpointStateIfPresent(
        url: URL,
        expectedConfigHash: String
    ) throws -> [String: BatchCheckpointItem] {
        guard fileManager.fileExists(atPath: url.path) else { return [:] }
        let data: Data
        do {
            data = try Data(contentsOf: url, options: [.mappedIfSafe])
        } catch {
            throw CLIError.unableToReadCheckpoint(url)
        }
        guard let text = String(data: data, encoding: .utf8) else {
            throw CLIError.unableToReadCheckpoint(url)
        }

        var items: [String: BatchCheckpointItem] = [:]
        var sawHeader = false

        for rawLine in text.split(whereSeparator: { $0.isNewline }) {
            let line = String(rawLine).trimmingCharacters(in: .whitespacesAndNewlines)
            if line.isEmpty { continue }
            guard let lineData = line.data(using: .utf8) else { continue }

            let obj: Any
            do {
                obj = try JSONSerialization.jsonObject(with: lineData, options: [])
            } catch {
                // Tolerate a partially-written trailing line after interruption.
                continue
            }
            guard let dict = obj as? [String: Any] else {
                continue
            }
            let kind = dict["kind"] as? String
            if kind == "header" {
                sawHeader = true
                let actual = dict["configHash"] as? String ?? "missing"
                if actual != expectedConfigHash {
                    throw CLIError.checkpointConfigMismatch(
                        url,
                        expected: expectedConfigHash,
                        actual: actual
                    )
                }
                continue
            }
            guard kind == "item" else { continue }

            let inputPath = dict["inputPath"] as? String ?? ""
            if inputPath.isEmpty { continue }
            let outputPath = dict["outputPath"] as? String ?? ""
            let statusRaw = dict["status"] as? String ?? ""
            let status = BatchCheckpointStatus(rawValue: statusRaw) ?? .failure

            let inputSize = (dict["inputSize"] as? NSNumber)?.int64Value
            let inputMtimeNs = (dict["inputMtimeNs"] as? NSNumber)?.int64Value
            items[inputPath] = BatchCheckpointItem(
                inputPath: inputPath,
                outputPath: outputPath,
                status: status,
                inputSize: inputSize,
                inputMtimeNs: inputMtimeNs
            )
        }

        guard sawHeader else {
            throw CLIError.invalidCheckpoint(url, "missing header")
        }
        return items
    }

    private static func batchConfigHash(_ cmd: BatchCommand) -> String {
        let entries: [(String, String)] = [
            ("family", cmd.family.rawValue),
            ("inputDir", cmd.inputDirURL.standardizedFileURL.path),
            ("inputProcessing", cmd.inputProcessingBranch.rawValue),
            ("oppoCameraTail", cmd.oppoCameraTail.rawValue),
            ("oppoCompat", cmd.oppoCompatibility.rawValue),
            ("appleFeatures", cmd.appleFeatures.stableDescription),
            ("appleStyleDataProducer", cmd.appleStyleDataProducer.rawValue),
            ("tmapFormat", cmd.tmapFormat.rawValue),
            ("outputDir", cmd.outputDirURL.standardizedFileURL.path),
            ("categorizeOutput", cmd.categorizeOutput ? "true" : "false"),
            ("categorizationLayout", cmd.categorizeOutput ? PhotoFolderProjection.layoutVersion : "off")
        ]
        let stable = entries.sorted(by: { $0.0 < $1.0 })
            .map { "\($0.0)=\($0.1)" }
            .joined(separator: "\n")
        return sha256Hex(Data(stable.utf8))
    }

    private static func resolvedCheckpointURL(
        cmd: BatchCommand,
        configHash: String
    ) -> URL {
        if let checkpointURL = cmd.checkpointURL {
            return checkpointURL
        }
        let short = String(configHash.prefix(16))
        return cmd.outputDirURL.appendingPathComponent(".xdremux-batch.\(short).jsonl")
    }

    private static func assertNoOutputCollisions(_ items: [BatchWorkItem]) throws {
        var seen: [String: URL] = [:]
        for item in items {
            let key = item.outputURL.standardizedFileURL.path
            if let prior = seen[key] {
                throw CLIError.outputPathCollision(
                    output: item.outputURL,
                    firstInput: prior,
                    secondInput: item.inputURL
                )
            }
            seen[key] = item.inputURL
        }
    }

    static func enumerateInputs(
        root: URL,
        glob: String,
        excluding outputDirectory: URL?,
        categorized: Bool
    ) throws -> [URL] {
        guard fileManager.fileExists(atPath: root.path) else {
            throw CLIError.inputNotFound(root)
        }

        let regex = try globToRegex(glob)
        guard let enumerator = fileManager.enumerator(
            at: root,
            includingPropertiesForKeys: [.isRegularFileKey, .isDirectoryKey],
            options: [.skipsHiddenFiles]
        ) else {
            throw CLIError.inputNotFound(root)
        }

        let rootPath = root.standardizedFileURL.path
        let outputPath = outputDirectory?.standardizedFileURL.path
        // Only skip the output tree when it is nested *inside* the scanned root.
        // When it is the root itself (in-place batch) skipping it would discard
        // every input.
        let excludedTree = outputPath.flatMap {
            $0 != rootPath && $0.hasPrefix(rootPath + "/") ? $0 : nil
        }
        // A categorized run files its results under per-capture-mode folders of
        // the output directory. Those folders only ever hold XDRemux output, so
        // skipping them keeps a repeated batch over the same directory
        // idempotent instead of re-converting yesterday's results.
        let categorizedParent = categorized ? (outputPath ?? rootPath) : nil
        let categorizedRootFolders = PhotoFolderProjection.rootFolderNames
            .union(Set(OppoCaptureMode.allCases.map(\.folderName)))

        var matched: [URL] = []
        for case let fileURL as URL in enumerator {
            let path = fileURL.standardizedFileURL.path
            let values = try fileURL.resourceValues(forKeys: [.isRegularFileKey, .isDirectoryKey])
            let isDirectory = values.isDirectory == true

            if let excludedTree, path == excludedTree || path.hasPrefix(excludedTree + "/") {
                if isDirectory { enumerator.skipDescendants() }
                continue
            }
            if isDirectory, let categorizedParent,
               fileURL.deletingLastPathComponent().standardizedFileURL.path == categorizedParent,
               categorizedRootFolders.contains(fileURL.lastPathComponent) {
                enumerator.skipDescendants()
                continue
            }
            guard values.isRegularFile == true else { continue }

            let relative = fileURL.path.replacingOccurrences(of: root.path + "/", with: "")
            let filename = fileURL.lastPathComponent
            if regex.firstMatch(
                in: relative,
                options: [],
                range: NSRange(relative.startIndex..., in: relative)
            ) != nil || regex.firstMatch(
                in: filename,
                options: [],
                range: NSRange(filename.startIndex..., in: filename)
            ) != nil {
                matched.append(fileURL)
            }
        }
        return matched.sorted { $0.path < $1.path }
    }

    private static func globToRegex(_ glob: String) throws -> NSRegularExpression {
        var pattern = "^"
        for scalar in glob.unicodeScalars {
            switch scalar {
            case "*":
                pattern += ".*"
            case "?":
                pattern += "."
            case ".", "(", ")", "[", "]", "{", "}", "+", "^", "$", "|", "\\":
                pattern += "\\\(scalar)"
            default:
                pattern.append(Character(scalar))
            }
        }
        pattern += "$"
        return try NSRegularExpression(pattern: pattern, options: [.caseInsensitive])
    }
}
