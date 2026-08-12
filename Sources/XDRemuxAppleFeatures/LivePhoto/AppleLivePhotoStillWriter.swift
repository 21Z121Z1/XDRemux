import Foundation
import ImageIO
import UniformTypeIdentifiers
import XDRemuxCore

public enum AppleLivePhotoStillWriter {
    public static let makerNoteAssetIdentifierKey = "17"

    /// Converts the already-bounded static image resource to HEIC using ImageIO only.
    /// No XDRemux ProXDR reconstruction path is involved.
    public static func write(
        stillInputURL: URL,
        outputURL: URL,
        assetIdentifier: String,
        lossyCompressionQuality: Double? = nil
    ) throws {
        let sourceOptions: [CFString: Any] = [
            kCGImageSourceShouldCache: false,
            kCGImageSourceShouldCacheImmediately: false,
        ]
        guard let source = CGImageSourceCreateWithURL(stillInputURL as CFURL, sourceOptions as CFDictionary),
              CGImageSourceGetCount(source) > 0 else {
            throw AppleLivePhotoError.unreadableStillImage
        }

        try FileManager.default.createDirectory(
            at: outputURL.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        if FileManager.default.fileExists(atPath: outputURL.path) {
            try FileManager.default.removeItem(at: outputURL)
        }
        guard let destination = CGImageDestinationCreateWithURL(
            outputURL as CFURL,
            UTType.heic.identifier as CFString,
            1,
            nil
        ) else {
            throw AppleLivePhotoError.cannotCreateStillDestination
        }

        var properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [String: Any] ?? [:]
        var makerApple = properties[kCGImagePropertyMakerAppleDictionary as String] as? [AnyHashable: Any] ?? [:]
        makerApple[makerNoteAssetIdentifierKey] = assetIdentifier
        properties[kCGImagePropertyMakerAppleDictionary as String] = makerApple

        // Motion Photo XMP describes the source's appended-video container and becomes stale once
        // the still and motion resources are split. Apple documents this flag specifically for
        // preserving EXIF/IPTC while excluding XMP from the destination.
        properties[kCGImageMetadataShouldExcludeXMP as String] = true
        properties[kCGImageDestinationMergeMetadata as String] = true
        properties[kCGImageDestinationPreserveGainMap as String] = true
        if let metadata = CGImageSourceCopyMetadataAtIndex(source, 0, nil) {
            properties[kCGImageDestinationMetadata as String] = metadata
        }
        if let quality = lossyCompressionQuality {
            properties[kCGImageDestinationLossyCompressionQuality as String] = min(1, max(0, quality))
        }

        CGImageDestinationAddImageFromSource(destination, source, 0, properties as CFDictionary)
        guard CGImageDestinationFinalize(destination) else {
            try? FileManager.default.removeItem(at: outputURL)
            throw AppleLivePhotoError.cannotFinalizeStillImage
        }
    }

    /// Returns the encoded still-image pixel dimensions used by Core Media's Live Photo transform
    /// reference-dimensions metadata. Orientation remains a separate image property and is preserved
    /// by the still writer, so this reports the underlying raster width and height without swapping.
    public static func pixelDimensions(in imageURL: URL) -> [Float]? {
        guard let source = CGImageSourceCreateWithURL(
            imageURL as CFURL,
            [kCGImageSourceShouldCache: false] as CFDictionary
        ),
        let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [String: Any],
        let width = (properties[kCGImagePropertyPixelWidth as String] as? NSNumber)?.floatValue,
        let height = (properties[kCGImagePropertyPixelHeight as String] as? NSNumber)?.floatValue,
        width > 0, height > 0 else {
            return nil
        }
        return [width, height]
    }

    public static func assetIdentifier(in imageURL: URL) -> String? {
        let options = [kCGImageSourceShouldCache: false] as CFDictionary
        guard let source = CGImageSourceCreateWithURL(imageURL as CFURL, options),
              let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [String: Any],
              let makerApple = properties[kCGImagePropertyMakerAppleDictionary as String] as? [AnyHashable: Any] else {
            return nil
        }
        if let value = makerApple[makerNoteAssetIdentifierKey] as? String { return value }
        if let value = makerApple[NSNumber(value: 17)] as? String { return value }
        return nil
    }

    public static func hasGainMap(_ imageURL: URL) -> Bool {
        let options = [kCGImageSourceShouldCache: false] as CFDictionary
        guard let source = CGImageSourceCreateWithURL(imageURL as CFURL, options) else { return false }
        if CGImageSourceCopyAuxiliaryDataInfoAtIndex(source, 0, kCGImageAuxiliaryDataTypeISOGainMap) != nil {
            return true
        }
        return CGImageSourceCopyAuxiliaryDataInfoAtIndex(source, 0, kCGImageAuxiliaryDataTypeHDRGainMap) != nil
    }
}

public enum AppleLivePhotoError: Error, LocalizedError {
    case unreadableStillImage
    case cannotCreateStillDestination
    case cannotFinalizeStillImage
    case missingVideoTrack
    case unsupportedVideoCodec(String)
    case cannotCreateVideoReader
    case cannotCreateVideoWriter
    case cannotStartVideoReader(String)
    case cannotStartVideoWriter(String)
    case videoWriteFailed(String)
    case invalidTimeline
    case pairValidationFailed(String)
    case transactionFailed(String)

    public var errorDescription: String? {
        switch self {
        case .unreadableStillImage: return "The Motion Photo still resource cannot be read by ImageIO."
        case .cannotCreateStillDestination: return "ImageIO could not create the HEIC Live Photo still destination."
        case .cannotFinalizeStillImage: return "ImageIO could not finalize the HEIC Live Photo still image."
        case .missingVideoTrack: return "The Motion Photo payload contains no video track."
        case let .unsupportedVideoCodec(codec): return "The source video codec cannot be passed through to a Live Photo MOV: \(codec)."
        case .cannotCreateVideoReader: return "AVFoundation could not create the Live Photo video reader."
        case .cannotCreateVideoWriter: return "AVFoundation could not create the Live Photo MOV writer."
        case let .cannotStartVideoReader(message): return "AVAssetReader could not start: \(message)."
        case let .cannotStartVideoWriter(message): return "AVAssetWriter could not start: \(message)."
        case let .videoWriteFailed(message): return "Live Photo MOV writing failed: \(message)."
        case .invalidTimeline: return "The Motion Photo still-image time could not be resolved to a valid video timestamp."
        case let .pairValidationFailed(message): return "Live Photo pair validation failed: \(message)."
        case let .transactionFailed(message): return "Live Photo output transaction failed: \(message)."
        }
    }
}
