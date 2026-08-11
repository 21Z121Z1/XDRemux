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
}
