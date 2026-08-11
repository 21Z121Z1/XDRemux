import Foundation
@preconcurrency import AVFoundation
import CoreMedia
import XDRemuxCore

public struct LivePhotoConversionResult: Sendable {
    public let imageURL: URL
    public let videoURL: URL
    public let assetIdentifier: String
    public let stillImageTime: CMTime
    public let sourceKind: MotionPhotoSourceKind
    public let diagnostics: [String]
}

public enum AppleLivePhotoConversionEngine {
    public static func isMotionPhotoInput(_ inputURL: URL) -> Bool {
        let ext = inputURL.pathExtension.lowercased()
        guard ext == "jpg" || ext == "jpeg" || ext == "heic" || ext == "heif" else {
            return false
        }
        return (try? OppoMotionPhotoParser.parse(url: inputURL)) != nil
    }

    public static func companionVideoURL(for imageURL: URL) -> URL {
        imageURL.deletingPathExtension().appendingPathExtension("mov")
    }

    public static func convert(
        inputURL: URL,
        outputImageURL: URL,
        requirePhotoKitValidation: Bool = true
    ) throws -> LivePhotoConversionResult {
        let semaphore = DispatchSemaphore(value: 0)
        let box = LivePhotoConversionResultBox()
        Task.detached(priority: .userInitiated) {
            do {
                let value = try await convertAsync(
                    inputURL: inputURL,
                    outputImageURL: outputImageURL,
                    requirePhotoKitValidation: requirePhotoKitValidation
                )
                box.set(.success(value))
            } catch {
                box.set(.failure(error))
            }
            semaphore.signal()
        }
        semaphore.wait()
        return try box.result.get()
    }

