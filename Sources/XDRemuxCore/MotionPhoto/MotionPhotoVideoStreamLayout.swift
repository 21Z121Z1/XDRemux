import Foundation

/// Semantic role of one ISO-BMFF stream embedded in a Motion Photo resource.
///
/// The Apple paired movie always uses ``primary``. An ``auxiliaryGeometry`` stream is analysis-only:
/// it may provide a wider or differently stabilized view for geometry estimation, but its compressed
/// samples are never copied into the paired Live Photo MOV.
public enum MotionPhotoVideoStreamRole: String, Sendable, Equatable {
    case primary
    case auxiliaryGeometry
}

public struct MotionPhotoVideoStream: Sendable, Equatable {
    public let index: Int
    public let role: MotionPhotoVideoStreamRole
    public let range: MotionPhotoByteRange

    public init(index: Int, role: MotionPhotoVideoStreamRole, range: MotionPhotoByteRange) {
        self.index = index
        self.role = role
        self.range = range
    }
}

public struct MotionPhotoVideoStreamLayout: Sendable, Equatable {
    public let primary: MotionPhotoVideoStream
    public let auxiliaryGeometry: [MotionPhotoVideoStream]

    public init(primary: MotionPhotoVideoStream, auxiliaryGeometry: [MotionPhotoVideoStream] = []) {
        self.primary = primary
        self.auxiliaryGeometry = auxiliaryGeometry
    }

    public var allStreams: [MotionPhotoVideoStream] {
        [primary] + auxiliaryGeometry
    }
}

/// Resolves the semantic video streams that XDRemux is allowed to consume.
///
/// Only layouts demonstrated by real fixtures belong here. The current dual-stream rule is the
/// ColorOS 16 OPPO layout: the penultimate `ftyp` starts Stream 1 (the paired movie) and the final
/// `ftyp` starts Stream 2 (an analysis-only auxiliary view). Generic Android/Samsung Motion Photo
/// resources remain a single primary stream even if unrelated vendor bytes happen to resemble BMFF.
/// Future vendors can add a new evidence-backed case without changing downstream geometry APIs.
public enum MotionPhotoVideoStreamLayoutResolver {
    public static func resolve(for asset: MotionPhotoAsset) throws -> MotionPhotoVideoStreamLayout {
        guard asset.sourceKind == .oppoLivePhoto,
              (asset.vendorMetadata?.streamCount ?? 1) >= 2 else {
            return MotionPhotoVideoStreamLayout(
                primary: MotionPhotoVideoStream(
                    index: 0,
                    role: .primary,
                    range: asset.videoResourceRange
                )
            )
        }

        let offsets = try ISOBaseMediaStreamScanner.ftypBoxOffsets(
            in: asset.sourceURL,
            range: asset.videoResourceRange
        )
        guard offsets.count >= 2 else {
            return MotionPhotoVideoStreamLayout(
                primary: MotionPhotoVideoStream(
                    index: 0,
                    role: .primary,
                    range: asset.videoResourceRange
                )
            )
        }

        let stream1Start = offsets[offsets.count - 2]
        let stream2Start = offsets[offsets.count - 1]
        guard stream1Start >= asset.videoResourceRange.lowerBound,
              stream2Start > stream1Start,
              stream2Start < asset.videoResourceRange.upperBound else {
            throw MotionPhotoParsingError.invalidVideoPayload
        }

        let primaryRange = try MotionPhotoByteRange(
            lowerBound: stream1Start,
            upperBound: stream2Start
        )
        let auxiliaryRange = try MotionPhotoByteRange(
            lowerBound: stream2Start,
            upperBound: asset.videoResourceRange.upperBound
        )
        return MotionPhotoVideoStreamLayout(
            primary: MotionPhotoVideoStream(index: 0, role: .primary, range: primaryRange),
            auxiliaryGeometry: [
                MotionPhotoVideoStream(index: 1, role: .auxiliaryGeometry, range: auxiliaryRange)
            ]
        )
    }
}
