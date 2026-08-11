import Foundation

/// Compatibility parser for OPPO JPEG Live Photos that predate or deviate from Android Motion
/// Photo V1 directory semantics. It is deliberately gated on OPPO-specific metadata so a random
/// JPEG with appended BMFF bytes is not accepted as an OPPO Live Photo.
public enum OppoMotionPhotoFallbackParser {
    private static let maxHeaderBytes = 4 * 1024 * 1024
    private static let maxTailScanBytes: Int64 = 512 * 1024 * 1024

    public static func parse(url: URL) throws -> MotionPhotoAsset? {
        let attributes = try FileManager.default.attributesOfItem(atPath: url.path)
        guard let sizeNumber = attributes[.size] as? NSNumber else { return nil }
        let fileSize = sizeNumber.int64Value
        guard fileSize >= 16 else { return nil }

        let header = try readPrefix(url: url, count: min(Int64(maxHeaderBytes), fileSize))
        let xmp = extractXMPString(from: header)
        let lpex = try OppoLpexParser.parse(from: url)
        let hasOppoSignature = lpex != nil
            || xmp?.contains("OpCamera:") == true
            || xmp?.localizedCaseInsensitiveContains("oppo") == true
            || xmp?.localizedCaseInsensitiveContains("oplus") == true
        guard hasOppoSignature else { return nil }

        let declaredLength = xmp.flatMap(extractVideoLength)
        let presentation = xmp.flatMap(extractPresentationTimestamp)
        let videoRange: MotionPhotoByteRange
        var streamCount = 1

        if let declaredLength, declaredLength > 0, declaredLength <= fileSize {
            let start = fileSize - declaredLength
            guard try ISOBaseMediaStreamScanner.isFTYPBoxStart(
                in: url,
                offset: start,
                upperBound: fileSize
            ) else {
                return nil
            }
            videoRange = try MotionPhotoByteRange(lowerBound: start, upperBound: fileSize)
            streamCount = max(
                1,
                try ISOBaseMediaStreamScanner.ftypBoxOffsets(in: url, range: videoRange).count
            )
        } else {
            let tailStart = max(0, fileSize - maxTailScanBytes)
            let scanRange = try MotionPhotoByteRange(lowerBound: tailStart, upperBound: fileSize)
            let offsets = try ISOBaseMediaStreamScanner.ftypBoxOffsets(in: url, range: scanRange)
            guard let last = offsets.last else { return nil }
            if let lpex, lpex.version >= 1, offsets.count >= 2 {
                videoRange = try MotionPhotoByteRange(
                    lowerBound: offsets[offsets.count - 2],
                    upperBound: fileSize
                )
                streamCount = 2
            } else {
                videoRange = try MotionPhotoByteRange(lowerBound: last, upperBound: fileSize)
                streamCount = 1
            }
        }

        let stillRange = try MotionPhotoByteRange(lowerBound: 0, upperBound: videoRange.lowerBound)
        var metadata = lpex ?? OppoMotionPhotoMetadata()
        metadata = OppoMotionPhotoMetadata(
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
        let selectedPresentation = presentation ?? metadata.coverFramePtsUs
        let selectedSource: MotionPhotoPresentationSource? = presentation != nil
            ? .androidXMP
            : (metadata.coverFramePtsUs != nil ? .oppoCoverFrame : nil)

        return MotionPhotoAsset(
            sourceURL: url,
            sourceKind: .oppoLivePhoto,
            items: [
                MotionPhotoItem(mime: "image/jpeg", semantic: "Primary", length: 0, padding: 0),
                MotionPhotoItem(
                    mime: "video/mp4",
                    semantic: "MotionPhoto",
                    length: videoRange.length,
                    padding: 0
                ),
            ],
            stillResourceRange: stillRange,
            videoResourceRange: videoRange,
            presentationTimestampUs: selectedPresentation,
            presentationSource: selectedSource,
            vendorMetadata: metadata
        )
    }

    private static func readPrefix(url: URL, count: Int64) throws -> Data {
        let handle = try FileHandle(forReadingFrom: url)
        defer { try? handle.close() }
        return try handle.read(upToCount: Int(count)) ?? Data()
    }

    private static func extractXMPString(from data: Data) -> String? {
        guard let start = data.range(of: Data("<x:xmpmeta".utf8))?.lowerBound,
              let endRange = data.range(
                of: Data("</x:xmpmeta>".utf8),
                in: start..<data.endIndex
              ) else { return nil }
        return String(data: data.subdata(in: start..<endRange.upperBound), encoding: .utf8)
    }

    private static func extractVideoLength(from xmp: String) -> Int64? {
        var values: [Int64] = []
        let patterns = [
            #"Item:Length\s*=\s*["'](\d+)["']"#,
            #"OpCamera:VideoLength\s*=\s*["'](\d+)["']"#,
            #"GCamera:VideoLength\s*=\s*["'](\d+)["']"#,
            #"<OpCamera:VideoLength>\s*(\d+)\s*</OpCamera:VideoLength>"#,
            #"<GCamera:VideoLength>\s*(\d+)\s*</GCamera:VideoLength>"#,
        ]
        for pattern in patterns {
            guard let regex = try? NSRegularExpression(pattern: pattern) else { continue }
            let range = NSRange(xmp.startIndex..<xmp.endIndex, in: xmp)
            for match in regex.matches(in: xmp, range: range) where match.numberOfRanges >= 2 {
                if let swiftRange = Range(match.range(at: 1), in: xmp),
                   let value = Int64(xmp[swiftRange]), value > 0 {
                    values.append(value)
                }
            }
        }
        return values.max()
    }

    private static func extractPresentationTimestamp(from xmp: String) -> Int64? {
        let names = [
            "Camera:MotionPhotoPresentationTimestampUs",
            "GCamera:MotionPhotoPresentationTimestampUs",
            "MotionPhotoPresentationTimestampUs",
            "GCamera:MicroVideoPresentationTimestampUs",
        ]
        for name in names {
            let escaped = NSRegularExpression.escapedPattern(for: name)
            let patterns = [
                "\(escaped)\\s*=\\s*[\"'](-?\\d+)[\"']",
                "<\(escaped)>\\s*(-?\\d+)\\s*</\(escaped)>",
            ]
            for pattern in patterns {
                guard let regex = try? NSRegularExpression(pattern: pattern) else { continue }
                let range = NSRange(xmp.startIndex..<xmp.endIndex, in: xmp)
                guard let match = regex.firstMatch(in: xmp, range: range),
                      match.numberOfRanges >= 2,
                      let swiftRange = Range(match.range(at: 1), in: xmp),
                      let value = Int64(xmp[swiftRange]) else { continue }
                return value == -1 ? nil : value
            }
        }
        return nil
    }
}
