// Public Apple consumer and compressed-sample checks for the pending envelope.
// No Photos import, rendering, resource download, or preprocessing occurs here.
import AVFoundation
import Cinematic
import CoreMedia
import CoreVideo
import CryptoKit
import Foundation

func failure(_ message: String) -> NSError {
    NSError(domain: "XDRemuxCinematicVerification", code: 1,
            userInfo: [NSLocalizedDescriptionKey: message])
}

func require(_ condition: Bool, _ message: String) throws {
    if !condition { throw failure(message) }
}

func sha256(_ data: Data) -> String {
    SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
}

struct Packet: Equatable {
    let bytes: String
    let count: Int
    let pts: CMTime
    let dts: CMTime
    let duration: CMTime
    var detail: String { "hash=\(bytes.prefix(12)) count=\(count) pts=\(pts) dts=\(dts) duration=\(duration)" }
    static func sameTime(_ a: CMTime, _ b: CMTime) -> Bool {
        if !a.isNumeric || !b.isNumeric { return a.flags == b.flags }
        return CMTimeCompare(a, b) == 0
    }
    static func == (a: Packet, b: Packet) -> Bool {
        // Trailing compressed decode buffers can extend beyond the movie edit.
        // AVAssetWriter can change that padding duration for reordered video.
        // Check bytes/PTS/DTS here, and verify the visible edit and decoded PTS below.
        a.bytes == b.bytes && a.count == b.count && sameTime(a.pts, b.pts) && sameTime(a.dts, b.dts)
    }
}

func mediaTracks(_ asset: AVAsset) async throws -> [AVAssetTrack] {
    try await asset.load(.tracks).filter { $0.mediaType == .video || $0.mediaType == .audio }
}

func packets(_ asset: AVAsset, _ track: AVAssetTrack) throws -> [Packet] {
    let reader = try AVAssetReader(asset: asset)
    let output = AVAssetReaderTrackOutput(track: track, outputSettings: nil)
    reader.add(output)
    try require(reader.startReading(), "compressed reader failed: \(String(describing: reader.error))")
    var rows: [Packet] = []
    while let sample = output.copyNextSampleBuffer() {
        let block = CMSampleBufferGetDataBuffer(sample)
        let length = block.map(CMBlockBufferGetDataLength) ?? 0
        try require(block != nil || CMSampleBufferGetTotalSampleSize(sample) == 0,
                    "missing compressed data in track \(track.trackID)")
        var bytes = Data(count: length)
        let status: OSStatus = length == 0 ? noErr : bytes.withUnsafeMutableBytes {
            CMBlockBufferCopyDataBytes(block!, atOffset: 0, dataLength: length, destination: $0.baseAddress!)
        }
        try require(status == noErr, "cannot read compressed bytes")
        // AVAssetReader also emits empty trim/discontinuity marker buffers.
        // They carry no encoded media and AVAssetWriter does not store them.
        if length == 0 { continue }
        rows.append(Packet(bytes: sha256(bytes), count: CMSampleBufferGetNumSamples(sample),
                           pts: CMSampleBufferGetPresentationTimeStamp(sample),
                           dts: CMSampleBufferGetDecodeTimeStamp(sample),
                           duration: CMSampleBufferGetDuration(sample)))
    }
    try require(reader.status == .completed, "compressed reader did not complete")
    return rows
}

func videoTimes(_ asset: AVAsset, _ track: AVAssetTrack) throws -> [CMTime] {
    let reader = try AVAssetReader(asset: asset)
    let output = AVAssetReaderTrackOutput(track: track, outputSettings: [
        kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange
    ])
    reader.add(output)
    try require(reader.startReading(), "decoded reader did not start")
    var times: [CMTime] = []
    while let sample = output.copyNextSampleBuffer() { times.append(CMSampleBufferGetPresentationTimeStamp(sample)) }
    try require(reader.status == .completed, "decoded reader did not complete")
    return times
}