    public static func convertAsync(
        inputURL: URL,
        outputImageURL: URL,
        requirePhotoKitValidation: Bool = true
    ) async throws -> LivePhotoConversionResult {
        guard let asset = try OppoMotionPhotoParser.parse(url: inputURL) else {
            throw AppleLivePhotoError.transactionFailed("input is not a supported Motion Photo")
        }
        let inputPath = inputURL.standardizedFileURL.path
        let outputPath = outputImageURL.standardizedFileURL.path
        guard inputPath != outputPath else {
            throw AppleLivePhotoError.transactionFailed("Motion Photo conversion never overwrites the source image in place")
        }
        let ext = outputImageURL.pathExtension.lowercased()
        guard ext == "heic" || ext == "heif" else {
            throw AppleLivePhotoError.transactionFailed("Live Photo still output must use .heic or .heif")
        }

        let outputVideoURL = companionVideoURL(for: outputImageURL)
        try FileManager.default.createDirectory(
            at: outputImageURL.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )

        let scratch = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-livephoto-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: scratch, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: scratch) }

        let stillExtension = asset.sourceKind == .androidHeifMotionPhotoV1 ? "heic" : "jpg"
        let stillSourceURL = scratch.appendingPathComponent("still.\(stillExtension)")
        let videoSourceURL = scratch.appendingPathComponent("motion.mp4")
        try MotionPhotoPayloadExtractor.copy(
            range: asset.stillResourceRange,
            from: inputURL,
            to: stillSourceURL
        )
        let primaryVideoRange = try OppoMotionPhotoStreamResolver.primaryVideoRange(for: asset)
        try MotionPhotoPayloadExtractor.copy(
            range: primaryVideoRange,
            from: inputURL,
            to: videoSourceURL
        )

        let timeline = try await AppleLivePhotoTimelineResolver.resolve(
            videoURL: videoSourceURL,
            requestedTimestampUs: asset.presentationTimestampUs,
            requestedSource: asset.presentationSource
        )
        let assetIdentifier = UUID().uuidString
        let transactionID = UUID().uuidString
        let directory = outputImageURL.deletingLastPathComponent()
        let stem = outputImageURL.deletingPathExtension().lastPathComponent
        let temporaryImageURL = directory.appendingPathComponent(".\(stem).\(transactionID).tmp.heic")
        let temporaryVideoURL = directory.appendingPathComponent(".\(stem).\(transactionID).tmp.mov")
        defer {
            try? FileManager.default.removeItem(at: temporaryImageURL)
            try? FileManager.default.removeItem(at: temporaryVideoURL)
        }

        let sourceHadGainMap = AppleLivePhotoStillWriter.hasGainMap(stillSourceURL)
        let sourceAsset = AVURLAsset(url: videoSourceURL)
        let sourceHadAudio = !(try await sourceAsset.loadTracks(withMediaType: .audio)).isEmpty

        try AppleLivePhotoStillWriter.write(
            stillInputURL: stillSourceURL,
            outputURL: temporaryImageURL,
            assetIdentifier: assetIdentifier
        )
        try await AppleLivePhotoVideoWriter.write(
            videoInputURL: videoSourceURL,
            outputURL: temporaryVideoURL,
            assetIdentifier: assetIdentifier,
            stillImageTime: timeline.stillImageTime,
            oppoMetadata: asset.vendorMetadata
        )

        let expectedTransform = asset.vendorMetadata.flatMap(OppoLivePhotoAlignment.transformMatrix) != nil
        _ = try await AppleLivePhotoValidator.validate(
            imageURL: temporaryImageURL,
            videoURL: temporaryVideoURL,
            expectedAssetIdentifier: assetIdentifier,
            expectedStillImageTime: timeline.stillImageTime,
            sourceStillURL: stillSourceURL,
            sourceVideoURL: videoSourceURL,
            sourceHadAudio: sourceHadAudio,
            sourceHadGainMap: sourceHadGainMap,
            expectsOppoTransform: expectedTransform,
            requirePhotoKitLoad: requirePhotoKitValidation
        )

        try commitPair(
            temporaryImageURL: temporaryImageURL,
            temporaryVideoURL: temporaryVideoURL,
            finalImageURL: outputImageURL,
            finalVideoURL: outputVideoURL
        )

        var diagnostics: [String] = []
        if let xmp = asset.presentationTimestampUs,
           let cover = asset.vendorMetadata?.coverFramePtsUs,
           xmp != cover {
            diagnostics.append(
                "XMP still time \(formatMicroseconds(xmp)) s differs from OPPO coverFramePts \(formatMicroseconds(cover)) s; selected \(timeline.source.rawValue)."
            )
        }
        if let metadata = asset.vendorMetadata, metadata.streamCount >= 2 {
            diagnostics.append("OPPO dual-stream input detected; selected Stream 1 for Apple paired video.")
        }
        if asset.sourceKind == .androidHeifMotionPhotoV1 {
            diagnostics.append("HEIF Motion Photo mpvd payload extracted without trailing vendor boxes.")
        }

        return LivePhotoConversionResult(
            imageURL: outputImageURL,
            videoURL: outputVideoURL,
            assetIdentifier: assetIdentifier,
            stillImageTime: timeline.stillImageTime,
            sourceKind: asset.sourceKind,
            diagnostics: diagnostics
        )
    }

    private static func commitPair(
        temporaryImageURL: URL,
        temporaryVideoURL: URL,
        finalImageURL: URL,
        finalVideoURL: URL
    ) throws {
        let fileManager = FileManager.default
        let backupID = UUID().uuidString
        let imageBackup = finalImageURL.deletingLastPathComponent()
            .appendingPathComponent(".\(finalImageURL.lastPathComponent).\(backupID).backup")
        let videoBackup = finalVideoURL.deletingLastPathComponent()
            .appendingPathComponent(".\(finalVideoURL.lastPathComponent).\(backupID).backup")
        let hadImage = fileManager.fileExists(atPath: finalImageURL.path)
        let hadVideo = fileManager.fileExists(atPath: finalVideoURL.path)
        var newImageInstalled = false

        do {
            if hadImage { try fileManager.moveItem(at: finalImageURL, to: imageBackup) }
            if hadVideo { try fileManager.moveItem(at: finalVideoURL, to: videoBackup) }
            try fileManager.moveItem(at: temporaryImageURL, to: finalImageURL)
            newImageInstalled = true
            try fileManager.moveItem(at: temporaryVideoURL, to: finalVideoURL)
            if hadImage { try? fileManager.removeItem(at: imageBackup) }
            if hadVideo { try? fileManager.removeItem(at: videoBackup) }
        } catch {
            if newImageInstalled { try? fileManager.removeItem(at: finalImageURL) }
            // A throwing move of the temporary video cannot have installed it successfully, and
            // there are no throwing operations after the successful video move. Therefore no
            // separate newVideoInstalled state is needed here.
            if hadImage, fileManager.fileExists(atPath: imageBackup.path) {
                try? fileManager.moveItem(at: imageBackup, to: finalImageURL)
            }
            if hadVideo, fileManager.fileExists(atPath: videoBackup.path) {
                try? fileManager.moveItem(at: videoBackup, to: finalVideoURL)
            }
            throw AppleLivePhotoError.transactionFailed(error.localizedDescription)
        }
    }

    private static func formatMicroseconds(_ value: Int64) -> String {
        String(format: "%.6f", Double(value) / 1_000_000.0)
    }
}

private final class LivePhotoConversionResultBox: @unchecked Sendable {
    private let lock = NSLock()
    private var stored: Result<LivePhotoConversionResult, Error>?
    var result: Result<LivePhotoConversionResult, Error> {
        lock.lock(); defer { lock.unlock() }
        return stored ?? .failure(AppleLivePhotoError.transactionFailed("conversion task did not produce a result"))
    }
    func set(_ value: Result<LivePhotoConversionResult, Error>) {
        lock.lock(); defer { lock.unlock() }; stored = value
    }
}
