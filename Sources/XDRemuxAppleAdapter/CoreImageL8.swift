import CoreGraphics
import CoreImage
import CoreImage.CIFilterBuiltins
import Foundation
import ImageIO

private func tightL8ByteCount(width: Int, height: Int, context: String) throws -> Int {
    guard width > 0, height > 0 else {
        throw NSError(
            domain: "XDRemuxAppleAdapter",
            code: 20,
            userInfo: [NSLocalizedDescriptionKey: "\(context) geometry is invalid"]
        )
    }
    let (byteCount, overflow) = width.multipliedReportingOverflow(by: height)
    guard !overflow else {
        throw NSError(
            domain: "XDRemuxAppleAdapter",
            code: 21,
            userInfo: [NSLocalizedDescriptionKey: "\(context) geometry overflows"]
        )
    }
    return byteCount
}

private func tightL8Image(
    path: String,
    width: Int,
    height: Int,
    context: String
) throws -> CIImage {
    let expected = try tightL8ByteCount(width: width, height: height, context: context)
    let pixels = try Data(
        contentsOf: URL(fileURLWithPath: path),
        options: [.mappedIfSafe]
    )
    guard pixels.count == expected else {
        throw NSError(
            domain: "XDRemuxAppleAdapter",
            code: 22,
            userInfo: [
                NSLocalizedDescriptionKey:
                    "\(context) has \(pixels.count) bytes; expected \(expected)"
            ]
        )
    }
    return CIImage(
        bitmapData: pixels,
        bytesPerRow: width,
        size: CGSize(width: width, height: height),
        format: .L8,
        colorSpace: nil
    )
}

private func writeTightL8(
    _ image: CIImage,
    outputPath: String,
    width: Int,
    height: Int,
    context: String
) throws {
    let byteCount = try tightL8ByteCount(width: width, height: height, context: context)
    let bounds = CGRect(x: 0, y: 0, width: width, height: height)
    var rendered = Data(count: byteCount)
    let coreImageContext = CIContext(options: [.cacheIntermediates: false])
    rendered.withUnsafeMutableBytes { bytes in
        guard let baseAddress = bytes.baseAddress else { return }
        coreImageContext.render(
            image.cropped(to: bounds),
            toBitmap: baseAddress,
            rowBytes: width,
            bounds: bounds,
            format: .L8,
            colorSpace: nil
        )
    }
    try rendered.write(to: URL(fileURLWithPath: outputPath), options: .atomic)
}

private func storedOrientation(_ image: CIImage, orientationRaw: UInt32) -> CIImage {
    // Preserve the legacy producer transform exactly. The Rust side owns which
    // orientation value applies; this helper only executes the Core Image pixel
    // transform required by the Apple framework coordinate system.
    switch orientationRaw {
    case 2: return image.oriented(.upMirrored)
    case 3: return image.oriented(.down)
    case 4: return image.oriented(.downMirrored)
    case 5: return image.oriented(.leftMirrored)
    case 6: return image.oriented(.left)
    case 7: return image.oriented(.rightMirrored)
    case 8: return image.oriented(.right)
    default: return image
    }
}

func renderL8(
    maskPath: String,
    outputPath: String,
    sourceWidth: Int,
    sourceHeight: Int,
    targetWidth: Int,
    targetHeight: Int,
    orientationRaw: UInt32
) throws {
    guard (1...8).contains(orientationRaw) else {
        throw NSError(
            domain: "XDRemuxAppleAdapter",
            code: 23,
            userInfo: [NSLocalizedDescriptionKey: "CoreImage L8 orientation must be 1 through 8"]
        )
    }
    let source = try tightL8Image(
        path: maskPath,
        width: sourceWidth,
        height: sourceHeight,
        context: "CoreImage L8 render input"
    )
    _ = try tightL8ByteCount(
        width: targetWidth,
        height: targetHeight,
        context: "CoreImage L8 render output"
    )

    let stored = storedOrientation(source, orientationRaw: orientationRaw)
    let originNormalized = stored.transformed(by: CGAffineTransform(
        translationX: -stored.extent.origin.x,
        y: -stored.extent.origin.y
    ))
    let resized = originNormalized.transformed(by: CGAffineTransform(
        scaleX: CGFloat(targetWidth) / originNormalized.extent.width,
        y: CGFloat(targetHeight) / originNormalized.extent.height
    ))
    try writeTightL8(
        resized,
        outputPath: outputPath,
        width: targetWidth,
        height: targetHeight,
        context: "CoreImage L8 render output"
    )
}

func edgePreserveUpsampleL8(
    guidePath: String,
    smallMaskPath: String,
    outputPath: String,
    smallWidth: Int,
    smallHeight: Int,
    targetWidth: Int,
    targetHeight: Int,
    spatialSigma: Float,
    lumaSigma: Float
) throws {
    guard spatialSigma.isFinite,
          spatialSigma > 0,
          lumaSigma.isFinite,
          lumaSigma > 0 else {
        throw NSError(
            domain: "XDRemuxAppleAdapter",
            code: 24,
            userInfo: [NSLocalizedDescriptionKey: "CoreImage L8 upsample sigma is invalid"]
        )
    }

    let small = try tightL8Image(
        path: smallMaskPath,
        width: smallWidth,
        height: smallHeight,
        context: "CoreImage L8 upsample input"
    )
    _ = try tightL8ByteCount(
        width: targetWidth,
        height: targetHeight,
        context: "CoreImage L8 upsample output"
    )

    let guideURL = URL(fileURLWithPath: guidePath)
    guard let source = CGImageSourceCreateWithURL(guideURL as CFURL, nil),
          let guideImage = CGImageSourceCreateImageAtIndex(source, 0, nil) else {
        throw NSError(
            domain: "XDRemuxAppleAdapter",
            code: 25,
            userInfo: [NSLocalizedDescriptionKey: "CoreImage cannot decode guide \(guidePath)"]
        )
    }

    let bounds = CGRect(x: 0, y: 0, width: targetWidth, height: targetHeight)
    let sourceGuide = CIImage(cgImage: guideImage)
    let guide = sourceGuide.transformed(by: CGAffineTransform(
        scaleX: CGFloat(targetWidth) / sourceGuide.extent.width,
        y: CGFloat(targetHeight) / sourceGuide.extent.height
    )).cropped(to: bounds)

    let filter = CIFilter.edgePreserveUpsample()
    filter.inputImage = guide
    filter.smallImage = small
    filter.spatialSigma = spatialSigma
    filter.lumaSigma = lumaSigma
    guard let output = filter.outputImage else {
        throw NSError(
            domain: "XDRemuxAppleAdapter",
            code: 26,
            userInfo: [NSLocalizedDescriptionKey: "CoreImage edge-preserve upsample produced no image"]
        )
    }

    try writeTightL8(
        output,
        outputPath: outputPath,
        width: targetWidth,
        height: targetHeight,
        context: "CoreImage L8 upsample output"
    )
}
