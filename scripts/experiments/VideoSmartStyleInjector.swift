import Foundation
import AVFoundation
import CoreMedia

private enum InjectError: Error, CustomStringConvertible {
    case usage
    case noVideoTrack
    case reader(String)
    case writer(String)
    case append(String)

    var description: String {
        switch self {
        case .usage:
            return "usage: VideoSmartStyleInjector <input.mov> <output.mov> <variant>"
        case .noVideoTrack:
            return "input has no video track"
        case .reader(let message):
            return "reader error: \(message)"
        case .writer(let message):
            return "writer error: \(message)"
        case .append(let message):
            return "append error: \(message)"
        }
    }
}

private struct Pipe {
    let output: AVAssetReaderTrackOutput
    let input: AVAssetWriterInput
}

private final class WriterState: @unchecked Sendable {
    private let lock = NSLock()
    private var storedError: Error?
    private var metadataIndex = 0

    func record(_ error: Error) {
        lock.lock()
        defer { lock.unlock() }
        if storedError == nil {
            storedError = error
        }
    }

    func error() -> Error? {
        lock.lock()
        defer { lock.unlock() }
        return storedError
    }

    func currentMetadataIndex() -> Int {
        lock.lock()
        defer { lock.unlock() }
        return metadataIndex
    }

    func advanceMetadataIndex() {
        lock.lock()
        metadataIndex += 1
        lock.unlock()
    }
}

private func metadataItem(_ identifier: String, _ value: NSObject & NSCopying, dataType: String? = nil) -> AVMutableMetadataItem {
    let item = AVMutableMetadataItem()
    item.identifier = AVMetadataIdentifier(rawValue: "mdta/\(identifier)")
    item.value = value
    if let dataType {
        item.dataType = dataType
    }
    return item
}

private func staticSmartStyleItems() -> [AVMetadataItem] {
    return [
        metadataItem("com.apple.quicktime.smartstyle.rendering-version", NSNumber(value: 1)),
        metadataItem("com.apple.quicktime.smartstyle.cast", NSNumber(value: 1)),
        metadataItem("com.apple.quicktime.smartstyle.intensity", NSNumber(value: 1.0)),
        metadataItem("com.apple.quicktime.smartstyle.tone", NSNumber(value: 0.0)),
        metadataItem("com.apple.quicktime.smartstyle.color", NSNumber(value: 0.0)),
        metadataItem("com.apple.quicktime.smartstyle.bypassed", NSNumber(value: false)),
    ]
}

private func payloadDictionary(for variant: String) throws -> [String: Any]? {
    switch variant {
    case "static":
        return nil
    case "lower":
        return [
            "smartStyleRenderingVersion": 1,
            "smartStyleCast": 1,
            "smartStyleCastIntensity": 1.0,
            "smartStyleToneBias": 0.0,
            "smartStyleColorBias": 0.0,
            "smartStyleIsReversible": true,
        ]
    case "compact":
        return [
            "renderingVersion": 1,
            "cast": 1,
            "intensity": 1.0,
            "tone": 0.0,
            "color": 0.0,
            "isReversible": true,
        ]
    case "upper":
        return [
            "SmartStyleRenderingVersion": 1,
            "SmartStyleCast": 1,
            "SmartStyleCastIntensity": 1.0,
            "SmartStyleToneBias": 0.0,
            "SmartStyleColorBias": 0.0,
            "SmartStyleIsReversible": true,
        ]
    default:
        throw InjectError.usage
    }
}

private func frameTimeRanges(asset: AVAsset, track: AVAssetTrack) throws -> [CMTimeRange] {
    let reader = try AVAssetReader(asset: asset)
    let output = AVAssetReaderTrackOutput(track: track, outputSettings: nil)
    output.alwaysCopiesSampleData = false
    guard reader.canAdd(output) else {
        throw InjectError.reader("cannot add frame timestamp reader output")
    }
    reader.add(output)
    guard reader.startReading() else {
        throw InjectError.reader(reader.error?.localizedDescription ?? "failed to start timestamp reader")
    }

    var result: [CMTimeRange] = []
    while let sample = output.copyNextSampleBuffer() {
        let pts = CMSampleBufferGetPresentationTimeStamp(sample)
        var duration = CMSampleBufferGetDuration(sample)
        if !duration.isValid || duration.isIndefinite || duration.value <= 0 {
            let fps = track.nominalFrameRate > 0 ? Double(track.nominalFrameRate) : 30.0
            duration = CMTime(seconds: 1.0 / fps, preferredTimescale: 60_000)
        }
        result.append(CMTimeRange(start: pts, duration: duration))
    }
    if reader.status == .failed {
        throw InjectError.reader(reader.error?.localizedDescription ?? "timestamp reader failed")
    }
    return result
}

