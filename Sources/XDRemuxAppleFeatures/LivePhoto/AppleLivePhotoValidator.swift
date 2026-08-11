import Foundation
@preconcurrency import AVFoundation
import CoreMedia
import Photos
import XDRemuxCore

public struct AppleLivePhotoValidationReport: Sendable {
    public let assetIdentifier: String
    public let stillImageTime: CMTime
    public let hasAudio: Bool
    public let hasGainMap: Bool
    public let hasTransform: Bool
    public let hasTransformReferenceDimensions: Bool
}

public enum AppleLivePhotoValidator {
    private static let stillImageTimeKey = "com.apple.quicktime.still-image-time"
    private static let transformKey = "com.apple.quicktime.live-photo-still-image-transform"
    private static let transformReferenceDimensionsKey = "com.apple.quicktime.live-photo-still-image-transform-reference-dimensions"

    public static func validate(
        imageURL: URL,
        videoURL: URL,
        expectedAssetIdentifier: String? = nil,
        expectedStillImageTime: CMTime? = nil,
        sourceHadAudio: Bool? = nil,
        sourceHadGainMap: Bool? = nil,
        expectsOppoTransform: Bool = false,
        requirePhotoKitLoad: Bool = true
    ) async throws -> AppleLivePhotoValidationReport {
        guard FileManager.default.fileExists(atPath: imageURL.path) else {
            throw AppleLivePhotoError.pairValidationFailed("HEIC resource is missing")
        }
        guard FileManager.default.fileExists(atPath: videoURL.path) else {
            throw AppleLivePhotoError.pairValidationFailed("MOV resource is missing")
        }
        guard let imageIdentifier = AppleLivePhotoStillWriter.assetIdentifier(in: imageURL),
              !imageIdentifier.isEmpty else {
            throw AppleLivePhotoError.pairValidationFailed("HEIC MakerApple[17] asset identifier is missing")
        }
        if let expectedAssetIdentifier, imageIdentifier != expectedAssetIdentifier {
            throw AppleLivePhotoError.pairValidationFailed("HEIC asset identifier does not match the requested identifier")
        }

        let asset = AVURLAsset(url: videoURL)
        let videoTracks = try await asset.loadTracks(withMediaType: .video)
        guard !videoTracks.isEmpty else {
            throw AppleLivePhotoError.pairValidationFailed("MOV contains no video track")
        }
        let duration = try await asset.load(.duration)
        guard duration.isNumeric, duration > .zero else {
            throw AppleLivePhotoError.pairValidationFailed("MOV duration is invalid")
        }

        guard let videoIdentifier = await AppleLivePhotoVideoWriter.contentIdentifier(in: videoURL),
              videoIdentifier == imageIdentifier else {
            throw AppleLivePhotoError.pairValidationFailed("HEIC and MOV content identifiers do not match")
        }

        let timed = try await readTimedMetadata(from: asset)
        guard timed.stillImageTimes.count == 1 else {
            throw AppleLivePhotoError.pairValidationFailed(
                "MOV must contain exactly one still-image-time sample; found \(timed.stillImageTimes.count)"
            )
        }
        let stillImageTime = timed.stillImageTimes[0]
        guard stillImageTime >= .zero, stillImageTime <= duration else {
            throw AppleLivePhotoError.pairValidationFailed("still-image-time lies outside the MOV timeline")
        }
        if let expectedStillImageTime {
            let delta = abs(CMTimeGetSeconds(stillImageTime) - CMTimeGetSeconds(expectedStillImageTime))
            guard delta <= 0.001 else {
                throw AppleLivePhotoError.pairValidationFailed(
                    "still-image-time differs from the resolved source timestamp by \(delta) seconds"
                )
            }
        }

        let audioTracks = try await asset.loadTracks(withMediaType: .audio)
        let hasAudio = !audioTracks.isEmpty
        if sourceHadAudio == true, !hasAudio {
            throw AppleLivePhotoError.pairValidationFailed("source audio track was not preserved")
        }
        if sourceHadAudio == false, hasAudio {
            throw AppleLivePhotoError.pairValidationFailed("output unexpectedly gained an audio track")
        }

        let hasGainMap = AppleLivePhotoStillWriter.hasGainMap(imageURL)
        if sourceHadGainMap == true, !hasGainMap {
            throw AppleLivePhotoError.pairValidationFailed("source gain map was not preserved in the HEIC still")
        }

        if expectsOppoTransform, !timed.hasTransform {
            throw AppleLivePhotoError.pairValidationFailed("expected OPPO Live Photo transform metadata is missing")
        }
        if timed.hasTransform, !timed.hasTransformReferenceDimensions {
            throw AppleLivePhotoError.pairValidationFailed("Live Photo transform is missing reference dimensions")
        }

        if requirePhotoKitLoad {
            try validateWithPhotoKit(imageURL: imageURL, videoURL: videoURL)
        }

        return AppleLivePhotoValidationReport(
            assetIdentifier: imageIdentifier,
            stillImageTime: stillImageTime,
            hasAudio: hasAudio,
            hasGainMap: hasGainMap,
            hasTransform: timed.hasTransform,
            hasTransformReferenceDimensions: timed.hasTransformReferenceDimensions
        )
    }

