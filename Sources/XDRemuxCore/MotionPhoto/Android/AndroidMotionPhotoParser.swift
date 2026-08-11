import Foundation
import FoundationXML

public enum AndroidMotionPhotoParser {
    private static let maxXMPScanBytes = 4 * 1024 * 1024
    private static let maxDirectoryItems = 64
    private static let maxMetadataStringLength = 4096

    public static func parse(url: URL) throws -> MotionPhotoAsset? {
        let attributes = try FileManager.default.attributesOfItem(atPath: url.path)
        guard let fileSizeNumber = attributes[.size] as? NSNumber else {
            throw MotionPhotoParsingError.fileTooSmall
        }
        let fileSize = fileSizeNumber.int64Value
        guard fileSize >= 16 else { throw MotionPhotoParsingError.fileTooSmall }

        guard let xmp = try extractXMP(from: url, fileSize: fileSize) else { return nil }
        let description = try parseXMP(xmp)
        guard description.motionPhotoEnabled else { return nil }

        let sourceKind: MotionPhotoSourceKind
        let items: [MotionPhotoItem]
        let timestampSource: MotionPhotoPresentationSource?

        if !description.items.isEmpty {
            guard description.version == 1 else {
                throw MotionPhotoParsingError.unsupportedVersion(description.version)
            }
            sourceKind = .androidMotionPhotoV1
            items = description.items
            timestampSource = description.presentationTimestampUs == nil ? nil : .androidXMP
        } else if let legacyOffset = description.legacyMicroVideoOffset {
            guard legacyOffset > 0 else { throw MotionPhotoParsingError.invalidItemLength }
            sourceKind = .legacyMicroVideoV1b
            items = [
                MotionPhotoItem(mime: "image/jpeg", semantic: "Primary", length: 0, padding: 0),
                MotionPhotoItem(mime: "video/mp4", semantic: "MotionPhoto", length: legacyOffset, padding: 0),
            ]
            timestampSource = description.presentationTimestampUs == nil ? nil : .legacyMicroVideoXMP
        } else {
            throw MotionPhotoParsingError.invalidDirectory
        }

        try validateDirectory(items)
        let ranges = try deriveRanges(items: items, fileSize: fileSize)
        try validateISOBaseMediaPayload(url: url, range: ranges.video)

        return MotionPhotoAsset(
            sourceURL: url,
            sourceKind: sourceKind,
            items: items,
            stillResourceRange: ranges.still,
            videoResourceRange: ranges.video,
            presentationTimestampUs: description.presentationTimestampUs,
            presentationSource: timestampSource
        )
    }

    private static func extractXMP(from url: URL, fileSize: Int64) throws -> Data? {
        let scanLength = min(Int64(maxXMPScanBytes), fileSize)
        let handle = try FileHandle(forReadingFrom: url)
        defer { try? handle.close() }
        let prefix = try handle.read(upToCount: Int(scanLength)) ?? Data()

        let openingCandidates = [Data("<x:xmpmeta".utf8), Data("<xmpmeta".utf8)]
        let closingCandidates = [Data("</x:xmpmeta>".utf8), Data("</xmpmeta>".utf8)]

        guard let start = openingCandidates.compactMap({ prefix.range(of: $0)?.lowerBound }).min() else {
            return nil
        }

        var matchingEnd: Data.Index?
        for closing in closingCandidates {
            if let range = prefix.range(of: closing, options: [], in: start..<prefix.endIndex) {
                let end = range.upperBound
                matchingEnd = matchingEnd.map { min($0, end) } ?? end
            }
        }

        guard let end = matchingEnd else {
            if fileSize > scanLength { throw MotionPhotoParsingError.xmpTooLarge }
            throw MotionPhotoParsingError.malformedXMP
        }
        return prefix.subdata(in: start..<end)
    }

    private static func parseXMP(_ data: Data) throws -> ParsedXMP {
        let delegate = XMPDelegate(maxItems: maxDirectoryItems, maxStringLength: maxMetadataStringLength)
        let parser = XMLParser(data: data)
        parser.shouldProcessNamespaces = false
        parser.shouldReportNamespacePrefixes = true
        parser.shouldResolveExternalEntities = false
        parser.externalEntityResolvingPolicy = .never
        parser.delegate = delegate
        guard parser.parse(), delegate.error == nil else {
            throw delegate.error ?? MotionPhotoParsingError.malformedXMP
        }
        return delegate.result
    }

