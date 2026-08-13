import CryptoKit
import Foundation
import ImageIO

public let photoClassificationLayoutVersion = "asset-type-v1"
public let photoUnclassifiedFolderName = "未分类"

public enum PhotoAssetType: String, CaseIterable, Codable, Sendable, Identifiable, Hashable {
    case staticPhoto = "static-photo"
    case livePhoto = "live-photo"

    public var id: String { rawValue }

    public var folderName: String {
        switch self {
        case .staticPhoto: return "静态照片"
        case .livePhoto: return "实况照片"
        }
    }

    public var tagID: String { "asset.\(rawValue)" }
}

public enum PhotoResourceRole: String, Codable, Sendable, Hashable {
    case primaryImage = "primary-image"
    case pairedVideo = "paired-video"
    case sidecar
}

public struct PhotoResource: Codable, Sendable, Hashable {
    public let url: URL
    public let role: PhotoResourceRole

    public init(url: URL, role: PhotoResourceRole) {
        self.url = url
        self.role = role
    }
}

public struct PhotoAsset: Codable, Sendable, Identifiable, Hashable {
    public let id: String
    public let type: PhotoAssetType
    public let resources: [PhotoResource]

    public init(id: String, type: PhotoAssetType, resources: [PhotoResource]) {
        self.id = id
        self.type = type
        self.resources = resources
    }

    public static func staticPhoto(_ url: URL) -> PhotoAsset {
        PhotoAsset(
            id: url.standardizedFileURL.path,
            type: .staticPhoto,
            resources: [PhotoResource(url: url, role: .primaryImage)]
        )
    }

    public static func livePhoto(
        imageURL: URL,
        videoURL: URL,
        id: String? = nil
    ) -> PhotoAsset {
        PhotoAsset(
            id: id ?? imageURL.standardizedFileURL.path,
            type: .livePhoto,
            resources: [
                PhotoResource(url: imageURL, role: .primaryImage),
                PhotoResource(url: videoURL, role: .pairedVideo)
            ]
        )
    }

    public var primaryImageURL: URL? {
        resources.first(where: { $0.role == .primaryImage })?.url
    }
}

public enum OppoCaptureMode: String, CaseIterable, Codable, Sendable, Identifiable, Hashable {
    case normal
    case master
    case ricohGR = "ricoh-gr"
    case professional
    case portrait
    case night
    case panorama
    case timeLapse = "time-lapse"
    case ultraHighResolution = "ultra-high-resolution"
    case idPhoto = "id-photo"
    case sticker
    case enhancedText = "enhanced-text"
    case groupPhoto = "group-photo"
    case doubleExposure = "double-exposure"
    case beauty

    public var id: String { rawValue }
    public var tagID: String { "capture.\(rawValue)" }

    public var folderName: String {
        switch self {
        case .normal: return "普通拍照"
        case .master: return "大师模式"
        case .ricohGR: return "RICOH GR"
        case .professional: return "专业模式"
        case .portrait: return "人像"
        case .night: return "夜景"
        case .panorama: return "全景"
        case .timeLapse: return "延时摄影"
        case .ultraHighResolution: return "超清"
        case .idPhoto: return "证件照"
        case .sticker: return "贴纸"
        case .enhancedText: return "超级文本"
        case .groupPhoto: return "合影"
        case .doubleExposure: return "双重曝光"
        case .beauty: return "美颜"
        }
    }

    fileprivate var bit: UInt64 {
        switch self {
        case .normal: return 0
        case .master: return 0x1_0000_0000
        case .ricohGR: return 0x8000_0000
        case .professional: return 0x100
        case .portrait: return 0x10
        case .night: return 0x800
        case .panorama: return 0x4
        case .timeLapse: return 0x8
        case .ultraHighResolution: return 0x2000
        case .idPhoto: return 0x4000
        case .sticker: return 0x200
        case .enhancedText: return 0x1000
        case .groupPhoto: return 0x40_0000
        case .doubleExposure: return 0x8000
        case .beauty: return 0x2
        }
    }

