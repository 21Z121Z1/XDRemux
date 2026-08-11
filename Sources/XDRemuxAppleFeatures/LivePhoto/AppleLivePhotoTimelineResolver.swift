import Foundation
@preconcurrency import AVFoundation
import CoreMedia
import XDRemuxCore

public struct ResolvedLivePhotoTimeline: Sendable {
    public let stillImageTime: CMTime
    public let source: MotionPhotoPresentationSource
    public let requestedTimestampUs: Int64?

    public var stillImageTimeSeconds: Double { CMTimeGetSeconds(stillImageTime) }
}

public enum AppleLivePhotoTimelineResolver {
    public static func resolve(
        videoURL: URL,
        requestedTimestampUs: Int64?,
        requestedSource: MotionPhotoPresentationSource?
    ) async throws -> ResolvedLivePhotoTimeline {
        let asset = AVURLAsset(url: videoURL)
        guard let videoTrack = try await asset.loadTracks(withMediaType: .video).first else {
            throw AppleLivePhotoError.missingVideoTrack
        }
        let duration = try await asset.load(.duration)
        guard duration.isNumeric, duration > .zero else { throw AppleLivePhotoError.invalidTimeline }

        let timestamps = try samplePresentationTimes(asset: asset, track: videoTrack)
        guard !timestamps.isEmpty else { throw AppleLivePhotoError.invalidTimeline }

        if let requestedTimestampUs {
            let requested = CMTime(value: requestedTimestampUs, timescale: 1_000_000)
            guard requested >= .zero, requested <= duration else { throw AppleLivePhotoError.invalidTimeline }
            let selected = timestamps.min { lhs, rhs in
                absoluteDistance(lhs, requested) < absoluteDistance(rhs, requested)
            } ?? requested
            return ResolvedLivePhotoTimeline(
                stillImageTime: selected,
                source: requestedSource ?? .androidXMP,
                requestedTimestampUs: requestedTimestampUs
            )
        }

        let midpoint = CMTimeMultiplyByFloat64(duration, multiplier: 0.5)
        var closestIndex = 0
        var closestDistance = absoluteDistance(timestamps[0], midpoint)
        if timestamps.count > 1 {
            for index in 1..<timestamps.count {
                let distance = absoluteDistance(timestamps[index], midpoint)
                if distance < closestDistance {
                    closestDistance = distance
                    closestIndex = index
                }
            }
        }
        // Android Motion Photo 1.0 specifies the presentation timestamp immediately preceding
        // the timestamp closest to the middle of the video track.
        let selectedIndex = max(0, closestIndex - 1)
        return ResolvedLivePhotoTimeline(
            stillImageTime: timestamps[selectedIndex],
            source: .timelineFallback,
            requestedTimestampUs: nil
        )
    }

    private static func samplePresentationTimes(asset: AVAsset, track: AVAssetTrack) throws -> [CMTime] {
        let reader = try AVAssetReader(asset: asset)
        let output = AVAssetReaderTrackOutput(track: track, outputSettings: nil)
        output.alwaysCopiesSampleData = false
        guard reader.canAdd(output) else { throw AppleLivePhotoError.cannotCreateVideoReader }
        reader.add(output)
        guard reader.startReading() else {
            throw AppleLivePhotoError.cannotStartVideoReader(reader.error?.localizedDescription ?? "unknown error")
        }

        var timestamps: [CMTime] = []
        timestamps.reserveCapacity(256)
        while let sample = output.copyNextSampleBuffer() {
            let pts = CMSampleBufferGetPresentationTimeStamp(sample)
            if pts.isNumeric, pts >= .zero { timestamps.append(pts) }
        }
        guard reader.status == .completed || reader.status == .reading else {
            throw AppleLivePhotoError.cannotStartVideoReader(reader.error?.localizedDescription ?? "sample scan failed")
        }
        return timestamps
    }

    private static func absoluteDistance(_ lhs: CMTime, _ rhs: CMTime) -> Double {
        abs(CMTimeGetSeconds(lhs) - CMTimeGetSeconds(rhs))
    }
}
