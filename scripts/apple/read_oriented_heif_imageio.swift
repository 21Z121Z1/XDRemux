import CoreGraphics
import Foundation
import ImageIO

struct Expected: Decodable {
    let primary_coded_width: Int
    let primary_coded_height: Int
    let primary_display_width: Int
    let primary_display_height: Int
    let quarter_turns_ccw: Int
    let gain_coded_width: Int
    let gain_coded_height: Int
    let gain_display_width: Int
    let gain_display_height: Int
}

struct Report: Encodable {
    let primary_width: Int
    let primary_height: Int
    let orientation: UInt32?
    let gain_width: Int
    let gain_height: Int
    let primary_space: String
    let gain_space: String
}

func fail(_ message: String) -> Never {
    fputs("\(message)\n", stderr)
    exit(1)
}

let arguments = CommandLine.arguments
guard arguments.count == 3 else {
    fail("usage: read_oriented_heif_imageio.swift <heic> <expected.json>")
}

let heicURL = URL(fileURLWithPath: arguments[1])
let expectedURL = URL(fileURLWithPath: arguments[2])
let expectedData = try Data(contentsOf: expectedURL)
let expected = try JSONDecoder().decode(Expected.self, from: expectedData)

guard let source = CGImageSourceCreateWithURL(heicURL as CFURL, nil),
      let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any],
      let primaryWidth = (properties[kCGImagePropertyPixelWidth] as? NSNumber)?.intValue,
      let primaryHeight = (properties[kCGImagePropertyPixelHeight] as? NSNumber)?.intValue,
      primaryWidth > 0,
      primaryHeight > 0
else {
    fail("ImageIO cannot read primary properties from \(heicURL.path)")
}

guard let auxiliary = CGImageSourceCopyAuxiliaryDataInfoAtIndex(
    source,
    0,
    kCGImageAuxiliaryDataTypeISOGainMap
) as? [CFString: Any],
      let description = auxiliary[kCGImageAuxiliaryDataInfoDataDescription] as? [CFString: Any],
      let gainWidth = (description[kCGImagePropertyWidth] as? NSNumber)?.intValue,
      let gainHeight = (description[kCGImagePropertyHeight] as? NSNumber)?.intValue,
      gainWidth > 0,
      gainHeight > 0
else {
    fail("ImageIO cannot read an ISO gain map from \(heicURL.path)")
}

func space(width: Int, height: Int, coded: (Int, Int), display: (Int, Int)) -> String? {
    if width == coded.0 && height == coded.1 {
        return "coded"
    }
    if width == display.0 && height == display.1 {
        return "display"
    }
    return nil
}

guard let primarySpace = space(
    width: primaryWidth,
    height: primaryHeight,
    coded: (expected.primary_coded_width, expected.primary_coded_height),
    display: (expected.primary_display_width, expected.primary_display_height)
) else {
    fail("ImageIO primary \(primaryWidth)x\(primaryHeight) is neither coded nor display size")
}
guard let gainSpace = space(
    width: gainWidth,
    height: gainHeight,
    coded: (expected.gain_coded_width, expected.gain_coded_height),
    display: (expected.gain_display_width, expected.gain_display_height)
) else {
    fail("ImageIO gain map \(gainWidth)x\(gainHeight) is neither coded nor display size")
}
if primarySpace != gainSpace {
    fail(
        "ImageIO reports primary as \(primarySpace) \(primaryWidth)x\(primaryHeight) and gain map as \(gainSpace) \(gainWidth)x\(gainHeight)"
    )
}

let orientation = (properties[kCGImagePropertyOrientation] as? NSNumber)?.uint32Value
let allowedOrientations: Set<UInt32> = expected.quarter_turns_ccw == 0 ? [1] : [1, 6]
if let orientation, !allowedOrientations.contains(orientation) {
    fail("ImageIO orientation \(orientation) does not match irot \(expected.quarter_turns_ccw)")
}

let report = Report(
    primary_width: primaryWidth,
    primary_height: primaryHeight,
    orientation: orientation,
    gain_width: gainWidth,
    gain_height: gainHeight,
    primary_space: primarySpace,
    gain_space: gainSpace
)
let encoder = JSONEncoder()
encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
let rendered = try encoder.encode(report)
FileHandle.standardOutput.write(rendered)
FileHandle.standardOutput.write(Data("\n".utf8))