    public static func isValidPair(imageURL: URL, videoURL: URL) -> Bool {
        let semaphore = DispatchSemaphore(value: 0)
        let box = ValidationResultBox()
        Task.detached {
            do {
                _ = try await validate(
                    imageURL: imageURL,
                    videoURL: videoURL,
                    requirePhotoKitLoad: false
                )
                box.set(true)
            } catch {
                box.set(false)
            }
            semaphore.signal()
        }
        guard semaphore.wait(timeout: .now() + 30) == .success else { return false }
        return box.value
    }

    private struct TimedMetadataSummary {
        var stillImageTimes: [CMTime] = []
        var hasTransform = false
        var hasTransformReferenceDimensions = false
    }

    private static func readTimedMetadata(from asset: AVAsset) async throws -> TimedMetadataSummary {
        let metadataTracks = try await asset.loadTracks(withMediaType: .metadata)
        var summary = TimedMetadataSummary()
        for track in metadataTracks {
            let reader = try AVAssetReader(asset: asset)
            let output = AVAssetReaderTrackOutput(track: track, outputSettings: nil)
            output.alwaysCopiesSampleData = false
            guard reader.canAdd(output) else { continue }
            reader.add(output)
            let adaptor = AVAssetReaderOutputMetadataAdaptor(assetReaderTrackOutput: output)
            guard reader.startReading() else { continue }
            while let group = adaptor.nextTimedMetadataGroup() {
                for item in group.items {
                    let key = metadataKey(item)
                    if key == stillImageTimeKey {
                        summary.stillImageTimes.append(group.timeRange.start)
                    } else if key == transformKey {
                        summary.hasTransform = true
                    } else if key == transformReferenceDimensionsKey {
                        summary.hasTransformReferenceDimensions = true
                    }
                }
            }
            if reader.status == .failed {
                throw AppleLivePhotoError.pairValidationFailed(
                    reader.error?.localizedDescription ?? "timed metadata track could not be read"
                )
            }
        }
        return summary
    }

    private static func metadataKey(_ item: AVMetadataItem) -> String? {
        if let key = item.key as? String { return key }
        if let key = item.key as? NSString { return key as String }
        if let identifier = item.identifier?.rawValue {
            if let slash = identifier.lastIndex(of: "/") {
                return String(identifier[identifier.index(after: slash)...])
            }
            return identifier
        }
        return nil
    }

    private static func validateWithPhotoKit(imageURL: URL, videoURL: URL) throws {
        let semaphore = DispatchSemaphore(value: 0)
        let box = PhotoKitValidationBox()
        PHLivePhoto.request(
            withResourceFileURLs: [imageURL, videoURL],
            placeholderImage: nil,
            targetSize: .zero,
            contentMode: .aspectFit
        ) { livePhoto, info in
            if (info[PHLivePhotoInfoIsDegradedKey] as? Bool) == true { return }
            if livePhoto != nil {
                box.set(.success(()))
            } else {
                let underlying = info[PHLivePhotoInfoErrorKey] as? Error
                box.set(.failure(AppleLivePhotoError.pairValidationFailed(
                    underlying?.localizedDescription
                        ?? "PhotoKit could not construct PHLivePhoto from the resource pair"
                )))
            }
            semaphore.signal()
        }
        guard semaphore.wait(timeout: .now() + 30) == .success else {
            throw AppleLivePhotoError.pairValidationFailed("PhotoKit validation timed out")
        }
        try box.result.get()
    }
}

private final class ValidationResultBox: @unchecked Sendable {
    private let lock = NSLock()
    private var stored = false
    var value: Bool { lock.lock(); defer { lock.unlock() }; return stored }
    func set(_ value: Bool) { lock.lock(); defer { lock.unlock() }; stored = value }
}

private final class PhotoKitValidationBox: @unchecked Sendable {
    private let lock = NSLock()
    private var stored: Result<Void, Error>?
    var result: Result<Void, Error> {
        lock.lock(); defer { lock.unlock() }
        return stored ?? .failure(AppleLivePhotoError.pairValidationFailed("PhotoKit validation produced no final result"))
    }
    func set(_ value: Result<Void, Error>) {
        lock.lock(); defer { lock.unlock() }
        guard stored == nil else { return }
        stored = value
    }
}
