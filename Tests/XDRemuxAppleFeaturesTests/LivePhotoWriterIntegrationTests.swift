import Foundation
@preconcurrency import AVFoundation
import CoreGraphics
import CoreMedia
import CoreVideo
import ImageIO
import UniformTypeIdentifiers
import XCTest
@testable import XDRemuxAppleFeatures

final class LivePhotoWriterIntegrationTests: XCTestCase {
    func testSyntheticPairLoadsInPhotoKitAndPreservesCompressedVideo() async throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-livephoto-integration-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        let sourceJPEG = directory.appendingPathComponent("source.jpg")
        let sourceMP4 = directory.appendingPathComponent("source.mp4")
        let outputHEIC = directory.appendingPathComponent("output.heic")
        let outputMOV = directory.appendingPathComponent("output.mov")
        try createJPEG(at: sourceJPEG, width: 64, height: 48)
        try await createH264Video(at: sourceMP4, width: 64, height: 48, frameCount: 8)

        let identifier = UUID().uuidString
        let stillTime = CMTime(value: 2, timescale: 30)
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
        XCTAssertEqual(CMTimeCompare(report.stillImageTime, stillTime), 0)
        XCTAssertFalse(report.hasAudio)
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

    private func createH264Video(
        at url: URL,
        width: Int,
        height: Int,
        frameCount: Int
    ) async throws {
        let writer = try AVAssetWriter(outputURL: url, fileType: .mp4)
        let input = AVAssetWriterInput(
            mediaType: .video,
            outputSettings: [
                AVVideoCodecKey: AVVideoCodecType.h264,
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
            XCTFail("H.264 writer input unavailable")
            return
        }
        writer.add(input)
        guard writer.startWriting() else {
            throw writer.error ?? AppleLivePhotoError.videoWriteFailed("synthetic H.264 writer did not start")
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
            throw writer.error ?? AppleLivePhotoError.videoWriteFailed("synthetic H.264 writer failed")
        }
    }
}
