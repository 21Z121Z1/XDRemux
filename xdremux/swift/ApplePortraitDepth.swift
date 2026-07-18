#!/usr/bin/env swift

import AVFoundation
import CoreGraphics
import CoreVideo
import Foundation
import ImageIO
import UniformTypeIdentifiers

private enum ToolError: Error, CustomStringConvertible {
    case usage(String)
    case missingArgument(String)
    case unknownOption(String)
    case invalidValue(option: String, value: String)
    case fileNotFound(URL)
    case outputExists(URL)
    case unableToRead(URL)
    case invalidDepthLength(expected: Int, actual: Int)
    case unableToCreateImageSource(URL)
    case unableToCreateDestination(URL)
    case unableToFinalize(URL)
    case missingDisparity(URL)
    case malformedAuxiliaryData(String)
    case unableToCreateMetadata(String)
    case referenceMetadataMissing(String)
    case verificationFailed(String)

    var description: String {
        switch self {
        case .usage(let message): return message
        case .missingArgument(let name): return "missing required argument: \(name)"
        case .unknownOption(let option): return "unknown option: \(option)"
        case .invalidValue(let option, let value): return "invalid value for \(option): \(value)"
        case .fileNotFound(let url): return "file not found: \(url.path)"
        case .outputExists(let url): return "output already exists (pass --overwrite to replace it): \(url.path)"
        case .unableToRead(let url): return "unable to read: \(url.path)"
        case .invalidDepthLength(let expected, let actual):
            return "depth byte count mismatch: expected \(expected), got \(actual)"
        case .unableToCreateImageSource(let url): return "unable to open image source: \(url.path)"
        case .unableToCreateDestination(let url): return "unable to create HEIC destination: \(url.path)"
        case .unableToFinalize(let url): return "ImageIO failed to finalize HEIC: \(url.path)"
        case .missingDisparity(let url): return "no Apple disparity auxiliary image found: \(url.path)"
        case .malformedAuxiliaryData(let message): return "malformed auxiliary data: \(message)"
        case .unableToCreateMetadata(let message): return "unable to create metadata: \(message)"
        case .referenceMetadataMissing(let path): return "reference disparity metadata is missing \(path)"
        case .verificationFailed(let message): return "output verification failed: \(message)"
        }
    }
}

private enum Profile: String {
    case publicDepth = "public"
    case portrait
}

private enum RankNear: String {
    case low
    case high
}

private enum ReferenceDepthMetadata: String {
    case render
    case full
}

private enum ReferencePrimaryMetadata: String {
    case none
    case maker
    case full
}

private enum SourcePrimaryMetadataLevel: String {
    case none
    case model
    case camera
    case exposure
    case exif
    case all
}

private struct ConvertOptions {
    let inputURL: URL
    let depthURL: URL
    let outputURL: URL
    let depthWidth: Int
    let depthHeight: Int
    let profile: Profile
    let referenceURL: URL?
    let referenceDepthMetadata: ReferenceDepthMetadata
    let referencePrimaryMetadata: ReferencePrimaryMetadata
    let copyReferencePrimaryXMP: Bool
    let copyReferenceHDRGainMap: Bool
    let sourcePrimaryMetadataURL: URL?
    let sourcePrimaryMetadataLevel: SourcePrimaryMetadataLevel
    let rankNear: RankNear
    let farDisparity: Float
    let nearDisparity: Float
    let orientationOverride: UInt32?
    let auxiliaryOrientation: UInt32
    let primaryOrientationOverride: UInt32?
    let customRendered: Int
    let appleMakerNoteVersion: Int?
    let appleImageCaptureType: Int?
    let appleFeatureFlags: Int?
    let overwrite: Bool
}

private struct RepackOptions {
    let inputURL: URL
    let outputURL: URL
    let overwrite: Bool
}

private struct Inspection {
    let width: Int
    let height: Int
    let bytesPerRow: Int
    let pixelFormat: UInt32
    let orientation: UInt32?
    let accuracy: AVDepthData.Accuracy
    let quality: AVDepthData.Quality
    let filtered: Bool
    let minimum: Float
    let maximum: Float
    let hasDepthBlurParameters: Bool
    let simulatedAperture: String?
    let portraitLightingStrength: String?
    let hasCameraCalibration: Bool
    let primaryMake: String?
    let primaryModel: String?
    let makerAppleFieldCount: Int
    let customRendered: Int?
    let appleMakerNoteVersion: Int?
    let appleImageCaptureType: Int?
    let appleFeatureFlags: Int?
    let hasHDRGainMap: Bool
    let hasISOGainMap: Bool
}

private struct OrientedRanks {
    let data: Data
    let width: Int
    let height: Int
}

private let usage = """
Usage:
  swift ApplePortraitDepth.swift convert \\
    --input PHOTO.heic --depth-u8 DEPTH.bin \\
    --depth-width WIDTH --depth-height HEIGHT --output OUTPUT.heic \\
    [--profile public|portrait] [--reference IPHONE_PORTRAIT.heic] \\
    [--reference-depth-metadata render|full] \\
    [--reference-primary-metadata none|maker|full] \\
    [--reference-primary-xmp] \\
    [--reference-hdr-gain-map] \\
    [--source-primary-metadata PHOTO.heic] \\
    [--source-primary-metadata-level none|model|camera|exposure|exif|all] \\
    [--rank-near low|high] [--far-disparity 1.4] [--near-disparity 4.3] \\
    [--orientation auto|1...8] [--auxiliary-orientation 1...8] \\
    [--primary-orientation auto|1...8] [--custom-rendered 9] \
    [--apple-maker-note-version N] [--apple-image-capture-type N] \
    [--apple-feature-flags N] [--overwrite]

  swift ApplePortraitDepth.swift repack \\
    --input IPHONE_PORTRAIT.heic --output OUTPUT.heic [--overwrite]

  swift ApplePortraitDepth.swift inspect --input PHOTO.heic

Notes:
  - The OPPO decoder emits one uint8 rank per pixel. Current rear-camera samples
    use lower ranks for nearer content, so --rank-near defaults to low.
  - --orientation describes the transform from the raw depth plane to the
    displayed primary image. The pixels are transformed before HEIC encoding.
  - --auxiliary-orientation sets the encoded disparity orientation tag after
    that pixel transform and defaults to 1.
  - profile=public writes only public AVDepthData/ImageIO metadata.
  - profile=portrait additionally copies depthBlurEffect and
    portraitLightingEffect metadata from --reference and sets CustomRendered.
  - reference-depth-metadata=full copies the reference disparity's complete
    metadata, including camera calibration. This is a classifier experiment;
    the reference calibration does not describe the input camera geometry.
  - reference-primary-metadata and reference-hdr-gain-map are diagnostic
    controls for isolating Photos primary-asset eligibility requirements.
"""

private func fourCC(_ value: UInt32) -> String {
    var bigEndian = value.bigEndian
    return withUnsafeBytes(of: &bigEndian) { bytes in
        String(bytes: bytes.map { (32...126).contains($0) ? $0 : UInt8(ascii: ".") }, encoding: .ascii) ?? "????"
    }
}

private func fileURL(_ path: String) -> URL {
    URL(fileURLWithPath: NSString(string: path).expandingTildeInPath).standardizedFileURL
}