func preserveMedia(_ source: AVAsset, _ output: AVAsset) async throws -> [[String: Any]] {
    let a = try await mediaTracks(source), b = try await mediaTracks(output)
    try require(a.count == b.count && !a.isEmpty, "media track count changed")
    var records: [[String: Any]] = []
    for index in a.indices {
        try require(a[index].mediaType == b[index].mediaType, "media track type/order changed")
        let sourcePackets = try packets(source, a[index]), outputPackets = try packets(output, b[index])
        let firstDifference = zip(sourcePackets, outputPackets).enumerated().first { $0.element.0 != $0.element.1 }?.offset
        if a[index].mediaType == .video {
            let rangeA = try await a[index].load(.timeRange), rangeB = try await b[index].load(.timeRange)
            try require(CMTimeRangeEqual(rangeA, rangeB), "visible video time range changed")
            let decodedA = try videoTimes(source, a[index]), decodedB = try videoTimes(output, b[index])
            try require(decodedA.count == decodedB.count && zip(decodedA, decodedB).allSatisfy { CMTimeCompare($0, $1) == 0 },
                        "visible decoded video timestamps changed")
            let detail = firstDifference.map { "source=\(sourcePackets[$0].detail); output=\(outputPackets[$0].detail)" } ?? ""
            try require(sourcePackets == outputPackets,
                        "compressed bytes or timestamps changed in video track \(index): \(sourcePackets.count)/\(outputPackets.count) groups, first difference \(String(describing: firstDifference)); \(detail)")
        }
        let formatsA = try await a[index].load(.formatDescriptions)
        let formatsB = try await b[index].load(.formatDescriptions)
        try require(formatsA.map(CMFormatDescriptionGetMediaSubType) == formatsB.map(CMFormatDescriptionGetMediaSubType),
                    "codec changed in track \(index)")
        var record: [String: Any] = ["media_type": a[index].mediaType.rawValue,
                                    "reader_groups_input": sourcePackets.count,
                                    "reader_groups_output": outputPackets.count]
        if a[index].mediaType == .video {
            let transformA = try await a[index].load(.preferredTransform)
            let transformB = try await b[index].load(.preferredTransform)
            try require(transformA == transformB, "display orientation changed")
            let sizeA = try await a[index].load(.naturalSize), sizeB = try await b[index].load(.naturalSize)
            try require(sizeA == sizeB, "video geometry changed")
            record["width"] = sizeA.width; record["height"] = sizeA.height
            record["preferred_transform"] = [transformA.a, transformA.b, transformA.c, transformA.d, transformA.tx, transformA.ty]
            record["compressed_bytes_and_timing_identical"] = true
        }
        records.append(record)
    }
    return records
}