    /// Historical ordering is retained only for deterministic folder projection.
    public static let folderProjectionPriority = allCases.filter { $0 != .normal }
}

public enum PhotoCapability: String, CaseIterable, Codable, Sendable, Hashable {
    case proXDR = "proxdr"
    case gainMap = "gain-map"
    case hdr
    case depth

    public var tagID: String { "capability.\(rawValue)" }
}

public enum CameraVendor: String, Codable, Sendable, Hashable {
    case oppo

    public var tagID: String { "vendor.\(rawValue)" }
}

public enum OppoPhotoClassificationStatus: String, Codable, Sendable, Equatable {
    case categorized
    case missingUserComment = "missing-user-comment"
    case malformedUserComment = "malformed-user-comment"
    case unknownFlags = "unknown-flags"
    case unreadableImage = "unreadable-image"
}

public enum PhotoMetadataReadStatus: String, Codable, Sendable, Equatable {
    case ok
    case missingUserComment = "missing-user-comment"
    case malformedUserComment = "malformed-user-comment"
    case unreadableImage = "unreadable-image"
}

public struct OppoFlagEvidence: Codable, Sendable, Equatable {
    public let rawFlags: UInt64
    public let recognizedFlags: UInt64
    public let knownUnmappedFlags: UInt64
    public let unknownFlags: UInt64

    public init(
        rawFlags: UInt64,
        recognizedFlags: UInt64,
        knownUnmappedFlags: UInt64,
        unknownFlags: UInt64
    ) {
        self.rawFlags = rawFlags
        self.recognizedFlags = recognizedFlags
        self.knownUnmappedFlags = knownUnmappedFlags
        self.unknownFlags = unknownFlags
    }
}

public struct OppoPhotoClassification: Codable, Sendable, Equatable {
    public let rawUserComment: String?
    public let flagEvidence: OppoFlagEvidence?
    public let captureModes: Set<OppoCaptureMode>
    public let metadataStatus: PhotoMetadataReadStatus

    public init(
        rawUserComment: String?,
        flagEvidence: OppoFlagEvidence?,
        captureModes: Set<OppoCaptureMode>,
        metadataStatus: PhotoMetadataReadStatus
    ) {
        self.rawUserComment = rawUserComment
        self.flagEvidence = flagEvidence
        self.captureModes = captureModes
        self.metadataStatus = metadataStatus
    }

    public var tagFlags: UInt64? { flagEvidence?.rawFlags }
    public var recognizedFlags: UInt64 { flagEvidence?.recognizedFlags ?? 0 }
    public var knownUnmappedFlags: UInt64 { flagEvidence?.knownUnmappedFlags ?? 0 }
    public var unknownFlags: UInt64 { flagEvidence?.unknownFlags ?? 0 }

    public var status: OppoPhotoClassificationStatus {
        switch metadataStatus {
        case .missingUserComment: return .missingUserComment
        case .malformedUserComment: return .malformedUserComment
        case .unreadableImage: return .unreadableImage
        case .ok:
            if !captureModes.isEmpty || unknownFlags == 0 { return .categorized }
            return .unknownFlags
        }
    }
}

public struct PhotoClassification: Codable, Sendable, Equatable {
    public let assetType: PhotoAssetType
    public let captureModes: Set<OppoCaptureMode>
    public let capabilities: Set<PhotoCapability>
    public let vendor: CameraVendor?
    public let evidence: OppoPhotoClassification

    public init(
        assetType: PhotoAssetType,
        captureModes: Set<OppoCaptureMode>,
        capabilities: Set<PhotoCapability>,
        vendor: CameraVendor?,
        evidence: OppoPhotoClassification
    ) {
        self.assetType = assetType
        self.captureModes = captureModes
        self.capabilities = capabilities
        self.vendor = vendor
        self.evidence = evidence
    }

    public var rawUserComment: String? { evidence.rawUserComment }
    public var tagFlags: UInt64? { evidence.tagFlags }
    public var recognizedFlags: UInt64 { evidence.recognizedFlags }
    public var knownUnmappedFlags: UInt64 { evidence.knownUnmappedFlags }
    public var unknownFlags: UInt64 { evidence.unknownFlags }
    public var metadataStatus: PhotoMetadataReadStatus { evidence.metadataStatus }
    public var status: OppoPhotoClassificationStatus { evidence.status }