private func requireExistingFile(_ url: URL) throws {
    var isDirectory: ObjCBool = false
    guard FileManager.default.fileExists(atPath: url.path, isDirectory: &isDirectory), !isDirectory.boolValue else {
        throw ToolError.fileNotFound(url)
    }
}

private func parseOptions(_ arguments: ArraySlice<String>) throws -> [String: String] {
    var result: [String: String] = [:]
    var iterator = arguments.makeIterator()
    while let option = iterator.next() {
        guard option.hasPrefix("--") else { throw ToolError.unknownOption(option) }
        if option == "--overwrite" || option == "--reference-hdr-gain-map" || option == "--reference-primary-xmp" {
            result[option] = "true"
            continue
        }
        guard let value = iterator.next(), !value.hasPrefix("--") else {
            throw ToolError.missingArgument(option)
        }
        result[option] = value
    }
    return result
}

private func required(_ name: String, in values: [String: String]) throws -> String {
    guard let value = values[name] else { throw ToolError.missingArgument(name) }
    return value
}

private func integer(_ name: String, in values: [String: String]) throws -> Int {
    let raw = try required(name, in: values)
    guard let value = Int(raw) else { throw ToolError.invalidValue(option: name, value: raw) }
    return value
}

private func parseConvertOptions(_ arguments: ArraySlice<String>) throws -> ConvertOptions {
    let values = try parseOptions(arguments)
    let known: Set<String> = [
        "--input", "--depth-u8", "--output", "--depth-width", "--depth-height",
        "--profile", "--reference", "--rank-near", "--far-disparity",
        "--near-disparity", "--orientation", "--custom-rendered",
        "--reference-depth-metadata", "--auxiliary-orientation",
        "--reference-primary-metadata", "--reference-primary-xmp", "--reference-hdr-gain-map",
        "--source-primary-metadata", "--source-primary-metadata-level",
        "--primary-orientation",
        "--apple-maker-note-version", "--apple-image-capture-type",
        "--apple-feature-flags", "--overwrite"
    ]
    if let unknown = values.keys.first(where: { !known.contains($0) }) {
        throw ToolError.unknownOption(unknown)
    }

    let inputURL = fileURL(try required("--input", in: values))
    let depthURL = fileURL(try required("--depth-u8", in: values))
    let outputURL = fileURL(try required("--output", in: values))
    let width = try integer("--depth-width", in: values)
    let height = try integer("--depth-height", in: values)
    guard width > 0 else { throw ToolError.invalidValue(option: "--depth-width", value: String(width)) }
    guard height > 0 else { throw ToolError.invalidValue(option: "--depth-height", value: String(height)) }

    let profileRaw = values["--profile"] ?? Profile.publicDepth.rawValue
    guard let profile = Profile(rawValue: profileRaw) else {
        throw ToolError.invalidValue(option: "--profile", value: profileRaw)
    }
    let rankRaw = values["--rank-near"] ?? RankNear.low.rawValue
    guard let rankNear = RankNear(rawValue: rankRaw) else {
        throw ToolError.invalidValue(option: "--rank-near", value: rankRaw)
    }
    let farRaw = values["--far-disparity"] ?? "1.4"
    let nearRaw = values["--near-disparity"] ?? "4.3"
    guard let far = Float(farRaw), far > 0 else {
        throw ToolError.invalidValue(option: "--far-disparity", value: farRaw)
    }
    guard let near = Float(nearRaw), near > far else {
        throw ToolError.invalidValue(option: "--near-disparity", value: nearRaw)
    }

    let orientationRaw = values["--orientation"] ?? "auto"
    let orientation: UInt32?
    if orientationRaw == "auto" {
        orientation = nil
    } else if let parsed = UInt32(orientationRaw), (1...8).contains(parsed) {
        orientation = parsed
    } else {
        throw ToolError.invalidValue(option: "--orientation", value: orientationRaw)
    }

    let auxiliaryOrientationRaw = values["--auxiliary-orientation"] ?? "1"
    guard let auxiliaryOrientation = UInt32(auxiliaryOrientationRaw),
          (1...8).contains(auxiliaryOrientation) else {
        throw ToolError.invalidValue(
            option: "--auxiliary-orientation",
            value: auxiliaryOrientationRaw
        )
    }

    let primaryOrientationRaw = values["--primary-orientation"] ?? "auto"
    let primaryOrientation: UInt32?
    if primaryOrientationRaw == "auto" {
        primaryOrientation = nil
    } else if let parsed = UInt32(primaryOrientationRaw), (1...8).contains(parsed) {
        primaryOrientation = parsed
    } else {
        throw ToolError.invalidValue(option: "--primary-orientation", value: primaryOrientationRaw)
    }

    let customRenderedRaw = values["--custom-rendered"] ?? "9"
    guard let customRendered = Int(customRenderedRaw), customRendered >= 0 else {
        throw ToolError.invalidValue(option: "--custom-rendered", value: customRenderedRaw)
    }
    func optionalNonnegativeInteger(_ name: String) throws -> Int? {
        guard let raw = values[name] else { return nil }
        guard let value = Int(raw), value >= 0 else {
            throw ToolError.invalidValue(option: name, value: raw)
        }
        return value
    }
    let appleMakerNoteVersion = try optionalNonnegativeInteger("--apple-maker-note-version")
    let appleImageCaptureType = try optionalNonnegativeInteger("--apple-image-capture-type")
    let appleFeatureFlags = try optionalNonnegativeInteger("--apple-feature-flags")
    let referenceURL = values["--reference"].map(fileURL)
    let sourcePrimaryMetadataURL = values["--source-primary-metadata"].map(fileURL)
    let sourcePrimaryMetadataLevelRaw = values["--source-primary-metadata-level"] ?? SourcePrimaryMetadataLevel.none.rawValue
    guard let sourcePrimaryMetadataLevel = SourcePrimaryMetadataLevel(rawValue: sourcePrimaryMetadataLevelRaw) else {
        throw ToolError.invalidValue(option: "--source-primary-metadata-level", value: sourcePrimaryMetadataLevelRaw)
    }
    if sourcePrimaryMetadataLevel != .none, sourcePrimaryMetadataURL == nil {
        throw ToolError.missingArgument("--source-primary-metadata")
    }
    let referenceDepthMetadataRaw = values["--reference-depth-metadata"] ?? ReferenceDepthMetadata.render.rawValue
    guard let referenceDepthMetadata = ReferenceDepthMetadata(rawValue: referenceDepthMetadataRaw) else {
        throw ToolError.invalidValue(
            option: "--reference-depth-metadata",
            value: referenceDepthMetadataRaw
        )
    }
    let referencePrimaryMetadataRaw = values["--reference-primary-metadata"] ?? ReferencePrimaryMetadata.none.rawValue
    guard let referencePrimaryMetadata = ReferencePrimaryMetadata(rawValue: referencePrimaryMetadataRaw) else {
        throw ToolError.invalidValue(
            option: "--reference-primary-metadata",
            value: referencePrimaryMetadataRaw
        )
    }
    if profile == .portrait, referenceURL == nil {
        throw ToolError.missingArgument("--reference (required for --profile portrait)")
    }
    if (referencePrimaryMetadata != .none || values["--reference-primary-xmp"] == "true" || values["--reference-hdr-gain-map"] == "true"),
       referenceURL == nil {
        throw ToolError.missingArgument("--reference (required for reference primary metadata)")
    }

    return ConvertOptions(
        inputURL: inputURL,
        depthURL: depthURL,
        outputURL: outputURL,
        depthWidth: width,
        depthHeight: height,
        profile: profile,
        referenceURL: referenceURL,
        referenceDepthMetadata: referenceDepthMetadata,
        referencePrimaryMetadata: referencePrimaryMetadata,
        copyReferencePrimaryXMP: values["--reference-primary-xmp"] == "true",
        copyReferenceHDRGainMap: values["--reference-hdr-gain-map"] == "true",
        sourcePrimaryMetadataURL: sourcePrimaryMetadataURL,
        sourcePrimaryMetadataLevel: sourcePrimaryMetadataLevel,
        rankNear: rankNear,
        farDisparity: far,
        nearDisparity: near,
        orientationOverride: orientation,
        auxiliaryOrientation: auxiliaryOrientation,
        primaryOrientationOverride: primaryOrientation,
        customRendered: customRendered,
        appleMakerNoteVersion: appleMakerNoteVersion,
        appleImageCaptureType: appleImageCaptureType,
        appleFeatureFlags: appleFeatureFlags,
        overwrite: values["--overwrite"] == "true"
    )
}

