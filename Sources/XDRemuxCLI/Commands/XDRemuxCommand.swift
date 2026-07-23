import Foundation
import CryptoKit
import XDRemuxCore
import XDRemuxAppleFeatures

enum XDRemuxCommand {
    private static let fileManager = FileManager.default
    private static let usage = """
    Usage:
         XDRemux.swift convert --input <file.heic|portrait.jpg> [--output <out.heic>] [--apple-photographic-styles] [--apple-styles-raw-dng <file.dng>] [--apple-style-data-producer constrained-solver|learn-node|identity-fallback] [--apple-portrait] [--oppo-compatible] [--discard-portrait-data] [--debug-dir <dir>]
         XDRemux.swift batch --input-dir <dir> [--output-dir <dir>] [--glob *.heic] [--jobs <n>] [--apple-photographic-styles] [--apple-styles-raw-dng <file.dng>] [--apple-style-data-producer constrained-solver|learn-node|identity-fallback] [--apple-portrait] [--oppo-compatible] [--discard-portrait-data] [--checkpoint <file>] [--resume|--no-resume] [--skip-existing|--no-skip-existing] [--debug-dir <dir>]
             XDRemux.swift validate-apple --input <file.heic> [--expect-portrait] [--json <report.json>]
             XDRemux.swift validate-portrait --input <file.heic> [--json <report.json>]
             XDRemux.swift portrait-self-test

    Notes:
      - Product output always uses the metadata-preserving source-primary remux path.
      - With neither product switch, output is standard ISO HDR and preserves the non-HDR metadata tail.
        Gain Maps retain their source channel structure and may use HEVC Range Extensions 4:4:4.
      - --oppo-compatible converts a high-spec Gain Map to OPPO-compatible Main Still Picture 4:2:0.
      - --no-oppo-compat is a legacy spelling for the default standard-ISO mode.
      - Existing 4:2:0 Gain Maps cannot be promoted to high-spec 4:4:4 because the discarded chroma is unrecoverable.
      - Source UserComment routing flags are preserved. Default output physically removes private HDR tail entries
        while retaining watermark, master-mode, portrait, depth, source-image, edit, live-photo, and unknown entries.
      - --oppo-compatible preserves the complete OPPO/QTI/FileExtendedContainer tail.
      - --discard-portrait-data removes large depth/re-edit resources without reintroducing private HDR tail entries.
      - Only the active Gain Map graph and its required container descriptions may change.
      - Batch defaults: --jobs min(cpu,4), --resume, --skip-existing.
      - A JSONL checkpoint is written under output-dir by default; it is deleted only when the batch finishes with zero failures.
      - --apple-photographic-styles enables donor-free Apple Photographic Styles generation.
        --apple-styles is accepted as a short alias; manifests and documentation use the canonical name.
      - --apple-styles-raw-dng supplies one matching OPPO RAW MAX DNG. Its embedded PreviewImage is
        paired against the input before use; source mismatch, MPF Gain Map extraction, or geometry
        failures are fatal instead of silently reusing an unrelated or differently oriented proxy.
        The current final-HEIC scene path is a research candidate: Apple Camera's capture-time
        pre-LTM Linear Thumbnail input is unavailable, so its manifest remains production-ineligible
        until a held-out response-equivalent proxy and real Photos persistence both pass.
        constrained-solver is the default key-1 producer and measures a bounded full-Neutrino neutral
        reconstruction; it fails instead of falling back when it cannot improve identity. A writer-time
        key-1 pass is necessary but never claims full Photos production acceptance. identity-fallback
        writes complete identity only when selected explicitly and records that no scene matching was
        performed. learn-node is an explicit diagnostic Base-to-Base
        near-identity control; it is not a scene-matched linear/HDR-to-current-render producer.
        Styles-only semantics follow native role tiers: sky-only without a credible person,
        or PEM+skin+sky when a person is present.
      - --apple-portrait requires rear.depth + rear.depth.config + src.image. The UserComment
        portrait bit is the strong route; an explicit run can recover a missing bit with a warning.
        The complete embedded src.image must be ImageIO-readable and contain an RGB 444f or
        grayscale L008 Gain Map; the source channel structure is preserved. The outer portrait
        container may be HEIC or JPEG. Styles-only and ordinary conversion remain HEIC-only.
        Vision supplies high-resolution person/skin/hair/teeth/glasses mattes. Geometry-aligned
        OPPO subject/hair planes are constrained topology priors for person/hair only. ImageIO
        converts the complete src.image Base/Gain pair with PreserveGainMap before the portrait
        auxiliary images are attached.
      - Photographic Styles and Apple Portrait are independent and may be enabled together.
        In a combined batch, a non-portrait input still produces styles output and records Portrait unavailable.
      - Apple feature output and OPPO-compatible output remain mutually exclusive. Apple Portrait output
        omits the redundant large OPPO portrait tail; without that capability, the normal tail policy applies.
      - If --output is omitted, the input file is overwritten in place.
      - If --output-dir is omitted, files are written to the input directory.
    """

