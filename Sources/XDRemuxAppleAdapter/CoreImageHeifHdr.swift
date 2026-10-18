import CoreGraphics
import CoreImage
import Darwin
import Foundation
import ImageIO

// Core Image calculates the gain map. These symbols are `extern const CFStringRef`
// in the macOS 27 SDK. A direct Swift reference to the Core Image constants did
// not compile in the header probe, so the adapter loads the C export and fails
// closed when the process does not provide it.

struct SystemHeifHdrFacts: Encodable {
    let width: UInt32
    let height: UInt32
    let contentHeadroomSent: Float?
    let decodedContentHeadroom: Float
    let decodedWidth: UInt32
    let decodedHeight: UInt32
    let readbackByteCount: Int
    let gainMapAsRgb: Bool?
    let appleHdrGainMap: AuxiliaryPlaneFacts
    let isoGainMap: AuxiliaryPlaneFacts

    enum CodingKeys: String, CodingKey {
        case width
        case height
        case contentHeadroomSent = "content_headroom_sent"
        case decodedContentHeadroom = "decoded_content_headroom"
        case decodedWidth = "decoded_width"
        case decodedHeight = "decoded_height"
        case readbackByteCount = "readback_byte_count"
        case gainMapAsRgb = "gain_map_as_rgb"
        case appleHdrGainMap = "apple_hdr_gain_map"
        case isoGainMap = "iso_gain_map"
    }
}

struct AuxiliaryPlaneFacts: Encodable {
    let present: Bool
    let pixelFormat: UInt32?
    let width: UInt32?
    let height: UInt32?

    enum CodingKeys: String, CodingKey {
        case present
        case pixelFormat = "pixel_format"
        case width
        case height
    }
}

func writeSystemHeifHdr(
    outputPath: String,
    sdrPath: String,
    hdrPath: String,
    readbackPath: String,
    width: Int,
    height: Int,
    contentHeadroom: Float,
    gainMapAsRgb: Bool
) throws -> SystemHeifHdrFacts {
    let sdr = try bitmapImage(
        path: sdrPath,
        width: width,
        height: height,
        bytesPerPixel: 4,
        format: .RGBA8,
        colorSpaceName: CGColorSpace.displayP3,
        context: "system HEIF SDR"
    )
    var hdr = try bitmapImage(
        path: hdrPath,
        width: width,
        height: height,
        bytesPerPixel: 8,
        format: .RGBAh,
        colorSpaceName: CGColorSpace.extendedLinearDisplayP3,
        context: "system HEIF HDR"
    )
    hdr = try imageBySettingHeadroom(hdr, contentHeadroom)
    let hdrKey = try representationOption("kCIImageRepresentationHDRImage")
    var options: [CIImageRepresentationOption: Any] = [hdrKey: hdr]
    if gainMapAsRgb {
        options[try representationOption("kCIImageRepresentationHDRGainMapAsRGB")] = true
    }
    guard let colorSpace = CGColorSpace(name: CGColorSpace.displayP3) else {
        throw adapterError("Display P3 is unavailable")
    }
    try writeAdapterHeifRepresentation(
        of: sdr,
        to: URL(fileURLWithPath: outputPath),
        format: .RGBA8,
        colorSpace: colorSpace,
        options: options
    )
    return try readSystemHeifHdr(
        inputPath: outputPath,
        readbackPath: readbackPath,
        contentHeadroomSent: contentHeadroom,
        gainMapAsRgb: gainMapAsRgb
    )
}

func readSystemHeifHdr(
    inputPath: String,
    readbackPath: String,
    contentHeadroomSent: Float?,
    gainMapAsRgb: Bool?
) throws -> SystemHeifHdrFacts {
    let url = URL(fileURLWithPath: inputPath)
    guard let source = CGImageSourceCreateWithURL(url as CFURL, nil) else {
        throw adapterError("ImageIO cannot open \(inputPath)")
    }
    let options: [CFString: Any] = [
        kCGImageSourceDecodeRequest: kCGImageSourceDecodeToHDR,
        kCGImageSourceGenerateImageSpecificLumaScaling: false,
    ]
    guard let image = CGImageSourceCreateImageAtIndex(source, 0, options as CFDictionary) else {
        throw adapterError("ImageIO cannot decode an HDR rendition from \(inputPath)")
    }
    let rendered = try renderExtendedLinearHalf(image, readbackPath: readbackPath)
    return SystemHeifHdrFacts(
        width: rendered.width,
        height: rendered.height,
        contentHeadroomSent: contentHeadroomSent,
        decodedContentHeadroom: image.contentHeadroom,
        decodedWidth: rendered.width,
        decodedHeight: rendered.height,
        readbackByteCount: rendered.byteCount,
        gainMapAsRgb: gainMapAsRgb,
        appleHdrGainMap: auxiliaryPlane(kCGImageAuxiliaryDataTypeHDRGainMap, source: source),
        isoGainMap: auxiliaryPlane(kCGImageAuxiliaryDataTypeISOGainMap, source: source)
    )
}

private func imageBySettingHeadroom(_ image: CIImage, _ headroom: Float) throws -> CIImage {
    guard headroom == 0 || headroom >= 1 else {
        throw adapterError("content headroom \(headroom) is not 0 and is below 1")
    }
    // CIImage.h declares `-imageBySettingContentHeadroom:` and does not give a
    // Swift overlay name. Call that selector directly.
    let selector = NSSelectorFromString("imageBySettingContentHeadroom:")
    guard #available(macOS 26, *), image.responds(to: selector) else {
        throw adapterError("CIImage imageBySettingContentHeadroom is unavailable")
    }
    typealias Setter = @convention(c) (AnyObject, Selector, Float) -> Unmanaged<CIImage>
    let implementation = unsafeBitCast(image.method(for: selector), to: Setter.self)
    return implementation(image, selector, headroom).takeUnretainedValue()
}