private func parseRepackOptions(_ arguments: ArraySlice<String>) throws -> RepackOptions {
    let values = try parseOptions(arguments)
    let known: Set<String> = ["--input", "--output", "--overwrite"]
    if let unknown = values.keys.first(where: { !known.contains($0) }) {
        throw ToolError.unknownOption(unknown)
    }
    return RepackOptions(
        inputURL: fileURL(try required("--input", in: values)),
        outputURL: fileURL(try required("--output", in: values)),
        overwrite: values["--overwrite"] == "true"
    )
}

private func metadataString(_ metadata: CGImageMetadata, path: String) -> String? {
    CGImageMetadataCopyStringValueWithPath(metadata, nil, path as CFString) as String?
}

private func imageMetadata(_ value: Any?) -> CGImageMetadata? {
    guard let value else { return nil }
    let object = value as CFTypeRef
    guard CFGetTypeID(object) == CGImageMetadataGetTypeID() else { return nil }
    return unsafeBitCast(object, to: CGImageMetadata.self)
}

private func dictionaryKey(_ value: CFString) -> AnyHashable {
    AnyHashable(value as String)
}

private func auxiliaryInfo(
    source: CGImageSource,
    type: CFString
) -> [AnyHashable: Any]? {
    let primaryIndex = CGImageSourceGetPrimaryImageIndex(source)
    var indices = [0]
    if primaryIndex != 0 { indices.append(primaryIndex) }
    for index in indices {
        if let raw = CGImageSourceCopyAuxiliaryDataInfoAtIndex(source, index, type) {
            let dictionary = raw as NSDictionary
            var info: [AnyHashable: Any] = [:]
            for key in dictionary.allKeys {
                guard let stringKey = key as? String else { continue }
                info[AnyHashable(stringKey)] = dictionary[key]
            }
            return info
        }
    }
    return nil
}

private func auxiliaryProbeSummary(source: CGImageSource, type: CFString) -> String {
    let primaryIndex = CGImageSourceGetPrimaryImageIndex(source)
    var indices = [0]
    if primaryIndex != 0 { indices.append(primaryIndex) }
    let results = indices.map { index in
        let found = CGImageSourceCopyAuxiliaryDataInfoAtIndex(source, index, type) != nil
        return "\(index):\(found ? "yes" : "no")"
    }
    return "count=\(CGImageSourceGetCount(source)) primary=\(primaryIndex) probes=\(results.joined(separator: ","))"
}

private func registerNamespace(
    _ metadata: CGMutableImageMetadata,
    namespace: String,
    prefix: String
) throws {
    var error: Unmanaged<CFError>?
    guard CGImageMetadataRegisterNamespaceForPrefix(
        metadata,
        namespace as CFString,
        prefix as CFString,
        &error
    ) else {
        if let error { throw error.takeRetainedValue() as Error }
        throw ToolError.unableToCreateMetadata("unable to register namespace \(prefix)")
    }
}

private func setMetadata(_ metadata: CGMutableImageMetadata, path: String, value: String) throws {
    guard CGImageMetadataSetValueWithPath(metadata, nil, path as CFString, value as CFString) else {
        throw ToolError.unableToCreateMetadata("unable to set \(path)")
    }
}

private func makeDepthMetadata() throws -> CGMutableImageMetadata {
    let metadata = CGImageMetadataCreateMutable()
    try registerNamespace(
        metadata,
        namespace: "http://ns.apple.com/depthData/1.0/",
        prefix: "depthData"
    )
    try setMetadata(metadata, path: "depthData:Quality", value: "high")
    try setMetadata(metadata, path: "depthData:Accuracy", value: "relative")
    try setMetadata(metadata, path: "depthData:Filtered", value: "True")
    return metadata
}

private func makeDisparityData(
    ranks: Data,
    rankNear: RankNear,
    far: Float,
    near: Float
) -> Data {
    var values = [UInt16](repeating: 0, count: ranks.count)
    ranks.withUnsafeBytes { rawBuffer in
        let bytes = rawBuffer.bindMemory(to: UInt8.self)
        for index in bytes.indices {
            let rank = Float(bytes[index]) / 255.0
            let nearness = rankNear == .low ? (1.0 - rank) : rank
            let disparity = far + nearness * (near - far)
            values[index] = Float16(disparity).bitPattern.littleEndian
        }
    }
    return values.withUnsafeBytes { Data($0) }
}

private func applyExifOrientation(
    _ ranks: Data,
    width: Int,
    height: Int,
    orientation: UInt32
) throws -> OrientedRanks {
    guard (1...8).contains(orientation) else {
        throw ToolError.invalidValue(option: "--orientation", value: String(orientation))
    }
    let swapsAxes = orientation >= 5
    let outputWidth = swapsAxes ? height : width
    let outputHeight = swapsAxes ? width : height
    if orientation == 1 {
        return OrientedRanks(data: ranks, width: width, height: height)
    }

    var output = Data(count: ranks.count)
    ranks.withUnsafeBytes { sourceBuffer in
        output.withUnsafeMutableBytes { outputBuffer in
            let source = sourceBuffer.bindMemory(to: UInt8.self)
            let destination = outputBuffer.bindMemory(to: UInt8.self)
            for y in 0..<height {
                for x in 0..<width {
                    let destinationPoint: (Int, Int)
                    switch orientation {
                    case 2: destinationPoint = (width - 1 - x, y)
                    case 3: destinationPoint = (width - 1 - x, height - 1 - y)
                    case 4: destinationPoint = (x, height - 1 - y)
                    case 5: destinationPoint = (y, x)
                    case 6: destinationPoint = (height - 1 - y, x)
                    case 7: destinationPoint = (height - 1 - y, width - 1 - x)
                    case 8: destinationPoint = (y, width - 1 - x)
                    default: destinationPoint = (x, y)
                    }
                    destination[destinationPoint.1 * outputWidth + destinationPoint.0] = source[y * width + x]
                }
            }
        }
    }
    return OrientedRanks(data: output, width: outputWidth, height: outputHeight)
}

