import Foundation
import AudioToolbox
@preconcurrency import AVFoundation
import CoreGraphics
import CoreMedia
import CoreVideo
import ImageIO
import UniformTypeIdentifiers
import XCTest
@testable import XDRemuxAppleFeatures

final class LivePhotoAudioPassthroughTests: XCTestCase {
    func testAACAudioTrackSurvivesLivePhotoRemux() async throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-livephoto-audio-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        let sourceJPEG = directory.appendingPathComponent("source.jpg")
        let sourceMP4 = directory.appendingPathComponent("source-with-audio.mp4")
        let outputHEIC = directory.appendingPathComponent("output.heic")
        let outputMOV = directory.appendingPathComponent("output.mov")
        try createJPEG(at: sourceJPEG)
        try await createH264AACVideo(at: sourceMP4)

        let sourceAsset = AVURLAsset(url: sourceMP4)
        XCTAssertEqual(try await sourceAsset.loadTracks(withMediaType: .video).count, 1)
        XCTAssertEqual(try await sourceAsset.loadTracks(withMediaType: .audio).count, 1)

        let identifier = UUID().uuidString
        let stillTime = CMTime(value: 3, timescale: 30)
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
            sourceHadAudio: true,
            sourceHadGainMap: false,
            requirePhotoKitLoad: true
        )
        XCTAssertTrue(report.hasAudio)
    }

    private func createJPEG(at url: URL) throws {
        let colorSpace = CGColorSpaceCreateDeviceRGB()
        guard let context = CGContext(
            data: nil,
            width: 64,
            height: 48,
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
        context.setFillColor(CGColor(red: 0.4, green: 0.3, blue: 0.2, alpha: 1))
        context.fill(CGRect(x: 0, y: 0, width: 64, height: 48))
        guard let image = context.makeImage() else {
            XCTFail("could not create JPEG fixture image")
            return
        }
        CGImageDestinationAddImage(destination, image, nil)
        XCTAssertTrue(CGImageDestinationFinalize(destination))
    }

    private func createH264AACVideo(at url: URL) async throws {
        let width = 64
        let height = 48
        let frameCount = 10
        let audioSampleRate = 44_100
        let audioFramesPerBuffer = 1_024
        let audioBufferCount = 15

        let writer = try AVAssetWriter(outputURL: url, fileType: .mp4)
        let videoInput = AVAssetWriterInput(
            mediaType: .video,
            outputSettings: [
                AVVideoCodecKey: AVVideoCodecType.h264,
                AVVideoWidthKey: width,
                AVVideoHeightKey: height,
            ]
        )
        let videoAdaptor = AVAssetWriterInputPixelBufferAdaptor(
            assetWriterInput: videoInput,
            sourcePixelBufferAttributes: [
                kCVPixelBufferPixelFormatTypeKey as String: Int(kCVPixelFormatType_32BGRA),
                kCVPixelBufferWidthKey as String: width,
                kCVPixelBufferHeightKey as String: height,
            ]
        )
        let audioInput = AVAssetWriterInput(
            mediaType: .audio,
            outputSettings: [
                AVFormatIDKey: kAudioFormatMPEG4AAC,
                AVSampleRateKey: audioSampleRate,
                AVNumberOfChannelsKey: 1,
                AVEncoderBitRateKey: 64_000,
            ]
        )
        guard writer.canAdd(videoInput), writer.canAdd(audioInput) else {
            throw XCTSkip("H.264/AAC encoder inputs are unavailable on this macOS runner")
        }
        writer.add(videoInput)
        writer.add(audioInput)
        guard writer.startWriting() else {
            throw writer.error ?? AppleLivePhotoError.videoWriteFailed("test H.264/AAC writer did not start")
        }
        writer.startSession(atSourceTime: .zero)

        for index in 0..<frameCount {
            while !videoInput.isReadyForMoreMediaData {
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
                XCTFail("could not allocate source video pixel buffer")
                return
            }
            CVPixelBufferLockBaseAddress(pixelBuffer, [])
            if let base = CVPixelBufferGetBaseAddress(pixelBuffer) {
                memset(base, Int32(64 + index), CVPixelBufferGetBytesPerRow(pixelBuffer) * height)
            }
            CVPixelBufferUnlockBaseAddress(pixelBuffer, [])
            XCTAssertTrue(
                videoAdaptor.append(
                    pixelBuffer,
                    withPresentationTime: CMTime(value: CMTimeValue(index), timescale: 30)
                )
            )
        }
        videoInput.markAsFinished()

        var asbd = AudioStreamBasicDescription(
            mSampleRate: Double(audioSampleRate),
            mFormatID: kAudioFormatLinearPCM,
            mFormatFlags: kLinearPCMFormatFlagIsSignedInteger | kAudioFormatFlagIsPacked,
            mBytesPerPacket: 2,
            mFramesPerPacket: 1,
            mBytesPerFrame: 2,
            mChannelsPerFrame: 1,
            mBitsPerChannel: 16,
            mReserved: 0
        )
        var audioFormatDescription: CMAudioFormatDescription?
        let formatStatus = CMAudioFormatDescriptionCreate(
            allocator: kCFAllocatorDefault,
            asbd: &asbd,
            layoutSize: 0,
            layout: nil,
            magicCookieSize: 0,
            magicCookie: nil,
            extensions: nil,
            formatDescriptionOut: &audioFormatDescription
        )
        XCTAssertEqual(formatStatus, noErr)
        guard let audioFormatDescription else {
            XCTFail("could not create PCM audio format description")
            return
        }

        for bufferIndex in 0..<audioBufferCount {
            while !audioInput.isReadyForMoreMediaData {
                try await Task.sleep(nanoseconds: 1_000_000)
            }
            let sampleBuffer = try makeSilentPCMSampleBuffer(
                formatDescription: audioFormatDescription,
                frameCount: audioFramesPerBuffer,
                frameOffset: bufferIndex * audioFramesPerBuffer,
                sampleRate: audioSampleRate
            )
            XCTAssertTrue(audioInput.append(sampleBuffer))
        }
        audioInput.markAsFinished()

        await writer.finishWriting()
        guard writer.status == .completed else {
            throw writer.error ?? AppleLivePhotoError.videoWriteFailed("test H.264/AAC writer failed")
        }
    }

    private func makeSilentPCMSampleBuffer(
        formatDescription: CMAudioFormatDescription,
        frameCount: Int,
        frameOffset: Int,
        sampleRate: Int
    ) throws -> CMSampleBuffer {
        let bytesPerSample = MemoryLayout<Int16>.size
        let byteCount = frameCount * bytesPerSample
        var blockBuffer: CMBlockBuffer?
        let blockStatus = CMBlockBufferCreateWithMemoryBlock(
            allocator: kCFAllocatorDefault,
            memoryBlock: nil,
            blockLength: byteCount,
            blockAllocator: kCFAllocatorDefault,
            customBlockSource: nil,
            offsetToData: 0,
            dataLength: byteCount,
            flags: 0,
            blockBufferOut: &blockBuffer
        )
        guard blockStatus == kCMBlockBufferNoErr, let blockBuffer else {
            throw AppleLivePhotoError.videoWriteFailed("could not allocate PCM block buffer")
        }
        let silence = Data(count: byteCount)
        let replaceStatus = silence.withUnsafeBytes { rawBuffer in
            CMBlockBufferReplaceDataBytes(
                with: rawBuffer.baseAddress!,
                blockBuffer: blockBuffer,
                offsetIntoDestination: 0,
                dataLength: byteCount
            )
        }
        guard replaceStatus == kCMBlockBufferNoErr else {
            throw AppleLivePhotoError.videoWriteFailed("could not fill PCM block buffer")
        }

        var timing = CMSampleTimingInfo(
            duration: CMTime(value: 1, timescale: CMTimeScale(sampleRate)),
            presentationTimeStamp: CMTime(
                value: CMTimeValue(frameOffset),
                timescale: CMTimeScale(sampleRate)
            ),
            decodeTimeStamp: .invalid
        )
        var sampleSize = bytesPerSample
        var sampleBuffer: CMSampleBuffer?
        let sampleStatus = CMSampleBufferCreateReady(
            allocator: kCFAllocatorDefault,
            dataBuffer: blockBuffer,
            formatDescription: formatDescription,
            sampleCount: frameCount,
            sampleTimingEntryCount: 1,
            sampleTimingArray: &timing,
            sampleSizeEntryCount: 1,
            sampleSizeArray: &sampleSize,
            sampleBufferOut: &sampleBuffer
        )
        guard sampleStatus == noErr, let sampleBuffer else {
            throw AppleLivePhotoError.videoWriteFailed("could not create PCM sample buffer")
        }
        return sampleBuffer
    }
}
