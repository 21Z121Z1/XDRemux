import Foundation

public enum OppoMotionPhotoParser {
    public static func parse(url: URL) throws -> MotionPhotoAsset? {
        do {
            if let base = try AndroidMotionPhotoParser.parse(url: url) {
                return try enrichIfPresent(base)
            }
        } catch {
            if let fallback = try OppoMotionPhotoFallbackParser.parse(url: url) {
                return fallback
            }
            throw error
        }
        return try OppoMotionPhotoFallbackParser.parse(url: url)
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

        // LivePhotoToolbox selected the XMP presentation timestamp first, then coverFramePts from
        // LPEX. Keep that behavior while retaining both values for diagnostics.
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
