import Foundation
@preconcurrency import AVFoundation
import CoreGraphics
import CoreMedia
import CoreVideo
import ImageIO
import UniformTypeIdentifiers
import XCTest
import XDRemuxCore
@testable import XDRemuxAppleFeatures

final class LivePhotoWriterIntegrationTests: XCTestCase {
    func testSyntheticH264PairLoadsInPhotoKitAndPreservesCompressedVideo() async throws {
        try await assertSyntheticPair(codec: .h264, filename: "h264.mp4")
    }

    func testSyntheticHEVCPairLoadsInPhotoKitAndPreservesCompressedVideo() async throws {
        try await assertSyntheticPair(codec: .hevc, filename: "hevc.mp4")
    }

    func testHighPrecisionStillImageTimeValidatesAtStoredMetadataPrecision() async throws {
        try await assertSyntheticPair(
            codec: .h264,
            filename: "high-precision.mp4",
            stillTime: CMTime(value: 3_038, timescale: 30_000),
            requireExactStoredTime: false
        )
    }

    func testColorOS16AlignmentMetadataStillLoadsInPhotoKit() async throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-livephoto-oppo-transform-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        let sourceJPEG = directory.appendingPathComponent("source.jpg")
        let sourceMP4 = directory.appendingPathComponent("source.mp4")
        let outputHEIC = directory.appendingPathComponent("output.heic")
        let outputMOV = directory.appendingPathComponent("output.mov")
        try createJPEG(at: sourceJPEG, width: 80, height: 60)
        try await createCompressedVideo(
            at: sourceMP4,
            codec: .hevc,
            width: 64,
            height: 48,
            frameCount: 8
        )

        let identifier = UUID().uuidString
        let stillTime = CMTime(value: 2, timescale: 30)
        let metadata = OppoMotionPhotoMetadata(
            coverFramePtsUs: 66_667,
            version: 1,
            matrixCount: 0,
            photoCropMatrix: [1, 0, 0, 0, 1, 0, 0, 0, 1],
            photoEisMatrix: [1, 0, 0, 0, 1, 0, 0, 0, 1],
            videoWidth: 64,
            videoHeight: 48
        )
        XCTAssertNotNil(OppoLivePhotoAlignment.transformMatrix(for: metadata))

        try AppleLivePhotoStillWriter.write(
            stillInputURL: sourceJPEG,
            outputURL: outputHEIC,
            assetIdentifier: identifier
        )
        try await AppleLivePhotoVideoWriter.write(
            videoInputURL: sourceMP4,
            outputURL: outputMOV,
            assetIdentifier: identifier,
            stillImageTime: stillTime,
            oppoMetadata: metadata,
            stillImageReferenceDimensions: [80, 60]
        )

        let report = try await AppleLivePhotoValidator.validate(
            imageURL: outputHEIC,
            videoURL: outputMOV,
            expectedAssetIdentifier: identifier,
            expectedStillImageTime: stillTime,
            sourceStillURL: sourceJPEG,
            sourceVideoURL: sourceMP4,
            sourceHadAudio: false,
            sourceHadGainMap: false,
            expectsOppoTransform: true,
            requirePhotoKitLoad: true
        )
        XCTAssertTrue(report.hasTransform)
        XCTAssertTrue(report.hasTransformReferenceDimensions)
        XCTAssertFalse(report.vitalityTransformLimitingAllowed)

