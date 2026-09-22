// A public-AVFoundation research carrier constructor, never a product backend.
// Every metadata value below is an explicit synthetic hypothesis, not a claim
// that an arbitrary movie was captured with Smart Style or is reversible.
import Foundation
import AVFoundation
import CoreMedia

private struct ProbeFailure: Error, CustomStringConvertible {
    let description: String
    init(_ message: String) { description = message }
}

private enum Variant: String {
    case `static`, lower, compact, upper

    var payload: [String: Any]? {
        switch self {
        case .static: return nil
        case .lower:
            return ["smartStyleRenderingVersion": 1, "smartStyleCast": 1,
                    "smartStyleCastIntensity": 1.0, "smartStyleToneBias": 0.0,
                    "smartStyleColorBias": 0.0, "smartStyleIsReversible": true]
        case .compact:
            return ["renderingVersion": 1, "cast": 1, "intensity": 1.0,
                    "tone": 0.0, "color": 0.0, "isReversible": true]
        case .upper:
            return ["SmartStyleRenderingVersion": 1, "SmartStyleCast": 1,
                    "SmartStyleCastIntensity": 1.0, "SmartStyleToneBias": 0.0,
                    "SmartStyleColorBias": 0.0, "SmartStyleIsReversible": true]
        }
    }
}

private let metadataIdentifier = "mdta/com.apple.quicktime.smartstyle-info"
private let rawDataType = "com.apple.metadata.datatype.raw-data"
private let staticFields: [(String, NSNumber, String)] = [
    ("rendering-version", 1, "com.apple.metadata.datatype.int32"),
    ("cast", 1, "com.apple.metadata.datatype.int32"),
    ("intensity", 1.0, "com.apple.metadata.datatype.float64"),
    ("tone", 0.0, "com.apple.metadata.datatype.float64"),
    ("color", 0.0, "com.apple.metadata.datatype.float64"),
    ("bypassed", NSNumber(value: false), "com.apple.metadata.datatype.int8")
]

private func staticItems() -> [AVMetadataItem] {
    staticFields.map { key, value, type in
        let item = AVMutableMetadataItem()
        item.identifier = AVMetadataIdentifier(rawValue: "mdta/com.apple.quicktime.smartstyle.\(key)")
        item.dataType = type
        item.value = value
        return item
    }
}

private func checkDeadline(_ start: ContinuousClock.Instant) throws {
    if start.duration(to: .now) > .seconds(100) {
        throw ProbeFailure("research reader/writer deadline exceeded")
    }
}

private func frameRanges(_ asset: AVAsset, _ track: AVAssetTrack) throws -> [CMTimeRange] {
    let reader = try AVAssetReader(asset: asset)
    let output = AVAssetReaderTrackOutput(track: track, outputSettings: nil)
    output.alwaysCopiesSampleData = false
    guard reader.canAdd(output) else { throw ProbeFailure("cannot add timestamp reader") }
    reader.add(output)
    guard reader.startReading() else { throw reader.error ?? ProbeFailure("timestamp reader did not start") }
    defer { if reader.status == .reading { reader.cancelReading() } }
    let started = ContinuousClock.now
    var ranges: [CMTimeRange] = []
    while let sample = output.copyNextSampleBuffer() {
        try checkDeadline(started)
        let pts = CMSampleBufferGetPresentationTimeStamp(sample)
        let duration = CMSampleBufferGetDuration(sample)
        guard CMSampleBufferGetNumSamples(sample) == 1,
              pts.isNumeric, duration.isNumeric, duration > .zero,
              ranges.count < 100_000 else {
            throw ProbeFailure("unsupported sample timing/count; no invented frame-rate fallback")
        }
        ranges.append(CMTimeRange(start: pts, duration: duration))
    }
    guard reader.status == .completed else { throw reader.error ?? ProbeFailure("timestamp reader incomplete") }
    guard !ranges.isEmpty else { throw ProbeFailure("empty video track") }
    // Compressed B-frames arrive in decoding order; metadata uses presentation order.
    ranges.sort { $0.start < $1.start }
    for index in 1..<ranges.count {
        guard ranges[index - 1].end <= ranges[index].start else {
            throw ProbeFailure("overlapping video presentation intervals are not supported")
        }
    }
    return ranges
}

private struct Pipe {
    let output: AVAssetReaderTrackOutput
    let input: AVAssetWriterInput
    var finished = false
}

