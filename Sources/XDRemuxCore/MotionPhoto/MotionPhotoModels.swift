import Foundation

/// Source format used to derive a normalized Motion Photo asset.
public enum MotionPhotoSourceKind: String, Sendable, Equatable {
    case androidMotionPhotoV1
    case androidHeifMotionPhotoV1
    case legacyMicroVideoV1b
    case oppoLivePhoto
}

/// Explains where the selected still-image presentation timestamp came from.
public enum MotionPhotoPresentationSource: String, Sendable, Equatable {
    case androidXMP
    case legacyMicroVideoXMP
    case oppoCoverFrame
    case timelineFallback
}

/// One logical item declared by the Android Motion Photo container directory.
public struct MotionPhotoItem: Sendable, Equatable {
    public let mime: String
    public let semantic: String
    public let length: Int64
    public let padding: Int64

    public init(mime: String, semantic: String, length: Int64, padding: Int64) {
        self.mime = mime
        self.semantic = semantic
        self.length = length
        self.padding = padding
    }
}

/// Checked byte range used instead of unchecked offset/length arithmetic.
public struct MotionPhotoByteRange: Sendable, Equatable {
    public let lowerBound: Int64
    public let upperBound: Int64

    public init(lowerBound: Int64, upperBound: Int64) throws {
        guard lowerBound >= 0, upperBound >= lowerBound else {
            throw MotionPhotoParsingError.invalidByteRange
        }
        self.lowerBound = lowerBound
        self.upperBound = upperBound
    }

    public var length: Int64 { upperBound - lowerBound }
}

/// OPPO vendor extension data. Generic Android parsing does not depend on any of these fields.
public struct OppoMotionPhotoMetadata: Sendable, Equatable {
    public let coverFramePtsUs: Int64?
    public let version: Int
    public let matrixCount: Int
    public let photoCropMatrix: [Double]?
    public let photoEisMatrix: [Double]?
    public let matrices: [String: [Double]]
    public let videoWidth: Int?
    public let videoHeight: Int?
    public let originPhotoWidth: Int?
    public let originPhotoHeight: Int?
    /// ColorOS 16 field observed on current devices. Values are commonly crop factors greater than
    /// one (for example 1.11), so Apple-facing compensation uses their reciprocal.
    public let photoEisCropFactor: [Double]?
    /// Older/alternate OPPO field. Existing samples can encode either a direct scale below one or
    /// a crop factor above one, so consumers must normalize its convention rather than assume one.
    public let eisCropFactor: [Double]?
    public let photoCropFactor: Double?
    public let streamCount: Int

    public init(
        coverFramePtsUs: Int64? = nil,
        version: Int = 0,
        matrixCount: Int = 0,
        photoCropMatrix: [Double]? = nil,
        photoEisMatrix: [Double]? = nil,
        matrices: [String: [Double]] = [:],
        videoWidth: Int? = nil,
        videoHeight: Int? = nil,
        originPhotoWidth: Int? = nil,
        originPhotoHeight: Int? = nil,
        photoEisCropFactor: [Double]? = nil,
        eisCropFactor: [Double]? = nil,
        photoCropFactor: Double? = nil,
        streamCount: Int = 1
    ) {
        self.coverFramePtsUs = coverFramePtsUs
        self.version = version
        self.matrixCount = matrixCount
        self.photoCropMatrix = photoCropMatrix
        self.photoEisMatrix = photoEisMatrix
        self.matrices = matrices
        self.videoWidth = videoWidth
        self.videoHeight = videoHeight
        self.originPhotoWidth = originPhotoWidth
        self.originPhotoHeight = originPhotoHeight
        self.photoEisCropFactor = photoEisCropFactor
        self.eisCropFactor = eisCropFactor
        self.photoCropFactor = photoCropFactor
        self.streamCount = max(1, streamCount)
    }
}

/// Normalized representation consumed by the Apple Live Photo writer.
public struct MotionPhotoAsset: Sendable, Equatable {
    public let sourceURL: URL
    public let sourceKind: MotionPhotoSourceKind
    public let items: [MotionPhotoItem]
    public let stillResourceRange: MotionPhotoByteRange
    public let videoResourceRange: MotionPhotoByteRange
    public let presentationTimestampUs: Int64?
    public let presentationSource: MotionPhotoPresentationSource?
    public let vendorMetadata: OppoMotionPhotoMetadata?

    public init(
        sourceURL: URL,
        sourceKind: MotionPhotoSourceKind,
        items: [MotionPhotoItem],
        stillResourceRange: MotionPhotoByteRange,
        videoResourceRange: MotionPhotoByteRange,
        presentationTimestampUs: Int64?,
        presentationSource: MotionPhotoPresentationSource?,
        vendorMetadata: OppoMotionPhotoMetadata? = nil
    ) {
        self.sourceURL = sourceURL
        self.sourceKind = sourceKind
        self.items = items
        self.stillResourceRange = stillResourceRange
        self.videoResourceRange = videoResourceRange
        self.presentationTimestampUs = presentationTimestampUs
        self.presentationSource = presentationSource
        self.vendorMetadata = vendorMetadata
    }

    public func enrichingWithOppoMetadata(
        _ metadata: OppoMotionPhotoMetadata,
        presentationTimestampUs: Int64? = nil,
        presentationSource: MotionPhotoPresentationSource? = nil
    ) -> MotionPhotoAsset {
        MotionPhotoAsset(
            sourceURL: sourceURL,
            sourceKind: .oppoLivePhoto,
            items: items,
            stillResourceRange: stillResourceRange,
            videoResourceRange: videoResourceRange,
            presentationTimestampUs: presentationTimestampUs ?? self.presentationTimestampUs,
            presentationSource: presentationSource ?? self.presentationSource,
            vendorMetadata: metadata
        )
    }
}

public enum MotionPhotoParsingError: Error, LocalizedError, Equatable {
    case fileTooSmall
    case xmpTooLarge
    case malformedXMP
    case unsupportedVersion(Int?)
    case invalidDirectory
    case invalidPrimaryItem
    case invalidMotionPhotoItem
    case invalidItemLength
    case arithmeticOverflow
    case invalidByteRange
    case invalidVideoPayload
    case payloadTooLarge

    public var errorDescription: String? {
        switch self {
        case .fileTooSmall:
            return "Motion Photo input is too small."
        case .xmpTooLarge:
            return "Motion Photo XMP exceeds the supported safety limit."
        case .malformedXMP:
            return "Motion Photo XMP is malformed."
        case let .unsupportedVersion(version):
            if let version { return "Unsupported Motion Photo version: \(version)." }
            return "Motion Photo version is missing."
        case .invalidDirectory:
            return "Motion Photo container directory is invalid."
        case .invalidPrimaryItem:
            return "Motion Photo must contain exactly one leading Primary item."
        case .invalidMotionPhotoItem:
            return "Motion Photo must contain exactly one trailing MP4/QuickTime MotionPhoto item."
        case .invalidItemLength:
            return "Motion Photo item length or padding is invalid."
        case .arithmeticOverflow:
            return "Motion Photo byte-range arithmetic overflowed."
        case .invalidByteRange:
            return "Motion Photo contains an invalid byte range."
        case .invalidVideoPayload:
            return "Motion Photo video payload is not a valid ISO BMFF stream."
        case .payloadTooLarge:
            return "Motion Photo payload exceeds the configured extraction limit."
        }
    }
}