    /// Compatibility view for old callers. The semantic source of truth is `captureModes`; this is
    /// only the deterministic physical-folder projection.
    public var mode: OppoCaptureMode? { PhotoFolderProjection.primaryCaptureMode(for: self) }

    public var tags: [String] {
        var values = Set<String>()
        values.insert(assetType.tagID)
        values.formUnion(captureModes.map(\.tagID))
        values.formUnion(capabilities.map(\.tagID))
        if let vendor { values.insert(vendor.tagID) }
        return values.sorted()
    }
}

public enum PhotoFolderProjection {
    public static let layoutVersion = photoClassificationLayoutVersion
    public static let rootFolderNames = Set(PhotoAssetType.allCases.map(\.folderName))
    public static let unclassifiedFolderName = photoUnclassifiedFolderName

    public static func primaryCaptureMode(for classification: PhotoClassification) -> OppoCaptureMode? {
        for candidate in OppoCaptureMode.folderProjectionPriority
        where classification.captureModes.contains(candidate) {
            return candidate
        }
        // `normal` is a folder/presentation fallback, not an activated semantic tag. Preserve the
        // historical normal handling for zero and known-but-unmapped OPPO bits, but do not hide
        // completely unknown flags under the normal folder.
        if classification.metadataStatus == .ok,
           classification.tagFlags != nil,
           classification.unknownFlags == 0 {
            return .normal
        }
        return nil
    }

    public static func relativeDirectoryComponents(
        for classification: PhotoClassification,
        assetType: PhotoAssetType? = nil
    ) -> [String] {
        let resolvedType = assetType ?? classification.assetType
        let leaf = primaryCaptureMode(for: classification)?.folderName ?? photoUnclassifiedFolderName
        return [resolvedType.folderName, leaf]
    }

    public static func relativeDirectory(
        for classification: PhotoClassification,
        assetType: PhotoAssetType? = nil
    ) -> String {
        relativeDirectoryComponents(for: classification, assetType: assetType).joined(separator: "/")
    }
}

public struct PhotoClassificationContract: Codable, Sendable, Equatable {
    public let assetType: String
    public let captureModes: [String]
    public let primaryCaptureMode: String?
    public let folder: String
    public let metadataStatus: String
    public let recognizedFlags: UInt64
    public let knownUnmappedFlags: UInt64
    public let unknownFlags: UInt64
    public let tags: [String]

    enum CodingKeys: String, CodingKey {
        case assetType = "asset_type"
        case captureModes = "capture_modes"
        case primaryCaptureMode = "primary_capture_mode"
        case folder
        case metadataStatus = "metadata_status"
        case recognizedFlags = "recognized_flags"
        case knownUnmappedFlags = "known_unmapped_flags"
        case unknownFlags = "unknown_flags"
        case tags
    }
}

public enum PhotoCategorizationDisposition: String, Codable, Sendable {
    case copy
    case duplicate
    case copied
    case failed
    case dryRun = "dry-run"
}

public struct PhotoCategorizationItem: Sendable, Identifiable {
    public let sourceURL: URL
    public let destinationURL: URL
    public let classification: PhotoClassification
    public let disposition: PhotoCategorizationDisposition
    public let errorDescription: String?

    public var id: String { sourceURL.standardizedFileURL.path }

    public init(
        sourceURL: URL,
        destinationURL: URL,
        classification: PhotoClassification,
        disposition: PhotoCategorizationDisposition,
        errorDescription: String? = nil
    ) {
        self.sourceURL = sourceURL
        self.destinationURL = destinationURL
        self.classification = classification
        self.disposition = disposition
        self.errorDescription = errorDescription
    }
}

public struct PhotoCategorizationPlan: Sendable {
    public let outputDirectory: URL?
    public let items: [PhotoCategorizationItem]

    public init(outputDirectory: URL?, items: [PhotoCategorizationItem]) {
        self.outputDirectory = outputDirectory
        self.items = items
    }
}

