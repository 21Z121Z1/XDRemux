import Foundation
@preconcurrency import AVFoundation
import CoreGraphics
import CoreMedia
import CoreVideo
import ImageIO
import UniformTypeIdentifiers
import XCTest
import XDRemuxAppleFeatures
@testable import XDRemuxCLI

final class MixedMotionPhotoBatchRoutingTests: XCTestCase {
    func testMixedHEICAndMotionPhotoDefersMotionOutputUntilAfterLegacyBatch() throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-mixed-motion-batch-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        // Only the filename/extension matters for planning the unchanged HEIC pass.
        try Data("not-a-real-heic".utf8).write(
            to: directory.appendingPathComponent("existing.heic"),
            options: .atomic
        )

        let motionURL = directory.appendingPathComponent("motion.jpg")
        try syntheticMotionPhoto().write(to: motionURL, options: .atomic)
        let motionOutput = directory.appendingPathComponent("motion.heic")

        let handled = try MotionPhotoCLIIntegration.handleIfNeeded([
            "batch",
            "--input-dir", directory.path,
        ])

        // False means XDRemuxCommand.main() must run the existing HEIC pass first. Crucially the
        // Motion Photo HEIC has not been written yet, so that pass cannot enumerate it as an input.
        XCTAssertFalse(handled)
        XCTAssertFalse(FileManager.default.fileExists(atPath: motionOutput.path))

        // Clear the queued plan. The deliberately minimal fake MP4 has no playable video track, so
        // the actual conversion is expected to fail after planning; the routing assertion above is
        // the behavior under test.
        XCTAssertThrowsError(try MotionPhotoCLIIntegration.finishPendingBatchIfNeeded())
        XCTAssertFalse(FileManager.default.fileExists(atPath: motionOutput.path))
    }

    func testRerunWithExistingLivePhotoStillNeverFallsThroughToLegacyHEICEnumerator() async throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-mixed-motion-rerun-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        let sourceJPEG = directory.appendingPathComponent("live-source.jpg")
        let sourceMP4 = directory.appendingPathComponent("live-source.mp4")
        let liveHEIC = directory.appendingPathComponent("already-converted.heic")
        let liveMOV = directory.appendingPathComponent("already-converted.mov")
        try createJPEG(at: sourceJPEG, width: 32, height: 24)
        try await createH264Video(at: sourceMP4, width: 32, height: 24, frameCount: 5)
        let identifier = UUID().uuidString
        let stillTime = CMTime(value: 1, timescale: 30)
        try AppleLivePhotoStillWriter.write(
            stillInputURL: sourceJPEG,
            outputURL: liveHEIC,
            assetIdentifier: identifier
        )
        try await AppleLivePhotoVideoWriter.write(
            videoInputURL: sourceMP4,
            outputURL: liveMOV,
            assetIdentifier: identifier,
            stillImageTime: stillTime
        )
        _ = try await AppleLivePhotoValidator.validate(
            imageURL: liveHEIC,
            videoURL: liveMOV,
            expectedAssetIdentifier: identifier,
            expectedStillImageTime: stillTime,
            requirePhotoKitLoad: true
        )

        let motionURL = directory.appendingPathComponent("motion.jpg")
        try syntheticMotionPhoto().write(to: motionURL, options: .atomic)

        // A repeated batch sees the valid paired HEIC as an output, not as a source HEIC. Because
        // the synthetic Motion Photo video intentionally has no playable AV track, the classified
        // Motion Photo pass fails immediately. If routing had fallen through to XDRemuxCommand's
        // legacy `*.heic` enumerator this call would return false instead of throwing here.
        XCTAssertThrowsError(
            try MotionPhotoCLIIntegration.handleIfNeeded([
                "batch",
                "--input-dir", directory.path,
            ])
        )
        XCTAssertNoThrow(try MotionPhotoCLIIntegration.finishPendingBatchIfNeeded())
        XCTAssertTrue(
            AppleLivePhotoValidator.isValidPair(imageURL: liveHEIC, videoURL: liveMOV),
            "the pre-existing Live Photo pair must remain untouched"
        )
    }

    private func syntheticMotionPhoto() -> Data {
        let video = fakeBMFF()
        let xmp = """
        <x:xmpmeta xmlns:x="adobe:ns:meta/">
          <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
            <rdf:Description xmlns:Camera="http://ns.google.com/photos/1.0/camera/"
                             xmlns:Container="http://ns.google.com/photos/1.0/container/"
                             xmlns:Item="http://ns.google.com/photos/1.0/container/item/"
                             Camera:MotionPhoto="1" Camera:MotionPhotoVersion="1">
              <Container:Directory><rdf:Seq>
                <rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="image/jpeg" Item:Semantic="Primary" Item:Length="0" Item:Padding="0"/></rdf:li>
                <rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="video/mp4" Item:Semantic="MotionPhoto" Item:Length="\(video.count)" Item:Padding="0"/></rdf:li>
              </rdf:Seq></Container:Directory>
            </rdf:Description>
          </rdf:RDF>
        </x:xmpmeta>
        """
        return Data([0xff, 0xd8]) + Data(xmp.utf8) + Data([0xff, 0xd9]) + video
    }

    private func fakeBMFF() -> Data {
        var data = Data([0, 0, 0, 16])
        data.append(Data("ftypisom".utf8))
        data.append(contentsOf: [0, 0, 0, 0])
        data.append(contentsOf: [0, 0, 0, 8])
        data.append(Data("mdat".utf8))
        return data
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
        ),
        let destination = CGImageDestinationCreateWithURL(
            url as CFURL,
            UTType.jpeg.identifier as CFString,
            1,
            nil
        ) else {
            XCTFail("could not create JPEG fixture")
            return
        }
        context.setFillColor(CGColor(red: 0.25, green: 0.5, blue: 0.75, alpha: 1))
        context.fill(CGRect(x: 0, y: 0, width: width, height: height))
        guard let image = context.makeImage() else {
            XCTFail("could not make JPEG fixture image")
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
        let adaptor = AVAssetWriterInputPixelBufferAdaptor(
            assetWriterInput: input,
            sourcePixelBufferAttributes: [
                kCVPixelBufferPixelFormatTypeKey as String: Int(kCVPixelFormatType_32BGRA),
                kCVPixelBufferWidthKey as String: width,
                kCVPixelBufferHeightKey as String: height,
            ]
        )
        guard writer.canAdd(input) else {
            XCTFail("could not add H.264 writer input")
            return
        }
        writer.add(input)
        guard writer.startWriting() else {
            throw writer.error ?? AppleLivePhotoError.videoWriteFailed("test H.264 writer did not start")
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
                XCTFail("could not allocate test pixel buffer")
                return
            }
            CVPixelBufferLockBaseAddress(pixelBuffer, [])
            if let base = CVPixelBufferGetBaseAddress(pixelBuffer) {
                memset(base, Int32(32 + index), CVPixelBufferGetBytesPerRow(pixelBuffer) * height)
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
            throw writer.error ?? AppleLivePhotoError.videoWriteFailed("test H.264 writer failed")
        }
    }
}