private func makeCanonicalAuxiliaryInfo(
    data: Data,
    width: Int,
    height: Int,
    orientation: UInt32
) throws -> [AnyHashable: Any] {
    let metadata = try makeDepthMetadata()
    let description: [CFString: Any] = [
        kCGImagePropertyWidth: NSNumber(value: width),
        kCGImagePropertyHeight: NSNumber(value: height),
        kCGImagePropertyBytesPerRow: NSNumber(value: width * MemoryLayout<Float16>.stride),
        kCGImagePropertyPixelFormat: NSNumber(value: kCVPixelFormatType_DisparityFloat16),
        kCGImagePropertyOrientation: NSNumber(value: orientation)
    ]
    let initial: [AnyHashable: Any] = [
        kCGImageAuxiliaryDataInfoData: data as CFData,
        kCGImageAuxiliaryDataInfoDataDescription: description as CFDictionary,
        kCGImageAuxiliaryDataInfoMetadata: metadata
    ]

    let depth: AVDepthData
    do {
        depth = try AVDepthData(fromDictionaryRepresentation: initial)
    } catch {
        throw ToolError.malformedAuxiliaryData(error.localizedDescription)
    }
    var auxiliaryType: NSString?
    guard var canonical = depth.dictionaryRepresentation(forAuxiliaryDataType: &auxiliaryType),
          auxiliaryType == kCGImageAuxiliaryDataTypeDisparity,
          var canonicalDescription = canonical[dictionaryKey(kCGImageAuxiliaryDataInfoDataDescription)] as? [AnyHashable: Any] else {
        throw ToolError.malformedAuxiliaryData("AVDepthData did not produce a disparity dictionary")
    }
    canonicalDescription[dictionaryKey(kCGImagePropertyOrientation)] = NSNumber(value: orientation)
    canonical[dictionaryKey(kCGImageAuxiliaryDataInfoDataDescription)] = canonicalDescription as CFDictionary
    return canonical
}

private func addPortraitMetadata(
    to info: inout [AnyHashable: Any],
    referenceURL: URL,
    mode: ReferenceDepthMetadata
) throws {
    guard let source = CGImageSourceCreateWithURL(referenceURL as CFURL, nil) else {
        throw ToolError.unableToCreateImageSource(referenceURL)
    }
    guard let referenceInfo = auxiliaryInfo(source: source, type: kCGImageAuxiliaryDataTypeDisparity),
          let referenceMetadata = imageMetadata(referenceInfo[dictionaryKey(kCGImageAuxiliaryDataInfoMetadata)]) else {
        throw ToolError.missingDisparity(referenceURL)
    }

    if mode == .full {
        guard let metadata = CGImageMetadataCreateMutableCopy(referenceMetadata) else {
            throw ToolError.unableToCreateMetadata("unable to copy reference disparity metadata")
        }
        info[dictionaryKey(kCGImageAuxiliaryDataInfoMetadata)] = metadata
        return
    }

    guard let existingMetadata = imageMetadata(info[dictionaryKey(kCGImageAuxiliaryDataInfoMetadata)]),
          let metadata = CGImageMetadataCreateMutableCopy(existingMetadata) else {
        throw ToolError.unableToCreateMetadata("unable to copy generated disparity metadata")
    }

    try registerNamespace(
        metadata,
        namespace: "http://ns.apple.com/depthBlurEffect/1.0/",
        prefix: "depthBlurEffect"
    )
    try registerNamespace(
        metadata,
        namespace: "http://ns.apple.com/portraitLightingEffect/1.0/",
        prefix: "portraitLightingEffect"
    )

    let requiredPaths = [
        "depthBlurEffect:RenderingParameters",
        "depthBlurEffect:SimulatedAperture",
        "portraitLightingEffect:EffectStrength"
    ]
    for path in requiredPaths {
        guard let value = metadataString(referenceMetadata, path: path) else {
            throw ToolError.referenceMetadataMissing(path)
        }
        try setMetadata(metadata, path: path, value: value)
    }
    if let version = metadataString(referenceMetadata, path: "depthData:DepthDataVersion") {
        try setMetadata(metadata, path: "depthData:DepthDataVersion", value: version)
    }
    info[dictionaryKey(kCGImageAuxiliaryDataInfoMetadata)] = metadata
}

private func imageOrientation(source: CGImageSource, index: Int) -> UInt32 {
    guard let properties = CGImageSourceCopyPropertiesAtIndex(source, index, nil) as? [CFString: Any] else {
        return 1
    }
    if let number = properties[kCGImagePropertyOrientation] as? NSNumber {
        return number.uint32Value
    }
    return 1
}

private func resolvedDepthOrientation(
    source: CGImageSource,
    index: Int,
    depthWidth: Int,
    depthHeight: Int,
    override: UInt32?
) -> UInt32 {
    if let override { return override }
    let sourceOrientation = imageOrientation(source: source, index: index)
    guard sourceOrientation == 1,
          let properties = CGImageSourceCopyPropertiesAtIndex(source, index, nil) as? [CFString: Any],
          let width = (properties[kCGImagePropertyPixelWidth] as? NSNumber)?.intValue,
          let height = (properties[kCGImagePropertyPixelHeight] as? NSNumber)?.intValue else {
        return sourceOrientation
    }
    let sourceIsPortrait = width < height
    let depthIsPortrait = depthWidth < depthHeight
    return sourceIsPortrait != depthIsPortrait ? 6 : sourceOrientation
}