    private static func validateDirectory(_ items: [MotionPhotoItem]) throws {
        guard items.count >= 2, items.count <= maxDirectoryItems else {
            throw MotionPhotoParsingError.invalidDirectory
        }
        guard items.allSatisfy({ $0.length >= 0 && $0.padding >= 0 }) else {
            throw MotionPhotoParsingError.invalidItemLength
        }

        let primaryIndices = items.indices.filter { items[$0].semantic.caseInsensitiveCompare("Primary") == .orderedSame }
        guard primaryIndices == [0] else { throw MotionPhotoParsingError.invalidPrimaryItem }

        let motionIndices = items.indices.filter { items[$0].semantic.caseInsensitiveCompare("MotionPhoto") == .orderedSame }
        guard motionIndices.count == 1, motionIndices[0] == items.count - 1 else {
            throw MotionPhotoParsingError.invalidMotionPhotoItem
        }

        let motion = items[motionIndices[0]]
        let supportedMime = motion.mime.caseInsensitiveCompare("video/mp4") == .orderedSame
            || motion.mime.caseInsensitiveCompare("video/quicktime") == .orderedSame
        guard supportedMime, motion.length > 0 else {
            throw MotionPhotoParsingError.invalidMotionPhotoItem
        }
    }

    private static func deriveRanges(
        items: [MotionPhotoItem],
        fileSize: Int64
    ) throws -> (still: MotionPhotoByteRange, video: MotionPhotoByteRange) {
        var itemStart = fileSize
        var itemEnd = fileSize
        var photoStart: Int64?
        var photoEnd: Int64?
        var videoStart: Int64?
        var videoEnd: Int64?

        for index in items.indices.reversed() {
            let item = items[index]
            itemEnd = itemStart

            if index == 0 {
                let (paddedEnd, overflow) = itemEnd.subtractingReportingOverflow(item.padding)
                guard !overflow, paddedEnd >= 0 else { throw MotionPhotoParsingError.arithmeticOverflow }
                itemStart = 0
                itemEnd = paddedEnd
            } else {
                let (candidate, overflow) = itemStart.subtractingReportingOverflow(item.length)
                guard !overflow, candidate >= 0 else { throw MotionPhotoParsingError.arithmeticOverflow }
                itemStart = candidate
            }

            let isVideo = item.mime.caseInsensitiveCompare("video/mp4") == .orderedSame
                || item.mime.caseInsensitiveCompare("video/quicktime") == .orderedSame
            if isVideo, itemStart != itemEnd {
                videoStart = itemStart
                videoEnd = itemEnd
            }
            if index == 0 {
                photoStart = itemStart
                photoEnd = itemEnd
            }
        }

        guard let photoStart, let photoEnd, let videoStart, let videoEnd else {
            throw MotionPhotoParsingError.invalidByteRange
        }
        guard photoEnd <= fileSize, videoEnd == fileSize, photoEnd <= videoStart else {
            throw MotionPhotoParsingError.invalidByteRange
        }

        return (
            try MotionPhotoByteRange(lowerBound: photoStart, upperBound: photoEnd),
            try MotionPhotoByteRange(lowerBound: videoStart, upperBound: videoEnd)
        )
    }

    private static func validateISOBaseMediaPayload(url: URL, range: MotionPhotoByteRange) throws {
        guard range.length >= 12 else { throw MotionPhotoParsingError.invalidVideoPayload }
        let handle = try FileHandle(forReadingFrom: url)
        defer { try? handle.close() }
        try handle.seek(toOffset: UInt64(range.lowerBound))
        let header = try handle.read(upToCount: 16) ?? Data()
        guard header.count >= 8 else { throw MotionPhotoParsingError.invalidVideoPayload }

        let size = header.prefix(4).reduce(UInt32(0)) { ($0 << 8) | UInt32($1) }
        let type = String(bytes: header[4..<8], encoding: .ascii)
        guard type == "ftyp" else { throw MotionPhotoParsingError.invalidVideoPayload }
        guard size == 1 || (size >= 8 && Int64(size) <= range.length) else {
            throw MotionPhotoParsingError.invalidVideoPayload
        }
        if size == 1 {
            guard header.count >= 16 else { throw MotionPhotoParsingError.invalidVideoPayload }
            let largeSize = header[8..<16].reduce(UInt64(0)) { ($0 << 8) | UInt64($1) }
            guard largeSize >= 16, largeSize <= UInt64(range.length) else {
                throw MotionPhotoParsingError.invalidVideoPayload
            }
        }
    }
}

private struct ParsedXMP {
    var motionPhotoEnabled = false
    var version: Int?
    var presentationTimestampUs: Int64?
    var legacyMicroVideoOffset: Int64?
    var items: [MotionPhotoItem] = []
}

private final class XMPDelegate: NSObject, XMLParserDelegate {
    private let maxItems: Int
    private let maxStringLength: Int
    fileprivate var result = ParsedXMP()
    fileprivate var error: MotionPhotoParsingError?
    private var directoryPrefix: String?

    init(maxItems: Int, maxStringLength: Int) {
        self.maxItems = maxItems
        self.maxStringLength = maxStringLength
    }

