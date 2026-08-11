import Foundation

public enum OppoMotionPhotoParser {
    public static func parse(url: URL) throws -> MotionPhotoAsset? {
        guard let base = try AndroidMotionPhotoParser.parse(url: url) else { return nil }
        return try enrichIfPresent(base)
    }

    public static func enrichIfPresent(_ asset: MotionPhotoAsset) throws -> MotionPhotoAsset {
        guard let metadata = try OppoLpexParser.parse(from: asset.sourceURL) else { return asset }
        let offsets = try ISOBaseMediaStreamScanner.ftypBoxOffsets(
            in: asset.sourceURL,
            range: asset.videoResourceRange
        )
        let enriched = OppoMotionPhotoMetadata(
            coverFramePtsUs: metadata.coverFramePtsUs,
            version: metadata.version,
            matrixCount: metadata.matrixCount,
            photoCropMatrix: metadata.photoCropMatrix,
            photoEisMatrix: metadata.photoEisMatrix,
            matrices: metadata.matrices,
            videoWidth: metadata.videoWidth,
            videoHeight: metadata.videoHeight,
            originPhotoWidth: metadata.originPhotoWidth,
            originPhotoHeight: metadata.originPhotoHeight,
            eisCropFactor: metadata.eisCropFactor,
            photoCropFactor: metadata.photoCropFactor,
            streamCount: max(1, offsets.count)
        )

        if asset.presentationTimestampUs == nil, let cover = enriched.coverFramePtsUs {
            return asset.enrichingWithOppoMetadata(
                enriched,
                presentationTimestampUs: cover,
                presentationSource: .oppoCoverFrame
            )
        }
        return asset.enrichingWithOppoMetadata(enriched)
    }
}