        let dimensions = try await readTransformReferenceDimensions(from: outputMOV)
        XCTAssertEqual(dimensions?.width ?? -1, 80, accuracy: 1e-6)
        XCTAssertEqual(dimensions?.height ?? -1, 60, accuracy: 1e-6)
    }

    func testVisionTransformWritesVitalityLimitingFlagWithoutChangingCompressedVideo() async throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-livephoto-vitality-limit-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        let sourceJPEG = directory.appendingPathComponent("source.jpg")
        let sourceMP4 = directory.appendingPathComponent("source.mp4")
        let outputHEIC = directory.appendingPathComponent("output.heic")
        let outputMOV = directory.appendingPathComponent("output.mov")
        try createJPEG(at: sourceJPEG, width: 80, height: 60)
        try await createCompressedVideo(
            at: sourceMP4,
            codec: .hevc,
            width: 64,
            height: 48,
            frameCount: 8
        )

        let identifier = UUID().uuidString
        let stillTime = CMTime(value: 2, timescale: 30)
        let transform = try XCTUnwrap(AppleLivePhotoStillTransform(
            matrix: [0.9, 0, 6, 0, 0.9, 4, 0, 0, 1],
            referenceDimensions: [64, 48],
            source: .colorOS16VisionTrajectory
        ))
        try AppleLivePhotoStillWriter.write(
            stillInputURL: sourceJPEG,
            outputURL: outputHEIC,
            assetIdentifier: identifier
        )
        try await AppleLivePhotoVideoWriter.write(
            videoInputURL: sourceMP4,
            outputURL: outputMOV,
            assetIdentifier: identifier,
            stillImageTime: stillTime,
            stillImageTransform: transform
        )

        let report = try await AppleLivePhotoValidator.validate(
            imageURL: outputHEIC,
            videoURL: outputMOV,
            expectedAssetIdentifier: identifier,
            expectedStillImageTime: stillTime,
            sourceStillURL: sourceJPEG,
            sourceVideoURL: sourceMP4,
            sourceHadAudio: false,
            sourceHadGainMap: false,
            expectsOppoTransform: true,
            expectedStillImageTransform: transform,
            requirePhotoKitLoad: true
        )
        XCTAssertTrue(report.vitalityTransformLimitingAllowed)
    }

    private func assertSyntheticPair(
        codec: AVVideoCodecType,
        filename: String,
        stillTime: CMTime = CMTime(value: 2, timescale: 30),
        requireExactStoredTime: Bool = true
    ) async throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-livephoto-integration-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        let sourceJPEG = directory.appendingPathComponent("source.jpg")
        let sourceMP4 = directory.appendingPathComponent(filename)
        let outputHEIC = directory.appendingPathComponent("output.heic")
        let outputMOV = directory.appendingPathComponent("output.mov")
        try createJPEG(at: sourceJPEG, width: 64, height: 48)
        try await createCompressedVideo(
            at: sourceMP4,
            codec: codec,
            width: 64,
            height: 48,
            frameCount: 8
        )

        let identifier = UUID().uuidString
        try AppleLivePhotoStillWriter.write(
            stillInputURL: sourceJPEG,
            outputURL: outputHEIC,
            assetIdentifier: identifier
        )
        try await AppleLivePhotoVideoWriter.write(
            videoInputURL: sourceMP4,
            outputURL: outputMOV,
            assetIdentifier: identifier,
            stillImageTime: stillTime
        )

        let report = try await AppleLivePhotoValidator.validate(
            imageURL: outputHEIC,
            videoURL: outputMOV,
            expectedAssetIdentifier: identifier,
            expectedStillImageTime: stillTime,
            sourceStillURL: sourceJPEG,
            sourceVideoURL: sourceMP4,
            sourceHadAudio: false,
            sourceHadGainMap: false,
            requirePhotoKitLoad: true
        )
        XCTAssertEqual(report.assetIdentifier, identifier)
        if requireExactStoredTime {
            XCTAssertEqual(CMTimeCompare(report.stillImageTime, stillTime), 0)
        } else {
            XCTAssertGreaterThan(report.stillImageTime.timescale, 0)
            let delta = abs(
                CMTimeGetSeconds(report.stillImageTime)
                    - CMTimeGetSeconds(stillTime)
            )
            XCTAssertLessThanOrEqual(
                delta,
                1.0 / Double(report.stillImageTime.timescale) + 1e-9
            )
        }
        XCTAssertFalse(report.hasAudio)
    }

    private func readTransformReferenceDimensions(from videoURL: URL) async throws -> CGSize? {
        let asset = AVURLAsset(url: videoURL)
        let metadataTracks = try await asset.loadTracks(withMediaType: .metadata)
        for track in metadataTracks {
            let reader = try AVAssetReader(asset: asset)
            let output = AVAssetReaderTrackOutput(track: track, outputSettings: nil)
            guard reader.canAdd(output) else { continue }
            reader.add(output)
            let adaptor = AVAssetReaderOutputMetadataAdaptor(assetReaderTrackOutput: output)
            guard reader.startReading() else { continue }

            while let group = adaptor.nextTimedMetadataGroup() {
                for item in group.items {
                    let key: String?
                    if let string = item.key as? String {
                        key = string
                    } else if let string = item.key as? NSString {
                        key = string as String
                    } else {
                        key = item.identifier?.rawValue.split(separator: "/").last.map(String.init)
                    }
                    guard key == "com.apple.quicktime.live-photo-still-image-transform-reference-dimensions" else {
                        continue
                    }
                    if let value = item.value as? NSValue {
                        return value.sizeValue
                    }
                }
            }
        }
        return nil
    }

    private func createJPEG(at url: URL, width: Int, height: Int) throws {
        let colorSpace = CGColorSpaceCreateDeviceRGB()
        guard let context = CGContext(
            data: nil,
            width: width,
            height: height,
            bitsPerComponent: 8,
            bytesPerRow: 0,
            space: colorSpace,
            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
        ) else {
            XCTFail("could not create CGContext")
            return
        }
        context.setFillColor(CGColor(red: 0.2, green: 0.4, blue: 0.6, alpha: 1.0))
        context.fill(CGRect(x: 0, y: 0, width: width, height: height))
        guard let image = context.makeImage(),
              let destination = CGImageDestinationCreateWithURL(
                url as CFURL,
                UTType.jpeg.identifier as CFString,
                1,
                nil
              ) else {
            XCTFail("could not create JPEG destination")
            return
        }
        CGImageDestinationAddImage(destination, image, nil)
        XCTAssertTrue(CGImageDestinationFinalize(destination))
    }

    private func createCompressedVideo(
        at url: URL,
        codec: AVVideoCodecType,
        width: Int,
        height: Int,
        frameCount: Int
    ) async throws {
        let writer = try AVAssetWriter(outputURL: url, fileType: .mp4)
        let input = AVAssetWriterInput(
            mediaType: .video,
            outputSettings: [
                AVVideoCodecKey: codec,
                AVVideoWidthKey: width,
                AVVideoHeightKey: height,
            ]
        )
        input.expectsMediaDataInRealTime = false
        let adaptor = AVAssetWriterInputPixelBufferAdaptor(
            assetWriterInput: input,
            sourcePixelBufferAttributes: [
                kCVPixelBufferPixelFormatTypeKey as String: Int(kCVPixelFormatType_32BGRA),
                kCVPixelBufferWidthKey as String: width,
                kCVPixelBufferHeightKey as String: height,
            ]
        )
        guard writer.canAdd(input) else {
            throw XCTSkip("\(codec.rawValue) encoder is unavailable on this macOS runner")
        }
        writer.add(input)
        guard writer.startWriting() else {
            if codec == .hevc {
                throw XCTSkip("HEVC encoder could not start on this macOS runner: \(writer.error?.localizedDescription ?? "unknown error")")
            }
            throw writer.error ?? AppleLivePhotoError.videoWriteFailed("synthetic compressed writer did not start")
        }
        writer.startSession(atSourceTime: .zero)

        for index in 0..<frameCount {
            while !input.isReadyForMoreMediaData {
                try await Task.sleep(nanoseconds: 1_000_000)
            }
            var pixelBuffer: CVPixelBuffer?
            let status = CVPixelBufferCreate(
                kCFAllocatorDefault,
                width,
                height,
                kCVPixelFormatType_32BGRA,
                [kCVPixelBufferIOSurfacePropertiesKey as String: [:]] as CFDictionary,
                &pixelBuffer
            )
            XCTAssertEqual(status, kCVReturnSuccess)
            guard let pixelBuffer else {
                XCTFail("could not allocate synthetic video pixel buffer")
                return
            }
            CVPixelBufferLockBaseAddress(pixelBuffer, [])
            if let base = CVPixelBufferGetBaseAddress(pixelBuffer) {
                let byteCount = CVPixelBufferGetBytesPerRow(pixelBuffer) * height
                memset(base, Int32(24 + index * 8), byteCount)
            }
            CVPixelBufferUnlockBaseAddress(pixelBuffer, [])
            XCTAssertTrue(
                adaptor.append(
                    pixelBuffer,
                    withPresentationTime: CMTime(value: CMTimeValue(index), timescale: 30)
                )
            )
        }

        input.markAsFinished()
        await writer.finishWriting()
        guard writer.status == .completed else {
            if codec == .hevc {
                throw XCTSkip("HEVC encoder failed on this macOS runner: \(writer.error?.localizedDescription ?? "unknown error")")
            }
            throw writer.error ?? AppleLivePhotoError.videoWriteFailed("synthetic compressed writer failed")
        }
    }
}
