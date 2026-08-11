import Foundation
@preconcurrency import AVFoundation
import CoreGraphics
import CoreMedia
import CoreVideo
import CryptoKit
import ImageIO
import UniformTypeIdentifiers
import XCTest
import XDRemuxAppleFeatures
@testable import XDRemuxCLI

final class MotionPhotoConvertIntegrationTests: XCTestCase {
    func testConvertAutoRoutesStandardMotionPhotoAndPreservesSourceJPEG() async throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-motion-convert-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        let cleanJPEG = directory.appendingPathComponent("clean.jpg")
        let sourceMP4 = directory.appendingPathComponent("motion.mp4")
        let motionJPEG = directory.appendingPathComponent("IMG_0001.jpg")
        try createJPEG(at: cleanJPEG, width: 64, height: 48)
        try await createH264Video(at: sourceMP4, width: 64, height: 48, frameCount: 8)
        let video = try Data(contentsOf: sourceMP4)
        try makeMotionPhoto(
            cleanJPEG: cleanJPEG,
            video: video,
            presentationTimestampUs: 100_000,
            outputURL: motionJPEG
        )
        let sourceDigest = SHA256.hash(data: try Data(contentsOf: motionJPEG))

        let handled = try MotionPhotoCLIIntegration.handleIfNeeded([
            "convert",
            "--input", motionJPEG.path,
        ])
        XCTAssertTrue(handled)

        let outputHEIC = directory.appendingPathComponent("IMG_0001.heic")
        let outputMOV = directory.appendingPathComponent("IMG_0001.mov")
        XCTAssertTrue(FileManager.default.fileExists(atPath: motionJPEG.path))
        XCTAssertTrue(FileManager.default.fileExists(atPath: outputHEIC.path))
        XCTAssertTrue(FileManager.default.fileExists(atPath: outputMOV.path))
        XCTAssertEqual(
            SHA256.hash(data: try Data(contentsOf: motionJPEG)),
            sourceDigest,
            "Motion Photo convert must never modify the source JPEG"
        )

        let report = try await AppleLivePhotoValidator.validate(
            imageURL: outputHEIC,
            videoURL: outputMOV,
            expectedStillImageTime: CMTime(value: 3, timescale: 30),
            sourceVideoURL: sourceMP4,
            sourceHadAudio: false,
            sourceHadGainMap: false,
            requirePhotoKitLoad: true
        )
        XCTAssertEqual(CMTimeCompare(report.stillImageTime, CMTime(value: 3, timescale: 30)), 0)
    }

    private func makeMotionPhoto(
        cleanJPEG: URL,
        video: Data,
        presentationTimestampUs: Int64,
        outputURL: URL
    ) throws {
        let original = try Data(contentsOf: cleanJPEG)
        guard original.count >= 2, original[0] == 0xff, original[1] == 0xd8 else {
            XCTFail("source fixture is not JPEG")
            return
        }
        let xmp = """
        <x:xmpmeta xmlns:x="adobe:ns:meta/">
          <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
            <rdf:Description xmlns:Camera="http://ns.google.com/photos/1.0/camera/"
                             xmlns:Container="http://ns.google.com/photos/1.0/container/"
                             xmlns:Item="http://ns.google.com/photos/1.0/container/item/"
                             Camera:MotionPhoto="1"
                             Camera:MotionPhotoVersion="1"
                             Camera:MotionPhotoPresentationTimestampUs="\(presentationTimestampUs)">
              <Container:Directory><rdf:Seq>
                <rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="image/jpeg" Item:Semantic="Primary" Item:Length="0" Item:Padding="0"/></rdf:li>
                <rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="video/mp4" Item:Semantic="MotionPhoto" Item:Length="\(video.count)" Item:Padding="0"/></rdf:li>
              </rdf:Seq></Container:Directory>
            </rdf:Description>
          </rdf:RDF>
        </x:xmpmeta>
        """
        var payload = Data("http://ns.adobe.com/xap/1.0/".utf8)
        payload.append(0)
        payload.append(Data(xmp.utf8))
        let segmentLength = payload.count + 2
        guard segmentLength <= Int(UInt16.max) else {
            XCTFail("Motion Photo test XMP is too large")
            return
        }
        var app1 = Data([0xff, 0xe1])
        app1.append(UInt8((segmentLength >> 8) & 0xff))
        app1.append(UInt8(segmentLength & 0xff))
        app1.append(payload)

        var output = Data(original.prefix(2))
        output.append(app1)
        output.append(original.dropFirst(2))
        output.append(video)
        try output.write(to: outputURL, options: .atomic)
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
        context.setFillColor(CGColor(red: 0.2, green: 0.4, blue: 0.6, alpha: 1))
        context.fill(CGRect(x: 0, y: 0, width: width, height: height))
        guard let image = context.makeImage() else {
            XCTFail("could not create JPEG image")
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
            XCTFail("H.264 encoder unavailable")
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
                XCTFail("could not allocate video pixel buffer")
                return
            }
            CVPixelBufferLockBaseAddress(pixelBuffer, [])
            if let base = CVPixelBufferGetBaseAddress(pixelBuffer) {
                memset(base, Int32(48 + index), CVPixelBufferGetBytesPerRow(pixelBuffer) * height)
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
