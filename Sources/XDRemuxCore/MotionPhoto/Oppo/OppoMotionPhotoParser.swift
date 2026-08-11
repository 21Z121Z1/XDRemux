import Foundation

public enum OppoMotionPhotoParser {
    private static let maxVendorTailScanBytes: Int64 = 512 * 1024 * 1024

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

        let attributes = try FileManager.default.attributesOfItem(atPath: asset.sourceURL.path)
        guard let sizeNumber = attributes[.size] as? NSNumber else { return asset }
        let fileSize = sizeNumber.int64Value
        let scanStart = max(0, fileSize - maxVendorTailScanBytes)
        let scanRange = try MotionPhotoByteRange(lowerBound: scanStart, upperBound: fileSize)
        let allTailOffsets = try ISOBaseMediaStreamScanner.ftypBoxOffsets(
            in: asset.sourceURL,
            range: scanRange
        )

        // ColorOS 16 OPPO Live Photos can carry two concatenated BMFF streams while the Android
        // directory / VideoLength metadata describes only the final stream. In that case treating
        // the directory's videoStart as authoritative incorrectly leaves Stream 1 attached to the
        // still image and later chooses Stream 2. LPEX version >= 1 is the vendor signal used by the
        // original toolbox; the last two validated ftyp starts therefore define Stream 1 + Stream 2.
        let correctedStillRange: MotionPhotoByteRange
        let correctedVideoRange: MotionPhotoByteRange
        let streamCount: Int
        if metadata.version >= 1, allTailOffsets.count >= 2 {
            let stream1Start = allTailOffsets[allTailOffsets.count - 2]
            correctedStillRange = try MotionPhotoByteRange(lowerBound: 0, upperBound: stream1Start)
            correctedVideoRange = try MotionPhotoByteRange(lowerBound: stream1Start, upperBound: fileSize)
            streamCount = 2
        } else {
            correctedStillRange = asset.stillResourceRange
            correctedVideoRange = asset.videoResourceRange
            let offsetsInsideDeclaredRange = allTailOffsets.filter {
                $0 >= asset.videoResourceRange.lowerBound && $0 < asset.videoResourceRange.upperBound
            }
            streamCount = max(1, offsetsInsideDeclaredRange.count)
        }

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
            streamCount: streamCount
        )

        let selectedPresentation = asset.presentationTimestampUs ?? enriched.coverFramePtsUs
        let selectedSource: MotionPhotoPresentationSource? = asset.presentationTimestampUs != nil
            ? asset.presentationSource
            : (enriched.coverFramePtsUs != nil ? .oppoCoverFrame : nil)

        return MotionPhotoAsset(
            sourceURL: asset.sourceURL,
            sourceKind: .oppoLivePhoto,
            items: asset.items,
            stillResourceRange: correctedStillRange,
            videoResourceRange: correctedVideoRange,
            presentationTimestampUs: selectedPresentation,
            presentationSource: selectedSource,
            vendorMetadata: enriched
        )
    }
}
