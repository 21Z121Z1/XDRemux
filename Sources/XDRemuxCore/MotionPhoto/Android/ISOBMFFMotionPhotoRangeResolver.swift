import Foundation

/// Resolves Android Motion Photo resources when the primary image is an ISO BMFF image (HEIC/HEIF).
///
/// Android Motion Photo V1 stores the video bytes inside a top-level `mpvd` box and declares a
/// Primary-item padding of 8 bytes for the normal box header. Real Samsung files may append a
/// vendor `sefd` box after `mpvd`; their XMP MotionPhoto Length then spans the `mpvd` payload plus
/// the trailing vendor box. The semantic start still points exactly at the `mpvd` payload. We use
/// that invariant to validate the directory, but extract only the `mpvd` payload as the video.
public enum ISOBMFFMotionPhotoRangeResolver {
    private static let maxTopLevelBoxes = 4_096

    public static func resolve(
        url: URL,
        items: [MotionPhotoItem],
        fileSize: Int64
    ) throws -> (still: MotionPhotoByteRange, video: MotionPhotoByteRange) {
        guard let primary = items.first,
              let motion = items.last,
              isHEIFMime(primary.mime),
              motion.semantic.caseInsensitiveCompare("MotionPhoto") == .orderedSame else {
            throw MotionPhotoParsingError.invalidDirectory
        }
        // Motion Photo V1 requires the 8-byte mpvd box header to be represented as Primary padding
        // for HEIC/AVIF primary images. We currently route HEIC/HEIF only; AVIF remains a follow-up.
        guard primary.padding == 8 else {
            throw MotionPhotoParsingError.invalidItemLength
        }

        let boxes = try topLevelBoxes(in: url, fileSize: fileSize)
        guard boxes.first?.type == "ftyp" else {
            throw MotionPhotoParsingError.invalidVideoPayload
        }
        let mpvdBoxes = boxes.filter { $0.type == "mpvd" }
        guard mpvdBoxes.count == 1, let mpvd = mpvdBoxes.first else {
            throw MotionPhotoParsingError.invalidVideoPayload
        }
        let payloadStart = mpvd.payloadOffset
        let payloadEnd = mpvd.endOffset
        guard payloadStart < payloadEnd else {
            throw MotionPhotoParsingError.invalidVideoPayload
        }

        let (declaredStart, overflow) = fileSize.subtractingReportingOverflow(motion.length)
        guard !overflow, declaredStart >= 0, declaredStart == payloadStart else {
            throw MotionPhotoParsingError.invalidByteRange
        }
        guard try ISOBaseMediaStreamScanner.isFTYPBoxStart(
            in: url,
            offset: payloadStart,
            upperBound: payloadEnd
        ) else {
            throw MotionPhotoParsingError.invalidVideoPayload
        }

        // Removing mpvd (and any vendor boxes after it) leaves a standalone HEIF still container.
        // The paired video is exactly the mpvd payload and never includes Samsung's trailing sefd.
        return (
            try MotionPhotoByteRange(lowerBound: 0, upperBound: mpvd.offset),
            try MotionPhotoByteRange(lowerBound: payloadStart, upperBound: payloadEnd)
        )
    }

    public static func isHEIFMime(_ mime: String) -> Bool {
        let lower = mime.lowercased()
        return lower == "image/heic" || lower == "image/heif"
    }

    private struct TopLevelBox {
        let offset: Int64
        let endOffset: Int64
        let headerSize: Int64
        let type: String

        var payloadOffset: Int64 { offset + headerSize }
    }

    private static func topLevelBoxes(in url: URL, fileSize: Int64) throws -> [TopLevelBox] {
        let handle = try FileHandle(forReadingFrom: url)
        defer { try? handle.close() }

        var boxes: [TopLevelBox] = []
        var offset: Int64 = 0
        while offset < fileSize {
            guard boxes.count < maxTopLevelBoxes else {
                throw MotionPhotoParsingError.invalidVideoPayload
            }
            let remaining = fileSize - offset
            guard remaining >= 8 else {
                throw MotionPhotoParsingError.invalidVideoPayload
            }
            try handle.seek(toOffset: UInt64(offset))
            guard let header = try handle.read(upToCount: 8), header.count == 8 else {
                throw MotionPhotoParsingError.invalidVideoPayload
            }
            let size32 = readUInt32BE(header, at: 0)
            guard let type = String(data: header.subdata(in: 4..<8), encoding: .ascii),
                  type.utf8.count == 4 else {
                throw MotionPhotoParsingError.invalidVideoPayload
            }

            let headerSize: Int64
            let boxSize: Int64
            if size32 == 1 {
                guard remaining >= 16,
                      let extended = try handle.read(upToCount: 8), extended.count == 8 else {
                    throw MotionPhotoParsingError.invalidVideoPayload
                }
                let size64 = readUInt64BE(extended, at: 0)
                guard size64 <= UInt64(Int64.max) else {
                    throw MotionPhotoParsingError.arithmeticOverflow
                }
                headerSize = 16
                boxSize = Int64(size64)
            } else if size32 == 0 {
                headerSize = 8
                boxSize = remaining
            } else {
                headerSize = 8
                boxSize = Int64(size32)
            }

            guard boxSize >= headerSize else {
                throw MotionPhotoParsingError.invalidVideoPayload
            }
            let (endOffset, overflow) = offset.addingReportingOverflow(boxSize)
            guard !overflow, endOffset > offset, endOffset <= fileSize else {
                throw MotionPhotoParsingError.arithmeticOverflow
            }
            boxes.append(
                TopLevelBox(
                    offset: offset,
                    endOffset: endOffset,
                    headerSize: headerSize,
                    type: type
                )
            )
            offset = endOffset
        }
        guard offset == fileSize else {
            throw MotionPhotoParsingError.invalidVideoPayload
        }
        return boxes
    }

    private static func readUInt32BE(_ data: Data, at offset: Int) -> UInt32 {
        data.withUnsafeBytes { raw in
            let bytes = raw.bindMemory(to: UInt8.self)
            return (UInt32(bytes[offset]) << 24)
                | (UInt32(bytes[offset + 1]) << 16)
                | (UInt32(bytes[offset + 2]) << 8)
                | UInt32(bytes[offset + 3])
        }
    }

    private static func readUInt64BE(_ data: Data, at offset: Int) -> UInt64 {
        data.withUnsafeBytes { raw in
            let bytes = raw.bindMemory(to: UInt8.self)
            var value: UInt64 = 0
            for index in 0..<8 {
                value = (value << 8) | UInt64(bytes[offset + index])
            }
            return value
        }
    }
}