    func parser(
        _ parser: XMLParser,
        didStartElement elementName: String,
        namespaceURI: String?,
        qualifiedName qName: String?,
        attributes attributeDict: [String: String]
    ) {
        guard error == nil else { parser.abortParsing(); return }
        let name = qName ?? elementName

        if name == "rdf:Description" || elementName == "rdf:Description" {
            do {
                try parseDescription(attributeDict)
            } catch let parsingError as MotionPhotoParsingError {
                error = parsingError
                parser.abortParsing()
            } catch {
                self.error = .malformedXMP
                parser.abortParsing()
            }
            return
        }

        if name == "Container:Directory" || elementName == "Container:Directory" {
            directoryPrefix = "Container"
            return
        }
        if name == "GContainer:Directory" || elementName == "GContainer:Directory" {
            directoryPrefix = "GContainer"
            return
        }

        guard let directoryPrefix else { return }
        let itemElement = "\(directoryPrefix):Item"
        guard name == itemElement || elementName == itemElement else { return }

        guard result.items.count < maxItems else {
            error = .invalidDirectory
            parser.abortParsing()
            return
        }

        let attributePrefix = directoryPrefix == "Container" ? "Item" : "GContainerItem"
        guard let mime = bounded(attributeDict["\(attributePrefix):Mime"]),
              let semantic = bounded(attributeDict["\(attributePrefix):Semantic"]) else {
            error = .invalidDirectory
            parser.abortParsing()
            return
        }
        let length = parseNonnegativeInt64(attributeDict["\(attributePrefix):Length"], defaultValue: 0)
        let padding = parseNonnegativeInt64(attributeDict["\(attributePrefix):Padding"], defaultValue: 0)
        guard let length, let padding else {
            error = .invalidItemLength
            parser.abortParsing()
            return
        }
        result.items.append(MotionPhotoItem(mime: mime, semantic: semantic, length: length, padding: padding))
    }

    func parser(
        _ parser: XMLParser,
        didEndElement elementName: String,
        namespaceURI: String?,
        qualifiedName qName: String?
    ) {
        let name = qName ?? elementName
        if name == "Container:Directory" || name == "GContainer:Directory"
            || elementName == "Container:Directory" || elementName == "GContainer:Directory" {
            directoryPrefix = nil
        }
    }

    func parser(_ parser: XMLParser, parseErrorOccurred parseError: Error) {
        if error == nil { error = .malformedXMP }
    }

    func parser(
        _ parser: XMLParser,
        resolveExternalEntityName name: String,
        systemID: String?
    ) -> Data? {
        error = .malformedXMP
        parser.abortParsing()
        return nil
    }

    private func parseDescription(_ attributes: [String: String]) throws {
        let motionNames = ["Camera:MotionPhoto", "GCamera:MotionPhoto"]
        let microVideoNames = ["Camera:MicroVideo", "GCamera:MicroVideo"]
        let versionNames = ["Camera:MotionPhotoVersion", "GCamera:MotionPhotoVersion"]
        let timestampNames = [
            "Camera:MotionPhotoPresentationTimestampUs",
            "GCamera:MotionPhotoPresentationTimestampUs",
            "Camera:MicroVideoPresentationTimestampUs",
            "GCamera:MicroVideoPresentationTimestampUs",
        ]
        let offsetNames = ["Camera:MicroVideoOffset", "GCamera:MicroVideoOffset"]

        if let flag = firstInt(attributes, names: motionNames) {
            result.motionPhotoEnabled = flag == 1
        } else if let flag = firstInt(attributes, names: microVideoNames) {
            result.motionPhotoEnabled = flag == 1
        }

        if let version = firstInt(attributes, names: versionNames) {
            result.version = version
        }
        if let timestamp = firstInt64(attributes, names: timestampNames) {
            result.presentationTimestampUs = timestamp == -1 ? nil : timestamp
        }
        if let offset = firstInt64(attributes, names: offsetNames) {
            result.legacyMicroVideoOffset = offset
        }
    }

    private func bounded(_ value: String?) -> String? {
        guard let value, !value.isEmpty, value.utf8.count <= maxStringLength else { return nil }
        return value
    }

    private func parseNonnegativeInt64(_ value: String?, defaultValue: Int64) -> Int64? {
        guard let value else { return defaultValue }
        guard value.utf8.count <= 32, let parsed = Int64(value), parsed >= 0 else { return nil }
        return parsed
    }

    private func firstInt(_ attributes: [String: String], names: [String]) -> Int? {
        for name in names {
            if let raw = attributes[name] {
                guard raw.utf8.count <= 32 else { return nil }
                return Int(raw)
            }
        }
        return nil
    }

    private func firstInt64(_ attributes: [String: String], names: [String]) -> Int64? {
        for name in names {
            if let raw = attributes[name] {
                guard raw.utf8.count <= 32 else { return nil }
                return Int64(raw)
            }
        }
        return nil
    }
}
