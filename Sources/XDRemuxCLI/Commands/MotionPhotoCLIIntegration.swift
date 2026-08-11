import Foundation
import XDRemuxCore
import XDRemuxAppleFeatures

/// Narrow CLI integration for Motion Photo inputs. Existing HEIC/ProXDR command behavior remains
/// owned by XDRemuxCommand; this layer only intercepts inputs that normalize as Motion Photos.
enum MotionPhotoCLIIntegration {
    static func handleIfNeeded(_ arguments: [String]) throws -> Bool {
        guard let command = arguments.first else { return false }
        switch command {
        case "convert":
            return try handleConvert(Array(arguments.dropFirst()))
        case "batch":
            return try handleDefaultBatchPrepass(Array(arguments.dropFirst()))
        default:
            return false
        }
    }

    static func printFailure(_ error: Error) {
        if let localized = error as? LocalizedError, let message = localized.errorDescription {
            FileHandle.standardError.write(Data("error: \(message)\n".utf8))
        } else {
            FileHandle.standardError.write(Data("error: \(error)\n".utf8))
        }
    }

    private static func handleConvert(_ rawArguments: [String]) throws -> Bool {
        guard let inputPath = optionValue("--input", in: rawArguments) else { return false }
        let inputURL = URL(fileURLWithPath: inputPath).standardizedFileURL
        guard isJPEG(inputURL), AppleLivePhotoConversionEngine.isMotionPhotoInput(inputURL) else {
            return false
        }

        let command = try ConversionArgumentParser.parseConvert(rawArguments)
        guard !command.appleFeatures.isEnabled else {
            throw CLIError.invalidValue(
                option: "--input",
                value: "Motion Photo conversion cannot be combined with Apple Portrait or Photographic Styles in the same pass"
            )
        }
        guard !command.oppoCompatibility.wantsOppoCompat else {
            throw CLIError.invalidValue(
                option: "--oppo-compatible",
                value: "Motion Photo conversion produces an Apple Live Photo pair"
            )
        }

        let outputWasExplicit = rawArguments.contains("--output")
        let outputImageURL: URL
        if outputWasExplicit {
            outputImageURL = command.outputURL.standardizedFileURL
        } else {
            outputImageURL = inputURL.deletingPathExtension().appendingPathExtension("heic")
        }
        try validateLivePhotoOutputExtension(outputImageURL)

        let result = try AppleLivePhotoConversionEngine.convert(
            inputURL: inputURL,
            outputImageURL: outputImageURL
        )
        print(
            "converted Motion Photo \(inputURL.lastPathComponent) -> "
                + "\(result.imageURL.path) + \(result.videoURL.path)"
        )
        for diagnostic in result.diagnostics {
            print("  \(diagnostic)")
        }
        return true
    }

    /// The existing batch implementation intentionally keeps its HEIC-centric behavior. For the
    /// default batch command we run a Motion Photo classification/conversion pass first, then return
    /// false when HEIC inputs remain so XDRemuxCommand can process them exactly as before.
    /// Ordinary JPEGs are ignored by this automatic pass.
    private static func handleDefaultBatchPrepass(_ rawArguments: [String]) throws -> Bool {
        // Explicit --glob retains the existing batch contract. Automatic JPEG discovery is only
        // enabled for the default batch mode so old scripts using custom globs remain unchanged.
        guard !rawArguments.contains("--glob") else { return false }

        let command = try ConversionArgumentParser.parseBatch(rawArguments)
        guard !command.appleFeatures.isEnabled else {
            // Portrait/style batch semantics remain entirely with XDRemuxCommand.
            return false
        }
        guard !command.oppoCompatibility.wantsOppoCompat else {
            return false
        }

        let jpegCandidates = try discoverJPEGs(
            under: command.inputDirURL,
            excluding: command.outputDirURL
        )
        let motionInputs = jpegCandidates.filter(AppleLivePhotoConversionEngine.isMotionPhotoInput)
        guard !motionInputs.isEmpty else { return false }

        let heicInputs = try discoverHEICs(
            under: command.inputDirURL,
            excluding: command.outputDirURL
        )
        let workItems = makeWorkItems(
            inputs: motionInputs,
            heicInputs: heicInputs,
            command: command
        )

        let queue = OperationQueue()
        queue.maxConcurrentOperationCount = max(1, command.jobs)
        queue.qualityOfService = .userInitiated
        let lock = NSLock()
        var converted = 0
        var skipped = 0
        var failures: [(URL, Error)] = []

        for item in workItems {
            queue.addOperation {
                autoreleasepool {
                    if command.skipExisting,
                       AppleLivePhotoValidator.isValidPair(
                           imageURL: item.outputImageURL,
                           videoURL: AppleLivePhotoConversionEngine.companionVideoURL(for: item.outputImageURL)
                       ) {
                        lock.lock(); skipped += 1; lock.unlock()
                        printSynchronized("skipped \(item.inputURL.lastPathComponent) (Live Photo pair already valid)")
                        return
                    }
                    do {
                        _ = try AppleLivePhotoConversionEngine.convert(
                            inputURL: item.inputURL,
                            outputImageURL: item.outputImageURL
                        )
                        lock.lock(); converted += 1; lock.unlock()
                        printSynchronized("converted Motion Photo \(item.inputURL.lastPathComponent)")
                    } catch {
                        lock.lock(); failures.append((item.inputURL, error)); lock.unlock()
                        printSynchronized("failed \(item.inputURL.lastPathComponent): \(singleLine(error))")
                    }
                }
            }
        }
        queue.waitUntilAllOperationsAreFinished()

        print(
            "Motion Photo batch pass: \(converted) converted, \(skipped) skipped, "
                + "\(failures.count) failed"
        )
        if !failures.isEmpty {
            throw CLIError.batchFailed(
                failures: failures.count,
                checkpoint: command.checkpointURL
                    ?? command.outputDirURL.appendingPathComponent(".xdremux-motion-photo-retry")
            )
        }

        // If HEIC inputs exist, let the original batch implementation continue with its unchanged
        // checkpoint/resume behavior. If this directory only contained Motion Photos, the batch is
        // complete and must not fall through to the old "no *.heic matched" error.
        return heicInputs.isEmpty
    }