    static func main() {
        do {
            let args = Array(CommandLine.arguments.dropFirst())
            guard let command = args.first else {
                throw CLIError.usage(usage)
            }

            switch command {
            case "convert":
                let cmd = try ConversionArgumentParser.parseConvert(Array(args.dropFirst()))
                try runConvert(cmd)
            case "batch":
                let cmd = try ConversionArgumentParser.parseBatch(Array(args.dropFirst()))
                try runBatch(cmd)
            case "validate-apple":
                try runAppleValidation(Array(args.dropFirst()))
            case "validate-portrait":
                try runPortraitValidation(Array(args.dropFirst()))
            case "portrait-self-test":
                guard args.count == 1 else { throw CLIError.usage(usage) }
                let report = try AppleFeatureConversionEngine.portraitSelfTestReport()
                let data = try JSONSerialization.data(
                    withJSONObject: report,
                    options: [.prettyPrinted, .sortedKeys]
                )
                FileHandle.standardOutput.write(data)
                FileHandle.standardOutput.write(Data("\n".utf8))
            case "-h", "--help", "help":
                print(usage)
            default:
                throw CLIError.invalidCommand(command)
            }
        } catch {
            if let cli = error as? CLIError {
                switch cli {
                case .usage(let message):
                    FileHandle.standardError.write(Data("\(message)\n".utf8))
                case .invalidCommand, .missingArgument, .unknownOption, .invalidValue:
                    FileHandle.standardError.write(Data("error: \(cli)\n\n\(usage)\n".utf8))
                default:
                    FileHandle.standardError.write(Data("error: \(cli)\n".utf8))
                }
            } else {
                FileHandle.standardError.write(Data("error: \(error)\n".utf8))
            }
            exit(1)
        }
    }

    private static func runAppleValidation(_ rawArgs: [String]) throws {
        var inputURL: URL?
        var reportURL: URL?
        var expectsPortrait = false
        var index = 0
        while index < rawArgs.count {
            let option = rawArgs[index]
            index += 1
            func nextValue() throws -> String {
                guard index < rawArgs.count else { throw CLIError.missingArgument(option) }
                defer { index += 1 }
                return rawArgs[index]
            }
            switch option {
            case "--input":
                inputURL = URL(fileURLWithPath: try nextValue()).standardizedFileURL
            case "--expect-portrait":
                expectsPortrait = true
            case "--json":
                reportURL = URL(fileURLWithPath: try nextValue()).standardizedFileURL
            default:
                throw CLIError.unknownOption(option)
            }
        }
        guard let inputURL else { throw CLIError.missingArgument("--input") }
        let report = try AppleFeatureConversionEngine.validationReport(
            for: inputURL,
            expectsPortrait: expectsPortrait
        )
        let data = try JSONSerialization.data(
            withJSONObject: report,
            options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        )
        if let reportURL {
            try ensureDirectory(reportURL.deletingLastPathComponent(), fileManager: fileManager)
            try data.write(to: reportURL, options: .atomic)
        }
        FileHandle.standardOutput.write(data)
        FileHandle.standardOutput.write(Data("\n".utf8))
    }

