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
        let resolved = try resolveVideoRange(
            url: url,
            fileSize: fileSize,
            declaredLength: declaredLength,
            lpex: lpex
        )
        guard let resolved else { return nil }
        let videoRange = resolved.range
        let streamCount = resolved.streamCount

        let stillRange = try MotionPhotoByteRange(lowerBound: 0, upperBound: videoRange.lowerBound)
        let rawMetadata = lpex ?? OppoMotionPhotoMetadata()
        let metadata = OppoMotionPhotoMetadata(
            coverFramePtsUs: rawMetadata.coverFramePtsUs,
            version: rawMetadata.version,
            matrixCount: rawMetadata.matrixCount,
            photoCropMatrix: rawMetadata.photoCropMatrix,
            photoEisMatrix: rawMetadata.photoEisMatrix,
            matrices: rawMetadata.matrices,
            videoWidth: rawMetadata.videoWidth,
            videoHeight: rawMetadata.videoHeight,
            originPhotoWidth: rawMetadata.originPhotoWidth,
            originPhotoHeight: rawMetadata.originPhotoHeight,
            eisCropFactor: rawMetadata.eisCropFactor,
            photoCropFactor: rawMetadata.photoCropFactor,
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

    private static func resolveVideoRange(
        url: URL,
        fileSize: Int64,
        declaredLength: Int64?,
        lpex: OppoMotionPhotoMetadata?
    ) throws -> (range: MotionPhotoByteRange, streamCount: Int)? {
        let tailStart = max(0, fileSize - maxTailScanBytes)
        let scanRange = try MotionPhotoByteRange(lowerBound: tailStart, upperBound: fileSize)

        // ColorOS 16 uses an LPEX v1+ layout with two concatenated BMFF streams. Its VideoLength
        // can legitimately point only to the final (Stream 2) payload, so accepting that length
        // before looking at topology would discard Stream 1. Resolve the two-stream layout first.
        if let lpex, lpex.version >= 1 {
            let offsets = try ISOBaseMediaStreamScanner.ftypBoxOffsets(in: url, range: scanRange)
            if offsets.count >= 2 {
                return (
                    try MotionPhotoByteRange(
                        lowerBound: offsets[offsets.count - 2],
                        upperBound: fileSize
                    ),
                    2
                )
            }
        }

        // Prefer the vendor-declared length for single-stream files when it resolves exactly to a
        // valid BMFF stream start. Old OPPO files can retain stale length metadata after edits, so
        // a bad declared length falls through to the bounded recovery scan instead of rejecting the
        // otherwise OPPO-signed input.
        if let declaredLength, declaredLength > 0, declaredLength <= fileSize {
            let start = fileSize - declaredLength
            if try ISOBaseMediaStreamScanner.isFTYPBoxStart(
                in: url,
                offset: start,
                upperBound: fileSize
            ) {
                let range = try MotionPhotoByteRange(lowerBound: start, upperBound: fileSize)
                let count = max(
                    1,
                    try ISOBaseMediaStreamScanner.ftypBoxOffsets(in: url, range: range).count
                )
                return (range, count)
            }
        }

        let offsets = try ISOBaseMediaStreamScanner.ftypBoxOffsets(in: url, range: scanRange)
        guard let last = offsets.last else { return nil }
        return (
            try MotionPhotoByteRange(lowerBound: last, upperBound: fileSize),
            1
        )
    }

    private static func readPrefix(url: URL, count: Int64) throws -> Data {
        let handle = try FileHandle(forReadingFrom: url)
        defer { try? handle.close() }
        return try handle.read(upToCount: Int(count)) ?? Data()
    }

    private static func extractXMPString(from data: Data) -> String? {
        let openingCandidates = [Data("<x:xmpmeta".utf8), Data("<xmpmeta".utf8)]
        let closingCandidates = [Data("</x:xmpmeta>".utf8), Data("</xmpmeta>".utf8)]
        guard let start = openingCandidates.compactMap({ data.range(of: $0)?.lowerBound }).min() else {
            return nil
        }
        var end: Data.Index?
        for closing in closingCandidates {
            if let range = data.range(of: closing, in: start..<data.endIndex) {
                end = end.map { min($0, range.upperBound) } ?? range.upperBound
            }
        }
        guard let end else { return nil }
        return String(data: data.subdata(in: start..<end), encoding: .utf8)
    }

    private static func extractVideoLength(from xmp: String) -> Int64? {
        // Preserve LivePhotoToolbox's successful heuristic for non-standard OPPO files: choose the
        // largest plausible Item:Length first, then fall back to explicit VideoLength tags. This is
        // only used after standards-compliant Container:Directory parsing has already failed.
        var genericLengths: [Int64] = []
        let genericPatterns = [
            #"Item:Length\s*=\s*["']?(\d+)"#,
            #"Item:Length\s*>(\d+)"#,
            #"Length\s*=\s*["']?(\d+)"#,
        ]
        for pattern in genericPatterns {
            genericLengths.append(contentsOf: integerMatches(pattern: pattern, in: xmp))
        }
        if let maxLength = genericLengths.max(), maxLength > 100_000 {
            return maxLength
        }

        let tags = ["OpCamera:VideoLength", "GCamera:VideoLength", "VideoLength"]
        for tag in tags {
            if let value = extractXMPValue(from: xmp, tagName: tag),
               let length = Int64(value), length > 100_000 {
                return length
            }
        }
        return nil
    }

    private static func extractPresentationTimestamp(from xmp: String) -> Int64? {
        let tags = [
            "GCamera:MotionPhotoPresentationTimestampUs",
            "MotionPhotoPresentationTimestampUs",
            "GCamera:MicroVideoPresentationTimestampUs",
        ]
        for tag in tags {
            if let value = extractXMPValue(from: xmp, tagName: tag),
               let timestamp = Int64(value) {
                return timestamp == -1 ? nil : timestamp
            }
        }
        return nil
    }

    private static func integerMatches(pattern: String, in string: String) -> [Int64] {
        guard let regex = try? NSRegularExpression(pattern: pattern) else { return [] }
        let range = NSRange(string.startIndex..<string.endIndex, in: string)
        return regex.matches(in: string, range: range).compactMap { match in
            guard match.numberOfRanges >= 2,
                  let valueRange = Range(match.range(at: 1), in: string),
                  let value = Int64(string[valueRange]), value > 0 else {
                return nil
            }
            return value
        }
    }

    private static func extractXMPValue(from xmp: String, tagName: String) -> String? {
        let escaped = NSRegularExpression.escapedPattern(for: tagName)
        let patterns = [
            "<\(escaped)>([^<]+)</\(escaped)>",
            "\(escaped)=[\"']([^\"']+)[\"']",
        ]
        for pattern in patterns {
            guard let regex = try? NSRegularExpression(pattern: pattern) else { continue }
            let range = NSRange(xmp.startIndex..<xmp.endIndex, in: xmp)
            guard let match = regex.firstMatch(in: xmp, range: range),
                  match.numberOfRanges >= 2,
                  let valueRange = Range(match.range(at: 1), in: xmp) else { continue }
            return String(xmp[valueRange]).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        return nil
    }
}