private func updatedImageOptions(
    source: CGImageSource,
    index: Int,
    referenceProperties: [CFString: Any]?,
    referencePrimaryXMP: CGImageMetadata?,
    sourcePrimaryProperties: [CFString: Any]?,
    sourcePrimaryMetadataLevel: SourcePrimaryMetadataLevel,
    referencePrimaryMetadata: ReferencePrimaryMetadata,
    primaryOrientationOverride: UInt32?,
    customRendered: Int?,
    appleMakerNoteVersion: Int?,
    appleImageCaptureType: Int?,
    appleFeatureFlags: Int?
) -> [CFString: Any] {
    var options: [CFString: Any] = [
        kCGImageDestinationPreserveGainMap: true,
        kCGImageDestinationLossyCompressionQuality: 1.0
    ]
    if let referencePrimaryXMP {
        options[kCGImageDestinationMetadata] = referencePrimaryXMP
        options[kCGImageDestinationMergeMetadata] = true
    }
    let sourceProperties = CGImageSourceCopyPropertiesAtIndex(source, index, nil) as? [CFString: Any]
    let metadataProperties = referencePrimaryMetadata == .full ? referenceProperties : sourceProperties
    if referencePrimaryMetadata == .full, let referenceProperties {
        let dictionaries = [
            kCGImagePropertyTIFFDictionary,
            kCGImagePropertyExifDictionary,
            kCGImagePropertyGPSDictionary,
            kCGImagePropertyIPTCDictionary,
            kCGImagePropertyMakerAppleDictionary
        ]
        for key in dictionaries {
            if let value = referenceProperties[key] { options[key] = value }
        }
    } else if referencePrimaryMetadata == .maker,
              let makerApple = referenceProperties?[kCGImagePropertyMakerAppleDictionary] {
        options[kCGImagePropertyMakerAppleDictionary] = makerApple
    }

    if sourcePrimaryMetadataLevel != .none, let sourcePrimaryProperties {
        let sourceTIFF = sourcePrimaryProperties[kCGImagePropertyTIFFDictionary] as? [CFString: Any]
        let sourceExif = sourcePrimaryProperties[kCGImagePropertyExifDictionary] as? [CFString: Any]
        switch sourcePrimaryMetadataLevel {
        case .none:
            break
        case .model, .camera, .exposure:
            var tiff = (options[kCGImagePropertyTIFFDictionary] as? [CFString: Any]) ?? [:]
            if let model = sourceTIFF?[kCGImagePropertyTIFFModel] {
                tiff[kCGImagePropertyTIFFModel] = model
            }
            if sourcePrimaryMetadataLevel != .model,
               let make = sourceTIFF?[kCGImagePropertyTIFFMake] {
                tiff[kCGImagePropertyTIFFMake] = make
            }
            options[kCGImagePropertyTIFFDictionary] = tiff as CFDictionary
            if sourcePrimaryMetadataLevel == .exposure, let sourceExif {
                var exif = (options[kCGImagePropertyExifDictionary] as? [CFString: Any]) ?? [:]
                let exposureKeys = [
                    "ExposureTime", "FNumber", "ISOSpeedRatings", "PhotographicSensitivity",
                    "FocalLength", "LensModel", "DateTimeOriginal", "OffsetTimeOriginal",
                    "ShutterSpeedValue", "ApertureValue", "BrightnessValue", "ExposureBiasValue",
                    "MaxApertureValue", "MeteringMode", "Flash", "FocalLenIn35mmFilm"
                ].map { $0 as CFString }
                for key in exposureKeys where sourceExif[key] != nil {
                    exif[key] = sourceExif[key]
                }
                options[kCGImagePropertyExifDictionary] = exif as CFDictionary
            }
        case .exif:
            if let sourceTIFF { options[kCGImagePropertyTIFFDictionary] = sourceTIFF as CFDictionary }
            if let sourceExif { options[kCGImagePropertyExifDictionary] = sourceExif as CFDictionary }
        case .all:
            for key in [
                kCGImagePropertyTIFFDictionary,
                kCGImagePropertyExifDictionary,
                kCGImagePropertyGPSDictionary,
                kCGImagePropertyIPTCDictionary
            ] {
                if let value = sourcePrimaryProperties[key] { options[key] = value }
            }
        }
    }

    let primaryOrientation = primaryOrientationOverride ?? imageOrientation(source: source, index: index)
    options[kCGImagePropertyOrientation] = NSNumber(value: primaryOrientation)
    var tiff = (options[kCGImagePropertyTIFFDictionary] as? [CFString: Any]) ?? [:]
    if tiff.isEmpty, let sourceTIFF = sourceProperties?[kCGImagePropertyTIFFDictionary] as? [CFString: Any] {
        tiff = sourceTIFF
    }
    tiff[kCGImagePropertyTIFFOrientation] = NSNumber(value: primaryOrientation)
    options[kCGImagePropertyTIFFDictionary] = tiff as CFDictionary

    if let customRendered {
        var exif = (options[kCGImagePropertyExifDictionary] as? [CFString: Any]) ?? [:]
        if exif.isEmpty,
           let metadataExif = metadataProperties?[kCGImagePropertyExifDictionary] as? [CFString: Any] {
            exif = metadataExif
        }
        exif[kCGImagePropertyExifCustomRendered] = NSNumber(value: customRendered)
        options[kCGImagePropertyExifDictionary] = exif as CFDictionary
    }
    if let pixelWidth = sourceProperties?[kCGImagePropertyPixelWidth] as? NSNumber,
       let pixelHeight = sourceProperties?[kCGImagePropertyPixelHeight] as? NSNumber {
        var exif = (options[kCGImagePropertyExifDictionary] as? [CFString: Any]) ?? [:]
        exif[kCGImagePropertyExifPixelXDimension] = pixelWidth
        exif[kCGImagePropertyExifPixelYDimension] = pixelHeight
        options[kCGImagePropertyExifDictionary] = exif as CFDictionary
    }
    if appleMakerNoteVersion != nil || appleImageCaptureType != nil || appleFeatureFlags != nil {
        var makerApple: [String: Any] = [:]
        if let existing = options[kCGImagePropertyMakerAppleDictionary] as? NSDictionary {
            for (key, value) in existing {
                if let key = key as? String { makerApple[key] = value }
            }
        }
        if let appleMakerNoteVersion { makerApple["1"] = NSNumber(value: appleMakerNoteVersion) }
        if let appleImageCaptureType { makerApple["20"] = NSNumber(value: appleImageCaptureType) }
        if let appleFeatureFlags { makerApple["31"] = NSNumber(value: appleFeatureFlags) }
        options[kCGImagePropertyMakerAppleDictionary] = makerApple as CFDictionary
    }
    return options
}

private func writeHEIC(
    source: CGImageSource,
    auxiliaryInfo: [AnyHashable: Any],
    referenceProperties: [CFString: Any]?,
    referencePrimaryXMP: CGImageMetadata?,
    sourcePrimaryProperties: [CFString: Any]?,
    sourcePrimaryMetadataLevel: SourcePrimaryMetadataLevel,
    referencePrimaryMetadata: ReferencePrimaryMetadata,
    primaryOrientationOverride: UInt32?,
    referenceHDRGainMapInfo: [AnyHashable: Any]?,
    profile: Profile,
    customRendered: Int,
    appleMakerNoteVersion: Int?,
    appleImageCaptureType: Int?,
    appleFeatureFlags: Int?,
    outputURL: URL
) throws {
    let index = CGImageSourceGetPrimaryImageIndex(source)
    guard let destination = CGImageDestinationCreateWithURL(
        outputURL as CFURL,
        UTType.heic.identifier as CFString,
        1,
        nil
    ) else {
        throw ToolError.unableToCreateDestination(outputURL)
    }
    let exifValue = profile == .portrait ? customRendered : nil
    let options = updatedImageOptions(
        source: source,
        index: index,
        referenceProperties: referenceProperties,
        referencePrimaryXMP: referencePrimaryXMP,
        sourcePrimaryProperties: sourcePrimaryProperties,
        sourcePrimaryMetadataLevel: sourcePrimaryMetadataLevel,
        referencePrimaryMetadata: referencePrimaryMetadata,
        primaryOrientationOverride: primaryOrientationOverride,
        customRendered: exifValue,
        appleMakerNoteVersion: appleMakerNoteVersion,
        appleImageCaptureType: appleImageCaptureType,
        appleFeatureFlags: appleFeatureFlags
    )
    CGImageDestinationAddImageFromSource(destination, source, index, options as CFDictionary)
    CGImageDestinationAddAuxiliaryDataInfo(
        destination,
        kCGImageAuxiliaryDataTypeDisparity,
        auxiliaryInfo as CFDictionary
    )
    if let referenceHDRGainMapInfo {
        CGImageDestinationAddAuxiliaryDataInfo(
            destination,
            kCGImageAuxiliaryDataTypeHDRGainMap,
            referenceHDRGainMapInfo as CFDictionary
        )
    }
    guard CGImageDestinationFinalize(destination) else {
        throw ToolError.unableToFinalize(outputURL)
    }
}

