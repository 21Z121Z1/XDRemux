import CoreImage
import Foundation
@preconcurrency import AVFoundation
import ImageIO

extension ColorOS16VisionCoverAlignmentAnalyzer {
    static func loadInputs(
        primaryVideoURL: URL,
        auxiliaryVideoURL: URL,
        stillImageURL: URL,
        stillImageTime: CMTime,
        referenceDimensions: [Float]
    ) async throws -> Inputs {
        guard referenceDimensions.count == 2,
              referenceDimensions.allSatisfy({ $0.isFinite && $0 > 0 }) else {
            throw ColorOS16VisionCoverAlignmentError.insufficientCoverEvidence
        }
        guard let still = loadStill(stillImageURL) else {
            throw ColorOS16VisionCoverAlignmentError.missingImage
        }
        let coverSeconds = CMTimeGetSeconds(stillImageTime)
        guard coverSeconds.isFinite else {
            throw ColorOS16VisionCoverAlignmentError.insufficientCoverEvidence
        }
        return try await Inputs(
            primary: readFrames(primaryVideoURL),
            auxiliary: readFrames(auxiliaryVideoURL),
            still: still,
            coverSeconds: coverSeconds,
            referenceDimensions: referenceDimensions
        )
    }

    private static func loadStill(_ url: URL) -> CGImage? {
        guard let source = CGImageSourceCreateWithURL(url as CFURL, nil) else { return nil }
        let options: [CFString: Any] = [
            kCGImageSourceCreateThumbnailFromImageAlways: true,
            kCGImageSourceThumbnailMaxPixelSize: maximumAnalysisDimension,
            kCGImageSourceCreateThumbnailWithTransform: true
        ]
        return CGImageSourceCreateThumbnailAtIndex(source, 0, options as CFDictionary)
    }

    private static func readFrames(_ url: URL) async throws -> VideoFrames {
        let asset = AVURLAsset(url: url)
        guard let track = try await asset.loadTracks(withMediaType: .video).first else {
            throw ColorOS16VisionCoverAlignmentError.missingVideo
        }
        let preferredTransform = try await track.load(.preferredTransform)
        let reader = try AVAssetReader(asset: asset)
        let output = AVAssetReaderTrackOutput(track: track, outputSettings: [
            kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA
        ])
        output.alwaysCopiesSampleData = false
        guard reader.canAdd(output) else {
            throw ColorOS16VisionCoverAlignmentError.missingVideo
        }
        reader.add(output)
        guard reader.startReading() else {
            throw ColorOS16VisionCoverAlignmentError.missingVideo
        }

        let context = CIContext(options: [.cacheIntermediates: false])
        var frames: [Frame] = []
        var index = 0
        while let sample = output.copyNextSampleBuffer() {
            defer { index += 1 }
            guard let buffer = CMSampleBufferGetImageBuffer(sample),
                  let image = displayImage(
                    buffer,
                    preferredTransform: preferredTransform,
                    context: context
                  ) else { continue }
            let pts = CMTimeGetSeconds(CMSampleBufferGetPresentationTimeStamp(sample))
            guard pts.isFinite else { continue }
            frames.append(Frame(index: index, ptsSeconds: pts, image: image))
        }
        guard !frames.isEmpty, reader.status != .failed else {
            throw ColorOS16VisionCoverAlignmentError.missingVideo
        }
        let loadedDuration = CMTimeGetSeconds(try await asset.load(.duration))
        let duration = loadedDuration.isFinite ? loadedDuration : (frames.last?.ptsSeconds ?? 0)
        return VideoFrames(frames: frames, durationSeconds: duration)
    }

    private static func displayImage(
        _ buffer: CVPixelBuffer,
        preferredTransform: CGAffineTransform,
        context: CIContext
    ) -> CGImage? {
        var image = CIImage(cvPixelBuffer: buffer).transformed(by: preferredTransform)
        let transformedExtent = image.extent
        image = image.transformed(by: CGAffineTransform(
            translationX: -transformedExtent.minX,
            y: -transformedExtent.minY
        ))
        let largest = max(image.extent.width, image.extent.height)
        guard largest > 0 else { return nil }
        let scale = min(1, CGFloat(maximumAnalysisDimension) / largest)
        if scale < 1 {
            image = image.transformed(by: CGAffineTransform(scaleX: scale, y: scale))
        }
        return context.createCGImage(image, from: image.extent)
    }
}