public struct PhotoCategorizationResult: Sendable {
    public let items: [PhotoCategorizationItem]

    public var copiedCount: Int { items.count { $0.disposition == .copied } }
    public var dryRunCount: Int { items.count { $0.disposition == .dryRun } }
    public var duplicateCount: Int { items.count { $0.disposition == .duplicate } }
    public var failedCount: Int { items.count { $0.disposition == .failed } }
    public var categorizedCount: Int { items.count { $0.classification.mode != nil } }
    public var unclassifiedCount: Int { items.count { $0.classification.mode == nil } }
    /// Compatibility alias. V1 no longer writes uncategorized photos to the output root.
    public var rootCount: Int { unclassifiedCount }
    public var issueCount: Int {
        items.count {
            $0.disposition == .failed
                || $0.classification.status == .malformedUserComment
                || $0.classification.status == .unreadableImage
        }
    }

    public init(items: [PhotoCategorizationItem]) {
        self.items = items
    }
}

public enum PhotoCategorizationEngine {
    private static let supportedExtensions = Set(["heic", "heif", "jpg", "jpeg"])
    private static let legacyCaptureModeFolderNames = Set(OppoCaptureMode.allCases.map(\.folderName))
    private static let projectionRootFolderNames = Set(PhotoAssetType.allCases.map(\.folderName))
    private static let knownFlagsMask: UInt64 = [
        0x1, 0x2, 0x4, 0x8, 0x10, 0x20, 0x40, 0x80,
        0x100, 0x200, 0x400, 0x800, 0x1000, 0x2000, 0x4000, 0x8000,
        0x1_0000, 0x2_0000, 0x4_0000, 0x8_0000, 0x10_0000, 0x20_0000,
        0x40_0000, 0x80_0000, 0x100_0000, 0x200_0000, 0x400_0000,
        0x800_0000, 0x1000_0000, 0x2000_0000, 0x4000_0000, 0x8000_0000,
        0x1_0000_0000, 0x2_0000_0000, 0x4000_0000_0000_0000
    ].reduce(0, |)
    private static let mappedFlagsMask: UInt64 = OppoCaptureMode.folderProjectionPriority
        .map(\.bit)
        .reduce(0, |)

    public static func classify(
        userComment: String?,
        assetType: PhotoAssetType = .staticPhoto,
        capabilities: Set<PhotoCapability> = []
    ) -> PhotoClassification {
        let evidence: OppoPhotoClassification
        let vendor: CameraVendor?
        guard let raw = userComment?.trimmingCharacters(in: .whitespacesAndNewlines), !raw.isEmpty else {
            evidence = OppoPhotoClassification(
                rawUserComment: userComment,
                flagEvidence: nil,
                captureModes: [],
                metadataStatus: .missingUserComment
            )
            return PhotoClassification(
                assetType: assetType,
                captureModes: [],
                capabilities: capabilities,
                vendor: nil,
                evidence: evidence
            )
        }
        guard let flags = parseFlags(from: raw) else {
            evidence = OppoPhotoClassification(
                rawUserComment: raw,
                flagEvidence: nil,
                captureModes: [],
                metadataStatus: .malformedUserComment
            )
            return PhotoClassification(
                assetType: assetType,
                captureModes: [],
                capabilities: capabilities,
                vendor: nil,
                evidence: evidence
            )
        }

        let recognized = flags & mappedFlagsMask
        let knownUnmapped = flags & knownFlagsMask & ~mappedFlagsMask
        let unknown = flags & ~knownFlagsMask
        let modes = Set(OppoCaptureMode.folderProjectionPriority.filter { flags & $0.bit != 0 })
        evidence = OppoPhotoClassification(
            rawUserComment: raw,
            flagEvidence: OppoFlagEvidence(
                rawFlags: flags,
                recognizedFlags: recognized,
                knownUnmappedFlags: knownUnmapped,
                unknownFlags: unknown
            ),
            captureModes: modes,
            metadataStatus: .ok
        )
        vendor = .oppo
        return PhotoClassification(
            assetType: assetType,
            captureModes: modes,
            capabilities: capabilities,
            vendor: vendor,
            evidence: evidence
        )
    }