private func canonicalizeAppleDisparityAuxSubtype(at url: URL) throws {
    let noncanonical = Data([0x4e, 0x01, 0xb1, 0x09, 0x35, 0x1f, 0x7b, 0x34, 0x01, 0x0b, 0xc4, 0xd0, 0x20])
    let canonical = Data([0x4e, 0x01, 0xb1, 0x09, 0x35, 0x1f, 0x7b, 0x4a, 0x01, 0x0b, 0xc4, 0xd0, 0x20])
    var data = try Data(contentsOf: url)
    guard let range = data.range(of: noncanonical) else { return }
    guard data.range(of: noncanonical, in: range.upperBound..<data.endIndex) == nil else {
        throw ToolError.verificationFailed("multiple noncanonical disparity aux subtype payloads")
    }
    data.replaceSubrange(range, with: canonical)
    try data.write(to: url, options: .atomic)
}

private func integerValue(_ dictionary: [AnyHashable: Any], _ key: CFString) -> Int? {
    if let number = dictionary[dictionaryKey(key)] as? NSNumber { return number.intValue }
    if let value = dictionary[dictionaryKey(key)] as? Int { return value }
    return nil
}

private func floatRange(pixelBuffer: CVPixelBuffer) -> (Float, Float) {
    CVPixelBufferLockBaseAddress(pixelBuffer, .readOnly)
    defer { CVPixelBufferUnlockBaseAddress(pixelBuffer, .readOnly) }
    guard let base = CVPixelBufferGetBaseAddress(pixelBuffer) else { return (.nan, .nan) }
    let width = CVPixelBufferGetWidth(pixelBuffer)
    let height = CVPixelBufferGetHeight(pixelBuffer)
    let rowStride = CVPixelBufferGetBytesPerRow(pixelBuffer) / MemoryLayout<UInt16>.stride
    let values = base.assumingMemoryBound(to: UInt16.self)
    var minimum = Float.greatestFiniteMagnitude
    var maximum = -Float.greatestFiniteMagnitude
    for y in 0..<height {
        for x in 0..<width {
            let value = Float(Float16(bitPattern: UInt16(littleEndian: values[y * rowStride + x])))
            if value.isFinite {
                minimum = min(minimum, value)
                maximum = max(maximum, value)
            }
        }
    }
    return (minimum, maximum)
}

private func verifyDisparityRoundTrip(
    url: URL,
    expectedData: Data,
    width: Int,
    height: Int
) throws -> (meanAbsoluteError: Double, maximumAbsoluteError: Float) {
    guard let source = CGImageSourceCreateWithURL(url as CFURL, nil),
          let info = auxiliaryInfo(source: source, type: kCGImageAuxiliaryDataTypeDisparity) else {
        throw ToolError.missingDisparity(url)
    }
    let depth: AVDepthData
    do {
        depth = try AVDepthData(fromDictionaryRepresentation: info)
    } catch {
        throw ToolError.verificationFailed("AVDepthData rejected round-trip data: \(error.localizedDescription)")
    }
    let pixelBuffer = depth.depthDataMap
    guard CVPixelBufferGetWidth(pixelBuffer) == width, CVPixelBufferGetHeight(pixelBuffer) == height else {
        throw ToolError.verificationFailed("round-trip disparity dimensions changed")
    }
    guard expectedData.count == width * height * MemoryLayout<UInt16>.stride else {
        throw ToolError.verificationFailed("internal expected disparity length mismatch")
    }

    CVPixelBufferLockBaseAddress(pixelBuffer, .readOnly)
    defer { CVPixelBufferUnlockBaseAddress(pixelBuffer, .readOnly) }
    guard let actualBase = CVPixelBufferGetBaseAddress(pixelBuffer) else {
        throw ToolError.verificationFailed("round-trip disparity has no base address")
    }
    let actualStride = CVPixelBufferGetBytesPerRow(pixelBuffer) / MemoryLayout<UInt16>.stride
    let actual = actualBase.assumingMemoryBound(to: UInt16.self)
    var totalError = 0.0
    var maximumError: Float = 0
    expectedData.withUnsafeBytes { expectedBuffer in
        let expected = expectedBuffer.bindMemory(to: UInt16.self)
        for y in 0..<height {
            for x in 0..<width {
                let expectedValue = Float(Float16(bitPattern: UInt16(littleEndian: expected[y * width + x])))
                let actualValue = Float(Float16(bitPattern: UInt16(littleEndian: actual[y * actualStride + x])))
                let error = abs(expectedValue - actualValue)
                totalError += Double(error)
                maximumError = max(maximumError, error)
            }
        }
    }
    let meanError = totalError / Double(width * height)
    guard meanError <= 0.02, maximumError <= 0.08 else {
        throw ToolError.verificationFailed(
            "disparity round-trip error too large (MAE=\(meanError), max=\(maximumError))"
        )
    }
    return (meanError, maximumError)
}

private func inspect(_ url: URL) throws -> Inspection {
    guard let source = CGImageSourceCreateWithURL(url as CFURL, nil) else {
        throw ToolError.unableToCreateImageSource(url)
    }
    let index = CGImageSourceGetPrimaryImageIndex(source)
    guard let info = auxiliaryInfo(source: source, type: kCGImageAuxiliaryDataTypeDisparity) else {
        throw ToolError.verificationFailed(
            "no disparity auxiliary image (\(auxiliaryProbeSummary(source: source, type: kCGImageAuxiliaryDataTypeDisparity))): \(url.path)"
        )
    }
    guard let rawDescription = info[dictionaryKey(kCGImageAuxiliaryDataInfoDataDescription)] else {
        throw ToolError.malformedAuxiliaryData("missing data description; keys=\(info.keys.map(String.init(describing:)).sorted())")
    }
    guard let descriptionDictionary = rawDescription as? NSDictionary else {
        throw ToolError.malformedAuxiliaryData("data description is not a dictionary")
    }
    var description: [AnyHashable: Any] = [:]
    for key in descriptionDictionary.allKeys {
        guard let stringKey = key as? String else { continue }
        description[AnyHashable(stringKey)] = descriptionDictionary[key]
    }
    let depth: AVDepthData
    do {
        depth = try AVDepthData(fromDictionaryRepresentation: info)
    } catch {
        throw ToolError.verificationFailed("AVDepthData rejected the output: \(error.localizedDescription)")
    }
    let metadata = imageMetadata(info[dictionaryKey(kCGImageAuxiliaryDataInfoMetadata)])
    let properties = CGImageSourceCopyPropertiesAtIndex(source, index, nil) as? [CFString: Any]
    let exif = properties?[kCGImagePropertyExifDictionary] as? [AnyHashable: Any]
    let tiff = properties?[kCGImagePropertyTIFFDictionary] as? [AnyHashable: Any]
    let makerApple = properties?[kCGImagePropertyMakerAppleDictionary] as? NSDictionary
    let range = floatRange(pixelBuffer: depth.depthDataMap)
    let hasGainMap = auxiliaryInfo(source: source, type: kCGImageAuxiliaryDataTypeHDRGainMap) != nil
    let hasISOGainMap = auxiliaryInfo(source: source, type: kCGImageAuxiliaryDataTypeISOGainMap) != nil
    return Inspection(
        width: integerValue(description, kCGImagePropertyWidth) ?? CVPixelBufferGetWidth(depth.depthDataMap),
        height: integerValue(description, kCGImagePropertyHeight) ?? CVPixelBufferGetHeight(depth.depthDataMap),
        bytesPerRow: integerValue(description, kCGImagePropertyBytesPerRow) ?? CVPixelBufferGetBytesPerRow(depth.depthDataMap),
        pixelFormat: depth.depthDataType,
        orientation: (description[dictionaryKey(kCGImagePropertyOrientation)] as? NSNumber)?.uint32Value,
        accuracy: depth.depthDataAccuracy,
        quality: depth.depthDataQuality,
        filtered: depth.isDepthDataFiltered,
        minimum: range.0,
        maximum: range.1,
        hasDepthBlurParameters: metadata.flatMap { metadataString($0, path: "depthBlurEffect:RenderingParameters") } != nil,
        simulatedAperture: metadata.flatMap { metadataString($0, path: "depthBlurEffect:SimulatedAperture") },
        portraitLightingStrength: metadata.flatMap { metadataString($0, path: "portraitLightingEffect:EffectStrength") },
        hasCameraCalibration: depth.cameraCalibrationData != nil,
        primaryMake: tiff?[dictionaryKey(kCGImagePropertyTIFFMake)] as? String,
        primaryModel: tiff?[dictionaryKey(kCGImagePropertyTIFFModel)] as? String,
        makerAppleFieldCount: makerApple?.count ?? 0,
        customRendered: exif.flatMap { integerValue($0, kCGImagePropertyExifCustomRendered) },
        appleMakerNoteVersion: (makerApple?["1"] as? NSNumber)?.intValue,
        appleImageCaptureType: (makerApple?["20"] as? NSNumber)?.intValue,
        appleFeatureFlags: (makerApple?["31"] as? NSNumber)?.intValue,
        hasHDRGainMap: hasGainMap,
        hasISOGainMap: hasISOGainMap
    )
}

