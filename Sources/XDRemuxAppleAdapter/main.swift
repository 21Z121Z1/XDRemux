import Foundation
import ImageIO
import XDRemuxAppleFeatures

private let schemaVersion = 1

private struct AdapterRequest: Decodable {
    let schemaVersion: Int
    let operation: String
    let inputPath: String?

    enum CodingKeys: String, CodingKey {
        case schemaVersion = "schema_version"
        case operation
        case inputPath = "input_path"
    }
}

private struct AuxiliaryFacts: Encodable {
    let isoGainMap: Bool
    let disparity: Bool
    let portraitEffectsMatte: Bool
    let skinMatte: Bool
    let hairMatte: Bool
    let teethMatte: Bool
    let glassesMatte: Bool

    enum CodingKeys: String, CodingKey {
        case isoGainMap = "iso_gain_map"
        case disparity
        case portraitEffectsMatte = "portrait_effects_matte"
        case skinMatte = "skin_matte"
        case hairMatte = "hair_matte"
        case teethMatte = "teeth_matte"
        case glassesMatte = "glasses_matte"
    }
}

private struct GainMapFacts: Encodable {
    let pixelFormat: UInt32
    let width: Int
    let height: Int

    enum CodingKeys: String, CodingKey {
        case pixelFormat = "pixel_format"
        case width
        case height
    }
}

private struct AdapterResponse: Encodable {
    let schemaVersion: Int
    let capabilities: [String]?
    let auxiliary: AuxiliaryFacts?
    let gainMap: GainMapFacts?

    enum CodingKeys: String, CodingKey {
        case schemaVersion = "schema_version"
        case capabilities
        case auxiliary
        case gainMap = "gain_map"
    }
}

private func fail(_ message: String, status: Int32 = 2) -> Never {
    FileHandle.standardError.write(Data((message + "\n").utf8))
    exit(status)
}

private func hasAuxiliary(_ type: CFString, source: CGImageSource) -> Bool {
    CGImageSourceCopyAuxiliaryDataInfoAtIndex(source, 0, type) != nil
}

private func imageIOAuxiliaryFacts(inputPath: String) -> AuxiliaryFacts {
    let inputURL = URL(fileURLWithPath: inputPath)
    guard let source = CGImageSourceCreateWithURL(inputURL as CFURL, nil) else {
        fail("ImageIO cannot open input \(inputPath)", status: 1)
    }
    return AuxiliaryFacts(
        isoGainMap: hasAuxiliary(kCGImageAuxiliaryDataTypeISOGainMap, source: source),
        disparity: hasAuxiliary(kCGImageAuxiliaryDataTypeDisparity, source: source),
        portraitEffectsMatte: hasAuxiliary(kCGImageAuxiliaryDataTypePortraitEffectsMatte, source: source),
        skinMatte: hasAuxiliary(kCGImageAuxiliaryDataTypeSemanticSegmentationSkinMatte, source: source),
        hairMatte: hasAuxiliary(kCGImageAuxiliaryDataTypeSemanticSegmentationHairMatte, source: source),
        teethMatte: hasAuxiliary(kCGImageAuxiliaryDataTypeSemanticSegmentationTeethMatte, source: source),
        glassesMatte: hasAuxiliary(kCGImageAuxiliaryDataTypeSemanticSegmentationGlassesMatte, source: source)
    )
}

private func fourCC(_ string: String) -> UInt32? {
    let bytes = Array(string.utf8)
    guard bytes.count == 4 else { return nil }
    return bytes.reduce(UInt32(0)) { ($0 << 8) | UInt32($1) }
}

private func pixelFormat(_ value: Any?) -> UInt32? {
    if let number = value as? NSNumber {
        return number.uint32Value
    }
    if let string = value as? String {
        return fourCC(string)
    }
    return nil
}

private func imageIOGainMapFacts(inputPath: String) -> GainMapFacts {
    let inputURL = URL(fileURLWithPath: inputPath)
    guard let source = CGImageSourceCreateWithURL(inputURL as CFURL, nil),
          CGImageSourceCreateImageAtIndex(source, 0, nil) != nil,
          let auxiliary = CGImageSourceCopyAuxiliaryDataInfoAtIndex(
              source,
              0,
              kCGImageAuxiliaryDataTypeISOGainMap
          ) as? [CFString: Any],
          let description = auxiliary[kCGImageAuxiliaryDataInfoDataDescription] as? [CFString: Any],
          let rawPixelFormat = pixelFormat(description[kCGImagePropertyPixelFormat]),
          let width = (description[kCGImagePropertyWidth] as? NSNumber)?.intValue,
          let height = (description[kCGImagePropertyHeight] as? NSNumber)?.intValue,
          width > 0,
          height > 0 else {
        fail("ImageIO cannot read ISO Gain Map facts from \(inputPath)", status: 1)
    }
    return GainMapFacts(pixelFormat: rawPixelFormat, width: width, height: height)
}

do {
    let input = FileHandle.standardInput.readDataToEndOfFile()
    guard !input.isEmpty else {
        fail("apple adapter request is empty")
    }
    let request = try JSONDecoder().decode(AdapterRequest.self, from: input)
    guard request.schemaVersion == schemaVersion else {
        fail("unsupported apple adapter schema_version \(request.schemaVersion)")
    }

    let response: AdapterResponse
    switch request.operation {
    case "capabilities":
        // This target links XDRemuxAppleFeatures. Advertising these facts does
        // not choose a conversion path; the Rust engine remains policy owner.
        response = AdapterResponse(
            schemaVersion: schemaVersion,
            capabilities: ["photographic-styles", "portrait"],
            auxiliary: nil,
            gainMap: nil
        )
    case "imageio-auxiliary-facts":
        guard let inputPath = request.inputPath, !inputPath.isEmpty else {
            fail("imageio-auxiliary-facts requires input_path")
        }
        response = AdapterResponse(
            schemaVersion: schemaVersion,
            capabilities: nil,
            auxiliary: imageIOAuxiliaryFacts(inputPath: inputPath),
            gainMap: nil
        )
    case "imageio-gain-map-facts":
        guard let inputPath = request.inputPath, !inputPath.isEmpty else {
            fail("imageio-gain-map-facts requires input_path")
        }
        response = AdapterResponse(
            schemaVersion: schemaVersion,
            capabilities: nil,
            auxiliary: nil,
            gainMap: imageIOGainMapFacts(inputPath: inputPath)
        )
    default:
        fail("unsupported apple adapter operation \(request.operation)")
    }

    var encoded = try JSONEncoder().encode(response)
    encoded.append(0x0A)
    FileHandle.standardOutput.write(encoded)
} catch {
    fail("invalid apple adapter request: \(error)")
}
