import Foundation

public enum ISOBaseMediaStreamScanner {
    public static func ftypBoxOffsets(
        in url: URL,
        range: MotionPhotoByteRange,
        bufferSize: Int = 1_048_576
    ) throws -> [Int64] {
        guard bufferSize >= 64 else { throw MotionPhotoParsingError.invalidByteRange }
        let handle = try FileHandle(forReadingFrom: url)
        defer { try? handle.close() }
        try handle.seek(toOffset: UInt64(range.lowerBound))

        let needle = Data("ftyp".utf8)
        var remaining = range.length
        var absoluteChunkStart = range.lowerBound
        var carry = Data()
        var offsets = Set<Int64>()

        while remaining > 0 {
            let count = Int(min(Int64(bufferSize), remaining))
            guard let chunk = try handle.read(upToCount: count), !chunk.isEmpty else { break }
            var window = carry
            window.append(chunk)
            let windowStart = absoluteChunkStart - Int64(carry.count)

            var search = window.startIndex
            while search < window.endIndex,
                  let found = window.range(of: needle, in: search..<window.endIndex) {
                let typeIndex = found.lowerBound
                if typeIndex >= window.index(window.startIndex, offsetBy: 4) {
                    let sizeStart = window.index(typeIndex, offsetBy: -4)
                    let sizeBytes = window[sizeStart..<typeIndex]
                    let size = sizeBytes.reduce(UInt32(0)) { ($0 << 8) | UInt32($1) }
                    let boxStart = windowStart + Int64(window.distance(from: window.startIndex, to: sizeStart))
                    if boxStart >= range.lowerBound,
                       boxStart < range.upperBound,
                       (size == 1 || size >= 8) {
                        offsets.insert(boxStart)
                    }
                }
                search = found.upperBound
            }

            carry = window.suffix(min(16, window.count))
            remaining -= Int64(chunk.count)
            absoluteChunkStart += Int64(chunk.count)
        }

        return offsets.sorted()
    }

    public static func isFTYPBoxStart(
        in url: URL,
        offset: Int64,
        upperBound: Int64
    ) throws -> Bool {
        guard offset >= 0, upperBound - offset >= 12 else { return false }
        let handle = try FileHandle(forReadingFrom: url)
        defer { try? handle.close() }
        try handle.seek(toOffset: UInt64(offset))
        let header = try handle.read(upToCount: 16) ?? Data()
        guard header.count >= 8,
              String(bytes: header[4..<8], encoding: .ascii) == "ftyp" else {
            return false
        }
        let size = header.prefix(4).reduce(UInt32(0)) { ($0 << 8) | UInt32($1) }
        if size == 1 {
            guard header.count >= 16 else { return false }
            let largeSize = header[8..<16].reduce(UInt64(0)) { ($0 << 8) | UInt64($1) }
            return largeSize >= 16 && largeSize <= UInt64(upperBound - offset)
        }
        return size >= 8 && Int64(size) <= upperBound - offset
    }
}
