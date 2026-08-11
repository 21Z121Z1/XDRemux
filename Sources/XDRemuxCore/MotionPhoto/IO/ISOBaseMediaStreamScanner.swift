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
        var roughOffsets = Set<Int64>()

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
                    let boxStart = windowStart + Int64(window.distance(from: window.startIndex, to: sizeStart))
                    if boxStart >= range.lowerBound, boxStart < range.upperBound {
                        roughOffsets.insert(boxStart)
                    }
                }
                search = found.upperBound
            }

            // Four bytes before a candidate plus the largest ftyp header fields we validate must be
            // available across a chunk boundary. A 32-byte overlap is deliberately conservative.
            carry = window.suffix(min(32, window.count))
            remaining -= Int64(chunk.count)
            absoluteChunkStart += Int64(chunk.count)
        }

        var validated: [Int64] = []
        validated.reserveCapacity(roughOffsets.count)
        for offset in roughOffsets.sorted() {
            if try isFTYPBoxStart(in: url, offset: offset, upperBound: range.upperBound) {
                validated.append(offset)
            }
        }
        return validated
    }

    public static func isFTYPBoxStart(
        in url: URL,
        offset: Int64,
        upperBound: Int64
    ) throws -> Bool {
        guard offset >= 0, upperBound > offset else { return false }
        let available = upperBound - offset
        // ISO BMFF FileTypeBox is at least: size(4) + type(4) + major_brand(4) + minor_version(4).
        guard available >= 16 else { return false }

        let handle = try FileHandle(forReadingFrom: url)
        defer { try? handle.close() }
        try handle.seek(toOffset: UInt64(offset))
        let header = try handle.read(upToCount: 24) ?? Data()
        guard header.count >= 16,
              String(bytes: header[4..<8], encoding: .ascii) == "ftyp" else {
            return false
        }

        let size32 = header.prefix(4).reduce(UInt32(0)) { ($0 << 8) | UInt32($1) }
        let majorBrandRange: Range<Data.Index>
        if size32 == 1 {
            // Large-size box adds an 8-byte largesize field before major_brand.
            guard available >= 24, header.count >= 24 else { return false }
            let largeSize = header[8..<16].reduce(UInt64(0)) { ($0 << 8) | UInt64($1) }
            guard largeSize >= 24, largeSize <= UInt64(available) else { return false }
            majorBrandRange = 16..<20
        } else {
            guard size32 >= 16, Int64(size32) <= available else { return false }
            majorBrandRange = 8..<12
        }

        // A valid brand is a printable four-character code (spaces are valid, e.g. QuickTime's
        // `qt  `). This removes the common false-positive case of the byte sequence "ftyp" inside
        // arbitrary media payloads without assuming a fixed brand allow-list.
        let majorBrand = header[majorBrandRange]
        guard majorBrand.count == 4,
              majorBrand.allSatisfy({ $0 >= 0x20 && $0 <= 0x7e }) else {
            return false
        }
        return true
    }
}
