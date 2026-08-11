import Foundation

public enum OppoMotionPhotoStreamResolver {
    /// Returns the primary OPPO Live Photo video stream. ColorOS 16 commonly stores two
    /// concatenated BMFF streams; the penultimate ftyp starts Stream 1 and the last starts Stream 2.
    /// This preserves the behavior of LivePhotoToolbox without making generic Android parsing
    /// depend on OPPO's dual-stream convention.
    public static func primaryVideoRange(for asset: MotionPhotoAsset) throws -> MotionPhotoByteRange {
        guard asset.sourceKind == .oppoLivePhoto,
              (asset.vendorMetadata?.streamCount ?? 1) >= 2 else {
            return asset.videoResourceRange
        }
        let offsets = try ISOBaseMediaStreamScanner.ftypBoxOffsets(
            in: asset.sourceURL,
            range: asset.videoResourceRange
        )
        guard offsets.count >= 2 else { return asset.videoResourceRange }
        let stream1Start = offsets[offsets.count - 2]
        let stream2Start = offsets[offsets.count - 1]
        guard stream1Start >= asset.videoResourceRange.lowerBound,
              stream2Start > stream1Start,
              stream2Start <= asset.videoResourceRange.upperBound else {
            throw MotionPhotoParsingError.invalidVideoPayload
        }
        return try MotionPhotoByteRange(lowerBound: stream1Start, upperBound: stream2Start)
    }
}