    private static func runPortraitValidation(_ rawArgs: [String]) throws {
        var inputURL: URL?
        var reportURL: URL?
        var index = 0
        while index < rawArgs.count {
            let option = rawArgs[index]
            index += 1
            func nextValue() throws -> String {
                guard index < rawArgs.count else { throw CLIError.missingArgument(option) }
                defer { index += 1 }
                return rawArgs[index]
            }
            switch option {
            case "--input":
                inputURL = URL(fileURLWithPath: try nextValue()).standardizedFileURL
            case "--json":
                reportURL = URL(fileURLWithPath: try nextValue()).standardizedFileURL
            default:
                throw CLIError.unknownOption(option)
            }
        }
        guard let inputURL else { throw CLIError.missingArgument("--input") }
        let report = try AppleFeatureConversionEngine.portraitValidationReport(for: inputURL)
        let data = try JSONSerialization.data(
            withJSONObject: report,
            options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        )
        if let reportURL {
            try ensureDirectory(reportURL.deletingLastPathComponent(), fileManager: fileManager)
            try data.write(to: reportURL, options: .atomic)
        }
        FileHandle.standardOutput.write(data)
        FileHandle.standardOutput.write(Data("\n".utf8))
    }

    private static func runConvert(_ cmd: ConvertCommand) throws {
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

    private static func runBatch(_ cmd: BatchCommand) throws {
        try ensureDirectory(cmd.outputDirURL, fileManager: fileManager)
        let discovered = try enumerateInputs(root: cmd.inputDirURL, glob: cmd.glob)
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
        let workItems = matched.map { inputURL -> BatchWorkItem in
            let stem = inputURL.deletingPathExtension().lastPathComponent
            let outputURL = cmd.outputDirURL.appendingPathComponent("\(stem).heic")
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
                                log("skipped-existing \(item.inputURL.lastPathComponent)")
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
                                log("skipped-existing \(item.inputURL.lastPathComponent)")
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
                        log("failed \(item.inputURL.lastPathComponent): \(error)")
                    }
                }
            }
        }

        queue.waitUntilAllOperationsAreFinished()
        try checkpointWriter.close()

        log("batch complete: converted \(convertedCount) files, skipped-existing \(skippedExistingCount) files, failed \(failureCount) files into \(cmd.outputDirURL.path)")
        if failureCount == 0 {
            try? fileManager.removeItem(at: checkpointURL)
        } else {
            log("checkpoint kept (failures present): \(checkpointURL.path)")
            throw CLIError.batchFailed(failures: failureCount, checkpoint: checkpointURL)
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

    private static func loadCheckpointStateIfPresent(url: URL, expectedConfigHash: String) throws -> [String: BatchCheckpointItem] {
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
                    throw CLIError.checkpointConfigMismatch(url, expected: expectedConfigHash, actual: actual)
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
            ("outputDir", cmd.outputDirURL.standardizedFileURL.path)
        ]
        let stable = entries.sorted(by: { $0.0 < $1.0 }).map { "\($0.0)=\($0.1)" }.joined(separator: "\n")
        return sha256Hex(Data(stable.utf8))
    }

    private static func resolvedCheckpointURL(cmd: BatchCommand, configHash: String) -> URL {
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
                throw CLIError.outputPathCollision(output: item.outputURL, firstInput: prior, secondInput: item.inputURL)
            }
            seen[key] = item.inputURL
        }
    }

    private static func enumerateInputs(root: URL, glob: String) throws -> [URL] {
        guard fileManager.fileExists(atPath: root.path) else {
            throw CLIError.inputNotFound(root)
        }

        let regex = try globToRegex(glob)
        guard let enumerator = fileManager.enumerator(
            at: root,
            includingPropertiesForKeys: [.isRegularFileKey],
            options: [.skipsHiddenFiles]
        ) else {
            throw CLIError.inputNotFound(root)
        }

        var matched: [URL] = []
        for case let fileURL as URL in enumerator {
            let values = try fileURL.resourceValues(forKeys: [.isRegularFileKey])
            guard values.isRegularFile == true else { continue }

            let relative = fileURL.path.replacingOccurrences(of: root.path + "/", with: "")
            let filename = fileURL.lastPathComponent
            if regex.firstMatch(in: relative, options: [], range: NSRange(relative.startIndex..., in: relative)) != nil ||
                regex.firstMatch(in: filename, options: [], range: NSRange(filename.startIndex..., in: filename)) != nil {
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
