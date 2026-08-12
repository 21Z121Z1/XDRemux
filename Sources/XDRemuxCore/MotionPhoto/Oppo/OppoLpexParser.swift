import Foundation

public enum OppoLpexParser {
    private static let needles = [
        Data("lpexLivePhotoExtension".utf8),
        Data("LivePhotoExtension".utf8),
        Data("pexLivePhotoExtension".utf8),
    ]
    private static let maxJSONBytes = 256 * 1024
    private static let scanChunkBytes = 2 * 1024 * 1024
    private static let overlapBytes = maxJSONBytes + 128

    public static func parse(from url: URL) throws -> OppoMotionPhotoMetadata? {
        let handle = try FileHandle(forReadingFrom: url)
        defer { try? handle.close() }
        let fileSize = try handle.seekToEnd()
        try handle.seek(toOffset: 0)

        var absoluteOffset: UInt64 = 0
        var carry = Data()
        while absoluteOffset < fileSize {
            let readCount = Int(min(UInt64(scanChunkBytes), fileSize - absoluteOffset))
            let chunk = try handle.read(upToCount: readCount) ?? Data()
            if chunk.isEmpty { break }

            var window = Data()
            window.reserveCapacity(carry.count + chunk.count)
            window.append(carry)
            window.append(chunk)
            if let metadata = parseFirstObject(in: window) { return metadata }

            if window.count > overlapBytes {
                carry = window.suffix(overlapBytes)
            } else {
                carry = window
            }
            absoluteOffset += UInt64(chunk.count)
        }
        return nil
    }

    static func parseFirstObject(in data: Data) -> OppoMotionPhotoMetadata? {
        for needle in needles {
            var searchStart = data.startIndex
            while searchStart < data.endIndex,
                  let found = data.range(of: needle, in: searchStart..<data.endIndex) {
                let after = found.upperBound
                guard after < data.endIndex else { break }
                guard let brace = data[after..<data.endIndex].firstIndex(of: UInt8(ascii: "{")) else {
                    searchStart = found.upperBound
                    continue
                }
                guard data.distance(from: after, to: brace) <= 32 else {
                    searchStart = found.upperBound
                    continue
                }
                if let objectRange = extractJSONObjectRange(in: data, startingAt: brace),
                   objectRange.count <= maxJSONBytes,
                   let parsed = parseJSON(data.subdata(in: objectRange)) {
                    return parsed
                }
                searchStart = found.upperBound
            }
        }
        return nil
    }

    static func extractJSONObjectRange(in data: Data, startingAt startIndex: Data.Index) -> Range<Data.Index>? {
        guard startIndex < data.endIndex, data[startIndex] == UInt8(ascii: "{") else { return nil }
        var depth = 0
        var inString = false
        var escaping = false
        var index = startIndex
        while index < data.endIndex {
            let byte = data[index]
            if inString {
                if escaping {
                    escaping = false
                } else if byte == UInt8(ascii: "\\") {
                    escaping = true
                } else if byte == UInt8(ascii: "\"") {
                    inString = false
                }
            } else if byte == UInt8(ascii: "\"") {
                inString = true
            } else if byte == UInt8(ascii: "{") {
                depth += 1
            } else if byte == UInt8(ascii: "}") {
                depth -= 1
                if depth == 0 { return startIndex..<data.index(after: index) }
                if depth < 0 { return nil }
            }
            if data.distance(from: startIndex, to: index) > maxJSONBytes { return nil }
            index = data.index(after: index)
        }
        return nil
    }

    static func parseJSON(_ data: Data) -> OppoMotionPhotoMetadata? {
        guard data.count <= maxJSONBytes,
              let object = try? JSONSerialization.jsonObject(with: data, options: []),
              let dictionary = object as? [String: Any] else {
            return nil
        }

        let coverFramePts = int64(dictionary["coverFramePts"])
        let version = int(dictionary["version"]) ?? 0
        let matrixCount = int(dictionary["matrixCount"]) ?? 0
        let photoCropMatrix = matrix(dictionary["photoCropMatrix"])
        let photoEisMatrix = matrix(dictionary["photoEisMatrix"])

        var matrices: [String: [Double]] = [:]
        if let raw = dictionary["matrices"] as? [String: Any], raw.count <= 4096 {
            for (key, value) in raw where key.utf8.count <= 128 {
                if let parsed = matrix(value) { matrices[key] = parsed }
            }
        }

        let videoSize = size(dictionary["videoSize"])
        let originPhotoSize = size(dictionary["originPhotoSize"])
        let photoEisCropFactor = numberArray(dictionary["photoEisCropFactor"], maxCount: 8)
        let eisCropFactor = numberArray(dictionary["eisCropFactor"], maxCount: 8)
        let photoCropFactor = double(dictionary["photoCropFactor"])

        return OppoMotionPhotoMetadata(
            coverFramePtsUs: coverFramePts,
            version: version,
            matrixCount: matrixCount,
            photoCropMatrix: photoCropMatrix,
            photoEisMatrix: photoEisMatrix,
            matrices: matrices,
            videoWidth: videoSize?.0,
            videoHeight: videoSize?.1,
            originPhotoWidth: originPhotoSize?.0,
            originPhotoHeight: originPhotoSize?.1,
            photoEisCropFactor: photoEisCropFactor,
            eisCropFactor: eisCropFactor,
            photoCropFactor: photoCropFactor
        )
    }

    private static func matrix(_ value: Any?) -> [Double]? {
        guard let array = numberArray(value, maxCount: 9), array.count == 9,
              array.allSatisfy(\.isFinite) else { return nil }
        return array
    }

    private static func size(_ value: Any?) -> (Int, Int)? {
        guard let array = value as? [Any], array.count >= 2,
              let width = int(array[0]), let height = int(array[1]),
              width > 0, height > 0 else { return nil }
        return (width, height)
    }

    private static func numberArray(_ value: Any?, maxCount: Int) -> [Double]? {
        guard let array = value as? [Any], array.count <= maxCount else { return nil }
        let result = array.compactMap(double)
        guard result.count == array.count, result.allSatisfy(\.isFinite) else { return nil }
        return result
    }

    private static func int64(_ value: Any?) -> Int64? {
        if let value = value as? NSNumber { return value.int64Value }
        if let value = value as? Int64 { return value }
        if let value = value as? Int { return Int64(value) }
        if let value = value as? String, value.utf8.count <= 32 { return Int64(value) }
        return nil
    }

    private static func int(_ value: Any?) -> Int? {
        if let value = value as? NSNumber { return value.intValue }
        if let value = value as? Int { return value }
        if let value = value as? String, value.utf8.count <= 32 { return Int(value) }
        return nil
    }

    private static func double(_ value: Any?) -> Double? {
        if let value = value as? NSNumber { return value.doubleValue }
        if let value = value as? Double { return value }
        if let value = value as? Int { return Double(value) }
        if let value = value as? String, value.utf8.count <= 64 { return Double(value) }
        return nil
    }
}
