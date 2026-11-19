import AVFoundation
import AudioToolbox
import CoreImage
import Foundation

struct LiveStyleVideoFacts: Encodable {
    let trackId: Int32
    let width: Int
    let height: Int
    let frames: [Time]
    let end: Time
    let aac: [AACFormat]

    struct AACFormat: Encodable {
        let trackId: Int32
        let framesPerPacket: UInt32
        let channelCount: UInt32
    }

    struct Time: Encodable {
        let value: Int64
        let timescale: Int32

        init(_ time: CMTime) {
            value = time.value
            timescale = time.timescale
        }
    }
}

/// Decode the original video in display order. Core Image supplies a linear
/// Rec.2020 observation in coded orientation; it does not choose a style.
func observeLiveStyleVideo(inputPath: String, outputPath: String) throws -> LiveStyleVideoFacts {
    func failure(_ message: String) -> NSError {
        NSError(domain: "XDRemuxAppleAdapter.LiveStyleVideo", code: 1,
                userInfo: [NSLocalizedDescriptionKey: message])
    }

    let asset = AVURLAsset(url: URL(fileURLWithPath: inputPath))
    let tracks = asset.tracks(withMediaType: .video).filter(\.isEnabled)
    guard tracks.count == 1, let track = tracks.first else {
        throw failure("expected exactly one enabled primary video track")
    }
    guard let space = CGColorSpace(name: CGColorSpace.extendedLinearITUR_2020) else {
        throw failure("extended linear Rec.2020 is unavailable")
    }
    let reader = try AVAssetReader(asset: asset)
    let frames = AVAssetReaderTrackOutput(track: track, outputSettings: [
        kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_64RGBAHalf,
    ])
    frames.alwaysCopiesSampleData = false
    guard reader.canAdd(frames) else { throw failure("cannot attach the video decoder") }
    reader.add(frames)
    guard reader.startReading() else { throw reader.error ?? failure("video decoder did not start") }
    defer { if reader.status == .reading { reader.cancelReading() } }

    guard FileManager.default.createFile(atPath: outputPath, contents: nil) else {
        throw failure("cannot create the linear video observation")
    }
    let file = try FileHandle(forWritingTo: URL(fileURLWithPath: outputPath))
    defer { try? file.close() }
    let context = CIContext(options: [
        .workingColorSpace: space, .cacheIntermediates: false,
    ])
    var stamps: [LiveStyleVideoFacts.Time] = []
    var width = 0, height = 0
    var previous = CMTime.invalid
    while let frame = frames.copyNextSampleBuffer() {
        try autoreleasepool {
            let time = frame.presentationTimeStamp
            guard time.isNumeric, !previous.isValid || time > previous,
                  let pixels = frame.imageBuffer else {
                throw failure("primary video has an invalid or non-increasing frame timestamp")
            }
            let image = CIImage(cvPixelBuffer: pixels)
            let extent = image.extent
            guard extent.width >= 2, extent.height >= 2 else { throw failure("invalid video geometry") }
            let scale = min(1, 128 / max(extent.width, extent.height))
            let targetWidth = max(2, Int((extent.width * scale / 2).rounded()) * 2)
            let targetHeight = max(2, Int((extent.height * scale / 2).rounded()) * 2)
            if stamps.isEmpty {
                width = targetWidth
                height = targetHeight
            }
            guard width == targetWidth, height == targetHeight else {
                throw failure("video dimensions change within the clip")
            }
            let fitted = image.transformed(by: CGAffineTransform(translationX: -extent.minX, y: -extent.minY))
                .transformed(by: CGAffineTransform(scaleX: CGFloat(width) / extent.width,
                                                   y: CGFloat(height) / extent.height))
            let bytesPerRow = width * 4 * MemoryLayout<Float>.size
            var linear = Data(count: bytesPerRow * height)
            linear.withUnsafeMutableBytes { bytes in
                context.render(fitted, toBitmap: bytes.baseAddress!, rowBytes: bytesPerRow,
                               bounds: CGRect(x: 0, y: 0, width: width, height: height),
                               format: .RGBAf, colorSpace: space)
            }
            try file.write(contentsOf: linear)
            stamps.append(LiveStyleVideoFacts.Time(time))
            previous = time
        }
    }
    let end = track.timeRange.end
    guard reader.status == .completed, !stamps.isEmpty, end.isNumeric, end > previous else {
        throw reader.error ?? failure("video returned no complete presentation timeline")
    }
    let aac = asset.tracks(withMediaType: .audio).compactMap { track -> LiveStyleVideoFacts.AACFormat? in
        guard track.formatDescriptions.count == 1,
              let description = track.formatDescriptions.first else { return nil }
        guard let asbd = CMAudioFormatDescriptionGetStreamBasicDescription(description as! CMAudioFormatDescription) else {
            return nil
        }
        let format = asbd.pointee
        guard format.mFormatID == kAudioFormatMPEG4AAC,
              format.mFramesPerPacket > 0, format.mChannelsPerFrame > 0 else { return nil }
        return LiveStyleVideoFacts.AACFormat(trackId: track.trackID, framesPerPacket: format.mFramesPerPacket,
                                            channelCount: format.mChannelsPerFrame)
    }
    return LiveStyleVideoFacts(trackId: track.trackID, width: width, height: height, frames: stamps,
                               end: LiveStyleVideoFacts.Time(end), aac: aac)
}