@available(macOS 27.0, *)
func strip(_ input: URL, _ output: URL) async throws -> [String: Any] {
    try require(!FileManager.default.fileExists(atPath: output.path), "strip output already exists")
    let source = AVURLAsset(url: input)
    let reader = try AVAssetReader(asset: source)
    let writer = try AVAssetWriter(outputURL: output, fileType: .mov)
    writer.metadata = []
    var pumps: [(AVAssetReaderTrackOutput, AVAssetWriterInput)] = []
    for track in try await mediaTracks(source) {
        let formats = try await track.load(.formatDescriptions)
        let copied = AVAssetWriterInput(mediaType: track.mediaType, outputSettings: nil, sourceFormatHint: formats.first)
        copied.transform = try await track.load(.preferredTransform)
        if track.mediaType == .video { copied.mediaTimeScale = try await track.load(.naturalTimeScale) }
        let from = AVAssetReaderTrackOutput(track: track, outputSettings: nil)
        try require(writer.canAdd(copied) && reader.canAdd(from), "passthrough track is unsupported")
        writer.add(copied); reader.add(from); pumps.append((from, copied))
    }
    try require(reader.startReading() && writer.startWriting(), "cannot start ordinary media copy")
    writer.startSession(atSourceTime: .zero)
    try await withThrowingTaskGroup(of: Void.self) { tasks in
        for (from, copied) in pumps {
            tasks.addTask {
                try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
                    let queue = DispatchQueue(label: "xdremux.cinematic.strip.\(copied.mediaType.rawValue)")
                    var finished = false
                    copied.requestMediaDataWhenReady(on: queue) {
                        guard !finished else { return }
                        while copied.isReadyForMoreMediaData {
                            guard let sample = from.copyNextSampleBuffer() else {
                                finished = true; copied.markAsFinished(); continuation.resume(); return
                            }
                            if !copied.append(sample) {
                                finished = true; copied.markAsFinished()
                                continuation.resume(throwing: writer.error ?? failure("ordinary passthrough failed")); return
                            }
                        }
                    }
                }
            }
        }
        try await tasks.waitForAll()
    }
    await writer.finishWriting()
    try require(reader.status == .completed && writer.status == .completed, "ordinary media copy did not complete")
    let ordinary = AVURLAsset(url: output)
    try require(await CNAssetInfo.cinematicCapability(for: ordinary) == .none, "stripped video still has Cinematic data")
    let tracks = try await ordinary.load(.tracks)
    try require(tracks.allSatisfy { $0.mediaType == .video || $0.mediaType == .audio }, "stripped asset retained an auxiliary track")
    let media = try await preserveMedia(source, ordinary)
    return ["capability": "none", "retained_tracks": media, "cinematic_data_removed": true]
}