private func printInspection(_ inspection: Inspection, url: URL) {
    print("file=\(url.path)")
    print("disparity=\(inspection.width)x\(inspection.height) bytesPerRow=\(inspection.bytesPerRow) pixelFormat=\(fourCC(inspection.pixelFormat))")
    print("range=\(inspection.minimum)...\(inspection.maximum) accuracy=\(inspection.accuracy == .relative ? "relative" : "absolute") quality=\(inspection.quality == .high ? "high" : "low") filtered=\(inspection.filtered)")
    print("orientation=\(inspection.orientation.map(String.init) ?? "none") customRendered=\(inspection.customRendered.map(String.init) ?? "none") hdrGainMap=\(inspection.hasHDRGainMap) isoGainMap=\(inspection.hasISOGainMap)")
    print("primaryMake=\(inspection.primaryMake ?? "none") primaryModel=\(inspection.primaryModel ?? "none") makerAppleFields=\(inspection.makerAppleFieldCount)")
    print("makerAppleVersion=\(inspection.appleMakerNoteVersion.map(String.init) ?? "none") imageCaptureType=\(inspection.appleImageCaptureType.map(String.init) ?? "none") photosAppFeatureFlags=\(inspection.appleFeatureFlags.map(String.init) ?? "none")")
    print("depthBlurParameters=\(inspection.hasDepthBlurParameters) aperture=\(inspection.simulatedAperture ?? "none") portraitLightingStrength=\(inspection.portraitLightingStrength ?? "none") cameraCalibration=\(inspection.hasCameraCalibration)")
}

