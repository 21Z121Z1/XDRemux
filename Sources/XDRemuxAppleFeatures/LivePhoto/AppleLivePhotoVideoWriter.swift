import Foundation
@preconcurrency import AVFoundation
import CoreMedia
import XDRemuxCore

public enum AppleLivePhotoVideoWriter {
    private static let quickTimeKeySpace = "mdta"
    private static let contentIdentifierKey = "com.apple.quicktime.content.identifier"
    private static let stillImageTimeKey = "com.apple.quicktime.still-image-time"
    private static let transformKey = "com.apple.quicktime.live-photo-still-image-transform"
    private static let transformReferenceDimensionsKey = "com.apple.quicktime.live-photo-still-image-transform-reference-dimensions"

    public static func write(
        videoInputURL: URL,
        outputURL: URL,
        assetIdentifier: String,
        stillImageTime: CMTime,
        oppoMetadata: OppoMotionPhotoMetadata? = nil,
        stillImageReferenceDimensions: [Float]? = nil
    ) async throws {
        let asset = AVURLAsset(url: videoInputURL)
        guard let videoTrack = try await asset.loadTracks(withMediaType: .video).first else {
            throw AppleLivePhotoError.missingVideoTrack
        }
        let videoFormatDescriptions = try await videoTrack.load(.formatDescriptions)
        guard let videoHint = videoFormatDescriptions.first else {
            throw AppleLivePhotoError.missingVideoTrack
        }

        try FileManager.default.createDirectory(
            at: outputURL.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        if FileManager.default.fileExists(atPath: outputURL.path) {
            try FileManager.default.removeItem(at: outputURL)
        }

        let reader: AVAssetReader
        let writer: AVAssetWriter
        do {
            reader = try AVAssetReader(asset: asset)
        } catch {
            throw AppleLivePhotoError.cannotCreateVideoReader
        }
        do {
            writer = try AVAssetWriter(outputURL: outputURL, fileType: .mov)
        } catch {
            throw AppleLivePhotoError.cannotCreateVideoWriter
        }

        // nil output settings are intentional. AVFoundation documents nil reader settings as
        // "samples in their original stored format" and nil writer settings as passthrough without
        // re-encoding. The pair validator independently hashes the encoded storage samples before
        // and after remuxing so this contract is also verified at runtime.
        let videoOutput = AVAssetReaderTrackOutput(track: videoTrack, outputSettings: nil)
        videoOutput.alwaysCopiesSampleData = false
        guard reader.canAdd(videoOutput) else { throw AppleLivePhotoError.cannotCreateVideoReader }
        reader.add(videoOutput)

        let videoInput = AVAssetWriterInput(mediaType: .video, outputSettings: nil, sourceFormatHint: videoHint)
        videoInput.expectsMediaDataInRealTime = false
        videoInput.transform = try await videoTrack.load(.preferredTransform)
        guard writer.canAdd(videoInput) else {
            throw AppleLivePhotoError.unsupportedVideoCodec(fourCC(CMFormatDescriptionGetMediaSubType(videoHint)))
        }
        writer.add(videoInput)

        var audioOutput: AVAssetReaderTrackOutput?
        var audioInput: AVAssetWriterInput?
        if let audioTrack = try await asset.loadTracks(withMediaType: .audio).first {
            let output = AVAssetReaderTrackOutput(track: audioTrack, outputSettings: nil)
            output.alwaysCopiesSampleData = false
            let audioFormats = try await audioTrack.load(.formatDescriptions)
            let input = AVAssetWriterInput(
                mediaType: .audio,
                outputSettings: nil,
                sourceFormatHint: audioFormats.first
            )
            input.expectsMediaDataInRealTime = false
            guard reader.canAdd(output), writer.canAdd(input) else {
                throw AppleLivePhotoError.unsupportedVideoCodec("audio passthrough")
            }
            reader.add(output)
            writer.add(input)
            audioOutput = output
            audioInput = input
        }

        writer.metadata = [contentIdentifierMetadata(assetIdentifier)]

        let transform = oppoMetadata.flatMap(OppoLivePhotoAlignment.transformMatrix)
        let referenceDimensions = transformReferenceDimensions(
            transform: transform,
            requestedStillImageDimensions: stillImageReferenceDimensions,
            oppoMetadata: oppoMetadata
        )
        let metadataSetup = try makeTimedMetadataSetup(
            includeTransform: transform != nil,
            includeReferenceDimensions: transform != nil && referenceDimensions != nil
        )
        guard writer.canAdd(metadataSetup.input) else {
            throw AppleLivePhotoError.videoWriteFailed("cannot add timed metadata track")
        }
        writer.add(metadataSetup.input)

        guard writer.startWriting() else {
            throw AppleLivePhotoError.cannotStartVideoWriter(writer.error?.localizedDescription ?? "unknown error")
        }
        guard reader.startReading() else {
            writer.cancelWriting()
            throw AppleLivePhotoError.cannotStartVideoReader(reader.error?.localizedDescription ?? "unknown error")
        }
        writer.startSession(atSourceTime: .zero)

        let failure = LockedFailure()
        let group = DispatchGroup()

        copyTrack(
            input: videoInput,
            output: videoOutput,
            queueLabel: "xdremux.livephoto.video",
            reader: reader,
            writer: writer,
            group: group,
            failure: failure
        )
        if let audioInput, let audioOutput {
            copyTrack(
                input: audioInput,
                output: audioOutput,
                queueLabel: "xdremux.livephoto.audio",
                reader: reader,
                writer: writer,
                group: group,
                failure: failure
            )
        }
        writeTimedMetadata(
            setup: metadataSetup,
            stillImageTime: stillImageTime,
            transform: transform,
            referenceDimensions: referenceDimensions,
            group: group,
            failure: failure
        )

        await waitForGroup(group)
        if let error = failure.error {
            reader.cancelReading()
            writer.cancelWriting()
            try? FileManager.default.removeItem(at: outputURL)
            throw error
        }

        await writer.finishWriting()
        reader.cancelReading()
        guard writer.status == .completed else {
            let message = writer.error?.localizedDescription ?? "writer did not complete"
            try? FileManager.default.removeItem(at: outputURL)
            throw AppleLivePhotoError.videoWriteFailed(message)
        }
    }

    public static func contentIdentifier(in videoURL: URL) async -> String? {
        let asset = AVURLAsset(url: videoURL)
        guard let metadata = try? await asset.load(.metadata) else { return nil }
        for item in metadata {
            if item.identifier == .quickTimeMetadataContentIdentifier,
               let value = try? await item.load(.stringValue) {
                return value
            }
            if let key = item.key as? String,
               key == contentIdentifierKey,
               let value = try? await item.load(.stringValue) {
                return value
            }
        }
        return nil
    }

    static func transformReferenceDimensions(
        transform: [Double]?,
        requestedStillImageDimensions: [Float]?,
        oppoMetadata: OppoMotionPhotoMetadata?
    ) -> [Float]? {
        guard transform != nil else { return nil }
        if let requestedStillImageDimensions,
           requestedStillImageDimensions.count == 2,
           requestedStillImageDimensions.allSatisfy({ $0.isFinite && $0 > 0 }) {
            return requestedStillImageDimensions
        }
        return oppoMetadata.flatMap(OppoLivePhotoAlignment.referenceDimensions)
    }

    private static func contentIdentifierMetadata(_ assetIdentifier: String) -> AVMetadataItem {
        let item = AVMutableMetadataItem()
        item.key = contentIdentifierKey as NSString
        item.keySpace = AVMetadataKeySpace(rawValue: quickTimeKeySpace)
        item.value = assetIdentifier as NSString
        item.dataType = kCMMetadataBaseDataType_UTF8 as String
        return item
    }

    /// AVFoundation owns these callback objects and serializes access through the queues passed to
    /// requestMediaDataWhenReady. Swift 6 does not annotate every legacy AVFoundation class as
    /// Sendable, so this small wrapper makes the queue confinement explicit instead of spreading
    /// unchecked annotations through the writer implementation.
    private struct UnsafeTransfer<Value>: @unchecked Sendable {
        let value: Value
    }

    private struct TimedMetadataSetup: @unchecked Sendable {
        let input: AVAssetWriterInput
        let adaptor: AVAssetWriterInputMetadataAdaptor
    }

    private static func makeTimedMetadataSetup(
        includeTransform: Bool,
        includeReferenceDimensions: Bool
    ) throws -> TimedMetadataSetup {
        var specifications: [[String: Any]] = [[
            kCMMetadataFormatDescriptionMetadataSpecificationKey_Identifier as String:
                "\(quickTimeKeySpace)/\(stillImageTimeKey)",
            kCMMetadataFormatDescriptionMetadataSpecificationKey_DataType as String:
                kCMMetadataBaseDataType_SInt8,
        ]]
        if includeTransform {
            specifications.append([
                kCMMetadataFormatDescriptionMetadataSpecificationKey_Identifier as String:
                    kCMMetadataIdentifier_QuickTimeMetadataLivePhotoStillImageTransform as String,
                kCMMetadataFormatDescriptionMetadataSpecificationKey_DataType as String:
                    "com.apple.metadata.perspective-transform-float64",
            ])
        }
        if includeReferenceDimensions {
            specifications.append([
                kCMMetadataFormatDescriptionMetadataSpecificationKey_Identifier as String:
                    kCMMetadataIdentifier_QuickTimeMetadataLivePhotoStillImageTransformReferenceDimensions as String,
                kCMMetadataFormatDescriptionMetadataSpecificationKey_DataType as String:
                    "com.apple.metadata.datatype.dimensions-float32",
            ])
        }

        var formatDescription: CMFormatDescription?
        let status = CMMetadataFormatDescriptionCreateWithMetadataSpecifications(
            allocator: kCFAllocatorDefault,
            metadataType: kCMMetadataFormatType_Boxed,
            metadataSpecifications: specifications as CFArray,
            formatDescriptionOut: &formatDescription
        )
        guard status == noErr, let formatDescription else {
            throw AppleLivePhotoError.videoWriteFailed("cannot create timed metadata format description")
        }
        let input = AVAssetWriterInput(
            mediaType: .metadata,
            outputSettings: nil,
            sourceFormatHint: formatDescription
        )
        input.expectsMediaDataInRealTime = false
        return TimedMetadataSetup(
            input: input,
            adaptor: AVAssetWriterInputMetadataAdaptor(assetWriterInput: input)
        )
    }

    private static func writeTimedMetadata(
        setup: TimedMetadataSetup,
        stillImageTime: CMTime,
        transform: [Double]?,
        referenceDimensions: [Float]?,
        group: DispatchGroup,
        failure: LockedFailure
    ) {
        group.enter()
        let state = OneShotState()
        setup.input.requestMediaDataWhenReady(on: DispatchQueue(label: "xdremux.livephoto.metadata")) {
            guard setup.input.isReadyForMoreMediaData, state.beginIfNeeded() else { return }
            var items: [AVMetadataItem] = []

            let still = AVMutableMetadataItem()
            still.key = stillImageTimeKey as NSString
            still.keySpace = AVMetadataKeySpace(rawValue: quickTimeKeySpace)
            still.value = NSNumber(value: Int8(0))
            still.dataType = kCMMetadataBaseDataType_SInt8 as String
            items.append(still)

            if let transform {
                let item = AVMutableMetadataItem()
                item.key = transformKey as NSString
                item.keySpace = AVMetadataKeySpace(rawValue: quickTimeKeySpace)
                item.value = transform as NSArray
                item.dataType = "com.apple.metadata.perspective-transform-float64"
                items.append(item)
            }
            if let referenceDimensions, referenceDimensions.count == 2 {
                let item = AVMutableMetadataItem()
                item.key = transformReferenceDimensionsKey as NSString
                item.keySpace = AVMetadataKeySpace(rawValue: quickTimeKeySpace)
                item.value = NSValue(size: NSSize(
                    width: CGFloat(referenceDimensions[0]),
                    height: CGFloat(referenceDimensions[1])
                ))
                item.dataType = "com.apple.metadata.datatype.dimensions-float32"
                items.append(item)
            }

            let groupValue = AVTimedMetadataGroup(
                items: items,
                timeRange: CMTimeRange(start: stillImageTime, duration: .zero)
            )
            if !setup.adaptor.append(groupValue) {
                failure.set(AppleLivePhotoError.videoWriteFailed("timed metadata append failed"))
            }
            setup.input.markAsFinished()
            group.leave()
        }
    }

    private static func copyTrack(
        input: AVAssetWriterInput,
        output: AVAssetReaderOutput,
        queueLabel: String,
        reader: AVAssetReader,
        writer: AVAssetWriter,
        group: DispatchGroup,
        failure: LockedFailure
    ) {
        group.enter()
        let state = TrackCopyState()
        let inputTransfer = UnsafeTransfer(value: input)
        let outputTransfer = UnsafeTransfer(value: output)
        let readerTransfer = UnsafeTransfer(value: reader)
        let writerTransfer = UnsafeTransfer(value: writer)
        input.requestMediaDataWhenReady(on: DispatchQueue(label: queueLabel, autoreleaseFrequency: .workItem)) {
            let input = inputTransfer.value
            let output = outputTransfer.value
            let reader = readerTransfer.value
            let writer = writerTransfer.value
            guard !state.finished else { return }
            while input.isReadyForMoreMediaData, !state.finished {
                autoreleasepool {
                    if let sample = output.copyNextSampleBuffer() {
                        if !input.append(sample) {
                            state.finished = true
                            input.markAsFinished()
                            failure.set(AppleLivePhotoError.videoWriteFailed(
                                writer.error?.localizedDescription ?? "sample append failed"
                            ))
                            reader.cancelReading()
                            group.leave()
                        }
                    } else {
                        state.finished = true
                        input.markAsFinished()
                        if reader.status == .failed {
                            failure.set(AppleLivePhotoError.videoWriteFailed(
                                reader.error?.localizedDescription ?? "reader failed"
                            ))
                        }
                        group.leave()
                    }
                }
            }
        }
    }

    private static func waitForGroup(_ group: DispatchGroup) async {
        await withCheckedContinuation { continuation in
            group.notify(queue: .global(qos: .userInitiated)) {
                continuation.resume()
            }
        }
    }

    private static func fourCC(_ code: FourCharCode) -> String {
        let bytes: [UInt8] = [
            UInt8((code >> 24) & 0xff), UInt8((code >> 16) & 0xff),
            UInt8((code >> 8) & 0xff), UInt8(code & 0xff),
        ]
        return String(bytes: bytes, encoding: .macOSRoman) ?? String(format: "0x%08x", code)
    }
}

private final class LockedFailure: @unchecked Sendable {
    private let lock = NSLock()
    private var stored: Error?
    var error: Error? {
        lock.lock(); defer { lock.unlock() }
        return stored
    }
    func set(_ error: Error) {
        lock.lock(); defer { lock.unlock() }
        if stored == nil { stored = error }
    }
}

private final class TrackCopyState: @unchecked Sendable {
    private let lock = NSLock()
    private var storedFinished = false
    var finished: Bool {
        get { lock.lock(); defer { lock.unlock() }; return storedFinished }
        set { lock.lock(); defer { lock.unlock() }; storedFinished = newValue }
    }
}

private final class OneShotState: @unchecked Sendable {
    private let lock = NSLock()
    private var started = false
    func beginIfNeeded() -> Bool {
        lock.lock(); defer { lock.unlock() }
        guard !started else { return false }
        started = true
        return true
    }
}