    private struct MotionBatchWorkItem {
        let inputURL: URL
        let outputImageURL: URL
    }

    private static func makeWorkItems(
        inputs: [URL],
        heicInputs: [URL],
        command: BatchCommand
    ) -> [MotionBatchWorkItem] {
        var reserved = Set<String>()

        // Reserve the outputs the unchanged HEIC batch pass will choose, preventing a Motion Photo
        // with the same stem from being overwritten when that pass runs afterward.
        for input in heicInputs {
            let directory = categorizedOutputDirectory(for: input, command: command)
            reserved.insert(
                directory.appendingPathComponent(input.deletingPathExtension().lastPathComponent)
                    .appendingPathExtension("heic").standardizedFileURL.path
            )
        }

        return inputs.sorted { $0.path < $1.path }.map { input in
            let directory = categorizedOutputDirectory(for: input, command: command)
            let stem = input.deletingPathExtension().lastPathComponent
            var sequence = 1
            var output = directory.appendingPathComponent(stem).appendingPathExtension("heic")
            while reserved.contains(output.standardizedFileURL.path) {
                sequence += 1
                output = directory.appendingPathComponent("\(stem) (\(sequence))").appendingPathExtension("heic")
            }
            reserved.insert(output.standardizedFileURL.path)
            reserved.insert(AppleLivePhotoConversionEngine.companionVideoURL(for: output).standardizedFileURL.path)
            return MotionBatchWorkItem(inputURL: input, outputImageURL: output)
        }
    }

    private static func categorizedOutputDirectory(for input: URL, command: BatchCommand) -> URL {
        if command.categorizeOutput,
           let folder = PhotoCategorizationEngine.categorizedDirectory(for: input) {
            return command.outputDirURL.appendingPathComponent(folder, isDirectory: true)
        }
        return command.outputDirURL
    }

    private static func discoverJPEGs(under root: URL, excluding outputRoot: URL) throws -> [URL] {
        try discover(under: root, excluding: outputRoot) { isJPEG($0) }
    }

    private static func discoverHEICs(under root: URL, excluding outputRoot: URL) throws -> [URL] {
        try discover(under: root, excluding: outputRoot) { url in
            let ext = url.pathExtension.lowercased()
            return ext == "heic" || ext == "heif"
        }
    }

    private static func discover(
        under root: URL,
        excluding outputRoot: URL,
        predicate: (URL) -> Bool
    ) throws -> [URL] {
        let root = root.standardizedFileURL
        let outputRoot = outputRoot.standardizedFileURL
        let shouldExcludeOutputSubtree = outputRoot.path != root.path
            && outputRoot.path.hasPrefix(root.path + "/")
        let manager = FileManager.default
        let keys: [URLResourceKey] = [.isRegularFileKey, .isDirectoryKey]
        guard let enumerator = manager.enumerator(
            at: root,
            includingPropertiesForKeys: keys,
            options: [.skipsHiddenFiles]
        ) else { return [] }

        var results: [URL] = []
        for case let url as URL in enumerator {
            let standardized = url.standardizedFileURL
            if shouldExcludeOutputSubtree,
               (standardized.path == outputRoot.path
                    || standardized.path.hasPrefix(outputRoot.path + "/")) {
                if (try? standardized.resourceValues(forKeys: [.isDirectoryKey]).isDirectory) == true {
                    enumerator.skipDescendants()
                }
                continue
            }
            let values = try standardized.resourceValues(forKeys: Set(keys))
            guard values.isRegularFile == true, predicate(standardized) else { continue }
            results.append(standardized)
        }
        return results
    }

    private static func isJPEG(_ url: URL) -> Bool {
        let ext = url.pathExtension.lowercased()
        return ext == "jpg" || ext == "jpeg"
    }

    private static func validateLivePhotoOutputExtension(_ outputURL: URL) throws {
        let ext = outputURL.pathExtension.lowercased()
        guard ext == "heic" || ext == "heif" else {
            throw CLIError.invalidValue(
                option: "--output",
                value: "Motion Photo output must end in .heic or .heif"
            )
        }
    }

    private static func optionValue(_ name: String, in arguments: [String]) -> String? {
        guard let index = arguments.firstIndex(of: name), index + 1 < arguments.count else { return nil }
        return arguments[index + 1]
    }

    private static let printLock = NSLock()
    private static func printSynchronized(_ message: String) {
        printLock.lock(); defer { printLock.unlock() }
        print(message)
    }

    private static func singleLine(_ error: Error) -> String {
        String(describing: error)
            .split(whereSeparator: \.isNewline)
            .map { $0.trimmingCharacters(in: .whitespaces) }
            .filter { !$0.isEmpty }
            .joined(separator: " ")
    }
}