private func convert(_ options: ConvertOptions) throws {
    try requireExistingFile(options.inputURL)
    try requireExistingFile(options.depthURL)
    if let referenceURL = options.referenceURL { try requireExistingFile(referenceURL) }
    if let sourcePrimaryMetadataURL = options.sourcePrimaryMetadataURL {
        try requireExistingFile(sourcePrimaryMetadataURL)
    }
    if FileManager.default.fileExists(atPath: options.outputURL.path) {
        guard options.overwrite else { throw ToolError.outputExists(options.outputURL) }
        try FileManager.default.removeItem(at: options.outputURL)
    }
    try FileManager.default.createDirectory(
        at: options.outputURL.deletingLastPathComponent(),
        withIntermediateDirectories: true
    )
    guard let ranks = try? Data(contentsOf: options.depthURL, options: .mappedIfSafe) else {
        throw ToolError.unableToRead(options.depthURL)
    }
    let expectedLength = options.depthWidth * options.depthHeight
    guard ranks.count == expectedLength else {
        throw ToolError.invalidDepthLength(expected: expectedLength, actual: ranks.count)
    }
    guard let source = CGImageSourceCreateWithURL(options.inputURL as CFURL, nil) else {
        throw ToolError.unableToCreateImageSource(options.inputURL)
    }
    let index = CGImageSourceGetPrimaryImageIndex(source)
    let orientation = resolvedDepthOrientation(
        source: source,
        index: index,
        depthWidth: options.depthWidth,
        depthHeight: options.depthHeight,
        override: options.orientationOverride
    )
    let orientedRanks = try applyExifOrientation(
        ranks,
        width: options.depthWidth,
        height: options.depthHeight,
        orientation: orientation
    )
    let disparity = makeDisparityData(
        ranks: orientedRanks.data,
        rankNear: options.rankNear,
        far: options.farDisparity,
        near: options.nearDisparity
    )
    var disparityInfo = try makeCanonicalAuxiliaryInfo(
        data: disparity,
        width: orientedRanks.width,
        height: orientedRanks.height,
        orientation: options.auxiliaryOrientation
    )
    if options.profile == .portrait, let referenceURL = options.referenceURL {
        try addPortraitMetadata(
            to: &disparityInfo,
            referenceURL: referenceURL,
            mode: options.referenceDepthMetadata
        )
    }
    var referenceProperties: [CFString: Any]?
    var referencePrimaryXMP: CGImageMetadata?
    var referenceHDRGainMapInfo: [AnyHashable: Any]?
    var sourcePrimaryProperties: [CFString: Any]?
    if (options.referencePrimaryMetadata != .none || options.copyReferencePrimaryXMP || options.copyReferenceHDRGainMap),
       let referenceURL = options.referenceURL {
        guard let referenceSource = CGImageSourceCreateWithURL(referenceURL as CFURL, nil) else {
            throw ToolError.unableToCreateImageSource(referenceURL)
        }
        let referenceIndex = CGImageSourceGetPrimaryImageIndex(referenceSource)
        referenceProperties = CGImageSourceCopyPropertiesAtIndex(
            referenceSource,
            referenceIndex,
            nil
        ) as? [CFString: Any]
        if options.copyReferencePrimaryXMP {
            referencePrimaryXMP = CGImageSourceCopyMetadataAtIndex(
                referenceSource,
                referenceIndex,
                nil
            )
        }
        if options.copyReferenceHDRGainMap {
            guard let gainMap = auxiliaryInfo(
                source: referenceSource,
                type: kCGImageAuxiliaryDataTypeHDRGainMap
            ) else {
                throw ToolError.verificationFailed("reference has no HDR gain map")
            }
            referenceHDRGainMapInfo = gainMap
        }
    }
    if let sourcePrimaryMetadataURL = options.sourcePrimaryMetadataURL {
        guard let metadataSource = CGImageSourceCreateWithURL(sourcePrimaryMetadataURL as CFURL, nil) else {
            throw ToolError.unableToCreateImageSource(sourcePrimaryMetadataURL)
        }
        sourcePrimaryProperties = CGImageSourceCopyPropertiesAtIndex(
            metadataSource,
            CGImageSourceGetPrimaryImageIndex(metadataSource),
            nil
        ) as? [CFString: Any]
    }
    try writeHEIC(
        source: source,
        auxiliaryInfo: disparityInfo,
        referenceProperties: referenceProperties,
        referencePrimaryXMP: referencePrimaryXMP,
        sourcePrimaryProperties: sourcePrimaryProperties,
        sourcePrimaryMetadataLevel: options.sourcePrimaryMetadataLevel,
        referencePrimaryMetadata: options.referencePrimaryMetadata,
        primaryOrientationOverride: options.primaryOrientationOverride,
        referenceHDRGainMapInfo: referenceHDRGainMapInfo,
        profile: options.profile,
        customRendered: options.customRendered,
        appleMakerNoteVersion: options.appleMakerNoteVersion,
        appleImageCaptureType: options.appleImageCaptureType,
        appleFeatureFlags: options.appleFeatureFlags,
        outputURL: options.outputURL
    )
    try canonicalizeAppleDisparityAuxSubtype(at: options.outputURL)
    let inspection = try inspect(options.outputURL)
    guard inspection.width == orientedRanks.width, inspection.height == orientedRanks.height else {
        throw ToolError.verificationFailed(
            "expected \(orientedRanks.width)x\(orientedRanks.height), got \(inspection.width)x\(inspection.height)"
        )
    }
    guard inspection.pixelFormat == kCVPixelFormatType_DisparityFloat16 else {
        throw ToolError.verificationFailed("expected hdis, got \(fourCC(inspection.pixelFormat))")
    }
    guard inspection.accuracy == .relative else {
        throw ToolError.verificationFailed("expected relative accuracy")
    }
    guard inspection.orientation == options.auxiliaryOrientation else {
        throw ToolError.verificationFailed(
            "expected auxiliary orientation \(options.auxiliaryOrientation), got \(inspection.orientation.map(String.init) ?? "none")"
        )
    }
    if options.profile == .portrait {
        guard inspection.hasDepthBlurParameters else {
            throw ToolError.verificationFailed("portrait profile lost depthBlurEffect metadata")
        }
        guard inspection.customRendered == options.customRendered else {
            throw ToolError.verificationFailed("portrait profile lost CustomRendered")
        }
    if options.referenceDepthMetadata == .full, !inspection.hasCameraCalibration {
            throw ToolError.verificationFailed("full reference depth metadata lost camera calibration")
        }
    }
    if options.referencePrimaryMetadata != .none, inspection.makerAppleFieldCount <= 3 {
        throw ToolError.verificationFailed("reference MakerApple dictionary did not round-trip")
    }
    if options.referencePrimaryMetadata == .full {
        let sourceTIFF = sourcePrimaryProperties?[kCGImagePropertyTIFFDictionary] as? [CFString: Any]
        let expectedMake: String?
        let expectedModel: String?
        switch options.sourcePrimaryMetadataLevel {
        case .none:
            let referenceTIFF = referenceProperties?[kCGImagePropertyTIFFDictionary] as? [CFString: Any]
            expectedMake = referenceTIFF?[kCGImagePropertyTIFFMake] as? String
            expectedModel = referenceTIFF?[kCGImagePropertyTIFFModel] as? String
        case .model:
            expectedMake = "Apple"
            expectedModel = sourceTIFF?[kCGImagePropertyTIFFModel] as? String
        case .camera, .exposure, .exif, .all:
            expectedMake = sourceTIFF?[kCGImagePropertyTIFFMake] as? String
            expectedModel = sourceTIFF?[kCGImagePropertyTIFFModel] as? String
        }
        if inspection.primaryMake != expectedMake || inspection.primaryModel != expectedModel {
            throw ToolError.verificationFailed("primary camera identity did not round-trip")
        }
    }
    if options.copyReferenceHDRGainMap, !inspection.hasHDRGainMap {
        throw ToolError.verificationFailed("reference HDR gain map did not round-trip")
    }
    if let expected = options.appleMakerNoteVersion,
       inspection.appleMakerNoteVersion != expected {
        throw ToolError.verificationFailed("MakerApple version did not round-trip")
    }
    if let expected = options.appleImageCaptureType,
       inspection.appleImageCaptureType != expected {
        throw ToolError.verificationFailed("MakerApple image capture type did not round-trip")
    }
    if let expected = options.appleFeatureFlags,
       inspection.appleFeatureFlags != expected {
        throw ToolError.verificationFailed("MakerApple feature flags did not round-trip")
    }
    printInspection(inspection, url: options.outputURL)
    let roundTrip = try verifyDisparityRoundTrip(
        url: options.outputURL,
        expectedData: disparity,
        width: orientedRanks.width,
        height: orientedRanks.height
    )
    print("roundTripMAE=\(roundTrip.meanAbsoluteError) roundTripMaxError=\(roundTrip.maximumAbsoluteError)")
}

private func repack(_ options: RepackOptions) throws {
    try requireExistingFile(options.inputURL)
    if FileManager.default.fileExists(atPath: options.outputURL.path) {
        guard options.overwrite else { throw ToolError.outputExists(options.outputURL) }
        try FileManager.default.removeItem(at: options.outputURL)
    }
    try FileManager.default.createDirectory(
        at: options.outputURL.deletingLastPathComponent(),
        withIntermediateDirectories: true
    )
    guard let source = CGImageSourceCreateWithURL(options.inputURL as CFURL, nil) else {
        throw ToolError.unableToCreateImageSource(options.inputURL)
    }
    guard let info = auxiliaryInfo(source: source, type: kCGImageAuxiliaryDataTypeDisparity) else {
        throw ToolError.missingDisparity(options.inputURL)
    }
    let before = try inspect(options.inputURL)
    try writeHEIC(
        source: source,
        auxiliaryInfo: info,
        referenceProperties: nil,
        referencePrimaryXMP: nil,
        sourcePrimaryProperties: nil,
        sourcePrimaryMetadataLevel: .none,
        referencePrimaryMetadata: .none,
        primaryOrientationOverride: nil,
        referenceHDRGainMapInfo: nil,
        profile: .publicDepth,
        customRendered: 9,
        appleMakerNoteVersion: nil,
        appleImageCaptureType: nil,
        appleFeatureFlags: nil,
        outputURL: options.outputURL
    )
    let after = try inspect(options.outputURL)
    guard before.width == after.width,
          before.height == after.height,
          before.pixelFormat == after.pixelFormat,
          before.orientation == after.orientation,
          before.hasCameraCalibration == after.hasCameraCalibration,
          before.hasDepthBlurParameters == after.hasDepthBlurParameters else {
        throw ToolError.verificationFailed("reference disparity contract changed during repack")
    }
    printInspection(after, url: options.outputURL)
}

private func run() throws {
    let arguments = CommandLine.arguments.dropFirst()
    guard let command = arguments.first else { throw ToolError.usage(usage) }
    switch command {
    case "convert":
        try convert(parseConvertOptions(arguments.dropFirst()))
    case "repack":
        try repack(parseRepackOptions(arguments.dropFirst()))
    case "inspect":
        let values = try parseOptions(arguments.dropFirst())
        let known: Set<String> = ["--input"]
        if let unknown = values.keys.first(where: { !known.contains($0) }) {
            throw ToolError.unknownOption(unknown)
        }
        let url = fileURL(try required("--input", in: values))
        try requireExistingFile(url)
        printInspection(try inspect(url), url: url)
    case "help", "--help", "-h":
        print(usage)
    default:
        throw ToolError.usage(usage)
    }
}

do {
    try run()
} catch {
    fputs("error: \(error)\n", stderr)
    exit(2)
}