    public static func classify(asset: PhotoAsset) -> PhotoClassification {
        guard let primaryImageURL = asset.primaryImageURL else {
            let evidence = OppoPhotoClassification(
                rawUserComment: nil,
                flagEvidence: nil,
                captureModes: [],
                metadataStatus: .unreadableImage
            )
            return PhotoClassification(
                assetType: asset.type,
                captureModes: [],
                capabilities: [],
                vendor: nil,
                evidence: evidence
            )
        }
        return classify(at: primaryImageURL, assetType: asset.type)
    }

    public static func classify(
        at url: URL,
        assetType: PhotoAssetType? = nil
    ) -> PhotoClassification {
        let resolvedAssetType = assetType ?? inferredAssetType(at: url)
        guard let data = try? Data(contentsOf: url) else {
            let evidence = OppoPhotoClassification(
                rawUserComment: nil,
                flagEvidence: nil,
                captureModes: [],
                metadataStatus: .unreadableImage
            )
            return PhotoClassification(
                assetType: resolvedAssetType,
                captureModes: [],
                capabilities: [],
                vendor: nil,
                evidence: evidence
            )
        }
        return classify(
            userComment: extractedUserComment(from: data),
            assetType: resolvedAssetType,
            capabilities: detectedCapabilities(in: data)
        )
    }

    public static func makePlan(
        inputs: [URL],
        outputDirectory: URL?,
        fileManager: FileManager = .default
    ) throws -> PhotoCategorizationPlan {
        let sources = try collectInputFiles(inputs, excluding: outputDirectory, fileManager: fileManager)
        var reserved: [String: (url: URL, digest: String)] = [:]
        var items: [PhotoCategorizationItem] = []

        for source in sources {
            let classification = classify(at: source)
            let destinationRoot = outputDirectory ?? source.deletingLastPathComponent()
            let destinationDirectory = PhotoFolderProjection.relativeDirectoryComponents(for: classification)
                .reduce(destinationRoot) { partial, component in
                    partial.appendingPathComponent(component, isDirectory: true)
                }
            let sourceDigest = try sha256Hex(source)
            var sequence = 1
            while true {
                let destination = destinationURL(
                    directory: destinationDirectory,
                    sourceName: source.lastPathComponent,
                    sequence: sequence
                )
                let key = destination.standardizedFileURL.path
                if let prior = reserved[key] {
                    if prior.digest == sourceDigest {
                        items.append(PhotoCategorizationItem(
                            sourceURL: source,
                            destinationURL: prior.url,
                            classification: classification,
                            disposition: .duplicate
                        ))
                        break
                    }
                    sequence += 1
                    continue
                }
                if fileManager.fileExists(atPath: destination.path) {
                    if try filesMatch(source, destination) {
                        reserved[key] = (destination, sourceDigest)
                        items.append(PhotoCategorizationItem(
                            sourceURL: source,
                            destinationURL: destination,
                            classification: classification,
                            disposition: .duplicate
                        ))
                        break
                    }
                    sequence += 1
                    continue
                }
                reserved[key] = (destination, sourceDigest)
                items.append(PhotoCategorizationItem(
                    sourceURL: source,
                    destinationURL: destination,
                    classification: classification,
                    disposition: .copy
                ))
                break
            }
        }
        return PhotoCategorizationPlan(outputDirectory: outputDirectory, items: items)
    }