private func writeVariant(inputURL: URL, outputURL: URL, variant: String) throws {
    try? FileManager.default.removeItem(at: outputURL)

    let asset = AVURLAsset(url: inputURL)
    guard let videoTrack = asset.tracks(withMediaType: .video).first else {
        throw InjectError.noVideoTrack
    }
    let times = try frameTimeRanges(asset: asset, track: videoTrack)
    let payloadDict = try payloadDictionary(for: variant)
    let payloadData: Data? = try payloadDict.map {
        try PropertyListSerialization.data(fromPropertyList: $0, format: .binary, options: 0)
    }

    let reader = try AVAssetReader(asset: asset)
    let writer = try AVAssetWriter(outputURL: outputURL, fileType: .mov)
    writer.shouldOptimizeForNetworkUse = false
    writer.metadata = asset.metadata + staticSmartStyleItems()

    var pipes: [Pipe] = []
    for track in asset.tracks where track.mediaType == .video || track.mediaType == .audio {
        let output = AVAssetReaderTrackOutput(track: track, outputSettings: nil)
        output.alwaysCopiesSampleData = false
        guard reader.canAdd(output) else {
            throw InjectError.reader("cannot add reader output for \(track.mediaType.rawValue)")
        }
        reader.add(output)

        let hint = track.formatDescriptions.first as? CMFormatDescription
        let input = AVAssetWriterInput(mediaType: track.mediaType, outputSettings: nil, sourceFormatHint: hint)
        input.expectsMediaDataInRealTime = false
        if track.mediaType == .video {
            input.transform = track.preferredTransform
        }
        guard writer.canAdd(input) else {
            throw InjectError.writer("cannot add writer input for \(track.mediaType.rawValue)")
        }
        writer.add(input)
        pipes.append(Pipe(output: output, input: input))
    }

    var metadataInput: AVAssetWriterInput?
    var metadataAdaptor: AVAssetWriterInputMetadataAdaptor?
    if payloadData != nil {
        let input = AVAssetWriterInput(mediaType: .metadata, outputSettings: nil)
        input.expectsMediaDataInRealTime = false
        guard writer.canAdd(input) else {
            throw InjectError.writer("cannot add timed metadata input")
        }
        writer.add(input)
        metadataInput = input
        metadataAdaptor = AVAssetWriterInputMetadataAdaptor(assetWriterInput: input)
    }

    guard writer.startWriting() else {
        throw InjectError.writer(writer.error?.localizedDescription ?? "startWriting failed")
    }
    guard reader.startReading() else {
        throw InjectError.reader(reader.error?.localizedDescription ?? "startReading failed")
    }
    writer.startSession(atSourceTime: .zero)

    let group = DispatchGroup()
    let state = WriterState()

    for (index, pipe) in pipes.enumerated() {
        group.enter()
        let queue = DispatchQueue(label: "xdremux.smartstyle.copy.\(index)")
        pipe.input.requestMediaDataWhenReady(on: queue) {
            while pipe.input.isReadyForMoreMediaData {
                if let sample = pipe.output.copyNextSampleBuffer() {
                    if !pipe.input.append(sample) {
                        state.record(InjectError.append(writer.error?.localizedDescription ?? "media sample append failed"))
                        pipe.input.markAsFinished()
                        group.leave()
                        return
                    }
                } else {
                    pipe.input.markAsFinished()
                    group.leave()
                    return
                }
            }
        }
    }

    if let metadataInput, let metadataAdaptor, let payloadData {
        group.enter()
        let queue = DispatchQueue(label: "xdremux.smartstyle.metadata")
        metadataInput.requestMediaDataWhenReady(on: queue) {
            while metadataInput.isReadyForMoreMediaData {
                let index = state.currentMetadataIndex()
                guard index < times.count else {
                    metadataInput.markAsFinished()
                    group.leave()
                    return
                }

                let item = AVMutableMetadataItem()
                item.identifier = AVMetadataIdentifier(rawValue: "mdta/com.apple.quicktime.smartstyle-info")
                item.dataType = "com.apple.metadata.datatype.raw-data"
                item.value = payloadData as NSData
                let timedGroup = AVTimedMetadataGroup(items: [item], timeRange: times[index])
                if !metadataAdaptor.append(timedGroup) {
                    state.record(InjectError.append(writer.error?.localizedDescription ?? "timed metadata append failed at frame \(index)"))
                    metadataInput.markAsFinished()
                    group.leave()
                    return
                }
                state.advanceMetadataIndex()
            }
        }
    }

    group.wait()
    if let firstError = state.error() { throw firstError }
    if reader.status == .failed {
        throw InjectError.reader(reader.error?.localizedDescription ?? "reader failed")
    }

    let finish = DispatchSemaphore(value: 0)
    writer.finishWriting { finish.signal() }
    finish.wait()
    guard writer.status == .completed else {
        throw InjectError.writer(writer.error?.localizedDescription ?? "finishWriting failed")
    }

    print("wrote \(outputURL.path) variant=\(variant) frames=\(times.count) timedMetadata=\(payloadData != nil)")
}

@main
private struct Main {
    static func main() {
        do {
            guard CommandLine.arguments.count == 4 else { throw InjectError.usage }
            try writeVariant(
                inputURL: URL(fileURLWithPath: CommandLine.arguments[1]),
                outputURL: URL(fileURLWithPath: CommandLine.arguments[2]),
                variant: CommandLine.arguments[3]
            )
        } catch {
            fputs("VideoSmartStyleInjector: \(error)\n", stderr)
            exit(2)
        }
    }
}
