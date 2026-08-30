import Foundation
import XDRemuxCore
import XDRemuxAppleFeatures

/// Narrow CLI integration for Motion Photo inputs. Existing HEIC/ProXDR command behavior remains
/// owned by XDRemuxCommand; this layer only intercepts inputs that normalize as Motion Photos.
enum MotionPhotoCLIIntegration {
    private static let pendingBatchStore = PendingBatchStore()

    static func handleIfNeeded(_ arguments: [String]) throws -> Bool {
        guard let command = arguments.first else { return false }
        switch command {
        case "convert":
            return try handleConvert(Array(arguments.dropFirst()))
        case "batch":
            return try prepareDefaultBatch(Array(arguments.dropFirst()))
        default:
            return false
        }
    }

    /// Called after XDRemuxCommand.main() returns successfully. JPEG-only mixed batches can defer
    /// Motion Photo outputs until the existing HEIC batch completes, preventing those newly created
    /// Live Photo HEICs from being discovered as ProXDR inputs in the same invocation.
    static func finishPendingBatchIfNeeded() throws {
        guard let pending = pendingBatchStore.take() else { return }
        try runMotionBatch(command: pending.command, workItems: pending.workItems)
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
        guard isSupportedMotionPhotoSourceExtension(inputURL),
              AppleLivePhotoConversionEngine.isMotionPhotoInput(inputURL) else {
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
            var reserved = Set<String>()
            outputImageURL = MotionPhotoBatchPlanner.reserveOutputImageURL(
                for: inputURL,
                inputRootURL: inputURL.deletingLastPathComponent(),
                outputDirectoryURL: inputURL.deletingLastPathComponent(),
                reservedPaths: &reserved,
                // Treat any existing HEIC or companion MOV as a collision. Single-file
                // conversion must never infer ownership of an unrelated destination resource.
                fileExists: { FileManager.default.fileExists(atPath: $0.path) }
            )
        }
        try validateLivePhotoOutputExtension(outputImageURL)
        if outputWasExplicit {
            let outputVideoURL = AppleLivePhotoConversionEngine.companionVideoURL(for: outputImageURL)
            if FileManager.default.fileExists(atPath: outputImageURL.path)
                || FileManager.default.fileExists(atPath: outputVideoURL.path) {
                throw CLIError.invalidValue(
                    option: "--output",
                    value: "target HEIC/MOV already exists; refusing to overwrite an output pair with unknown provenance"
                )
            }
        }

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

    /// Plans the entire Motion Photo portion of a default batch before the first write. Explicit
    /// --glob retains the old contract. User basenames are preserved; deterministic `(2)`, `(3)`, …
    /// suffixes are introduced only when assets actually converge on the same destination namespace.
    private static func prepareDefaultBatch(_ rawArguments: [String]) throws -> Bool {
        guard !rawArguments.contains("--glob") else { return false }

        let command = try ConversionArgumentParser.parseBatch(rawArguments)
        guard !command.appleFeatures.isEnabled else { return false }
        guard !command.oppoCompatibility.wantsOppoCompat else { return false }

        let jpegCandidates = try discoverJPEGs(
            under: command.inputDirURL,
            excluding: command.outputDirURL
        )
        let jpegMotionInputs = jpegCandidates.filter(AppleLivePhotoConversionEngine.isMotionPhotoInput)

        let allHEICInputs = try discoverHEICs(
            under: command.inputDirURL,
            excluding: command.outputDirURL
        )
        let existingLivePhotoStills = allHEICInputs.filter(isExistingLivePhotoStill)
        let existingLivePhotoPaths = Set(
            existingLivePhotoStills.map { $0.standardizedFileURL.path }
        )
        let unpairedHEICInputs = allHEICInputs.filter {
            !existingLivePhotoPaths.contains($0.standardizedFileURL.path)
        }
        let heifMotionInputs = unpairedHEICInputs.filter(AppleLivePhotoConversionEngine.isMotionPhotoInput)
        let heifMotionPaths = Set(heifMotionInputs.map { $0.standardizedFileURL.path })
        let sourceHEICInputs = unpairedHEICInputs.filter {
            !heifMotionPaths.contains($0.standardizedFileURL.path)
        }
        let motionInputs = jpegMotionInputs + heifMotionInputs
        guard !motionInputs.isEmpty else { return false }

        let workItems = try makeWorkItems(
            inputs: motionInputs,
            heicInputs: sourceHEICInputs,
            command: command
        )

        if sourceHEICInputs.isEmpty {
            try runMotionBatch(command: command, workItems: workItems)
            return true
        }

        if heifMotionInputs.isEmpty, existingLivePhotoStills.isEmpty {
            pendingBatchStore.set(PendingBatch(command: command, workItems: workItems))
            return false
        }

        try runClassifiedHEICBatch(command: command, inputs: sourceHEICInputs)
        try runMotionBatch(command: command, workItems: workItems)
        return true
    }

    private static func runClassifiedHEICBatch(
        command: BatchCommand,
        inputs: [URL]
    ) throws {
        let workItems = makeHEICWorkItems(inputs: inputs, command: command)
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
                    func outputIsValid() -> Bool {
                        guard FileManager.default.fileExists(atPath: item.outputURL.path) else {
                            return false
                        }
                        return ConversionEngine.isValidOutput(
                            item.outputURL,
                            config: command.conversionConfiguration
                        )
                    }

                    if command.skipExisting, outputIsValid() {
                        lock.lock(); skipped += 1; lock.unlock()
                        printSynchronized("skipped \(item.inputURL.lastPathComponent) (HEIC output already up to date)")
                        return
                    }

                    do {
                        try FileManager.default.createDirectory(
                            at: item.outputURL.deletingLastPathComponent(),
                            withIntermediateDirectories: true
                        )
                        if item.outputURL.standardizedFileURL.path != item.inputURL.standardizedFileURL.path,
                           FileManager.default.fileExists(atPath: item.outputURL.path) {
                            try FileManager.default.removeItem(at: item.outputURL)
                        }
                        try ConversionEngine.convert(
                            inputURL: item.inputURL,
                            outputURL: item.outputURL,
                            config: command.conversionConfiguration
                        )
                        lock.lock(); converted += 1; lock.unlock()
                        printSynchronized("converted \(item.inputURL.lastPathComponent)")
                    } catch {
                        lock.lock(); failures.append((item.inputURL, error)); lock.unlock()
                        printSynchronized("failed \(item.inputURL.lastPathComponent): \(singleLine(error))")
                    }
                }
            }
        }
        queue.waitUntilAllOperationsAreFinished()

        print(
            "classified HEIC batch pass: \(converted) converted, \(skipped) skipped, "
                + "\(failures.count) failed"
        )
        if !failures.isEmpty {
            throw ClassifiedHEICBatchError(failures: failures.count)
        }
    }

    private static func runMotionBatch(
        command: BatchCommand,
        workItems: [MotionBatchWorkItem]
    ) throws {
        let checkpointURL = MotionPhotoBatchCheckpoint.resolvedURL(for: command)
        // Keep successful entries as durable provenance instead of deleting/resetting them. Old
        // schema-1 entries remain readable but cannot authorize reuse because they have no SHA-256
        // or asset identifier.
        let checkpointState = try MotionPhotoBatchCheckpoint.load(url: checkpointURL)
        let checkpointWriter = try MotionPhotoBatchCheckpoint.Writer(url: checkpointURL)
        defer { try? checkpointWriter.close() }

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
                    let outputVideoURL = AppleLivePhotoConversionEngine.companionVideoURL(
                        for: item.outputImageURL
                    )
                    let inputKey = item.inputURL.standardizedFileURL.path
                    let prior = checkpointState[inputKey]

                    let signature: MotionPhotoBatchCheckpoint.FileSignature
                    do {
                        signature = try MotionPhotoBatchCheckpoint.signature(for: item.inputURL)
                    } catch {
                        lock.lock(); failures.append((item.inputURL, error)); lock.unlock()
                        printSynchronized("failed \(item.inputURL.lastPathComponent): could not fingerprint source: \(singleLine(error))")
                        return
                    }

                    func pairIsValid(expectedAssetIdentifier: String? = nil) -> Bool {
                        if let expectedAssetIdentifier,
                           AppleLivePhotoStillWriter.assetIdentifier(in: item.outputImageURL) != expectedAssetIdentifier {
                            return false
                        }
                        return AppleLivePhotoValidator.isValidPair(
                            imageURL: item.outputImageURL,
                            videoURL: outputVideoURL
                        )
                    }

                    func record(
                        _ status: MotionPhotoBatchCheckpoint.Status,
                        assetIdentifier: String? = nil,
                        error: String? = nil
                    ) {
                        do {
                            try checkpointWriter.append(
                                inputURL: item.inputURL,
                                inputRootURL: command.inputDirURL,
                                outputImageURL: item.outputImageURL,
                                outputVideoURL: outputVideoURL,
                                status: status,
                                signature: signature,
                                assetIdentifier: assetIdentifier,
                                error: error
                            )
                        } catch {
                            printSynchronized("Motion Photo checkpoint write failed: \(singleLine(error))")
                        }
                    }

                    let signatureMatches = prior?.matchesSignature(signature) == true
                    let outputsMatch = prior?.matchesOutputs(
                        imageURL: item.outputImageURL,
                        videoURL: outputVideoURL
                    ) == true
                    let priorSucceeded = prior?.status == .success || prior?.status == .skippedExisting
                    let priorAssetIdentifier = prior?.assetIdentifier
                    let provenanceMatches = signatureMatches
                        && outputsMatch
                        && priorSucceeded
                        && priorAssetIdentifier != nil
                        && pairIsValid(expectedAssetIdentifier: priorAssetIdentifier)

                    if (command.resume || command.skipExisting), provenanceMatches,
                       let priorAssetIdentifier {
                        lock.lock(); skipped += 1; lock.unlock()
                        record(.skippedExisting, assetIdentifier: priorAssetIdentifier)
                        printSynchronized("skipped \(item.inputURL.lastPathComponent) (Live Photo source provenance matched)")
                        return
                    }

                    if command.resume || command.skipExisting {
                        let existingPairValid = pairIsValid()
                        let knownLineage = outputsMatch
                            && priorAssetIdentifier != nil
                            && pairIsValid(expectedAssetIdentifier: priorAssetIdentifier)
                        if existingPairValid && !knownLineage {
                            let error = MotionBatchOutputConflictError(
                                input: item.inputURL,
                                image: item.outputImageURL,
                                video: outputVideoURL
                            )
                            lock.lock(); failures.append((item.inputURL, error)); lock.unlock()
                            record(.failure, error: error.localizedDescription)
                            printSynchronized("failed \(item.inputURL.lastPathComponent): \(singleLine(error))")
                            return
                        }
                        // A known pair from the same source path with a different content digest is
                        // stale, not foreign. Rebuild it rather than silently reusing old content.
                    }

                    do {
                        let result = try AppleLivePhotoConversionEngine.convert(
                            inputURL: item.inputURL,
                            outputImageURL: item.outputImageURL
                        )
                        lock.lock(); converted += 1; lock.unlock()
                        record(.success, assetIdentifier: result.assetIdentifier)
                        printSynchronized("converted Motion Photo \(item.inputURL.lastPathComponent)")
                    } catch {
                        lock.lock(); failures.append((item.inputURL, error)); lock.unlock()
                        record(.failure, error: String(describing: error))
                        printSynchronized("failed \(item.inputURL.lastPathComponent): \(singleLine(error))")
                    }
                }
            }
        }
        queue.waitUntilAllOperationsAreFinished()
        try checkpointWriter.close()

        print(
            "Motion Photo batch pass: \(converted) converted, \(skipped) skipped, "
                + "\(failures.count) failed"
        )
        if !failures.isEmpty {
            print("run the same command again to retry the \(failures.count) unresolved Motion Photo file(s)")
            throw CLIError.batchFailed(
                failures: failures.count,
                checkpoint: checkpointURL
            )
        }
    }

    private struct PendingBatch {
        let command: BatchCommand
        let workItems: [MotionBatchWorkItem]
    }

    private struct HEICWorkItem {
        let inputURL: URL
        let outputURL: URL
    }

    private struct ClassifiedHEICBatchError: LocalizedError {
        let failures: Int
        var errorDescription: String? {
            "classified HEIC batch pass failed for \(failures) file(s); rerun the same command to retry unresolved inputs"
        }
    }

    private struct MotionBatchOutputConflictError: LocalizedError {
        let input: URL
        let image: URL
        let video: URL

        var errorDescription: String? {
            "existing Live Photo pair \(image.lastPathComponent) + \(video.lastPathComponent) has no matching provenance for \(input.path); refusing silent reuse"
        }
    }

    private final class PendingBatchStore: @unchecked Sendable {
        private let lock = NSLock()
        private var pending: PendingBatch?

        func set(_ value: PendingBatch) {
            lock.lock(); defer { lock.unlock() }
            pending = value
        }

        func take() -> PendingBatch? {
            lock.lock(); defer { lock.unlock() }
            defer { pending = nil }
            return pending
        }
    }

    private struct MotionBatchWorkItem {
        let inputURL: URL
        let outputImageURL: URL
    }

    private static func makeHEICWorkItems(
        inputs: [URL],
        command: BatchCommand
    ) -> [HEICWorkItem] {
        var reserved = Set<String>()
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
            return HEICWorkItem(inputURL: input, outputURL: output)
        }
    }

    private static func makeWorkItems(
        inputs: [URL],
        heicInputs: [URL],
        command: BatchCommand
    ) throws -> [MotionBatchWorkItem] {
        let heicItems = makeHEICWorkItems(inputs: heicInputs, command: command)
        var reserved = Set(heicItems.map { $0.outputURL.standardizedFileURL.path })
        let checkpointState = try MotionPhotoBatchCheckpoint.load(
            url: MotionPhotoBatchCheckpoint.resolvedURL(for: command)
        )

        var result: [MotionBatchWorkItem] = []
        for input in inputs.sorted(by: { $0.path < $1.path }) {
            let prior = checkpointState[input.standardizedFileURL.path]
            let output = MotionPhotoBatchPlanner.reserveOutputImageURL(
                for: input,
                inputRootURL: command.inputDirURL,
                outputDirectoryURL: categorizedOutputDirectory(for: input, command: command),
                reservedPaths: &reserved,
                candidateBelongsToSource: { image, video in
                    prior?.matchesOutputs(imageURL: image, videoURL: video) == true
                }
            )
            result.append(MotionBatchWorkItem(inputURL: input, outputImageURL: output))
        }
        return result
    }

    private static func categorizedOutputDirectory(for input: URL, command: BatchCommand) -> URL {
        if command.categorizeOutput,
           let folder = PhotoCategorizationEngine.categorizedDirectory(for: input) {
            return command.outputDirURL.appendingPathComponent(folder, isDirectory: true)
        }
        return command.outputDirURL
    }

    private static func isExistingLivePhotoStill(_ imageURL: URL) -> Bool {
        let companion = AppleLivePhotoConversionEngine.companionVideoURL(for: imageURL)
        guard FileManager.default.fileExists(atPath: companion.path) else { return false }
        return AppleLivePhotoValidator.isValidPair(imageURL: imageURL, videoURL: companion)
    }

    private static func discoverJPEGs(under root: URL, excluding outputRoot: URL) throws -> [URL] {
        try discover(under: root, excluding: outputRoot) { isJPEG($0) }
    }

    private static func discoverHEICs(under root: URL, excluding outputRoot: URL) throws -> [URL] {
        try discover(under: root, excluding: outputRoot) { isHEIC($0) }
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

    private static func isSupportedMotionPhotoSourceExtension(_ url: URL) -> Bool {
        isJPEG(url) || isHEIC(url)
    }

    private static func isJPEG(_ url: URL) -> Bool {
        let ext = url.pathExtension.lowercased()
        return ext == "jpg" || ext == "jpeg"
    }

    private static func isHEIC(_ url: URL) -> Bool {
        let ext = url.pathExtension.lowercased()
        return ext == "heic" || ext == "heif"
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