private func representationOption(_ symbol: String) throws -> CIImageRepresentationOption {
    try CIImageRepresentationOption(rawValue: exportedString(symbol))
}

private func exportedString(_ symbol: String) throws -> String {
    // dlfcn.h defines RTLD_DEFAULT as ((void *) -2). The Swift Darwin overlay
    // exports RTLD_NOW and does not export this pointer macro.
    guard let handle = UnsafeMutableRawPointer(bitPattern: -2) else {
        throw adapterError("RTLD_DEFAULT cannot be formed")
    }
    guard let address = symbol.withCString({ name in
        dlsym(handle, name)
    }) else {
        throw adapterError("Core Image export \(symbol) is not available")
    }
    let object = address.load(as: UnsafeRawPointer.self)
    let value = Unmanaged<CFString>.fromOpaque(object).takeUnretainedValue()
    return value as String
}

private func bitmapImage(
    path: String,
    width: Int,
    height: Int,
    bytesPerPixel: Int,
    format: CIFormat,
    colorSpaceName: CFString,
    context: String
) throws -> CIImage {
    guard width > 0, height > 0 else {
        throw adapterError("\(context) geometry is invalid")
    }
    let (rowBytes, rowOverflow) = width.multipliedReportingOverflow(by: bytesPerPixel)
    let (byteCount, countOverflow) = rowBytes.multipliedReportingOverflow(by: height)
    guard !rowOverflow, !countOverflow else {
        throw adapterError("\(context) geometry overflows")
    }
    let pixels = try Data(contentsOf: URL(fileURLWithPath: path), options: [.mappedIfSafe])
    guard pixels.count == byteCount else {
        throw adapterError("\(context) has \(pixels.count) bytes; expected \(byteCount)")
    }
    guard let colorSpace = CGColorSpace(name: colorSpaceName) else {
        throw adapterError("\(context) color space is unavailable")
    }
    return CIImage(
        bitmapData: pixels,
        bytesPerRow: rowBytes,
        size: CGSize(width: width, height: height),
        format: format,
        colorSpace: colorSpace
    )
}

private func renderExtendedLinearHalf(
    _ image: CGImage,
    readbackPath: String
) throws -> (width: UInt32, height: UInt32, byteCount: Int) {
    let source = CIImage(cgImage: image)
    let extent = source.extent
    guard extent.origin == .zero, extent.width.isFinite, extent.height.isFinite else {
        throw adapterError("decoded HDR extent is not finite at the origin")
    }
    let width = Int(extent.width)
    let height = Int(extent.height)
    guard width > 0, height > 0, CGFloat(width) == extent.width, CGFloat(height) == extent.height else {
        throw adapterError("decoded HDR extent is not an integral image")
    }
    guard let storedWidth = UInt32(exactly: width), let storedHeight = UInt32(exactly: height) else {
        throw adapterError("decoded HDR geometry exceeds UInt32")
    }
    guard let colorSpace = CGColorSpace(name: CGColorSpace.extendedLinearDisplayP3) else {
        throw adapterError("extended linear Display P3 is unavailable")
    }
    let rowBytes = width * 8
    let byteCount = rowBytes * height
    var rendered = Data(count: byteCount)
    let wrote = rendered.withUnsafeMutableBytes { bytes -> Bool in
        guard let baseAddress = bytes.baseAddress else { return false }
        renderAdapterBitmap(
            source,
            toBitmap: baseAddress,
            rowBytes: rowBytes,
            bounds: CGRect(x: 0, y: 0, width: width, height: height),
            format: .RGBAh,
            colorSpace: colorSpace
        )
        return true
    }
    guard wrote else {
        throw adapterError("HDR readback bitmap is empty")
    }
    try rendered.write(to: URL(fileURLWithPath: readbackPath), options: .atomic)
    return (storedWidth, storedHeight, byteCount)
}

private func auxiliaryPlane(_ type: CFString, source: CGImageSource) -> AuxiliaryPlaneFacts {
    guard let auxiliary = CGImageSourceCopyAuxiliaryDataInfoAtIndex(source, 0, type) as? [CFString: Any] else {
        return AuxiliaryPlaneFacts(present: false, pixelFormat: nil, width: nil, height: nil)
    }
    let description = auxiliary[kCGImageAuxiliaryDataInfoDataDescription] as? [CFString: Any]
    return AuxiliaryPlaneFacts(
        present: true,
        pixelFormat: description.flatMap { planePixelFormat($0[kCGImagePropertyPixelFormat]) },
        width: description.flatMap { ($0[kCGImagePropertyWidth] as? NSNumber).flatMap { UInt32(exactly: $0.int64Value) } },
        height: description.flatMap { ($0[kCGImagePropertyHeight] as? NSNumber).flatMap { UInt32(exactly: $0.int64Value) } }
    )
}

private func planePixelFormat(_ value: Any?) -> UInt32? {
    if let number = value as? NSNumber {
        return number.uint32Value
    }
    if let string = value as? String {
        let bytes = Array(string.utf8)
        guard bytes.count == 4 else { return nil }
        return bytes.reduce(UInt32(0)) { ($0 << 8) | UInt32($1) }
    }
    return nil
}

private func adapterError(_ message: String) -> NSError {
    NSError(
        domain: "XDRemuxAppleAdapter",
        code: 30,
        userInfo: [NSLocalizedDescriptionKey: message]
    )
}