private func metadataHint() throws -> CMMetadataFormatDescription {
    var result: CMMetadataFormatDescription?
    let specification: [String: Any] = [
        kCMMetadataFormatDescriptionMetadataSpecificationKey_Identifier as String: metadataIdentifier,
        kCMMetadataFormatDescriptionMetadataSpecificationKey_DataType as String: rawDataType
    ]
    let status = CMMetadataFormatDescriptionCreateWithMetadataSpecifications(
        allocator: kCFAllocatorDefault, metadataType: kCMMetadataFormatType_Boxed,
        metadataSpecifications: [specification] as CFArray, formatDescriptionOut: &result)
    guard status == noErr, let result else {
        throw ProbeFailure("metadata format description failed: \(status)")
    }
    return result
}

private func construct(source: URL, destination: URL, variant: Variant) async throws -> Int {
    guard source.standardizedFileURL != destination.standardizedFileURL,
          !FileManager.default.fileExists(atPath: destination.path) else {
        throw ProbeFailure("destination already exists or names the source")
    }
    let asset = AVURLAsset(url: source)
    let tracks = try await asset.load(.tracks)
    guard tracks.filter({ $0.mediaType == .video }).count == 1,
          tracks.allSatisfy({ $0.mediaType == .video || $0.mediaType == .audio }),
          let video = tracks.first(where: { $0.mediaType == .video }) else {
        throw ProbeFailure("one video and optional audio tracks required; other tracks cannot be discarded")
    }
    let ranges = try frameRanges(asset, video)
    let payload = try variant.payload.map {
        try PropertyListSerialization.data(fromPropertyList: $0, format: .binary, options: 0)
    }
    let existing = try await asset.load(.metadata)
    guard !existing.contains(where: { $0.identifier?.rawValue.contains("smartstyle") == true }) else {
        throw ProbeFailure("existing Smart Style metadata cannot be overwritten")
    }
    let reader = try AVAssetReader(asset: asset)
    let writer = try AVAssetWriter(outputURL: destination, fileType: .mov)
    defer {
        if reader.status == .reading { reader.cancelReading() }
        if writer.status == .writing { writer.cancelWriting() }
    }
    writer.metadata = existing + staticItems()
    var pipes: [Pipe] = []
    for track in tracks {
        let formats = try await track.load(.formatDescriptions)
        guard formats.count == 1, let format = formats.first else {
            throw ProbeFailure("changing or missing source format descriptions are unsupported")
        }
        let output = AVAssetReaderTrackOutput(track: track, outputSettings: nil)
        output.alwaysCopiesSampleData = false
        guard reader.canAdd(output) else { throw ProbeFailure("cannot add compressed reader output") }
        reader.add(output)
        let input = AVAssetWriterInput(mediaType: track.mediaType, outputSettings: nil, sourceFormatHint: format)
        input.expectsMediaDataInRealTime = false
        input.metadata = try await track.load(.metadata)
        input.mediaTimeScale = try await track.load(.naturalTimeScale)
        if track.mediaType == .video { input.transform = try await track.load(.preferredTransform) }
        guard writer.canAdd(input) else { throw ProbeFailure("cannot add compressed writer input") }
        writer.add(input)
        pipes.append(Pipe(output: output, input: input))
    }
    var adaptor: AVAssetWriterInputMetadataAdaptor?
    if payload != nil {
        let input = AVAssetWriterInput(mediaType: .metadata, outputSettings: nil, sourceFormatHint: try metadataHint())
        input.expectsMediaDataInRealTime = false
        input.mediaTimeScale = try await video.load(.naturalTimeScale)
        guard writer.canAdd(input) else { throw ProbeFailure("cannot add metadata input") }
        writer.add(input)
        adaptor = AVAssetWriterInputMetadataAdaptor(assetWriterInput: input)
    }
    guard writer.startWriting() else { throw writer.error ?? ProbeFailure("writer did not start") }
    guard reader.startReading() else { throw reader.error ?? ProbeFailure("reader did not start") }
    writer.startSession(atSourceTime: .zero)
    var metadataIndex = 0
    var metadataFinished = adaptor == nil
    let started = ContinuousClock.now
    // One bounded loop owns every input and completion transition. It cannot
    // double-leave a dispatch group, race a shared index, or wait forever on EOF.
    while pipes.contains(where: { !$0.finished }) || !metadataFinished {
        try checkDeadline(started)
        guard writer.status == .writing else { throw writer.error ?? ProbeFailure("writer stopped prematurely") }
        var progressed = false
        for index in pipes.indices where !pipes[index].finished && pipes[index].input.isReadyForMoreMediaData {
            if let sample = pipes[index].output.copyNextSampleBuffer() {
                guard pipes[index].input.append(sample) else { throw writer.error ?? ProbeFailure("media append failed") }
            } else {
                if reader.status == .failed || reader.status == .cancelled {
                    throw reader.error ?? ProbeFailure("reader stopped prematurely")
                }
                pipes[index].input.markAsFinished()
                pipes[index].finished = true
            }
            progressed = true
        }
        if let adaptor, let payload, !metadataFinished, adaptor.assetWriterInput.isReadyForMoreMediaData {
            if metadataIndex == ranges.count {
                adaptor.assetWriterInput.markAsFinished()
                metadataFinished = true
            } else {
                let item = AVMutableMetadataItem()
                item.identifier = AVMetadataIdentifier(rawValue: metadataIdentifier)
                item.dataType = rawDataType
                item.value = payload as NSData
                let group = AVTimedMetadataGroup(items: [item], timeRange: ranges[metadataIndex])
                guard adaptor.append(group) else { throw writer.error ?? ProbeFailure("metadata append failed") }
                metadataIndex += 1
            }
            progressed = true
        }
        if !progressed { try await Task.sleep(for: .milliseconds(1)) }
    }
    guard reader.status == .completed else { throw reader.error ?? ProbeFailure("media reader incomplete") }
    // The Python supervisor also bounds native finalization and synchronous API calls.
    await writer.finishWriting()
    guard writer.status == .completed else { throw writer.error ?? ProbeFailure("writer incomplete") }
    try await readback(destination, variant: variant, ranges: ranges)
    return ranges.count
}

