import Foundation
import AVFoundation
import CoreMedia

private enum ValidationError: Error, CustomStringConvertible {
    case usage
    case noVideoTrack
    case missingMetadataTrack
    case unexpectedMetadataTrack
    case cannotAddReaderOutput
    case cannotStartReader(String)
    case noReadableVideoSample

    var description: String {
        switch self {
        case .usage:
            return "usage: VideoSmartStyleValidator <input.mov> <expected-metadata-track:0|1>"
        case .noVideoTrack:
            return "asset has no video track"
        case .missingMetadataTrack:
            return "expected a timed metadata track but none was present"
        case .unexpectedMetadataTrack:
            return "did not expect a timed metadata track"
        case .cannotAddReaderOutput:
            return "AVAssetReader could not add passthrough video output"
        case .cannotStartReader(let message):
            return "AVAssetReader failed to start: \(message)"
        case .noReadableVideoSample:
            return "AVAssetReader could not read the first encoded video sample"
        }
    }
}

private func validate(url: URL, expectsMetadataTrack: Bool) throws {
    let asset = AVURLAsset(url: url)
    let tracks = asset.tracks
    let videoTracks = tracks.filter { $0.mediaType == .video }
    let audioTracks = tracks.filter { $0.mediaType == .audio }
    let metadataTracks = tracks.filter { $0.mediaType == .metadata }

    guard let video = videoTracks.first else { throw ValidationError.noVideoTrack }
    if expectsMetadataTrack && metadataTracks.isEmpty {
        throw ValidationError.missingMetadataTrack
    }
    if !expectsMetadataTrack && !metadataTracks.isEmpty {
        throw ValidationError.unexpectedMetadataTrack
    }

    let reader = try AVAssetReader(asset: asset)
    let output = AVAssetReaderTrackOutput(track: video, outputSettings: nil)
    output.alwaysCopiesSampleData = false
    guard reader.canAdd(output) else { throw ValidationError.cannotAddReaderOutput }
    reader.add(output)
    guard reader.startReading() else {
        throw ValidationError.cannotStartReader(reader.error?.localizedDescription ?? "unknown")
    }
    guard let sample = output.copyNextSampleBuffer() else {
        throw ValidationError.noReadableVideoSample
    }

    let pts = CMSampleBufferGetPresentationTimeStamp(sample)
    print("OK file=\(url.lastPathComponent) videoTracks=\(videoTracks.count) audioTracks=\(audioTracks.count) metadataTracks=\(metadataTracks.count) firstVideoPTS=\(CMTimeGetSeconds(pts)) readerStatus=\(reader.status.rawValue)")
}

@main
private struct Main {
    static func main() {
        do {
            guard CommandLine.arguments.count == 3,
                  let expected = Int(CommandLine.arguments[2]),
                  expected == 0 || expected == 1 else {
                throw ValidationError.usage
            }
            try validate(
                url: URL(fileURLWithPath: CommandLine.arguments[1]),
                expectsMetadataTrack: expected == 1
            )
        } catch {
            fputs("VideoSmartStyleValidator: \(error)\n", stderr)
            exit(2)
        }
    }
}
