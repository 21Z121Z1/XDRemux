import Foundation

public enum MotionPhotoPayloadExtractor {
    public static func copy(
        range: MotionPhotoByteRange,
        from sourceURL: URL,
        to destinationURL: URL,
        maxBytes: Int64 = 1_073_741_824,
        bufferSize: Int = 1_048_576
    ) throws {
        guard range.length <= maxBytes else { throw MotionPhotoParsingError.payloadTooLarge }
        guard bufferSize > 0 else { throw MotionPhotoParsingError.invalidByteRange }

        let attributes = try FileManager.default.attributesOfItem(atPath: sourceURL.path)
        guard let fileSizeNumber = attributes[.size] as? NSNumber else {
            throw MotionPhotoParsingError.invalidByteRange
        }
        let fileSize = fileSizeNumber.int64Value
        guard range.upperBound <= fileSize else { throw MotionPhotoParsingError.invalidByteRange }

        let parent = destinationURL.deletingLastPathComponent()
        try FileManager.default.createDirectory(at: parent, withIntermediateDirectories: true)
        if FileManager.default.fileExists(atPath: destinationURL.path) {
            try FileManager.default.removeItem(at: destinationURL)
        }
        guard FileManager.default.createFile(atPath: destinationURL.path, contents: nil) else {
            throw CocoaError(.fileWriteUnknown)
        }

        let source = try FileHandle(forReadingFrom: sourceURL)
        let destination = try FileHandle(forWritingTo: destinationURL)
        var succeeded = false
        defer {
            try? source.close()
            try? destination.close()
            if !succeeded { try? FileManager.default.removeItem(at: destinationURL) }
        }

        try source.seek(toOffset: UInt64(range.lowerBound))
        var remaining = range.length
        while remaining > 0 {
            let chunkSize = Int(min(Int64(bufferSize), remaining))
            guard let chunk = try source.read(upToCount: chunkSize), !chunk.isEmpty else {
                throw MotionPhotoParsingError.invalidByteRange
            }
            try destination.write(contentsOf: chunk)
            remaining -= Int64(chunk.count)
        }
        try destination.synchronize()
        succeeded = true
    }
}
