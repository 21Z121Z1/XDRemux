import AVFoundation
import CoreGraphics
import CoreVideo
import Foundation
import ImageIO
import UniformTypeIdentifiers

// Audit-only source generator: geometric pixels, no fixture or camera input.
// The Python driver adds explicit synthetic Google Motion Photo XMP afterward.
@main
struct GenerateSyntheticMotion {
    static func main() throws {
        guard CommandLine.arguments.count == 2 else {
            throw NSError(domain: "PVTProbe", code: 1, userInfo: [NSLocalizedDescriptionKey: "expected owned output directory"])
        }
        let directory = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
        let width = 256, height = 256, fps: Int32 = 30, frames = 60
        let movie = directory.appendingPathComponent("synthetic.mp4")
        let writer = try AVAssetWriter(outputURL: movie, fileType: .mp4)
        let input = AVAssetWriterInput(mediaType: .video, outputSettings: [
            AVVideoCodecKey: AVVideoCodecType.h264,
            AVVideoWidthKey: width, AVVideoHeightKey: height,
            AVVideoCompressionPropertiesKey: [
                AVVideoAverageBitRateKey: 250_000,
                AVVideoMaxKeyFrameIntervalKey: 30,
                AVVideoProfileLevelKey: AVVideoProfileLevelH264MainAutoLevel,
            ],
        ])
        input.expectsMediaDataInRealTime = false
        guard writer.canAdd(input) else { throw failure("H264 writer input unavailable") }
        writer.add(input)
        let adapter = AVAssetWriterInputPixelBufferAdaptor(assetWriterInput: input, sourcePixelBufferAttributes: [
            kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA,
            kCVPixelBufferWidthKey as String: width,
            kCVPixelBufferHeightKey as String: height,
        ])
        guard writer.startWriting() else { throw writer.error ?? failure("startWriting failed") }
        writer.startSession(atSourceTime: .zero)
        let deadline = Date().addingTimeInterval(60)
        for frame in 0..<frames {
            while !input.isReadyForMoreMediaData {
                guard Date() < deadline, writer.status == .writing else {
                    throw writer.error ?? failure("writer readiness timeout")
                }
                Thread.sleep(forTimeInterval: 0.005)
            }
            var optional: CVPixelBuffer?
            let result = CVPixelBufferCreate(kCFAllocatorDefault, width, height, kCVPixelFormatType_32BGRA,
                [kCVPixelBufferCGImageCompatibilityKey: true, kCVPixelBufferCGBitmapContextCompatibilityKey: true] as CFDictionary, &optional)
            guard result == kCVReturnSuccess, let buffer = optional else { throw failure("pixel allocation failed") }
            CVPixelBufferLockBaseAddress(buffer, [])
            guard let context = CGContext(data: CVPixelBufferGetBaseAddress(buffer), width: width, height: height,
                bitsPerComponent: 8, bytesPerRow: CVPixelBufferGetBytesPerRow(buffer),
                space: CGColorSpace(name: CGColorSpace.sRGB)!,
                bitmapInfo: CGImageAlphaInfo.premultipliedFirst.rawValue | CGBitmapInfo.byteOrder32Little.rawValue) else {
                CVPixelBufferUnlockBaseAddress(buffer, [])
                throw failure("bitmap context unavailable")
            }
            context.setFillColor(CGColor(red: 0.08, green: 0.18, blue: 0.32, alpha: 1))
            context.fill(CGRect(x: 0, y: 0, width: CGFloat(width), height: CGFloat(height)))
            context.setFillColor(CGColor(red: 0.9, green: 0.65, blue: 0.12, alpha: 1))
            context.fill(CGRect(x: CGFloat(frame * 3 % 224), y: 64, width: 32, height: 128))
            context.setFillColor(CGColor(red: 0.1, green: 0.8, blue: 0.55, alpha: 1))
            context.fill(CGRect(x: 32, y: CGFloat(frame * 2 % 224), width: 128, height: 16))
            if frame == 30 {
                guard let image = context.makeImage(),
                    let destination = CGImageDestinationCreateWithURL(directory.appendingPathComponent("synthetic.jpg") as CFURL,
                        UTType.jpeg.identifier as CFString, 1, nil) else { throw failure("JPEG destination unavailable") }
                CGImageDestinationAddImage(destination, image, [kCGImageDestinationLossyCompressionQuality: 0.9] as CFDictionary)
                guard CGImageDestinationFinalize(destination) else { throw failure("JPEG encode failed") }
            }
            CVPixelBufferUnlockBaseAddress(buffer, [])
            guard adapter.append(buffer, withPresentationTime: CMTime(value: Int64(frame), timescale: fps)) else {
                throw writer.error ?? failure("video append failed")
            }
        }
        writer.endSession(atSourceTime: CMTime(value: Int64(frames), timescale: fps))
        input.markAsFinished()
        let finished = DispatchSemaphore(value: 0)
        writer.finishWriting { finished.signal() }
        guard finished.wait(timeout: .now() + 60) == .success, writer.status == .completed else {
            throw writer.error ?? failure("video finalization failed")
        }
        let report: [String: Any] = ["synthetic_only": true, "width": width, "height": height,
            "frames": frames, "fps": fps, "cover_time": ["value": 30, "timescale": fps], "audio": false]
        try JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys])
            .write(to: directory.appendingPathComponent("synthetic-source.json"), options: .atomic)
    }

    static func failure(_ message: String) -> NSError {
        NSError(domain: "PVTProbe", code: 1, userInfo: [NSLocalizedDescriptionKey: message])
    }
}