    public static func execute(
        _ plan: PhotoCategorizationPlan,
        jobs: Int = min(ProcessInfo.processInfo.activeProcessorCount, 4),
        dryRun: Bool = false,
        fileManager: FileManager = .default
    ) -> PhotoCategorizationResult {
        let lock = NSLock()
        var completed: [String: PhotoCategorizationItem] = [:]
        let queue = OperationQueue()
        queue.maxConcurrentOperationCount = max(1, jobs)
        queue.qualityOfService = .userInitiated

        for item in plan.items {
            queue.addOperation {
                let result: PhotoCategorizationItem
                if item.disposition == .duplicate {
                    result = item
                } else if dryRun {
                    result = PhotoCategorizationItem(
                        sourceURL: item.sourceURL,
                        destinationURL: item.destinationURL,
                        classification: item.classification,
                        disposition: .dryRun
                    )
                } else {
                    do {
                        try copyAtomically(item.sourceURL, to: item.destinationURL, fileManager: fileManager)
                        result = PhotoCategorizationItem(
                            sourceURL: item.sourceURL,
                            destinationURL: item.destinationURL,
                            classification: item.classification,
                            disposition: .copied
                        )
                    } catch {
                        result = PhotoCategorizationItem(
                            sourceURL: item.sourceURL,
                            destinationURL: item.destinationURL,
                            classification: item.classification,
                            disposition: .failed,
                            errorDescription: String(describing: error)
                        )
                    }
                }
                lock.lock()
                completed[item.sourceURL.standardizedFileURL.path] = result
                lock.unlock()
            }
        }
        queue.waitUntilAllOperationsAreFinished()
        let ordered = plan.items.compactMap { completed[$0.sourceURL.standardizedFileURL.path] }
        return PhotoCategorizationResult(items: ordered)
    }

    public static func categorizedDirectory(
        for url: URL,
        assetType: PhotoAssetType? = nil
    ) -> String? {
        PhotoFolderProjection.relativeDirectory(for: classify(at: url, assetType: assetType))
    }

    public static func classificationContract(
        for classification: PhotoClassification
    ) -> PhotoClassificationContract {
        let mode = classification.mode
        return PhotoClassificationContract(
            assetType: classification.assetType.rawValue,
            captureModes: OppoCaptureMode.folderProjectionPriority
                .filter { classification.captureModes.contains($0) }
                .map(\.rawValue),
            primaryCaptureMode: mode?.rawValue,
            folder: mode?.folderName ?? photoUnclassifiedFolderName,
            metadataStatus: classification.metadataStatus.rawValue,
            recognizedFlags: classification.recognizedFlags,
            knownUnmappedFlags: classification.knownUnmappedFlags,
            unknownFlags: classification.unknownFlags,
            tags: classification.tags
        )
    }

    public static func inferredAssetType(at url: URL) -> PhotoAssetType {
        let ext = url.pathExtension.lowercased()
        guard ext == "jpg" || ext == "jpeg" || ext == "heic" || ext == "heif" else {
            return .staticPhoto
        }
        return ((try? OppoMotionPhotoParser.parse(url: url)) ?? nil) == nil ? .staticPhoto : .livePhoto
    }

    private static func detectedCapabilities(in data: Data) -> Set<PhotoCapability> {
        var capabilities = Set<PhotoCapability>()
        let privateGainMapNames = ["\"local.uhdr.gainmap.data\"", "\"local.uhdr.gainmap.info\""]
        if privateGainMapNames.contains(where: { data.range(of: Data($0.utf8)) != nil }) {
            capabilities.formUnion([.proXDR, .gainMap, .hdr])
        }
        if data.range(of: Data("\"rear.depth\"".utf8)) != nil,
           data.range(of: Data("\"rear.depth.config\"".utf8)) != nil {
            capabilities.insert(.depth)
        }
        return capabilities
    }

    private static func parseFlags(from raw: String) -> UInt64? {
        if let data = raw.data(using: .utf8),
           let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
           let value = object.first(where: { $0.key.caseInsensitiveCompare("oplustag") == .orderedSame })?.value {
            if let number = value as? NSNumber { return UInt64(number.stringValue) }
            if let string = value as? String {
                return UInt64(string.trimmingCharacters(in: .whitespacesAndNewlines))
            }
        }

        let normalized = raw.replacingOccurrences(of: "\0", with: "")
        let lower = normalized.lowercased()
        for prefix in ["oplus_", "oppo_"] {
            guard let range = lower.range(of: prefix) else { continue }
            let suffix = normalized[range.upperBound...]
            let digits = suffix.prefix { $0.isNumber }
            guard !digits.isEmpty else { continue }
            return UInt64(digits)
        }
        return nil
    }

