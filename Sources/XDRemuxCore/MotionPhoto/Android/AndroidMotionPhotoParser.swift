import Foundation

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
            items = description.items
            sourceKind = ISOBMFFMotionPhotoRangeResolver.isHEIFMime(items[0].mime)
                ? .androidHeifMotionPhotoV1
                : .androidMotionPhotoV1
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
        let ranges: (still: MotionPhotoByteRange, video: MotionPhotoByteRange)
        if sourceKind == .androidHeifMotionPhotoV1 {
            ranges = try ISOBMFFMotionPhotoRangeResolver.resolve(
                url: url,
                items: items,
                fileSize: fileSize
            )
        } else {
            ranges = try deriveJPEGStyleRanges(items: items, fileSize: fileSize)
            guard try ISOBaseMediaStreamScanner.isFTYPBoxStart(
                in: url,
                offset: ranges.video.lowerBound,
                upperBound: ranges.video.upperBound
            ) else {
                throw MotionPhotoParsingError.invalidVideoPayload
            }
        }

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
        let openings = [Data("<x:xmpmeta".utf8), Data("<xmpmeta".utf8)]
        let closings = [Data("</x:xmpmeta>".utf8), Data("</xmpmeta>".utf8)]
        guard let start = openings.compactMap({ prefix.range(of: $0)?.lowerBound }).min() else {
            return nil
        }
        var endIndex: Data.Index?
        for closing in closings {
            if let range = prefix.range(of: closing, in: start..<prefix.endIndex) {
                endIndex = endIndex.map { min($0, range.upperBound) } ?? range.upperBound
            }
        }
        guard let endIndex else {
            if fileSize > scanLength { throw MotionPhotoParsingError.xmpTooLarge }
            throw MotionPhotoParsingError.malformedXMP
        }
        return prefix.subdata(in: start..<endIndex)
    }

    private static func parseXMP(_ data: Data) throws -> ParsedXMP {
        // Motion Photo metadata does not require DTDs or custom entities. Reject declarations
        // before invoking XMLParser, in addition to disabling external entity resolution below.
        if data.range(of: Data("<!DOCTYPE".utf8)) != nil
            || data.range(of: Data("<!ENTITY".utf8)) != nil {
            throw MotionPhotoParsingError.malformedXMP
        }

        let delegate = XMPDelegate(
            maxItems: maxDirectoryItems,
            maxStringLength: maxMetadataStringLength
        )
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
        guard items[0].semantic.caseInsensitiveCompare("Primary") == .orderedSame,
              items.dropFirst().allSatisfy({ $0.semantic.caseInsensitiveCompare("Primary") != .orderedSame }),
              items[0].length == 0 else {
            throw MotionPhotoParsingError.invalidPrimaryItem
        }
        guard items.dropFirst().allSatisfy({ $0.padding == 0 }) else {
            throw MotionPhotoParsingError.invalidItemLength
        }
        let motionIndices = items.indices.filter {
            items[$0].semantic.caseInsensitiveCompare("MotionPhoto") == .orderedSame
        }
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

    private static func deriveJPEGStyleRanges(
        items: [MotionPhotoItem],
        fileSize: Int64
    ) throws -> (still: MotionPhotoByteRange, video: MotionPhotoByteRange) {
        var itemStart = fileSize
        var itemEnd = fileSize
        var primaryEncodingEnd: Int64?
        var videoStart: Int64?
        var videoEnd: Int64?

        for index in items.indices.reversed() {
            let item = items[index]
            itemEnd = itemStart
            if index == 0 {
                let (unpaddedEnd, overflow) = itemEnd.subtractingReportingOverflow(item.padding)
                guard !overflow, unpaddedEnd >= 0 else {
                    throw MotionPhotoParsingError.arithmeticOverflow
                }
                itemStart = 0
                itemEnd = unpaddedEnd
                primaryEncodingEnd = itemEnd
            } else {
                let (candidate, overflow) = itemStart.subtractingReportingOverflow(item.length)
                guard !overflow, candidate >= 0 else {
                    throw MotionPhotoParsingError.arithmeticOverflow
                }
                itemStart = candidate
            }
            let isVideo = item.mime.caseInsensitiveCompare("video/mp4") == .orderedSame
                || item.mime.caseInsensitiveCompare("video/quicktime") == .orderedSame
            if isVideo, itemStart != itemEnd {
                videoStart = itemStart
                videoEnd = itemEnd
            }
        }

        guard let primaryEncodingEnd, let videoStart, let videoEnd,
              primaryEncodingEnd <= videoStart,
              videoStart >= 0,
              videoEnd == fileSize else {
            throw MotionPhotoParsingError.invalidByteRange
        }

        // For JPEG conversion we need the complete still-image resource, not only the Primary item.
        // Ultra HDR Motion Photos can carry a positive-length GainMap secondary JPEG between the
        // primary encoding and the trailing MotionPhoto video. Supplying 0..<videoStart to ImageIO
        // preserves that resource while excluding only the appended MP4.
        return (
            try MotionPhotoByteRange(lowerBound: 0, upperBound: videoStart),
            try MotionPhotoByteRange(lowerBound: videoStart, upperBound: videoEnd)
        )
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
        if name == "rdf:Description" {
            parseDescription(attributeDict)
            return
        }
        if name == "Container:Directory" {
            directoryPrefix = "Container"
            return
        }
        if name == "GContainer:Directory" {
            directoryPrefix = "GContainer"
            return
        }
        guard let directoryPrefix,
              name == "\(directoryPrefix):Item" else { return }
        guard result.items.count < maxItems else {
            error = .invalidDirectory
            parser.abortParsing()
            return
        }
        let prefix = directoryPrefix == "Container" ? "Item" : "GContainerItem"
        guard let mime = bounded(attributeDict["\(prefix):Mime"]),
              let semantic = bounded(attributeDict["\(prefix):Semantic"]),
              let length = nonnegative(attributeDict["\(prefix):Length"], defaultValue: 0),
              let padding = nonnegative(attributeDict["\(prefix):Padding"], defaultValue: 0) else {
            error = .invalidDirectory
            parser.abortParsing()
            return
        }
        result.items.append(
            MotionPhotoItem(mime: mime, semantic: semantic, length: length, padding: padding)
        )
    }

    func parser(
        _ parser: XMLParser,
        didEndElement elementName: String,
        namespaceURI: String?,
        qualifiedName qName: String?
    ) {
        let name = qName ?? elementName
        if name == "Container:Directory" || name == "GContainer:Directory" {
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

    private func parseDescription(_ attributes: [String: String]) {
        let motionNames = ["Camera:MotionPhoto", "GCamera:MotionPhoto"]
        let legacyNames = ["Camera:MicroVideo", "GCamera:MicroVideo"]
        if let flag = firstInt(attributes, names: motionNames) {
            result.motionPhotoEnabled = flag == 1
        } else if let flag = firstInt(attributes, names: legacyNames) {
            result.motionPhotoEnabled = flag == 1
        }
        result.version = firstInt(
            attributes,
            names: ["Camera:MotionPhotoVersion", "GCamera:MotionPhotoVersion"]
        )
        if let value = firstInt64(
            attributes,
            names: [
                "Camera:MotionPhotoPresentationTimestampUs",
                "GCamera:MotionPhotoPresentationTimestampUs",
                "Camera:MicroVideoPresentationTimestampUs",
                "GCamera:MicroVideoPresentationTimestampUs",
            ]
        ) {
            result.presentationTimestampUs = value == -1 ? nil : value
        }
        result.legacyMicroVideoOffset = firstInt64(
            attributes,
            names: ["Camera:MicroVideoOffset", "GCamera:MicroVideoOffset"]
        )
    }

    private func bounded(_ value: String?) -> String? {
        guard let value, !value.isEmpty, value.utf8.count <= maxStringLength else { return nil }
        return value
    }

    private func nonnegative(_ value: String?, defaultValue: Int64) -> Int64? {
        guard let value else { return defaultValue }
        guard value.utf8.count <= 32, let parsed = Int64(value), parsed >= 0 else { return nil }
        return parsed
    }

    private func firstInt(_ attributes: [String: String], names: [String]) -> Int? {
        for name in names where attributes[name] != nil {
            guard let raw = attributes[name], raw.utf8.count <= 32 else { return nil }
            return Int(raw)
        }
        return nil
    }

    private func firstInt64(_ attributes: [String: String], names: [String]) -> Int64? {
        for name in names where attributes[name] != nil {
            guard let raw = attributes[name], raw.utf8.count <= 32 else { return nil }
            return Int64(raw)
        }
        return nil
    }
}
