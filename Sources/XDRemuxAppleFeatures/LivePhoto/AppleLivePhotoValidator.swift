import Foundation
import AppKit
@preconcurrency import AVFoundation
import CoreMedia
import CryptoKit
import ImageIO
import Photos
import XDRemuxCore

public struct AppleLivePhotoValidationReport: Sendable {
    public let assetIdentifier: String
    public let stillImageTime: CMTime
    public let hasAudio: Bool
    public let hasGainMap: Bool
    public let hasTransform: Bool
    public let hasTransformReferenceDimensions: Bool
    public let stillImageTransform: [Double]?
    public let stillImageTransformReferenceDimensions: [Float]?
    public let vitalityTransformLimitingAllowed: Bool
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
        sourceStillURL: URL? = nil,
        sourceVideoURL: URL? = nil,
        sourceHadAudio: Bool? = nil,
        sourceHadGainMap: Bool? = nil,
        expectsOppoTransform: Bool = false,
        expectedStillImageTransform: AppleLivePhotoStillTransform? = nil,
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
        if let sourceStillURL {
            try validateStillGeometry(source: sourceStillURL, output: imageURL)
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
        if let sourceVideoURL {
            try await validateCompressedPassthrough(sourceURL: sourceVideoURL, outputAsset: asset)
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
            // AVAssetWriter stores timed metadata on the MOV metadata track's integer timebase.
            // Real camera timestamps can have microsecond or video-track precision, so the exact
            // source PTS is not always representable after muxing. Validate at the precision of the
            // stored metadata timestamp rather than using an arbitrary fixed 1 ms threshold.
            let storedScale = stillImageTime.timescale
            guard storedScale > 0 else {
                throw AppleLivePhotoError.pairValidationFailed("still-image-time has an invalid timescale")
            }
            let quantizedExpected = CMTimeConvertScale(
                expectedStillImageTime,
                timescale: storedScale,
                method: .roundHalfAwayFromZero
            )
            let delta = abs(CMTimeGetSeconds(stillImageTime) - CMTimeGetSeconds(quantizedExpected))
            let oneStoredTick = 1.0 / Double(storedScale)
            guard delta <= oneStoredTick + 1e-9 else {
                throw AppleLivePhotoError.pairValidationFailed(
                    "still-image-time differs from the resolved source timestamp beyond one stored metadata tick: delta=\(delta) s, timescale=\(storedScale)"
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
        let vitalityTransformLimitingAllowed = await AppleLivePhotoVideoWriter
            .vitalityTransformLimitingAllowed(in: videoURL)
        if let expectedStillImageTransform {
            guard let storedMatrix = timed.transform,
                  approximatelyEqual(storedMatrix, expectedStillImageTransform.matrix, tolerance: 1e-9) else {
                throw AppleLivePhotoError.pairValidationFailed(
                    "stored Live Photo transform differs from the selected transform"
                )
            }
            guard let storedDimensions = timed.transformReferenceDimensions,
                  approximatelyEqual(
                    storedDimensions.map(Double.init),
                    expectedStillImageTransform.referenceDimensions.map(Double.init),
                    tolerance: 1e-6
                  ) else {
                throw AppleLivePhotoError.pairValidationFailed(
                    "stored Live Photo transform reference dimensions differ from the selected dimensions"
                )
            }
            if expectedStillImageTransform.source == .colorOS16VisionTrajectory,
               !vitalityTransformLimitingAllowed {
                throw AppleLivePhotoError.pairValidationFailed(
                    "Vision-selected Live Photo transform is missing the vitality limiting flag"
                )
            }
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
            hasTransformReferenceDimensions: timed.hasTransformReferenceDimensions,
            stillImageTransform: timed.transform,
            stillImageTransformReferenceDimensions: timed.transformReferenceDimensions,
            vitalityTransformLimitingAllowed: vitalityTransformLimitingAllowed
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
        var transform: [Double]?
        var transformReferenceDimensions: [Float]?
    }

    private struct StillGeometry: Equatable {
        let width: Int
        let height: Int
        let orientation: Int
    }

    private struct CompressedTrackFingerprint: Equatable {
        let mediaSubtype: FourCharCode
        let byteCount: Int64
        let sampleCount: Int64
        let sha256: String
    }

    private static func validateStillGeometry(source: URL, output: URL) throws {
        guard let sourceGeometry = stillGeometry(source),
              let outputGeometry = stillGeometry(output) else {
            throw AppleLivePhotoError.pairValidationFailed("could not read source/output still geometry")
        }
        guard sourceGeometry == outputGeometry else {
            throw AppleLivePhotoError.pairValidationFailed(
                "HEIC still geometry/orientation changed from \(sourceGeometry) to \(outputGeometry)"
            )
        }
    }

    private static func stillGeometry(_ url: URL) -> StillGeometry? {
        guard let source = CGImageSourceCreateWithURL(
            url as CFURL,
            [kCGImageSourceShouldCache: false] as CFDictionary
        ),
        let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any],
        let width = (properties[kCGImagePropertyPixelWidth] as? NSNumber)?.intValue,
        let height = (properties[kCGImagePropertyPixelHeight] as? NSNumber)?.intValue else {
            return nil
        }
        let orientation = (properties[kCGImagePropertyOrientation] as? NSNumber)?.intValue ?? 1
        return StillGeometry(width: width, height: height, orientation: orientation)
    }

    private static func validateCompressedPassthrough(
        sourceURL: URL,
        outputAsset: AVAsset
    ) async throws {
        let sourceAsset = AVURLAsset(url: sourceURL)
        let sourceVideo = try await compressedFingerprints(asset: sourceAsset, mediaType: .video)
        let outputVideo = try await compressedFingerprints(asset: outputAsset, mediaType: .video)
        guard sourceVideo == outputVideo else {
            throw AppleLivePhotoError.pairValidationFailed(
                "compressed video payload changed during passthrough: \(fingerprintSummary(sourceVideo)) -> \(fingerprintSummary(outputVideo))"
            )
        }

        let sourceAudio = try await compressedFingerprints(asset: sourceAsset, mediaType: .audio)
        let outputAudio = try await compressedFingerprints(asset: outputAsset, mediaType: .audio)
        guard sourceAudio == outputAudio else {
            throw AppleLivePhotoError.pairValidationFailed(
                "compressed audio payload changed during passthrough: \(fingerprintSummary(sourceAudio)) -> \(fingerprintSummary(outputAudio))"
            )
        }
    }

    /// Hashes exact compressed sample bytes directly from each track's storage ranges. This avoids
    /// relying on CMSampleBufferGetDataBuffer, which may legitimately be nil for samples represented
    /// by another backing object. AVSampleCursor exposes the encoded sample's storage URL, offset and
    /// length, so equality across MP4 -> MOV proves remuxing without a decode/re-encode step.
    private static func compressedFingerprints(
        asset: AVAsset,
        mediaType: AVMediaType
    ) async throws -> [CompressedTrackFingerprint] {
        let tracks = try await asset.loadTracks(withMediaType: mediaType)
        var fingerprints: [CompressedTrackFingerprint] = []
        fingerprints.reserveCapacity(tracks.count)

        for track in tracks {
            guard let cursor = track.makeSampleCursorAtFirstSampleInDecodeOrder() else {
                throw AppleLivePhotoError.pairValidationFailed("media track cannot provide a sample cursor")
            }
            let descriptions = try await track.load(.formatDescriptions)
            guard let description = descriptions.first else {
                throw AppleLivePhotoError.pairValidationFailed("media track has no format description")
            }

            var hasher = SHA256()
            var byteCount: Int64 = 0
            var sampleCount: Int64 = 0
            var handles: [URL: FileHandle] = [:]
            defer {
                for handle in handles.values { try? handle.close() }
            }

            while true {
                let range = cursor.currentSampleStorageRange
                guard range.offset >= 0, range.length > 0 else {
                    throw AppleLivePhotoError.pairValidationFailed(
                        "compressed media sample is not stored contiguously"
                    )
                }
                guard let storageURL = cursor.currentChunkStorageURL else {
                    throw AppleLivePhotoError.pairValidationFailed(
                        "compressed media sample has no storage URL"
                    )
                }
                let standardizedURL = storageURL.standardizedFileURL
                let handle: FileHandle
                if let existing = handles[standardizedURL] {
                    handle = existing
                } else {
                    let created = try FileHandle(forReadingFrom: standardizedURL)
                    handles[standardizedURL] = created
                    handle = created
                }
                try handle.seek(toOffset: UInt64(range.offset))

                var remaining = range.length
                while remaining > 0 {
                    let request = Int(min(remaining, 1 << 20))
                    guard let chunk = try handle.read(upToCount: request), !chunk.isEmpty else {
                        throw AppleLivePhotoError.pairValidationFailed(
                            "compressed media sample storage is truncated"
                        )
                    }
                    hasher.update(data: chunk)
                    byteCount += Int64(chunk.count)
                    remaining -= Int64(chunk.count)
                }
                sampleCount += 1

                if cursor.stepInDecodeOrder(byCount: 1) != 1 { break }
            }

            fingerprints.append(
                CompressedTrackFingerprint(
                    mediaSubtype: CMFormatDescriptionGetMediaSubType(description),
                    byteCount: byteCount,
                    sampleCount: sampleCount,
                    sha256: hasher.finalize().map { String(format: "%02x", $0) }.joined()
                )
            )
        }
        return fingerprints
    }

    private static func fingerprintSummary(_ fingerprints: [CompressedTrackFingerprint]) -> String {
        fingerprints.map {
            "\(fourCC($0.mediaSubtype))/\($0.sampleCount)/\($0.byteCount)/\($0.sha256.prefix(12))"
        }.joined(separator: ",")
    }

    private static func fourCC(_ code: FourCharCode) -> String {
        let bytes: [UInt8] = [
            UInt8((code >> 24) & 0xff),
            UInt8((code >> 16) & 0xff),
            UInt8((code >> 8) & 0xff),
            UInt8(code & 0xff),
        ]
        return String(bytes: bytes, encoding: .macOSRoman) ?? String(format: "0x%08x", code)
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
                        if let values = try? await item.load(.value) as? [NSNumber] {
                            summary.transform = values.map(\.doubleValue)
                        } else if let values = try? await item.load(.value) as? NSArray {
                            summary.transform = values.compactMap { ($0 as? NSNumber)?.doubleValue }
                        }
                    } else if key == transformReferenceDimensionsKey {
                        summary.hasTransformReferenceDimensions = true
                        if let value = try? await item.load(.value) as? NSValue {
                            let size = value.sizeValue
                            summary.transformReferenceDimensions = [Float(size.width), Float(size.height)]
                        }
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

    private static func approximatelyEqual(
        _ lhs: [Double],
        _ rhs: [Double],
        tolerance: Double
    ) -> Bool {
        lhs.count == rhs.count
            && zip(lhs, rhs).allSatisfy { abs($0 - $1) <= tolerance }
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

    private static var photoKitValidationTimeout: TimeInterval {
    guard let raw = ProcessInfo.processInfo.environment["XDREMUX_PHOTOKIT_VALIDATION_TIMEOUT_SECONDS"],
          let seconds = Double(raw), seconds.isFinite, seconds >= 1 else {
        return 30
    }
    return min(seconds, 300)
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
        let timeout = photoKitValidationTimeout
        guard semaphore.wait(timeout: .now() + timeout) == .success else {
            throw AppleLivePhotoError.pairValidationFailed(
                "PhotoKit validation timed out after \(Int(timeout)) seconds"
            )
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