    private static func extractedUserComment(from data: Data) -> String? {
        if let source = CGImageSourceCreateWithData(data as CFData, nil),
           let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any],
           let exif = properties[kCGImagePropertyExifDictionary] as? [CFString: Any],
           let value = exif[kCGImagePropertyExifUserComment] {
            if let string = value as? String { return string }
            if let bytes = value as? Data, let decoded = decodeUserComment(bytes) { return decoded }
        }

        if let decoded = extractTIFFUserComment(from: data) {
            return decoded
        }

        for prefix in ["Oplus_", "oplus_", "Oppo_", "oppo_"] {
            guard let range = data.range(of: Data(prefix.utf8)) else { continue }
            var end = range.upperBound
            while end < data.endIndex, (48...57).contains(data[end]) { end += 1 }
            guard end > range.upperBound else { continue }
            return String(data: data[range.lowerBound..<end], encoding: .ascii)
        }
        if let text = String(data: data, encoding: .isoLatin1),
           let match = text.range(
               of: #"\{\s*[\"']oplustag[\"']\s*:\s*[\"']?[0-9]+[\"']?\s*\}"#,
               options: [.regularExpression, .caseInsensitive]
           ) {
            return String(text[match])
        }
        return nil
    }

    private static func decodeUserComment(_ data: Data) -> String? {
        let payload = data.count >= 8 ? data.dropFirst(8) : data[...]
        return String(data: payload, encoding: .utf8)?
            .trimmingCharacters(in: .controlCharacters.union(.whitespacesAndNewlines))
    }

    private static func extractTIFFUserComment(from container: Data) -> String? {
        let exifMarker = Data("Exif\0\0".utf8)
        var starts = [0]
        var searchStart = container.startIndex
        while searchStart < container.endIndex,
              let range = container.range(of: exifMarker, in: searchStart..<container.endIndex) {
            starts.append(range.upperBound)
            searchStart = range.upperBound
        }

        for start in starts where start + 8 <= container.count {
            let marker = Array(container[start..<(start + 2)])
            guard marker == [0x49, 0x49] || marker == [0x4d, 0x4d] else { continue }
            let littleEndian = marker[0] == 0x49
            guard readUInt16(container, at: start + 2, littleEndian: littleEndian) == 42,
                  let firstIFD = readUInt32(container, at: start + 4, littleEndian: littleEndian) else { continue }
            var pending = [Int(firstIFD)]
            var visited = Set<Int>()
            while let relativeOffset = pending.popLast() {
                guard visited.insert(relativeOffset).inserted else { continue }
                let ifd = start + relativeOffset
                guard let count = readUInt16(container, at: ifd, littleEndian: littleEndian), count <= 4096 else { continue }
                for index in 0..<Int(count) {
                    let entry = ifd + 2 + index * 12
                    guard let tag = readUInt16(container, at: entry, littleEndian: littleEndian),
                          let type = readUInt16(container, at: entry + 2, littleEndian: littleEndian),
                          let valueCount = readUInt32(container, at: entry + 4, littleEndian: littleEndian),
                          let valueOffset = readUInt32(container, at: entry + 8, littleEndian: littleEndian) else { break }
                    if tag == 0x8769 || tag == 0x8825 {
                        pending.append(Int(valueOffset))
                    } else if tag == 0x9286, type == 7 {
                        let length = Int(valueCount)
                        let valueStart = length <= 4 ? entry + 8 : start + Int(valueOffset)
                        guard valueStart >= 0, valueStart + length <= container.count else { continue }
                        let bytes = container.subdata(in: valueStart..<(valueStart + length))
                        if let value = decodeUserComment(bytes), !value.isEmpty {
                            return value
                        }
                    }
                }
            }
        }
        return nil
    }

    private static func readUInt16(_ data: Data, at offset: Int, littleEndian: Bool) -> UInt16? {
        guard offset >= 0, offset + 2 <= data.count else { return nil }
        let first = UInt16(data[offset])
        let second = UInt16(data[offset + 1])
        return littleEndian ? first | second << 8 : first << 8 | second
    }

    private static func readUInt32(_ data: Data, at offset: Int, littleEndian: Bool) -> UInt32? {
        guard offset >= 0, offset + 4 <= data.count else { return nil }
        let values = (0..<4).map { UInt32(data[offset + $0]) }
        return littleEndian
            ? values[0] | values[1] << 8 | values[2] << 16 | values[3] << 24
            : values[0] << 24 | values[1] << 16 | values[2] << 8 | values[3]
    }

    private static func collectInputFiles(
        _ inputs: [URL],
        excluding outputDirectory: URL?,
        fileManager: FileManager
    ) throws -> [URL] {
        let excluded = outputDirectory?.standardizedFileURL.path
        var collected: [String: URL] = [:]
        let inPlaceSkipRoots = projectionRootFolderNames.union(legacyCaptureModeFolderNames)
        for input in inputs {
            var isDirectory: ObjCBool = false
            guard fileManager.fileExists(atPath: input.path, isDirectory: &isDirectory) else {
                throw XDRemuxError.inputNotFound(input)
            }
            if !isDirectory.boolValue {
                if supportedExtensions.contains(input.pathExtension.lowercased()) {
                    collected[input.standardizedFileURL.path] = input
                }
                continue
            }
            guard let enumerator = fileManager.enumerator(
                at: input,
                includingPropertiesForKeys: [.isRegularFileKey, .isDirectoryKey],
                options: [.skipsHiddenFiles]
            ) else { throw XDRemuxError.unableToRead(input) }
            let inputRoot = input.standardizedFileURL.path
            for case let url as URL in enumerator {
                let path = url.standardizedFileURL.path
                let isDirectory = (try? url.resourceValues(forKeys: [.isDirectoryKey]).isDirectory) == true
                if let excluded, path == excluded || path.hasPrefix(excluded + "/") {
                    if isDirectory { enumerator.skipDescendants() }
                    continue
                }
                if outputDirectory == nil, isDirectory,
                   url.deletingLastPathComponent().standardizedFileURL.path == inputRoot,
                   inPlaceSkipRoots.contains(url.lastPathComponent) {
                    enumerator.skipDescendants()
                    continue
                }
                let values = try url.resourceValues(forKeys: [.isRegularFileKey])
                guard values.isRegularFile == true,
                      supportedExtensions.contains(url.pathExtension.lowercased()) else { continue }
                collected[path] = url
            }
        }
        return collected.values.sorted { $0.standardizedFileURL.path < $1.standardizedFileURL.path }
    }

    private static func destinationURL(directory: URL, sourceName: String, sequence: Int) -> URL {
        guard sequence > 1 else { return directory.appendingPathComponent(sourceName) }
        let source = URL(fileURLWithPath: sourceName)
        let ext = source.pathExtension
        let stem = source.deletingPathExtension().lastPathComponent
        let name = ext.isEmpty ? "\(stem) (\(sequence))" : "\(stem) (\(sequence)).\(ext)"
        return directory.appendingPathComponent(name)
    }

    private static func filesMatch(_ left: URL, _ right: URL) throws -> Bool {
        let leftSize = try left.resourceValues(forKeys: [.fileSizeKey]).fileSize
        let rightSize = try right.resourceValues(forKeys: [.fileSizeKey]).fileSize
        guard leftSize == rightSize else { return false }
        return try sha256Hex(left) == sha256Hex(right)
    }

    private static func sha256Hex(_ url: URL) throws -> String {
        let data = try Data(contentsOf: url, options: [.mappedIfSafe])
        return SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
    }

    private static func copyAtomically(_ source: URL, to destination: URL, fileManager: FileManager) throws {
        let parent = destination.deletingLastPathComponent()
        try fileManager.createDirectory(at: parent, withIntermediateDirectories: true)
        let temporary = parent.appendingPathComponent(".xdremux-categorize-\(UUID().uuidString).tmp")
        defer { try? fileManager.removeItem(at: temporary) }
        try fileManager.copyItem(at: source, to: temporary)
        try fileManager.moveItem(at: temporary, to: destination)
    }
}