private func readback(_ url: URL, variant: Variant, ranges: [CMTimeRange]) async throws {
    let asset = AVURLAsset(url: url)
    let items = try await asset.load(.metadata)
    for (field, value, _) in staticFields {
        let identifier = AVMetadataIdentifier(rawValue: "mdta/com.apple.quicktime.smartstyle.\(field)")
        let matches = items.filter { $0.identifier == identifier }
        guard matches.count == 1, let match = matches.first,
              let number = try await match.load(.numberValue), number == value else {
            throw ProbeFailure("static metadata did not round-trip: \(field)")
        }
    }
    let tracks = try await asset.loadTracks(withMediaType: .metadata)
    guard tracks.count == (variant == .static ? 0 : 1) else { throw ProbeFailure("metadata track count mismatch") }
    guard let expected = variant.payload, let track = tracks.first else { return }
    let reader = try AVAssetReader(asset: asset)
    let output = AVAssetReaderTrackOutput(track: track, outputSettings: nil)
    guard reader.canAdd(output) else { throw ProbeFailure("cannot add metadata readback") }
    reader.add(output)
    let adaptor = AVAssetReaderOutputMetadataAdaptor(assetReaderTrackOutput: output)
    guard reader.startReading() else { throw reader.error ?? ProbeFailure("metadata reader did not start") }
    defer { if reader.status == .reading { reader.cancelReading() } }
    var count = 0
    let started = ContinuousClock.now
    while let group = adaptor.nextTimedMetadataGroup() {
        try checkDeadline(started)
        guard count < ranges.count, group.timeRange == ranges[count], group.items.count == 1,
              let item = group.items.first, item.identifier?.rawValue == metadataIdentifier,
              let data = try await item.load(.dataValue),
              let decoded = try PropertyListSerialization.propertyList(from: data, options: [], format: nil) as? NSDictionary,
              decoded.isEqual(to: expected) else {
            throw ProbeFailure("timed metadata payload/timing did not round-trip at sample \(count)")
        }
        count += 1
    }
    guard reader.status == .completed, count == ranges.count else {
        throw reader.error ?? ProbeFailure("metadata readback truncated")
    }
}

@main private struct Main {
    static func main() async {
        do {
            guard CommandLine.arguments.count == 4, let variant = Variant(rawValue: CommandLine.arguments[3]) else {
                throw ProbeFailure("usage: CarrierProbe <source> <private-output.mov> <static|lower|compact|upper>; use probe.py for publication")
            }
            let count = try await construct(source: URL(fileURLWithPath: CommandLine.arguments[1]),
                                            destination: URL(fileURLWithPath: CommandLine.arguments[2]), variant: variant)
            let report: [String: Any] = ["schema": 1, "variant": variant.rawValue,
                "videoSamples": count, "metadataReadback": true, "photosEditingValidated": false]
            let data = try JSONSerialization.data(withJSONObject: report, options: [.sortedKeys])
            FileHandle.standardOutput.write(data)
            FileHandle.standardOutput.write(Data([10]))
        } catch {
            FileHandle.standardError.write(Data("CarrierProbe: \(error)\n".utf8))
            exit(2)
        }
    }
}
