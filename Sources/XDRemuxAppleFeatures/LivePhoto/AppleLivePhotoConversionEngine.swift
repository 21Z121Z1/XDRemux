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
        requirePhotoKitValidation: Bool = true,
        photographicStylesConfiguration: ConversionConfiguration? = nil
    ) throws -> LivePhotoConversionResult {
        let semaphore = DispatchSemaphore(value: 0)
        let box = LivePhotoConversionResultBox()
        Task.detached(priority: .userInitiated) {
            do {
                let value = try await convertAsync(
                    inputURL: inputURL,
                    outputImageURL: outputImageURL,
                    requirePhotoKitValidation: requirePhotoKitValidation,
                    photographicStylesConfiguration: photographicStylesConfiguration
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
        requirePhotoKitValidation: Bool = true,
        photographicStylesConfiguration: ConversionConfiguration? = nil
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

        let stylesConfiguration: ConversionConfiguration?
        if let configuration = photographicStylesConfiguration, configuration.appleFeaturesEnabled {
            guard configuration.applePhotographicStyles, !configuration.applePortrait else {
                throw AppleLivePhotoError.transactionFailed(
                    "Live Photo conversion currently supports Photographic Styles but not Apple Portrait in the same pass"
                )
            }
            guard !configuration.oppoCompatibility.wantsOppoCompat else {
                throw AppleLivePhotoError.transactionFailed(
                    "Live Photo Photographic Styles output cannot use OPPO-compatible HDR mode"
                )
            }
            stylesConfiguration = configuration
        } else {
            stylesConfiguration = nil
        }

        let outputVideoURL = companionVideoURL(for: outputImageURL)
        let outputDirectory = outputImageURL.deletingLastPathComponent()
        try FileManager.default.createDirectory(
            at: outputDirectory,
            withIntermediateDirectories: true
        )
        try LivePhotoPairPublisher.reconcile(
            finalImageURL: outputImageURL,
            finalVideoURL: outputVideoURL,
            validatePair: { image, video in
                AppleLivePhotoValidator.isValidPair(imageURL: image, videoURL: video)
            }
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

        let streamLayout = try MotionPhotoVideoStreamLayoutResolver.resolve(for: asset)
        try MotionPhotoPayloadExtractor.copy(
            range: streamLayout.primary.range,
            from: inputURL,
            to: videoSourceURL
        )
        let geometryPlan = try VendorLivePhotoGeometryPolicy.plan(
            for: asset,
            stillResourceURL: stillSourceURL
        )
        var geometryPreparationDiagnostics: [String] = []
        let auxiliaryVideoURL: URL?
        if let auxiliary = geometryPlan?.streamLayout.auxiliaryGeometry.first {
            let extractedURL = scratch.appendingPathComponent("auxiliary-geometry.mp4")
            do {
                try MotionPhotoPayloadExtractor.copy(
                    range: auxiliary.range,
                    from: inputURL,
                    to: extractedURL
                )
                auxiliaryVideoURL = extractedURL
            } catch {
                auxiliaryVideoURL = nil
                geometryPreparationDiagnostics.append(
                    "Auxiliary geometry extraction failed; retained the metadata-only fallback: \(error.localizedDescription)"
                )
            }
        } else {
            auxiliaryVideoURL = nil
        }

        let timeline = try await AppleLivePhotoTimelineResolver.resolve(
            videoURL: videoSourceURL,
            requestedTimestampUs: asset.presentationTimestampUs,
            requestedSource: asset.presentationSource
        )
        let assetIdentifier = UUID().uuidString
        let publicationID = UUID().uuidString
        let directory = outputImageURL.deletingLastPathComponent()
        let stem = outputImageURL.deletingPathExtension().lastPathComponent
        let temporaryImageURL = directory.appendingPathComponent(".\(stem).\(publicationID).tmp.heic")
        let styledTemporaryImageURL = directory.appendingPathComponent(
            ".\(stem).\(publicationID).tmp.styles.heic"
        )
        let failedStylesImageURL = directory.appendingPathComponent("failed-styles.heic")
        let temporaryVideoURL = directory.appendingPathComponent(".\(stem).\(publicationID).tmp.mov")
        defer {
            try? FileManager.default.removeItem(at: temporaryImageURL)
            try? FileManager.default.removeItem(at: styledTemporaryImageURL)
            try? FileManager.default.removeItem(at: temporaryVideoURL)
        }

        let sourceHadGainMap = AppleLivePhotoStillWriter.hasGainMap(stillSourceURL)
        let sourceAsset = AVURLAsset(url: videoSourceURL)
        let sourceHadAudio = !(try await sourceAsset.loadTracks(withMediaType: .audio)).isEmpty
        let transformResolution = await VendorLivePhotoStillTransformResolver.resolve(.init(
            asset: asset,
            geometryPlan: geometryPlan,
            primaryVideoURL: videoSourceURL,
            auxiliaryVideoURL: auxiliaryVideoURL,
            stillImageURL: stillSourceURL,
            stillImageTime: timeline.stillImageTime
        ))

        try AppleLivePhotoStillWriter.write(
            stillInputURL: stillSourceURL,
            outputURL: temporaryImageURL,
            assetIdentifier: assetIdentifier,
            lossyCompressionQuality: stylesConfiguration == nil ? nil : 1.0
        )

        let pairedImageURL: URL
        if let stylesConfiguration {
            try? FileManager.default.removeItem(at: failedStylesImageURL)
            do {
                try AppleFeatureConversionEngine.convert(
                    inputURL: temporaryImageURL,
                    outputURL: styledTemporaryImageURL,
                    configuration: stylesConfiguration
                )
            } catch {
                if FileManager.default.fileExists(atPath: styledTemporaryImageURL.path) {
                    try? FileManager.default.copyItem(
                        at: styledTemporaryImageURL,
                        to: failedStylesImageURL
                    )
                }
                throw error
            }
            let stylesReport = try AppleFeatureConversionEngine.validationReport(
                for: styledTemporaryImageURL,
                expectsPortrait: false
            )
            guard stylesReport["passed"] as? Bool == true,
                  AppleLivePhotoStillWriter.assetIdentifier(in: styledTemporaryImageURL) == assetIdentifier else {
                throw AppleLivePhotoError.transactionFailed(
                    "Photographic Styles generation did not preserve a valid Live Photo still identity"
                )
            }
            pairedImageURL = styledTemporaryImageURL
        } else {
            pairedImageURL = temporaryImageURL
        }

        try await AppleLivePhotoVideoWriter.write(
            videoInputURL: videoSourceURL,
            outputURL: temporaryVideoURL,
            assetIdentifier: assetIdentifier,
            stillImageTime: timeline.stillImageTime,
            oppoMetadata: asset.vendorMetadata,
            stillImageTransform: transformResolution.transform
        )

        let expectedTransform = transformResolution.transform != nil
        _ = try await AppleLivePhotoValidator.validate(
            imageURL: pairedImageURL,
            videoURL: temporaryVideoURL,
            expectedAssetIdentifier: assetIdentifier,
            expectedStillImageTime: timeline.stillImageTime,
            sourceStillURL: stillSourceURL,
            sourceVideoURL: videoSourceURL,
            sourceHadAudio: sourceHadAudio,
            sourceHadGainMap: sourceHadGainMap,
            expectsOppoTransform: expectedTransform,
            expectedStillImageTransform: transformResolution.transform,
            requirePhotoKitLoad: requirePhotoKitValidation
        )

        try LivePhotoPairPublisher.publish(
            temporaryImageURL: pairedImageURL,
            temporaryVideoURL: temporaryVideoURL,
            finalImageURL: outputImageURL,
            finalVideoURL: outputVideoURL
        )
        try? FileManager.default.removeItem(at: failedStylesImageURL)

        var diagnostics: [String] = []
        diagnostics.append(contentsOf: geometryPreparationDiagnostics)
        diagnostics.append(contentsOf: transformResolution.diagnostics)
        if stylesConfiguration != nil {
            diagnostics.append(
                "Photographic Styles generated on the converted Live Photo still and validated through NeutrinoCore before publication."
            )
        }
        if let xmp = asset.presentationTimestampUs,
           let cover = asset.vendorMetadata?.coverFramePtsUs,
           xmp != cover {
            diagnostics.append(
                "XMP still time \(formatMicroseconds(xmp)) s differs from OPPO coverFramePts \(formatMicroseconds(cover)) s; selected \(timeline.source.rawValue)."
            )
        }
        if let geometryPlan {
            switch geometryPlan.kind {
            case .colorOS16:
                diagnostics.append(
                    "ColorOS 16 geometry scope enabled: Stream 1 is the paired MOV and \(geometryPlan.streamLayout.auxiliaryGeometry.count) auxiliary stream(s) remain analysis-only."
                )
            case .samsung:
                diagnostics.append(
                    "Samsung geometry scope enabled: semantic Motion Photo video remains the only paired stream; vendor BMFF/SEFD regions are not treated as auxiliary video."
                )
            }
        } else if let metadata = asset.vendorMetadata, metadata.streamCount >= 2 {
            diagnostics.append("OPPO multi-stream input detected outside the ColorOS 16 geometry policy; selected the semantic primary stream only.")
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
