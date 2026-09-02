import CoreGraphics
import CoreImage
import CoreImage.CIFilterBuiltins
import Foundation
import ImageIO

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
    guard smallWidth > 0,
          smallHeight > 0,
          targetWidth > 0,
          targetHeight > 0,
          spatialSigma.isFinite,
          spatialSigma > 0,
          lumaSigma.isFinite,
          lumaSigma > 0 else {
        throw NSError(
            domain: "XDRemuxAppleAdapter",
            code: 20,
            userInfo: [NSLocalizedDescriptionKey: "CoreImage L8 upsample arguments are invalid"]
        )
    }

    let (smallByteCount, smallOverflow) = smallWidth.multipliedReportingOverflow(by: smallHeight)
    let (targetByteCount, targetOverflow) = targetWidth.multipliedReportingOverflow(by: targetHeight)
    guard !smallOverflow, !targetOverflow else {
        throw NSError(
            domain: "XDRemuxAppleAdapter",
            code: 21,
            userInfo: [NSLocalizedDescriptionKey: "CoreImage L8 upsample geometry overflows"]
        )
    }

    let smallMask = try Data(
        contentsOf: URL(fileURLWithPath: smallMaskPath),
        options: [.mappedIfSafe]
    )
    guard smallMask.count == smallByteCount else {
        throw NSError(
            domain: "XDRemuxAppleAdapter",
            code: 22,
            userInfo: [
                NSLocalizedDescriptionKey:
                    "CoreImage L8 input has \(smallMask.count) bytes; expected \(smallByteCount)"
            ]
        )
    }

    let guideURL = URL(fileURLWithPath: guidePath)
    guard let source = CGImageSourceCreateWithURL(guideURL as CFURL, nil),
          let guideImage = CGImageSourceCreateImageAtIndex(source, 0, nil) else {
        throw NSError(
            domain: "XDRemuxAppleAdapter",
            code: 23,
            userInfo: [NSLocalizedDescriptionKey: "CoreImage cannot decode guide \(guidePath)"]
        )
    }

    let bounds = CGRect(x: 0, y: 0, width: targetWidth, height: targetHeight)
    let sourceGuide = CIImage(cgImage: guideImage)
    let guide = sourceGuide.transformed(by: CGAffineTransform(
        scaleX: CGFloat(targetWidth) / sourceGuide.extent.width,
        y: CGFloat(targetHeight) / sourceGuide.extent.height
    )).cropped(to: bounds)
    let small = CIImage(
        bitmapData: smallMask,
        bytesPerRow: smallWidth,
        size: CGSize(width: smallWidth, height: smallHeight),
        format: .L8,
        colorSpace: nil
    )

    let filter = CIFilter.edgePreserveUpsample()
    filter.inputImage = guide
    filter.smallImage = small
    filter.spatialSigma = spatialSigma
    filter.lumaSigma = lumaSigma
    guard let output = filter.outputImage?.cropped(to: bounds) else {
        throw NSError(
            domain: "XDRemuxAppleAdapter",
            code: 24,
            userInfo: [NSLocalizedDescriptionKey: "CoreImage edge-preserve upsample produced no image"]
        )
    }

    var rendered = Data(count: targetByteCount)
    let context = CIContext(options: [.cacheIntermediates: false])
    rendered.withUnsafeMutableBytes { bytes in
        guard let baseAddress = bytes.baseAddress else { return }
        context.render(
            output,
            toBitmap: baseAddress,
            rowBytes: targetWidth,
            bounds: bounds,
            format: .L8,
            colorSpace: nil
        )
    }
    try rendered.write(to: URL(fileURLWithPath: outputPath), options: .atomic)
}
