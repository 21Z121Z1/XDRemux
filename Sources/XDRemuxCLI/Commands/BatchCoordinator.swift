import Foundation
import XDRemuxCore

struct BatchWorkItem: Sendable {
    let inputURL: URL
    let outputURL: URL
}

struct BatchFailureRecord: Codable, Sendable, Equatable {
    let input: String
    let output: String
    let errorCode: String
    let diagnosticsAvailable: Bool

    enum CodingKeys: String, CodingKey {
        case input
        case output
        case errorCode = "error_code"
        case diagnosticsAvailable = "diagnostics_available"
    }
}

struct BatchRunResult: Sendable {
    let converted: Int
    let skipped: Int
    let failures: [BatchFailureRecord]
}

enum BatchCoordinator {
    static func outputURL(input: URL, inputRoot: URL, outputRoot: URL) -> URL {
        let rootPath = inputRoot.standardizedFileURL.path
        let inputPath = input.standardizedFileURL.path
        let relative: String
        if inputPath == rootPath {
            relative = input.lastPathComponent
        } else if inputPath.hasPrefix(rootPath + "/") {
            relative = String(inputPath.dropFirst(rootPath.count + 1))
        } else {
            relative = input.lastPathComponent
        }
        return outputRoot
            .appendingPathComponent(relative)
            .deletingPathExtension()
            .appendingPathExtension("heic")
    }

    static func run(
        items: [BatchWorkItem],
        jobs: Int,
        overwrite: Bool,
        diagnosticsAvailable: Bool,
        reporter: CLIReporter,
        isValid: @escaping @Sendable (BatchWorkItem) -> Bool,
        convert: @escaping @Sendable (BatchWorkItem) throws -> Void
    ) -> BatchRunResult {
        let stateLock = NSLock()
        var converted = 0
        var skipped = 0
        var failures: [BatchFailureRecord] = []
        let queue = OperationQueue()
        queue.maxConcurrentOperationCount = max(1, jobs)
        queue.qualityOfService = .userInitiated

        for item in items {
            queue.addOperation {
                autoreleasepool {
                    reporter.batchFileStarted(item.inputURL)
                    if !overwrite, isValid(item) {
                        stateLock.lock()
                        skipped += 1
                        stateLock.unlock()
                        reporter.batchFileFinished(
                            input: item.inputURL,
                            output: item.outputURL,
                            outcome: .skipped
                        )
                        return
                    }

                    do {
                        try convert(item)
                        guard isValid(item) else {
                            throw XDRemuxError.outputVerificationFailed(item.outputURL)
                        }
                        stateLock.lock()
                        converted += 1
                        stateLock.unlock()
                        reporter.batchFileFinished(
                            input: item.inputURL,
                            output: item.outputURL,
                            outcome: .converted
                        )
                    } catch {
                        let failure = ConversionFailure.classify(error)
                        stateLock.lock()
                        failures.append(BatchFailureRecord(
                            input: item.inputURL.path,
                            output: item.outputURL.path,
                            errorCode: failure.code.rawValue,
                            diagnosticsAvailable: diagnosticsAvailable
                        ))
                        stateLock.unlock()
                        reporter.batchFileFinished(
                            input: item.inputURL,
                            output: item.outputURL,
                            outcome: .failed(failure)
                        )
                    }
                }
            }
        }
        queue.waitUntilAllOperationsAreFinished()
        return BatchRunResult(converted: converted, skipped: skipped, failures: failures)
    }

    static func convertAtomically(
        item: BatchWorkItem,
        validateTemporary: ((URL) -> Bool)? = nil,
        convert: (URL, URL) throws -> Void
    ) throws {
        let fileManager = FileManager.default
        let parent = item.outputURL.deletingLastPathComponent()
        try ensureDirectory(parent, fileManager: fileManager)
        let temporary = parent.appendingPathComponent(
            ".\(item.outputURL.deletingPathExtension().lastPathComponent).xdremux-batch-\(UUID().uuidString).heic"
        )
        let temporaryManifest = temporary.deletingPathExtension()
            .appendingPathExtension("portrait-manifest.json")
        defer {
            try? fileManager.removeItem(at: temporary)
            try? fileManager.removeItem(at: temporaryManifest)
        }

        try convert(item.inputURL, temporary)
        if let validateTemporary, !validateTemporary(temporary) {
            throw XDRemuxError.outputVerificationFailed(temporary)
        }
        if fileManager.fileExists(atPath: item.outputURL.path) {
            _ = try fileManager.replaceItemAt(item.outputURL, withItemAt: temporary)
        } else {
            try fileManager.moveItem(at: temporary, to: item.outputURL)
        }

        if fileManager.fileExists(atPath: temporaryManifest.path) {
            let finalManifest = item.outputURL.deletingPathExtension()
                .appendingPathExtension("portrait-manifest.json")
            if fileManager.fileExists(atPath: finalManifest.path) {
                _ = try fileManager.replaceItemAt(finalManifest, withItemAt: temporaryManifest)
            } else {
                try fileManager.moveItem(at: temporaryManifest, to: finalManifest)
            }
        }
    }

    static func writeFailureReport(
        _ failures: [BatchFailureRecord],
        outputDirectory: URL
    ) throws -> URL? {
        let url = outputDirectory.appendingPathComponent("xdremux-failures.json")
        guard !failures.isEmpty else {
            try? FileManager.default.removeItem(at: url)
            return nil
        }
        let orderedFailures = failures.sorted {
            ($0.input, $0.output, $0.errorCode) < ($1.input, $1.output, $1.errorCode)
        }
        let report = FailureReport(schemaVersion: 1, failures: orderedFailures)
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        try encoder.encode(report).write(to: url, options: .atomic)
        return url
    }

    private struct FailureReport: Codable {
        let schemaVersion: Int
        let failures: [BatchFailureRecord]

        enum CodingKeys: String, CodingKey {
            case schemaVersion = "schema_version"
            case failures
        }
    }
}