@available(macOS 27.0, *)
func verify(_ ordinaryURL: URL, _ candidateURL: URL, _ expectedAperture: Float) async throws -> [String: Any] {
    let ordinary = AVURLAsset(url: ordinaryURL), candidate = AVURLAsset(url: candidateURL)
    try require(await CNAssetInfo.cinematicCapability(for: ordinary) == .none, "test input is not ordinary video")
    try require(await CNAssetInfo.cinematicCapability(for: candidate) == .needsPreprocessing, "public capability is not needsPreprocessing")
    let info = try await CNAssetInfo(asset: candidate)
    try require(info.cinematicCapability == .needsPreprocessing && info.resourceStatus == .ready && !info.isPreprocessed,
                "public asset info rejected pending resources")
    let tracks = try await candidate.load(.tracks)
    try require(!tracks.contains { $0.mediaType.rawValue == "auxv" }, "candidate materialized an auxiliary raster track")
    let script = try await CNScript(asset: candidate)
    try require(abs(script.fNumber - expectedAperture) < 0.001, "script aperture does not match requested aperture")
    let frames = script.frames(in: script.timeRange)
    let decoded = try videoTimes(candidate, info.cinematicVideoTrack)
    try require(!decoded.isEmpty && decoded.count == frames.count, "script and decoded frame counts differ")
    for index in frames.indices {
        try require(CMTimeCompare(frames[index].time, decoded[index]) == 0, "script/video PTS differ at frame \(index)")
        try require(!frames[index].allDetections.isEmpty, "frame has no generated detection graph")
        let selected = frames[index].focusDetection
        try require(selected.detectionID != nil, "selected detection has an invalid track ID")
    }
    let attributes = try await CNRenderingSession.Attributes(asset: candidate)
    let encodedSize = try await info.cinematicVideoTrack.load(.naturalSize)
    let reader = try AVAssetReader(asset: candidate)
    let output = AVAssetReaderTrackOutput(track: info.cinematicMetadataTrack, outputSettings: nil)
    reader.add(output)
    let adaptor = AVAssetReaderOutputMetadataAdaptor(assetReaderTrackOutput: output)
    try require(reader.startReading(), "metadata reader did not start")
    let expectedKeys: Set<String> = ["cinematic-video.cinematography", "cinematic-video.rendering", "aperture-float", "disparity-float"]
    var groupCount = 0
    var payloadDigest = SHA256()
    while let group = adaptor.nextTimedMetadataGroup() {
        let keys = Set(group.items.compactMap { $0.identifier?.rawValue.components(separatedBy: "com.apple.quicktime.").last })
        try require(keys == expectedKeys, "timed metadata keys are incomplete")
        try require(groupCount < decoded.count && CMTimeCompare(group.timeRange.start, decoded[groupCount]) == 0,
                    "timed metadata/video PTS differ at frame \(groupCount)")
        guard let parsed = CNRenderingSession.FrameAttributes(timedMetadataGroup: group, sessionAttributes: attributes) else {
            throw failure("public rendering frame attributes cannot parse")
        }
        try require(abs(parsed.fNumber - expectedAperture) < 0.001 && parsed.focusDisparity == -1,
                    "invalid pending focus/aperture")
        guard let renderingItem = group.items.first(where: { $0.identifier?.rawValue.hasSuffix("cinematic-video.rendering") == true }),
              let renderingData = renderingItem.value as? Data else { throw failure("missing rendering geometry") }
        let bytes = Array(renderingData)
        try require(bytes.count == 112 && Array(bytes[4..<8]) == Array("rend".utf8), "unsupported timed rendering format")
        var geometry: [UInt16: Float] = [:]
        for offset in stride(from: 8, to: bytes.count, by: 8) {
            let tag = UInt16(bytes[offset]) << 8 | UInt16(bytes[offset + 1])
            let type = UInt16(bytes[offset + 2]) << 8 | UInt16(bytes[offset + 3])
            if type == 1 {
                let bits = bytes[offset+4..<offset+8].reduce(UInt32(0)) { ($0 << 8) | UInt32($1) }
                geometry[tag] = Float(bitPattern: bits)
            }
        }
        try require(geometry[4] == Float(encodedSize.width) && geometry[5] == Float(encodedSize.height),
                    "virtual sensor region does not match encoded video geometry")
        try require(geometry[6] == 1 && geometry[7] == 1, "virtual visible crop is not identity")
        for item in group.items { if let data = item.value as? Data { payloadDigest.update(data: data) } }
        groupCount += 1
    }
    try require(reader.status == .completed && groupCount == decoded.count, "incomplete timed metadata sequence")
    let media = try await preserveMedia(ordinary, candidate)
    return ["input_capability": "none", "output_capability": "needsPreprocessing", "resource_status": "ready",
            "is_preprocessed": false, "raster_disparity_track": false, "decoded_frames": decoded.count,
            "script_frames": frames.count, "timed_metadata_groups": groupCount, "script_f_number": script.fNumber,
            "rendering_version": attributes.renderingVersion,
            "virtual_sensor_region": [encodedSize.width, encodedSize.height], "virtual_visible_crop": [1, 1],
            "generated_payload_sha256": payloadDigest.finalize().map { String(format: "%02x", $0) }.joined(),
            "media": media]
}

@main
struct Verification {
    static func main() async {
        do {
            let args = Array(CommandLine.arguments.dropFirst())
            guard args.count >= 4 else { throw failure("usage: probe strip INPUT.mov ORDINARY.mov REPORT.json | verify ORDINARY.mov PENDING.mov REPORT.json APERTURE") }
            guard #available(macOS 27.0, *) else { throw failure("verification requires macOS 27 or later") }
            let a = URL(fileURLWithPath: args[1]), b = URL(fileURLWithPath: args[2])
            let report: [String: Any]
            if args[0] == "strip" && args.count == 4 { report = try await strip(a, b) }
            else if args[0] == "verify" && args.count == 5, let aperture = Float(args[4]) { report = try await verify(a, b, aperture) }
            else { throw failure("invalid verification arguments") }
            let data = try JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys])
            try data.write(to: URL(fileURLWithPath: args[3]), options: .atomic)
            print(String(decoding: data, as: UTF8.self))
        } catch { fputs("Cinematic verification failed: \(error.localizedDescription)\n", stderr); exit(1) }
    }
}
