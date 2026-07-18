import Foundation
import AVFoundation
import CoreGraphics
import CoreImage
import CoreVideo
import Darwin
import ImageIO
import UniformTypeIdentifiers
import CryptoKit
import Vision
import XDRemuxCore

package enum ApplePhotographicStylesPipeline {
    private struct StyleDataResult {
        let styleData: Data
        let styleDataSHA256: String
        let polynomialCount: Int
        let blockValueCount: Int
        let tileCount: Int
    }

    private struct LinearSceneRaster {
        let width: Int
        let height: Int
        let encodedRGB8: Data
        let toneLinearRGB: [Float]
        let toneLuma: [Float]
        let hdrLuma: [Float]
    }

    private struct EncodedHEVCResource {
        let itemPayload: Data
        let hvcC: Data
        let sourcePNGURL: URL
        let annexBSHA256: String
        let itemPayloadSHA256: String
        let hvcCSHA256: String
    }

    private struct GraphWriteResult {
        let primaryItemID: Int
        let gainMapItemID: Int
        let toneMapItemID: Int
        let styleDeltaItemID: Int
        let linearThumbnailItemID: Int
        let styleMetadataItemID: Int
        let originalMdatPayloadSHA256: String
        let outputOriginalMdatPrefixSHA256: String
        let itemCount: Int
        let propertyCount: Int
    }

    private struct Options {
        let family: Family
        let debugRootURL: URL?
        let oppoCompatibility: OppoCompatibility
        let inputProcessingBranch: InputProcessingBranch
        let oppoCameraTail: OppoCameraTail
        let tmapFormat: TmapFormat
        let features: AppleFeatureFlags
        let eventHandler: ConversionEventHandler?
    }

    static func convert(
        inputURL: URL,
        outputURL: URL,
        configuration: ConversionConfiguration
    ) throws {
        try convert(
            inputURL: inputURL,
            outputURL: outputURL,
            options: Options(
                family: configuration.family,
                debugRootURL: configuration.debugDirectory,
                oppoCompatibility: configuration.oppoCompatibility,
                inputProcessingBranch: configuration.inputProcessingBranch,
                oppoCameraTail: configuration.oppoCameraTail,
                tmapFormat: configuration.tmapFormat,
                features: configuration.appleFeatureOptions,
                eventHandler: configuration.eventHandler
            )
        )
    }

    static func isValidOutput(_ outputURL: URL, expectsPortrait: Bool) -> Bool {
        guard PortraitConversionPipeline.hasValidISOGainMap(outputURL) else { return false }
        return (try? validatePhotographicStylesOutput(outputURL, expectsPortrait: expectsPortrait)) != nil
    }

    static func validateExistingOutput(
        _ outputURL: URL,
        expectsPortrait: Bool
    ) throws -> [String: Any] {
        let validation = try validatePhotographicStylesOutput(
            outputURL,
            expectsPortrait: expectsPortrait
        )
        return [
            "schema": "xdremux-apple-output-validation-v1",
            "passed": true,
            "output": outputURL.path,
            "outputSHA256": sha256Hex(validation.outputData),
            "expectsPortrait": expectsPortrait,
            "isoGainMap": true,
            "semanticStyleProperties": true,
            "styleDataLength": 51_840,
            "donorContamination": validation.contaminationReport,
        ]
    }

    private static func completeIdentityStyleData(
        outputDirectory: URL
    ) throws -> StyleDataResult {
        try FileManager.default.createDirectory(at: outputDirectory, withIntermediateDirectories: true)
        let polynomialCount = 10
        let channelCount = 3
        let blockValueCount = polynomialCount * channelCount
        let tileCount = 12 * 9 * 8
        let identityIndices = Set([3, 7, 11])
        var block = Data()
        block.reserveCapacity(blockValueCount * 2)
        for index in 0..<blockValueCount {
            var bits = Float16(identityIndices.contains(index) ? 1 : 0).bitPattern.littleEndian
            withUnsafeBytes(of: &bits) { block.append(contentsOf: $0) }
        }
        var styleData = Data()
        styleData.reserveCapacity(block.count * tileCount)
        for _ in 0..<tileCount {
            styleData.append(block)
        }
        let digest = sha256Hex(styleData)
        let expectedDigest = "43e0ae73508cc10684d4be708fa1d19f3b55b8de15cb8e3544ef16300db91dbe"
        guard styleData.count == 51_840, digest == expectedDigest else {
            throw CLIError.invalidContainer(
                "generated complete identity key 1 does not match the verified CMImaging coefficient layout"
            )
        }
        try styleData.write(
            to: outputDirectory.appendingPathComponent("complete-identity-style-data.f16.bin"),
            options: .atomic
        )
        return StyleDataResult(
            styleData: styleData,
            styleDataSHA256: digest,
            polynomialCount: polynomialCount,
            blockValueCount: blockValueCount,
            tileCount: tileCount
        )
    }

    private static func sourceScale(
        sourceURL: URL,
        portraitWritten: Bool
    ) throws -> ResolvedScale {
        let sourceData = try Data(contentsOf: sourceURL, options: [.mappedIfSafe])
        if portraitWritten,
           let blocks = try? LHDRExtractor.portraitBlocks(from: sourceData),
           let info = try? PortraitConversionPipeline.resolveGainInfoFloats(
               privateInfo: blocks["local.uhdr.gainmap.info"],
               inputURL: sourceURL
           ) {
            return try EDRScaleResolver.resolve(metaFloats: info, mode: .uhdr)
        }
        let extracted = try LHDRExtractor.extract(from: sourceData)
        return try EDRScaleResolver.resolve(
            metaFloats: extracted.metaFloats,
            mode: extracted.mode
        )
    }

    private static func fittedSize(
        sourceWidth: Int,
        sourceHeight: Int,
        maximumWidth: Int,
        maximumHeight: Int
    ) -> (Int, Int) {
        let scale = min(
            1.0,
            Double(maximumWidth) / Double(sourceWidth),
            Double(maximumHeight) / Double(sourceHeight)
        )
        let width = max(2, Int((Double(sourceWidth) * scale / 2).rounded()) * 2)
        let height = max(2, Int((Double(sourceHeight) * scale / 2).rounded()) * 2)
        return (min(width, maximumWidth), min(height, maximumHeight))
    }

    private static func renderedRGBAFloat(
        image: CIImage,
        width: Int,
        height: Int,
        colorSpace: CGColorSpace?
    ) -> [Float] {
        let normalized = image.transformed(by: CGAffineTransform(
            translationX: -image.extent.origin.x,
            y: -image.extent.origin.y
        ))
        let resized = normalized.transformed(by: CGAffineTransform(
            scaleX: CGFloat(width) / normalized.extent.width,
            y: CGFloat(height) / normalized.extent.height
        )).cropped(to: CGRect(x: 0, y: 0, width: width, height: height))
        var pixels = Array(repeating: Float(0), count: width * height * 4)
        let contextColorSpace: Any = colorSpace ?? NSNull()
        pixels.withUnsafeMutableBytes { raw in
            guard let base = raw.baseAddress else { return }
            CIContext(options: [
                .cacheIntermediates: false,
                .workingColorSpace: contextColorSpace,
                .outputColorSpace: contextColorSpace,
            ]).render(
                resized,
                toBitmap: base,
                rowBytes: width * 4 * MemoryLayout<Float>.size,
                bounds: CGRect(x: 0, y: 0, width: width, height: height),
                format: .RGBAf,
                colorSpace: colorSpace
            )
        }
        return pixels
    }

    private static func halfRounded(_ value: Float) -> Float {
        Float(Float16(value))
    }

    private static func appleEncodeLinear(_ input: Float) -> Float {
        let x = halfRounded(input)
        let highThreshold = Float(Float16(bitPattern: 0x211f))
        let lowThreshold = Float(Float16(bitPattern: 0xab38))
        if x >= highThreshold {
            let logInput = halfRounded(x + Float(Float16(bitPattern: 0x20f0)))
            let logged = halfRounded(log2f(logInput))
            return halfRounded(
                halfRounded(logged * Float(Float16(bitPattern: 0x2d79)))
                    + Float(Float16(bitPattern: 0x398c))
            )
        }
        if x > lowThreshold {
            let toe = halfRounded(x + Float(Float16(bitPattern: 0x2b38)))
            return halfRounded(
                halfRounded(toe * toe) * Float(Float16(bitPattern: 0x51e9))
            )
        }
        return 0
    }

    private static func linearSceneRaster(
        standardHDRURL: URL,
        scale: ResolvedScale
    ) throws -> LinearSceneRaster {
        guard let primary = CIImage(
            contentsOf: standardHDRURL,
            options: [.applyOrientationProperty: true]
        ), let gain = CIImage(
            contentsOf: standardHDRURL,
            options: [
                .auxiliaryHDRGainMap: true,
                .applyOrientationProperty: true,
                .colorSpace: NSNull(),
            ]
        ) else {
            throw CLIError.invalidContainer("Core Image cannot decode the coherent base/gain bundle")
        }
        let sourceWidth = max(1, Int(primary.extent.width.rounded()))
        let sourceHeight = max(1, Int(primary.extent.height.rounded()))
        let size = fittedSize(
            sourceWidth: sourceWidth,
            sourceHeight: sourceHeight,
            maximumWidth: 1024,
            maximumHeight: 1024
        )
        guard let linearP3 = CGColorSpace(name: CGColorSpace.extendedLinearDisplayP3) else {
            throw CLIError.invalidContainer("required linear Display P3 color space is unavailable")
        }
        let basePixels = renderedRGBAFloat(
            image: primary,
            width: size.0,
            height: size.1,
            colorSpace: linearP3
        )
        let gainPixels = renderedRGBAFloat(
            image: gain,
            width: size.0,
            height: size.1,
            colorSpace: nil
        )
        var encoded = Data(count: size.0 * size.1 * 3)
        var toneRGB = Array(repeating: Float(0), count: size.0 * size.1 * 3)
        var toneLuma = Array(repeating: Float(0), count: size.0 * size.1)
        var hdrLuma = Array(repeating: Float(0), count: size.0 * size.1)
        let channelCount = max(1, scale.channelCount)
        func channel(_ values: [Double], _ index: Int, _ fallback: Double) -> Float {
            Float(values[min(index, values.count - 1)] as Double? ?? fallback)
        }
        encoded.withUnsafeMutableBytes { encodedRaw in
            guard let destination = encodedRaw.bindMemory(to: UInt8.self).baseAddress else { return }
            for pixel in 0..<(size.0 * size.1) {
                var baseChannels = [Float](repeating: 0, count: 3)
                var hdrChannels = [Float](repeating: 0, count: 3)
                for component in 0..<3 {
                    let parameterIndex = channelCount == 1 ? 0 : component
                    let base = basePixels[pixel * 4 + component]
                    let code = min(max(gainPixels[pixel * 4 + component], 0), 1)
                    let gamma = channel(scale.perChannelGamma, parameterIndex, scale.gamma)
                    let minimum = channel(scale.perChannelGainMapMin, parameterIndex, scale.gainMapMin)
                    let maximum = channel(scale.perChannelGainMapMax, parameterIndex, scale.gainMapMax)
                    let baseOffset = channel(
                        scale.perChannelBaseOffset,
                        parameterIndex,
                        scale.epsilonSdr
                    )
                    let alternateOffset = channel(
                        scale.perChannelAlternateOffset,
                        parameterIndex,
                        scale.epsilonHdr
                    )
                    let weight = powf(code, gamma)
                    let logGain = minimum + weight * (maximum - minimum)
                    let reconstructed = max(base + baseOffset, 0) * exp2f(logGain) - alternateOffset
                    baseChannels[component] = base
                    hdrChannels[component] = reconstructed
                    toneRGB[pixel * 3 + component] = base
                    let encodedValue = min(max(appleEncodeLinear(reconstructed), 0), 1)
                    destination[pixel * 3 + component] = UInt8(
                        min(255, max(0, Int((encodedValue * 255).rounded())))
                    )
                }
                toneLuma[pixel] = 0.2126 * baseChannels[0]
                    + 0.7152 * baseChannels[1]
                    + 0.0722 * baseChannels[2]
                hdrLuma[pixel] = 0.2126 * hdrChannels[0]
                    + 0.7152 * hdrChannels[1]
                    + 0.0722 * hdrChannels[2]
            }
        }
        return LinearSceneRaster(
            width: size.0,
            height: size.1,
            encodedRGB8: encoded,
            toneLinearRGB: toneRGB,
            toneLuma: toneLuma,
            hdrLuma: hdrLuma
        )
    }

    private static func writeRGBPNG(
        pixels: Data,
        width: Int,
        height: Int,
        outputURL: URL
    ) throws {
        guard pixels.count == width * height * 3,
              let provider = CGDataProvider(data: pixels as CFData),
              let image = CGImage(
                  width: width,
                  height: height,
                  bitsPerComponent: 8,
                  bitsPerPixel: 24,
                  bytesPerRow: width * 3,
                  space: CGColorSpaceCreateDeviceRGB(),
                  bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.none.rawValue),
                  provider: provider,
                  decode: nil,
                  shouldInterpolate: false,
                  intent: .defaultIntent
              ),
              let destination = CGImageDestinationCreateWithURL(
                  outputURL as CFURL,
                  UTType.png.identifier as CFString,
                  1,
                  nil
              ) else {
            throw CLIError.invalidContainer("cannot create Apple auxiliary PNG writer input")
        }
        CGImageDestinationAddImage(destination, image, nil)
        guard CGImageDestinationFinalize(destination) else {
            throw CLIError.invalidContainer("cannot finalize Apple auxiliary PNG writer input")
        }
    }

    private static func singleIDRPayload(from annexB: Data) throws -> Data {
        let bytes = [UInt8](annexB)
        var starts: [(offset: Int, length: Int)] = []
        var index = 0
        while index + 3 < bytes.count {
            if bytes[index...min(index + 3, bytes.count - 1)] == [0, 0, 0, 1] {
                starts.append((index, 4)); index += 4
            } else if bytes[index...min(index + 2, bytes.count - 1)] == [0, 0, 1] {
                starts.append((index, 3)); index += 3
            } else {
                index += 1
            }
        }
        for position in starts.indices {
            let start = starts[position].offset + starts[position].length
            let end = position + 1 < starts.count ? starts[position + 1].offset : bytes.count
            guard start < end else { continue }
            let type = (bytes[start] >> 1) & 0x3f
            guard type == 19 || type == 20 else { continue }
            var result = Data()
            appendUInt32BE(end - start, to: &result)
            result.append(contentsOf: bytes[start..<end])
            return result
        }
        throw CLIError.invalidContainer("VideoToolbox emitted no HEVC IDR NAL")
    }

    private static func encodeHEVC(
        rgbPNGURL: URL,
        outputDirectory: URL,
        stem: String,
        quality: Double
    ) throws -> EncodedHEVCResource {
        let annexBURL = outputDirectory.appendingPathComponent("\(stem).hevc")
        let hvcCURL = outputDirectory.appendingPathComponent("\(stem).hvcc")
        let executable = try AppleNativeToolchain.hevcEncoderExecutable()
        let result = try AppleNativeToolchain.run(
            executable,
            arguments: [
                rgbPNGURL.path,
                annexBURL.path,
                String(format: "%.6f", quality),
                "rgb10",
                hvcCURL.path,
            ]
        )
        guard result.status == 0 else {
            let error = String(data: result.stderr, encoding: .utf8) ?? ""
            throw CLIError.invalidContainer("VideoToolbox auxiliary encoding failed: \(error)")
        }
        let annexB = try Data(contentsOf: annexBURL)
        let hvcC = try Data(contentsOf: hvcCURL)
        let itemPayload = try singleIDRPayload(from: annexB)
        return EncodedHEVCResource(
            itemPayload: itemPayload,
            hvcC: hvcC,
            sourcePNGURL: rgbPNGURL,
            annexBSHA256: sha256Hex(annexB),
            itemPayloadSHA256: sha256Hex(itemPayload),
            hvcCSHA256: sha256Hex(hvcC)
        )
    }

    private static func percentile(_ sortedValues: [Double], _ percent: Double) -> Double {
        let values = sortedValues
        guard !values.isEmpty else { return 0 }
        let position = min(max(percent, 0), 100) / 100 * Double(values.count - 1)
        let lower = Int(floor(position))
        let upper = Int(ceil(position))
        guard lower != upper else { return values[lower] }
        let fraction = position - Double(lower)
        return values[lower] * (1 - fraction) + values[upper] * fraction
    }

    package static func distribution(_ values: [Float]) -> [String: Double] {
        let sorted = values.lazy.filter(\.isFinite).map(Double.init).sorted()
        guard !sorted.isEmpty else {
            return [
                "blackPoint": 0, "highKey": 1, "p02": 0, "p10": 0,
                "p25": 0, "p50": 0, "p75": 0, "p98": 0, "whitePoint": 0,
            ]
        }
        let names = ["blackPoint", "highKey", "p02", "p10", "p25", "p50", "p75", "p98", "whitePoint"]
        let percents = [0.5, 95, 2, 10, 25, 50, 75, 98, 99.5]
        return Dictionary(uniqueKeysWithValues: zip(names, percents.map { percentile(sorted, $0) }))
    }

    private static func maskValue(
        _ matte: AppleSemanticMatte?,
        x: Int,
        y: Int,
        rasterWidth: Int,
        rasterHeight: Int
    ) -> Float {
        guard let matte, matte.width > 0, matte.height > 0 else { return 0 }
        let sourceX = min(matte.width - 1, max(0, Int((Double(x) + 0.5) * Double(matte.width) / Double(rasterWidth))))
        let sourceY = min(matte.height - 1, max(0, Int((Double(y) + 0.5) * Double(matte.height) / Double(rasterHeight))))
        return Float(matte.pixels[sourceY * matte.bytesPerRow + sourceX]) / 255
    }

    package static func selectedStyleSamples(
        toneLuma: [Float],
        hdrLuma: [Float],
        toneLinearRGB: [Float],
        width: Int,
        height: Int,
        person: AppleSemanticMatte?,
        skin: AppleSemanticMatte?
    ) -> [String: [Float]] {
        var personTone: [Float] = []
        var personHDR: [Float] = []
        var skinTone: [Float] = []
        var skinHDR: [Float] = []
        var skinRed: [Float] = []
        var skinGreen: [Float] = []
        var skinBlue: [Float] = []
        let reserve = width * height / 4
        personTone.reserveCapacity(reserve)
        personHDR.reserveCapacity(reserve)
        skinTone.reserveCapacity(reserve)
        skinHDR.reserveCapacity(reserve)
        skinRed.reserveCapacity(reserve / 2)
        skinGreen.reserveCapacity(reserve / 2)
        skinBlue.reserveCapacity(reserve / 2)
        for y in 0..<height {
            for x in 0..<width {
                let pixel = y * width + x
                if maskValue(
                    person, x: x, y: y, rasterWidth: width, rasterHeight: height
                ) >= 0.5 {
                    personTone.append(toneLuma[pixel])
                    personHDR.append(hdrLuma[pixel])
                }
                if maskValue(
                    skin, x: x, y: y, rasterWidth: width, rasterHeight: height
                ) >= 0.5 {
                    skinTone.append(toneLuma[pixel])
                    skinHDR.append(hdrLuma[pixel])
                    skinRed.append(toneLinearRGB[pixel * 3])
                    skinGreen.append(toneLinearRGB[pixel * 3 + 1])
                    skinBlue.append(toneLinearRGB[pixel * 3 + 2])
                }
            }
        }
        return [
            "personTone": personTone,
            "personHDR": personHDR,
            "skinTone": skinTone,
            "skinHDR": skinHDR,
            "skinRed": skinRed,
            "skinGreen": skinGreen,
            "skinBlue": skinBlue,
        ]
    }

    private static func protocolIdentityGTC() -> Data {
        func sRGBEncode(_ linear: Double) -> Double {
            linear <= 0.0031308
                ? linear * 12.92
                : 1.055 * pow(linear, 1 / 2.4) - 0.055
        }
        var samples: [UInt16] = []
        samples.reserveCapacity(257)
        for index in 0..<256 {
            let encoded = min(max(sRGBEncode(Double(index) / 255), 0), 1)
            samples.append(UInt16(min(65_534, max(0, Int((encoded * 65_534).rounded())))))
        }
        samples[0] = 0
        samples[255] = 65_534
        for index in 1..<samples.count where samples[index] < samples[index - 1] {
            samples[index] = samples[index - 1]
        }
        samples.append(65_534)
        var payload = Data()
        var count = UInt16(257).littleEndian
        withUnsafeBytes(of: &count) { payload.append(contentsOf: $0) }
        for sample in samples {
            var value = sample.littleEndian
            withUnsafeBytes(of: &value) { payload.append(contentsOf: $0) }
        }
        return payload
    }

    private static func storageOrderedLightMap(
        _ presentationOrder: Data,
        width: Int,
        height: Int,
        orientation: UInt32
    ) throws -> Data {
        guard width == height, presentationOrder.count == width * height * 2 else {
            throw CLIError.invalidContainer("style light-map orientation requires a square packed Float16 plane")
        }
        let side = width
        var output = Data(count: presentationOrder.count)
        output.withUnsafeMutableBytes { outputRaw in
            presentationOrder.withUnsafeBytes { inputRaw in
                guard let destination = outputRaw.bindMemory(to: UInt16.self).baseAddress,
                      let source = inputRaw.bindMemory(to: UInt16.self).baseAddress else { return }
                for storageY in 0..<side {
                    for storageX in 0..<side {
                        let display: (x: Int, y: Int)
                        switch orientation {
                        case 2: display = (side - 1 - storageX, storageY)
                        case 3: display = (side - 1 - storageX, side - 1 - storageY)
                        case 4: display = (storageX, side - 1 - storageY)
                        case 5: display = (storageY, storageX)
                        case 6: display = (side - 1 - storageY, storageX)
                        case 7: display = (side - 1 - storageY, side - 1 - storageX)
                        case 8: display = (storageY, side - 1 - storageX)
                        default: display = (storageX, storageY)
                        }
                        destination[storageY * side + storageX] = source[display.y * side + display.x]
                    }
                }
            }
        }
        return output
    }

    private static func lightMap(
        _ luma: [Float],
        width: Int,
        height: Int,
        valueScale: Float,
        storageOrientation: UInt32
    ) throws -> Data {
        var presentationOrder = Data()
        presentationOrder.reserveCapacity(32 * 32 * 2)
        for targetY in 0..<32 {
            let y0 = targetY * height / 32
            let y1 = max(y0 + 1, (targetY + 1) * height / 32)
            for targetX in 0..<32 {
                let x0 = targetX * width / 32
                let x1 = max(x0 + 1, (targetX + 1) * width / 32)
                var sum = Double(0)
                var count = 0
                for y in y0..<min(y1, height) {
                    for x in x0..<min(x1, width) {
                        sum += Double(min(max(luma[y * width + x], 0), 1))
                        count += 1
                    }
                }
                let average = count == 0 ? Float(0) : Float(sum / Double(count))
                let scaled = min(max(average * valueScale, 0), 1)
                var bits = Float16(scaled).bitPattern.littleEndian
                withUnsafeBytes(of: &bits) { presentationOrder.append(contentsOf: $0) }
            }
        }
        return try storageOrderedLightMap(
            presentationOrder,
            width: 32,
            height: 32,
            orientation: storageOrientation
        )
    }

    private static func styleStatistics(
        raster: LinearSceneRaster,
        semantics: AppleSemanticSceneAnalysis
    ) -> [String: [String: Double]] {
        let gtcLuma = raster.hdrLuma.map { value -> Float in
            let linear = min(max(Double(value), 0), 1)
            return Float(
                linear <= 0.0031308
                    ? linear * 12.92
                    : 1.055 * pow(linear, 1 / 2.4) - 0.055
            )
        }
        let samples = selectedStyleSamples(
            toneLuma: raster.toneLuma,
            hdrLuma: raster.hdrLuma,
            toneLinearRGB: raster.toneLinearRGB,
            width: raster.width,
            height: raster.height,
            person: semantics.person,
            skin: semantics.skin
        )
        return [
            "LinearGTCImage": distribution(gtcLuma),
            "LinearImage": distribution(raster.hdrLuma),
            "LinearImagePersonSegmentBased": distribution(samples["personHDR"] ?? []),
            "LinearImageSkinBased": distribution(samples["skinHDR"] ?? []),
            "ToneMappedImage": distribution(raster.toneLuma),
            "ToneMappedImageBlueChannelSkinBased": distribution(samples["skinBlue"] ?? []),
            "ToneMappedImageGreenChannelSkinBased": distribution(samples["skinGreen"] ?? []),
            "ToneMappedImagePersonSegmentBased": distribution(samples["personTone"] ?? []),
            "ToneMappedImageRedChannelSkinBased": distribution(samples["skinRed"] ?? []),
            "ToneMappedImageSkinBased": distribution(samples["skinTone"] ?? []),
        ]
    }

    private static func makeStylePropertyList(
        styleData: StyleDataResult,
        raster: LinearSceneRaster,
        semantics: AppleSemanticSceneAnalysis,
        storageOrientation: UInt32
    ) throws -> (data: Data, manifest: [String: Any]) {
        let gtc = protocolIdentityGTC()
        guard gtc.count == 516 else {
            throw CLIError.invalidContainer("generated GTC must be 516 bytes")
        }
        let statistics = styleStatistics(raster: raster, semantics: semantics)
        guard let linear = statistics["LinearImage"],
              statistics["ToneMappedImage"] != nil else {
            throw CLIError.invalidContainer("generated style statistics are incomplete")
        }
        let rangeMin = linear["blackPoint"] ?? 0
        let rangeMax = linear["whitePoint"] ?? 0
        let robustRange = max(rangeMax - rangeMin, 1 / 4096)
        let baseGain = min(max(0.5 / robustRange, 0.5), 2.5)
        let toneLightMap = try lightMap(
            raster.toneLuma,
            width: raster.width,
            height: raster.height,
            valueScale: 1,
            storageOrientation: storageOrientation
        )
        let linearLightMap = try lightMap(
            raster.hdrLuma,
            width: raster.width,
            height: raster.height,
            valueScale: 1,
            storageOrientation: storageOrientation
        )
        guard toneLightMap.count == 2_048, linearLightMap.count == 2_048 else {
            throw CLIError.invalidContainer("generated style light maps must each be 2,048 bytes")
        }
        let baselineExposure = min(max(-log2(max(linear["p50"] ?? 0, 1 / 4096)), 4), 10.4)
        let peopleRatio = min(max((semantics.person?.statistics.mean ?? 0) / 255, 0), 1)
        let skinRatio = min(max((semantics.skin?.statistics.mean ?? 0) / 255, 0), 1)
        let personMasksValidHint = semantics.hasCrediblePerson ? 1.0 : 0.0
        // Native sceneType and face boost are real producer fields, but neither
        // is derivable from the current coverage/exposure heuristics.
        let sceneType = 0
        let faceBoost = 1.0
        let object: [String: Any] = [
            "0": 15,
            "1": styleData.styleData,
            "2": true,
            "3": gtc,
            "4": baselineExposure,
            "5": sceneType,
            "6": statistics,
            "7": [
                "PeopleRatio": peopleRatio,
                "PersonMasksValidHint": personMasksValidHint,
                "SkinRatio": skinRatio,
            ],
            "c": toneLightMap,
            "d": linearLightMap,
            "e": 32,
            "f": 32,
            "g": 0x4C303068,
            "h": baseGain,
            "i": [
                "Gain": baseGain * 4,
                "OriginalRangeMin": rangeMin,
                "OriginalRangeMax": rangeMax,
            ],
            "j": faceBoost,
            "k": false,
        ]
        let data = try PropertyListSerialization.data(
            fromPropertyList: object,
            format: .binary,
            options: 0
        )
        guard data.starts(with: Data("bplist00".utf8)),
              let readback = try PropertyListSerialization.propertyList(
                  from: data, options: [], format: nil
              ) as? [String: Any],
              let styleReadback = readback["1"] as? Data,
              styleReadback == styleData.styleData else {
            throw CLIError.invalidContainer("style plist key 1 readback differs from generated identity data")
        }
        return (data, [
            "schema": "xdremux-apple-photographic-style-payload-v1",
            "styleVersion": 15,
            "styleData": [
                "byteCount": styleData.styleData.count,
                "sha256": styleData.styleDataSHA256,
                "evidence": AppleEvidenceClass.privateFrameworkIdentity.rawValue,
                "producer": "deterministic-complete-identity-v1",
                "polynomialCount": styleData.polynomialCount,
                "blockValueCount": styleData.blockValueCount,
                "tileCount": styleData.tileCount,
            ],
            "gtc": ["byteCount": gtc.count, "sha256": sha256Hex(gtc), "algorithm": "protocol-neutral-linear-runtime-gtc-v1"],
            "statistics": statistics,
            "peopleRatio": peopleRatio,
            "skinRatio": skinRatio,
            "personMasksValidHint": personMasksValidHint,
            "sceneType": sceneType,
            "sceneTypeFallback": "neutral-until-native-producer-semantics-are-recovered",
            "baselineExposure": baselineExposure,
            "baseGain": baseGain,
            "linearRange": ["minimum": rangeMin, "maximum": rangeMax],
            "faceExposureBoost": faceBoost,
            "lightMap": [
                "byteCount": toneLightMap.count,
                "sha256": sha256Hex(toneLightMap),
                "coordinateOrder": "primary-item-storage",
                "sourceOrientation": storageOrientation,
            ],
            "linearLightMap": [
                "byteCount": linearLightMap.count,
                "sha256": sha256Hex(linearLightMap),
                "coordinateOrder": "primary-item-storage",
                "sourceOrientation": storageOrientation,
                "linearBaseGainApplied": false,
            ],
            "faceExposureBoostFallback": "neutral-until-native-producer-semantics-are-recovered",
            "stylePropertyList": ["byteCount": data.count, "sha256": sha256Hex(data), "format": "binary plist v1 CF format 200"],
        ])
    }

    private static func validateWithSemanticStyleProperties(
        stylePropertyList: Data,
        expectedStyleData: Data,
        outputDirectory: URL
    ) throws -> [String: Any] {
        let metadataURL = outputDirectory.appendingPathComponent("style-metadata.bplist")
        let readbackURL = outputDirectory.appendingPathComponent("style-data-neutrino-readback.bin")
        let probeURL = outputDirectory.appendingPathComponent("semantic-style-properties-probe.json")
        try stylePropertyList.write(to: metadataURL, options: .atomic)
        let executable = try AppleNativeToolchain.stylePropertiesProbeExecutable()
        let result = try AppleNativeToolchain.run(
            executable,
            arguments: [metadataURL.path, readbackURL.path]
        )
        try result.stdout.write(to: probeURL, options: .atomic)
        guard result.status == 0,
              let readback = try? Data(contentsOf: readbackURL),
              readback == expectedStyleData,
              let object = try? JSONSerialization.jsonObject(with: result.stdout) as? [String: Any],
              object["parseSucceeded"] as? Bool == true,
              (object["styleDataLength"] as? NSNumber)?.intValue == 51_840 else {
            let diagnostic = String(data: result.stderr, encoding: .utf8) ?? ""
            throw CLIError.invalidContainer(
                "_NUSemanticStyleProperties rejected generated metadata or changed key 1: \(diagnostic)"
            )
        }
        return [
            "parseSucceeded": true,
            "styleDataLength": readback.count,
            "readbackSHA256": sha256Hex(readback),
            "matchesExpectedStyleData": true,
            "probe": probeURL.path,
        ]
    }

    private static func buildStylePayload(
        sourceURL: URL,
        standardHDRURL: URL,
        semantics: AppleSemanticSceneAnalysis,
        portraitWritten: Bool,
        outputDirectory: URL,
        photoIdentifier: String
    ) throws -> ApplePhotographicStylePayload {
        let payloadStartedAt = CFAbsoluteTimeGetCurrent()
        try FileManager.default.createDirectory(at: outputDirectory, withIntermediateDirectories: true)
        let styleDataDirectory = outputDirectory.appendingPathComponent("style-data")
        let styleData = try completeIdentityStyleData(outputDirectory: styleDataDirectory)
        let scale = try sourceScale(sourceURL: sourceURL, portraitWritten: portraitWritten)
        let raster = try linearSceneRaster(standardHDRURL: standardHDRURL, scale: scale)
        let rasterCompletedAt = CFAbsoluteTimeGetCurrent()
        let linearPNG = outputDirectory.appendingPathComponent("linear-thumbnail.png")
        try writeRGBPNG(
            pixels: raster.encodedRGB8,
            width: raster.width,
            height: raster.height,
            outputURL: linearPNG
        )
        let linearQuality = EncodingQualityPolicy.value(
            environmentKey: "XDREMUX_STYLES_LINEAR_QUALITY",
            defaultValue: 0.85
        )
        let linearHEVC = try encodeHEVC(
            rgbPNGURL: linearPNG,
            outputDirectory: outputDirectory,
            stem: "linear-thumbnail",
            quality: linearQuality
        )
        let linearHEVCCompletedAt = CFAbsoluteTimeGetCurrent()

        let neutralTile = Data(repeating: 128, count: 512 * 512 * 3)
        let deltaPNG = outputDirectory.appendingPathComponent("style-delta-neutral-tile.png")
        try writeRGBPNG(pixels: neutralTile, width: 512, height: 512, outputURL: deltaPNG)
        let deltaQuality = EncodingQualityPolicy.value(
            environmentKey: "XDREMUX_STYLES_DELTA_QUALITY",
            defaultValue: 0.3
        )
        let deltaHEVC = try encodeHEVC(
            rgbPNGURL: deltaPNG,
            outputDirectory: outputDirectory,
            stem: "style-delta-neutral-tile",
            quality: deltaQuality
        )
        let deltaHEVCCompletedAt = CFAbsoluteTimeGetCurrent()
        guard let primary = CIImage(
            contentsOf: standardHDRURL,
            options: [.applyOrientationProperty: true]
        ) else {
            throw CLIError.invalidContainer("cannot derive Photographic Styles presentation geometry")
        }
        let sourceWidth = max(1, Int(primary.extent.width.rounded()))
        let sourceHeight = max(1, Int(primary.extent.height.rounded()))
        let landscape = sourceWidth >= sourceHeight
        let deltaSize = fittedSize(
            sourceWidth: sourceWidth,
            sourceHeight: sourceHeight,
            maximumWidth: landscape ? 2880 : 2560,
            maximumHeight: landscape ? 2560 : 2880
        )
        let rows = landscape ? 5 : 6
        let columns = landscape ? 6 : 5
        let style = try makeStylePropertyList(
            styleData: styleData,
            raster: raster,
            semantics: semantics,
            storageOrientation: exifOrientation(at: standardHDRURL)
        )
        let semanticStyleValidation = try validateWithSemanticStyleProperties(
            stylePropertyList: style.data,
            expectedStyleData: styleData.styleData,
            outputDirectory: outputDirectory
        )
        let payloadCompletedAt = CFAbsoluteTimeGetCurrent()
        let payloadTimings: [String: Double] = [
            "setupAndRaster": rasterCompletedAt - payloadStartedAt,
            "linearHEVC": linearHEVCCompletedAt - rasterCompletedAt,
            "deltaHEVC": deltaHEVCCompletedAt - linearHEVCCompletedAt,
            "metadataAndValidation": payloadCompletedAt - deltaHEVCCompletedAt,
            "total": payloadCompletedAt - payloadStartedAt,
        ]
        print(String(
            format: "styles payload setup+raster=%.3fs linearHEVC=%.3fs deltaHEVC=%.3fs metadata+validation=%.3fs total=%.3fs",
            payloadTimings["setupAndRaster"] ?? 0,
            payloadTimings["linearHEVC"] ?? 0,
            payloadTimings["deltaHEVC"] ?? 0,
            payloadTimings["metadataAndValidation"] ?? 0,
            payloadTimings["total"] ?? 0
        ))
        let inputSHA = sha256Hex(try Data(contentsOf: sourceURL, options: [.mappedIfSafe]))
        let provenance: [String: AppleResourceProvenance] = [
            "styleData": AppleResourceProvenance(
                producer: "XDRemux deterministic complete StyleEngine identity",
                inputSHA256: inputSHA,
                evidence: .privateFrameworkIdentity,
                detail: "864 repeated 30-Float16 identity blocks; SHA-256 matched to local CMImaging identity creator"
            ),
            "linearThumbnail": AppleResourceProvenance(
                producer: "XDRemux coherent HDR scene reconstruction + Apple encodeLinear",
                inputSHA256: inputSHA,
                evidence: .sourceDerivedApproximation,
                detail: "Display P3 Base plus unmanaged normalized Gain Map code values and source gain metadata; capture-linear LTM attachments unavailable"
            ),
            "styleDelta": AppleResourceProvenance(
                producer: "XDRemux iPhone18,1/23F84 zero-residual profile",
                inputSHA256: inputSHA,
                evidence: .profileExact,
                detail: "pre-HEVC normalized RGB is exactly 0.5; encoder is VideoToolbox Main10 4:2:0"
            ),
            "metadata": AppleResourceProvenance(
                producer: "XDRemux conservative source-derived metadata",
                inputSHA256: inputSHA,
                evidence: .sourceDerivedApproximation,
                detail: "source statistics retained; unsupported sceneType and face boost use explicit neutral fallbacks"
            ),
        ]
        var manifest = style.manifest
        manifest["semanticStylePropertiesValidation"] = semanticStyleValidation
        manifest["timingsSeconds"] = payloadTimings
        manifest["input"] = ["path": sourceURL.path, "sha256": inputSHA]
        manifest["photoIdentifier"] = photoIdentifier
        manifest["linearThumbnail"] = [
            "width": raster.width, "height": raster.height,
            "encodingQuality": linearQuality,
            "itemPayloadSHA256": linearHEVC.itemPayloadSHA256,
            "hvcCSHA256": linearHEVC.hvcCSHA256,
            "evidence": AppleEvidenceClass.sourceDerivedApproximation.rawValue,
            "gainMapDecode": [
                "domain": "raw-normalized-parameter-code-value",
                "colorManagementApplied": false,
                "transferFunctionApplied": false,
            ],
        ]
        manifest["styleDelta"] = [
            "profile": "iPhone18,1/23F84-zero-residual",
            "normalizedRGB": 0.5,
            "encodingQuality": deltaQuality,
            "tile": ["width": 512, "height": 512],
            "grid": ["width": deltaSize.0, "height": deltaSize.1, "rows": rows, "columns": columns],
            "itemPayloadSHA256": deltaHEVC.itemPayloadSHA256,
            "hvcCSHA256": deltaHEVC.hvcCSHA256,
            "codecEvidenceBoundary": "profile exact before HEVC; VideoToolbox Main10 4:2:0 is behaviorally tested but not byte-identical to Apple camera 4:4:4",
        ]
        manifest["provenance"] = Dictionary(uniqueKeysWithValues: provenance.map { key, value in
            (key, [
                "producer": value.producer,
                "inputSHA256": value.inputSHA256,
                "evidence": value.evidence.rawValue,
                "detail": value.detail,
            ])
        })
        let manifestJSON = try JSONSerialization.data(
            withJSONObject: manifest,
            options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        )
        return ApplePhotographicStylePayload(
            styleData: styleData.styleData,
            stylePropertyList: style.data,
            linearThumbnailHEVC: linearHEVC.itemPayload,
            linearThumbnailHVCC: linearHEVC.hvcC,
            linearThumbnailWidth: raster.width,
            linearThumbnailHeight: raster.height,
            styleDeltaHEVC: deltaHEVC.itemPayload,
            styleDeltaHVCC: deltaHEVC.hvcC,
            styleDeltaTileWidth: 512,
            styleDeltaTileHeight: 512,
            styleDeltaGridWidth: deltaSize.0,
            styleDeltaGridHeight: deltaSize.1,
            styleDeltaRows: rows,
            styleDeltaColumns: columns,
            photoIdentifier: photoIdentifier,
            manifestJSON: manifestJSON,
            resourceProvenance: provenance
        )
    }

    private static func mergeSemanticAuxiliaryGraph(
        sourceHDRURL: URL,
        semanticScaffoldURL: URL,
        outputURL: URL,
        profile: AppleSemanticWriteProfile
    ) throws {
        struct ImportRecord {
            let oldID: Int
            let newID: Int
            let info: ISOBMFFItemInfo
            let payload: Data
            let constructionMethod: Int
        }
        struct ParsedFile {
            let data: Data
            let top: [ISOBMFFBox]
            let meta: ISOBMFFBox
            let mdat: ISOBMFFBox
            let children: [ISOBMFFBox]
            let primaryID: Int
            let toneMapID: Int
            let items: [ISOBMFFItemInfo]
            let iinfVersion: UInt8
            let locations: [ISOBMFFILocEntry]
            let refsVersion: UInt8
            let refs: [ISOBMFFIRefEntry]
            let properties: [ISOBMFFPropertyInfo]
            let ipmaVersion: UInt8
            let ipmaFlags: Int
            let ipmaEntries: [ISOBMFFIPMAEntry]
            let idat: ISOBMFFBox?
        }
        func parse(_ url: URL, owner: String) throws -> ParsedFile {
            let data = try Data(contentsOf: url, options: [.mappedIfSafe])
            let top = isobmffBoxes(in: data, start: 0, end: data.count)
            guard let meta = top.first(where: { $0.type == "meta" }),
                  let mdat = top.first(where: { $0.type == "mdat" }) else {
                throw CLIError.invalidContainer("\(owner) semantic merge input has no meta/mdat")
            }
            let children = isobmffBoxes(in: data, start: meta.dataStart + 4, end: meta.dataEnd)
            func child(_ type: String) throws -> ISOBMFFBox {
                guard let box = children.first(where: { $0.type == type }) else {
                    throw CLIError.invalidContainer("\(owner) semantic merge input has no \(type)")
                }
                return box
            }
            let iinf = try child("iinf")
            let iloc = try child("iloc")
            let pitm = try child("pitm")
            let iprp = try child("iprp")
            let iref = children.first(where: { $0.type == "iref" })
            let itemInfo = parseISOBMFFItemInfos(data, iinf)
            guard let toneMapID = itemInfo.items.first(where: { $0.type == "tmap" })?.itemID,
                  let ipmaBox = isobmffBoxes(
                      in: data, start: iprp.dataStart, end: iprp.dataEnd
                  ).first(where: { $0.type == "ipma" }) else {
                throw CLIError.invalidContainer("\(owner) semantic merge item graph is incomplete")
            }
            let refs = parseISOBMFFIRefs(data, iref)
            let ipma = parseISOBMFFIPMA(data, ipmaBox)
            return ParsedFile(
                data: data,
                top: top,
                meta: meta,
                mdat: mdat,
                children: children,
                primaryID: parseISOBMFFPITM(data, pitm),
                toneMapID: toneMapID,
                items: itemInfo.items,
                iinfVersion: itemInfo.version,
                locations: try parseISOBMFFILoc(data, iloc),
                refsVersion: refs.version,
                refs: refs.refs,
                properties: try parseISOBMFFIPCOPropertyInfos(data, iprp),
                ipmaVersion: ipma.version,
                ipmaFlags: ipma.flags,
                ipmaEntries: ipma.entries,
                idat: children.first(where: { $0.type == "idat" })
            )
        }

        let source = try parse(sourceHDRURL, owner: "source HDR")
        let scaffold = try parse(semanticScaffoldURL, owner: "semantic scaffold")
        let scaffoldItemsByID = Dictionary(uniqueKeysWithValues: scaffold.items.map { ($0.itemID, $0) })
        let scaffoldLocationsByID = Dictionary(uniqueKeysWithValues: scaffold.locations.map { ($0.itemID, $0) })
        guard let sourceExifID = source.items.first(where: { $0.type == "Exif" })?.itemID,
              let scaffoldExifID = scaffold.items.first(where: { $0.type == "Exif" })?.itemID,
              let scaffoldExifLocation = scaffoldLocationsByID[scaffoldExifID] else {
            throw CLIError.invalidContainer("semantic merge requires source and scaffold Exif items")
        }
        let semanticImageIDs = scaffold.refs.compactMap { ref -> Int? in
            guard ref.type == "auxl",
                  ref.to.contains(scaffold.primaryID),
                  ref.to.contains(scaffold.toneMapID),
                  scaffoldItemsByID[ref.from]?.type == "hvc1" else { return nil }
            return ref.from
        }
        guard semanticImageIDs.count == profile.roles.count else {
            throw CLIError.invalidContainer(
                "semantic scaffold \(profile.kind.rawValue) expected \(profile.roles.count) roles, found \(semanticImageIDs.count)"
            )
        }
        let semanticMetadataIDs = scaffold.refs.compactMap { ref -> Int? in
            guard ref.type == "cdsc", ref.to.count == 1,
                  semanticImageIDs.contains(ref.to[0]),
                  scaffoldItemsByID[ref.from]?.type == "mime" else { return nil }
            return ref.from
        }
        guard semanticMetadataIDs.count == semanticImageIDs.count else {
            throw CLIError.invalidContainer("semantic scaffold metadata/image pairs are incomplete")
        }

        let groupIDs: [Int] = source.children
            .filter { $0.type == "grpl" }
            .flatMap { container in
                isobmffBoxes(
                    in: source.data, start: container.dataStart, end: container.dataEnd
                ).compactMap { group -> Int? in
                    guard group.dataEnd - group.dataStart >= 8 else { return nil }
                    return readUInt32BEUnchecked(source.data, at: group.dataStart + 4)
                }
            }
        var nextID = max(
            source.items.map(\.itemID).max() ?? 0,
            max(source.locations.map(\.itemID).max() ?? 0, groupIDs.max() ?? 0)
        ) + 1
        var itemIDMap: [Int: Int] = [:]
        for oldID in semanticImageIDs + semanticMetadataIDs {
            guard nextID <= 65_535 else {
                throw CLIError.invalidContainer("semantic merge exhausted HEIF UInt16 item IDs")
            }
            itemIDMap[oldID] = nextID
            nextID += 1
        }
        let importIDs = semanticImageIDs + semanticMetadataIDs
        let records: [ImportRecord] = try importIDs.map { oldID in
            guard let newID = itemIDMap[oldID],
                  let info = scaffoldItemsByID[oldID],
                  let location = scaffoldLocationsByID[oldID] else {
                throw CLIError.invalidContainer("semantic import item \(oldID) is incomplete")
            }
            return ImportRecord(
                oldID: oldID,
                newID: newID,
                info: info,
                payload: try itemPayload(in: scaffold.data, entry: location, idat: scaffold.idat),
                constructionMethod: info.type == "mime" ? 1 : 0
            )
        }
        let scaffoldExifPayload = try itemPayload(
            in: scaffold.data, entry: scaffoldExifLocation, idat: scaffold.idat
        )

        var ipcoPayload = Data()
        for property in source.properties { ipcoPayload.append(property.rawBox) }
        let scaffoldPropertiesByIndex = Dictionary(
            uniqueKeysWithValues: scaffold.properties.map { ($0.index, $0) }
        )
        var propertyIndexMap: [Int: Int] = [:]
        func mappedProperty(_ oldIndex: Int) throws -> Int {
            if let current = propertyIndexMap[oldIndex] { return current }
            guard let property = scaffoldPropertiesByIndex[oldIndex] else {
                throw CLIError.invalidContainer("semantic scaffold property \(oldIndex) is missing")
            }
            let newIndex = source.properties.count + propertyIndexMap.count + 1
            propertyIndexMap[oldIndex] = newIndex
            ipcoPayload.append(property.rawBox)
            return newIndex
        }
        let scaffoldIPMAByID = Dictionary(
            uniqueKeysWithValues: scaffold.ipmaEntries.map { ($0.itemID, $0) }
        )
        var importedAssociations: [Int: [(Int, Bool)]] = [:]
        for oldID in semanticImageIDs {
            guard let entry = scaffoldIPMAByID[oldID], let newID = itemIDMap[oldID] else {
                throw CLIError.invalidContainer("semantic image \(oldID) has no property associations")
            }
            importedAssociations[newID] = try assocPairs(
                entry.associations, flags: scaffold.ipmaFlags
            ).map { (try mappedProperty($0.0), $0.1) }
        }
        var ipmaEntries = Data()
        var ipmaCount = 0
        for entry in source.ipmaEntries {
            ipmaEntries.append(try makeIPMAEntry(
                entry.itemID,
                assocPairs(entry.associations, flags: source.ipmaFlags),
                flags: source.ipmaFlags,
                version: source.ipmaVersion
            ))
            ipmaCount += 1
        }
        for newID in importedAssociations.keys.sorted() {
            ipmaEntries.append(try makeIPMAEntry(
                newID,
                importedAssociations[newID] ?? [],
                flags: source.ipmaFlags,
                version: source.ipmaVersion
            ))
            ipmaCount += 1
        }
        var ipmaPayload = Data([
            source.ipmaVersion,
            UInt8((source.ipmaFlags >> 16) & 0xff),
            UInt8((source.ipmaFlags >> 8) & 0xff),
            UInt8(source.ipmaFlags & 0xff),
        ])
        appendUInt32BE(ipmaCount, to: &ipmaPayload)
        ipmaPayload.append(ipmaEntries)
        var iprpPayload = Data()
        iprpPayload.append(makeBox("ipco", payload: ipcoPayload))
        iprpPayload.append(makeBox("ipma", payload: ipmaPayload))
        let outputIPRP = makeBox("iprp", payload: iprpPayload)

        var rawInfes = source.items.map(\.rawInfe)
        for record in records {
            rawInfes.append(try remapInfeItemID(record.info.rawInfe, to: record.newID))
        }
        let outputIINF = makeIinfBox(version: source.iinfVersion, rawInfes: rawInfes)

        var outputRefs = source.refs
        func mappedTarget(_ oldID: Int) throws -> Int {
            if oldID == scaffold.primaryID { return source.primaryID }
            if oldID == scaffold.toneMapID { return source.toneMapID }
            if let mapped = itemIDMap[oldID] { return mapped }
            throw CLIError.invalidContainer("semantic reference target \(oldID) cannot be remapped")
        }
        for ref in scaffold.refs where itemIDMap[ref.from] != nil {
            outputRefs.append(ISOBMFFIRefEntry(
                type: ref.type,
                from: itemIDMap[ref.from]!,
                to: try ref.to.map(mappedTarget)
            ))
        }
        if !outputRefs.contains(where: {
            $0.type == "cdsc" && $0.from == sourceExifID
        }) {
            outputRefs.append(ISOBMFFIRefEntry(
                type: "cdsc", from: sourceExifID, to: [source.primaryID, source.toneMapID]
            ))
        }
        let outputIREF = makeIrefFullBox(version: source.refsVersion, refs: outputRefs)

        var idatPayload = source.idat.map {
            source.data.subdata(in: $0.dataStart..<$0.dataEnd)
        } ?? Data()
        var idatLocations: [Int: (offset: Int, length: Int)] = [:]
        for record in records where record.constructionMethod == 1 {
            let offset = idatPayload.count
            idatPayload.append(record.payload)
            idatLocations[record.newID] = (offset, record.payload.count)
        }
        let outputIDAT = makeBox("idat", payload: idatPayload)

        func buildMeta(_ locations: [ISOBMFFILocEntry]) -> Data {
            let outputILOC = makeIlocV1Box(entries: locations)
            var metaPayload = source.data.subdata(
                in: source.meta.dataStart..<source.meta.dataStart + 4
            )
            var emittedIREF = false
            var emittedIDAT = false
            for child in source.children {
                switch child.type {
                case "iinf": metaPayload.append(outputIINF)
                case "iloc": metaPayload.append(outputILOC)
                case "iref": metaPayload.append(outputIREF); emittedIREF = true
                case "iprp": metaPayload.append(outputIPRP)
                case "idat": metaPayload.append(outputIDAT); emittedIDAT = true
                default:
                    metaPayload.append(source.data.subdata(in: child.boxStart..<child.boxStart + child.size))
                }
            }
            if !emittedIREF { metaPayload.append(outputIREF) }
            if !emittedIDAT { metaPayload.append(outputIDAT) }
            return makeBox("meta", payload: metaPayload)
        }

        var placeholders = source.locations.filter { $0.itemID != sourceExifID }
        placeholders.append(ISOBMFFILocEntry(
            itemID: sourceExifID, constructionMethod: 0, dataReferenceIndex: 0,
            extents: [(0, scaffoldExifPayload.count)]
        ))
        for record in records {
            let extent = record.constructionMethod == 1
                ? idatLocations[record.newID]!
                : (offset: 0, length: record.payload.count)
            placeholders.append(ISOBMFFILocEntry(
                itemID: record.newID,
                constructionMethod: record.constructionMethod,
                dataReferenceIndex: 0,
                extents: [extent]
            ))
        }
        let preliminaryMeta = buildMeta(placeholders)
        var prefixByteCount = 0
        for box in source.top {
            if box.boxStart == source.mdat.boxStart { break }
            prefixByteCount += box.boxStart == source.meta.boxStart ? preliminaryMeta.count : box.size
        }
        let newMdatDataStart = prefixByteCount + 8
        let fileDelta = newMdatDataStart - source.mdat.dataStart
        let sourceMdatPayload = source.data.subdata(in: source.mdat.dataStart..<source.mdat.dataEnd)
        var finalLocations = source.locations.compactMap { entry -> ISOBMFFILocEntry? in
            guard entry.itemID != sourceExifID else { return nil }
            let extents = entry.extents.map { extent -> (offset: Int, length: Int) in
                let shift = entry.constructionMethod == 0
                    && extent.offset >= source.mdat.dataStart
                    && extent.offset < source.mdat.dataEnd
                return (extent.offset + (shift ? fileDelta : 0), extent.length)
            }
            return ISOBMFFILocEntry(
                itemID: entry.itemID,
                constructionMethod: entry.constructionMethod,
                dataReferenceIndex: entry.dataReferenceIndex,
                extents: extents
            )
        }
        var appendedMdat = Data()
        let newExifOffset = newMdatDataStart + sourceMdatPayload.count
        appendedMdat.append(scaffoldExifPayload)
        finalLocations.append(ISOBMFFILocEntry(
            itemID: sourceExifID, constructionMethod: 0, dataReferenceIndex: 0,
            extents: [(newExifOffset, scaffoldExifPayload.count)]
        ))
        for record in records {
            if record.constructionMethod == 0 {
                let offset = newMdatDataStart + sourceMdatPayload.count + appendedMdat.count
                appendedMdat.append(record.payload)
                finalLocations.append(ISOBMFFILocEntry(
                    itemID: record.newID, constructionMethod: 0, dataReferenceIndex: 0,
                    extents: [(offset, record.payload.count)]
                ))
            } else {
                let extent = idatLocations[record.newID]!
                finalLocations.append(ISOBMFFILocEntry(
                    itemID: record.newID, constructionMethod: 1, dataReferenceIndex: 0,
                    extents: [extent]
                ))
            }
        }
        let finalMeta = buildMeta(finalLocations)
        guard finalMeta.count == preliminaryMeta.count else {
            throw CLIError.invalidContainer("semantic merge meta layout was not size stable")
        }
        var finalMdatPayload = sourceMdatPayload
        finalMdatPayload.append(appendedMdat)
        let outputMdat = makeBox("mdat", payload: finalMdatPayload)
        var output = Data()
        for box in source.top {
            if box.boxStart == source.meta.boxStart {
                output.append(finalMeta)
            } else if box.boxStart == source.mdat.boxStart {
                output.append(outputMdat)
            } else {
                output.append(source.data.subdata(in: box.boxStart..<box.boxStart + box.size))
            }
        }
        try output.write(to: outputURL, options: .atomic)
        let written = try Data(contentsOf: outputURL, options: [.mappedIfSafe])
        let writtenTop = isobmffBoxes(in: written, start: 0, end: written.count)
        guard let writtenMdat = writtenTop.first(where: { $0.type == "mdat" }),
              sha256Hex(sourceMdatPayload) == sha256Hex(written.subdata(
                  in: writtenMdat.dataStart..<writtenMdat.dataStart + sourceMdatPayload.count
              )),
              let imageSource = CGImageSourceCreateWithURL(outputURL as CFURL, nil),
              CGImageSourceCopyAuxiliaryDataInfoAtIndex(
                  imageSource, 0, kCGImageAuxiliaryDataTypeISOGainMap
              ) != nil else {
            throw CLIError.invalidContainer("semantic merge changed HDR data or lost its ISO Gain Map")
        }
        func auxiliaryType(for role: AppleSemanticRole) -> CFString {
            switch role {
            case .person: return kCGImageAuxiliaryDataTypePortraitEffectsMatte
            case .skin: return kCGImageAuxiliaryDataTypeSemanticSegmentationSkinMatte
            case .hair: return kCGImageAuxiliaryDataTypeSemanticSegmentationHairMatte
            case .teeth: return kCGImageAuxiliaryDataTypeSemanticSegmentationTeethMatte
            case .glasses: return kCGImageAuxiliaryDataTypeSemanticSegmentationGlassesMatte
            case .sky: return kCGImageAuxiliaryDataTypeSemanticSegmentationSkyMatte
            }
        }
        for role in profile.orderedRoles {
            guard CGImageSourceCopyAuxiliaryDataInfoAtIndex(
                imageSource, 0, auxiliaryType(for: role)
            ) != nil else {
                throw CLIError.invalidContainer(
                    "semantic merge did not preserve expected \(role.rawValue) matte"
                )
            }
        }
    }

    private static func writeIncrementalStylesGraph(
        sourceURL: URL,
        outputURL: URL,
        payload: ApplePhotographicStylePayload
    ) throws -> GraphWriteResult {
        let source = try Data(contentsOf: sourceURL, options: [.mappedIfSafe])
        let top = isobmffBoxes(in: source, start: 0, end: source.count)
        guard let meta = top.first(where: { $0.type == "meta" }),
              let mdat = top.first(where: { $0.type == "mdat" }) else {
            throw CLIError.invalidContainer("Photographic Styles writer requires meta and mdat boxes")
        }
        let children = isobmffBoxes(in: source, start: meta.dataStart + 4, end: meta.dataEnd)
        func required(_ type: String) throws -> ISOBMFFBox {
            guard let box = children.first(where: { $0.type == type }) else {
                throw CLIError.invalidContainer("Photographic Styles writer requires meta/\(type)")
            }
            return box
        }
        let iinf = try required("iinf")
        let iloc = try required("iloc")
        let pitm = try required("pitm")
        let iprp = try required("iprp")
        let sourceIDAT = children.first(where: { $0.type == "idat" })
        let sourceIREF = children.first(where: { $0.type == "iref" })
        let primaryID = parseISOBMFFPITM(source, pitm)
        let itemInfo = parseISOBMFFItemInfos(source, iinf)
        let ilocEntries = try parseISOBMFFILoc(source, iloc)
        let refsInfo = parseISOBMFFIRefs(source, sourceIREF)
        guard let tmapID = itemInfo.items.first(where: { $0.type == "tmap" })?.itemID else {
            throw CLIError.invalidContainer("Photographic Styles writer cannot locate ISO tmap item")
        }
        let gainMapID = refsInfo.refs.first(where: {
            $0.type == "dimg" && $0.from == tmapID
        })?.to.first(where: { $0 != primaryID })
            ?? refsInfo.refs.first(where: {
                $0.type == "auxl" && $0.to.contains(primaryID) && $0.to.contains(tmapID)
            })?.from
        guard let gainMapID else {
            throw CLIError.invalidContainer("Photographic Styles writer cannot locate HDR Gain Map item")
        }
        guard let ipmaBox = isobmffBoxes(
            in: source, start: iprp.dataStart, end: iprp.dataEnd
        ).first(where: { $0.type == "ipma" }) else {
            throw CLIError.invalidContainer("Photographic Styles writer requires ipma")
        }
        let sourceIPMA = parseISOBMFFIPMA(source, ipmaBox)
        let properties = try parseISOBMFFIPCOPropertyInfos(source, iprp)
        let propertyByIndex = Dictionary(uniqueKeysWithValues: properties.map { ($0.index, $0) })
        let primaryColorIndex = sourceIPMA.entries.first(where: { $0.itemID == primaryID })?
            .associations
            .map { assocPropertyIndex($0, flags: sourceIPMA.flags) }
            .first(where: { propertyByIndex[$0]?.type == "colr" })

        let entityGroupIDs: [Int] = children
            .filter { $0.type == "grpl" }
            .flatMap { groupContainer in
                isobmffBoxes(
                    in: source,
                    start: groupContainer.dataStart,
                    end: groupContainer.dataEnd
                ).compactMap { group -> Int? in
                    guard group.dataEnd - group.dataStart >= 8 else { return nil }
                    return readUInt32BEUnchecked(source, at: group.dataStart + 4)
                }
            }
        let existingMaximumID = max(
            primaryID,
            max(
                itemInfo.items.map(\.itemID).max() ?? 0,
                max(ilocEntries.map(\.itemID).max() ?? 0, entityGroupIDs.max() ?? 0)
            )
        )
        let tileCount = payload.styleDeltaRows * payload.styleDeltaColumns
        guard tileCount == 30, existingMaximumID + tileCount + 3 <= 65_535 else {
            throw CLIError.invalidContainer("unsupported Style Delta grid or HEIF item ID range")
        }
        let deltaTileIDs = Array((existingMaximumID + 1)...(existingMaximumID + tileCount))
        let deltaGridID = existingMaximumID + tileCount + 1
        let linearThumbnailID = deltaGridID + 1
        let styleMetadataID = linearThumbnailID + 1

        var ipcoPayload = Data()
        for property in properties { ipcoPayload.append(property.rawBox) }
        func appendProperty(_ box: Data) -> Int {
            ipcoPayload.append(box)
            return properties.count + isobmffBoxes(
                in: ipcoPayload, start: 0, end: ipcoPayload.count
            ).count - properties.count
        }
        // appendProperty's parse count is deliberately based on full raw boxes so each
        // resulting index remains stable even if an ICC profile is large.
        let deltaHVCCIndex = appendProperty(makeBox("hvcC", payload: payload.styleDeltaHVCC))
        let deltaTileISPEIndex = appendProperty(makeIspeBox(
            width: payload.styleDeltaTileWidth,
            height: payload.styleDeltaTileHeight
        ))
        let pixi10Index = appendProperty(makePixiBox(bits: [10, 10, 10]))
        let deltaGridISPEIndex = appendProperty(makeIspeBox(
            width: payload.styleDeltaGridWidth,
            height: payload.styleDeltaGridHeight
        ))
        let deltaAuxCIndex = appendProperty(makeAuxCBox(
            "tag:apple.com,2023:photo:aux:styledeltamap"
        ))
        let identityIrotIndex = appendProperty(makeIrotBox())
        let linearHVCCIndex = appendProperty(makeBox("hvcC", payload: payload.linearThumbnailHVCC))
        let linearISPEIndex = appendProperty(makeIspeBox(
            width: payload.linearThumbnailWidth,
            height: payload.linearThumbnailHeight
        ))
        guard let linearP3 = CGColorSpace(name: CGColorSpace.extendedLinearDisplayP3) else {
            throw CLIError.invalidContainer("extended linear Display P3 is unavailable")
        }
        let linearColorIndex = appendProperty(try makeICCColorBox(linearP3))
        let linearAuxCIndex = appendProperty(makeAuxCBox(
            "tag:apple.com,2023:photo:aux:linearthumbnail"
        ))
        let deltaColorIndex = primaryColorIndex ?? linearColorIndex

        let outputIPMAFlags = sourceIPMA.flags
        var ipmaEntriesPayload = Data()
        var ipmaEntryCount = 0
        for entry in sourceIPMA.entries {
            let associations = assocPairs(entry.associations, flags: sourceIPMA.flags)
            ipmaEntriesPayload.append(try makeIPMAEntry(
                entry.itemID,
                associations,
                flags: outputIPMAFlags,
                version: sourceIPMA.version
            ))
            ipmaEntryCount += 1
        }
        for tileID in deltaTileIDs {
            ipmaEntriesPayload.append(try makeIPMAEntry(
                tileID,
                [(deltaTileISPEIndex, true), (deltaColorIndex, true), (deltaHVCCIndex, true)],
                flags: outputIPMAFlags,
                version: sourceIPMA.version
            ))
            ipmaEntryCount += 1
        }
        ipmaEntriesPayload.append(try makeIPMAEntry(
            deltaGridID,
            [
                (deltaColorIndex, true), (deltaGridISPEIndex, false), (pixi10Index, false),
                (deltaAuxCIndex, true), (identityIrotIndex, true),
            ],
            flags: outputIPMAFlags,
            version: sourceIPMA.version
        ))
        ipmaEntryCount += 1
        ipmaEntriesPayload.append(try makeIPMAEntry(
            linearThumbnailID,
            [
                (linearISPEIndex, true), (linearColorIndex, true), (pixi10Index, false),
                (linearHVCCIndex, true), (linearAuxCIndex, true), (identityIrotIndex, true),
            ],
            flags: outputIPMAFlags,
            version: sourceIPMA.version
        ))
        ipmaEntryCount += 1
        var ipmaPayload = Data([
            sourceIPMA.version,
            UInt8((outputIPMAFlags >> 16) & 0xff),
            UInt8((outputIPMAFlags >> 8) & 0xff),
            UInt8(outputIPMAFlags & 0xff),
        ])
        appendUInt32BE(ipmaEntryCount, to: &ipmaPayload)
        ipmaPayload.append(ipmaEntriesPayload)
        var outputIPRPPayload = Data()
        outputIPRPPayload.append(makeBox("ipco", payload: ipcoPayload))
        outputIPRPPayload.append(makeBox("ipma", payload: ipmaPayload))
        for child in isobmffBoxes(in: source, start: iprp.dataStart, end: iprp.dataEnd)
            where child.type != "ipco" && child.type != "ipma" {
            outputIPRPPayload.append(source.subdata(in: child.boxStart..<child.boxStart + child.size))
        }
        let outputIPRP = makeBox("iprp", payload: outputIPRPPayload)

        var rawInfes = itemInfo.items.map(\.rawInfe)
        for tileID in deltaTileIDs {
            rawInfes.append(makeInfeBox(itemID: tileID, type: "hvc1", flags: 1))
        }
        rawInfes.append(makeInfeBox(itemID: deltaGridID, type: "grid", flags: 1))
        rawInfes.append(makeInfeBox(itemID: linearThumbnailID, type: "hvc1", flags: 1))
        rawInfes.append(makeURIInfeBox(
            itemID: styleMetadataID,
            name: "styleMetadata",
            uri: "tag:apple.com,2023:photo:metadata:styles"
        ))
        let outputIINF = makeIinfBox(version: itemInfo.version, rawInfes: rawInfes)

        var refs = refsInfo.refs
        refs.append(ISOBMFFIRefEntry(type: "dimg", from: deltaGridID, to: deltaTileIDs))
        refs.append(ISOBMFFIRefEntry(type: "auxl", from: deltaGridID, to: [primaryID, tmapID]))
        refs.append(ISOBMFFIRefEntry(type: "auxl", from: linearThumbnailID, to: [primaryID, tmapID]))
        refs.append(ISOBMFFIRefEntry(type: "cdsc", from: styleMetadataID, to: [primaryID, tmapID]))
        let outputIREF = makeIrefFullBox(version: refsInfo.version, refs: refs)

        var idatPayload = sourceIDAT.map {
            source.subdata(in: $0.dataStart..<$0.dataEnd)
        } ?? Data()
        let deltaGridOffset = idatPayload.count
        let deltaGridPayload = try makeGridPayload(
            rows: payload.styleDeltaRows,
            columns: payload.styleDeltaColumns,
            width: payload.styleDeltaGridWidth,
            height: payload.styleDeltaGridHeight
        )
        idatPayload.append(deltaGridPayload)
        let styleMetadataOffset = idatPayload.count
        idatPayload.append(payload.stylePropertyList)
        let outputIDAT = makeBox("idat", payload: idatPayload)

        func buildMeta(_ locations: [ISOBMFFILocEntry]) -> Data {
            let outputILOC = makeIlocV1Box(entries: locations)
            var metaPayload = source.subdata(in: meta.dataStart..<meta.dataStart + 4)
            var emittedIREF = false
            var emittedIDAT = false
            for child in children {
                switch child.type {
                case "iinf": metaPayload.append(outputIINF)
                case "iloc": metaPayload.append(outputILOC)
                case "iref": metaPayload.append(outputIREF); emittedIREF = true
                case "iprp": metaPayload.append(outputIPRP)
                case "idat": metaPayload.append(outputIDAT); emittedIDAT = true
                default:
                    metaPayload.append(source.subdata(in: child.boxStart..<child.boxStart + child.size))
                }
            }
            if !emittedIREF { metaPayload.append(outputIREF) }
            if !emittedIDAT { metaPayload.append(outputIDAT) }
            return makeBox("meta", payload: metaPayload)
        }

        var placeholders = ilocEntries
        for tileID in deltaTileIDs {
            placeholders.append(ISOBMFFILocEntry(
                itemID: tileID, constructionMethod: 0, dataReferenceIndex: 0,
                extents: [(0, payload.styleDeltaHEVC.count)]
            ))
        }
        placeholders.append(ISOBMFFILocEntry(
            itemID: deltaGridID, constructionMethod: 1, dataReferenceIndex: 0,
            extents: [(deltaGridOffset, deltaGridPayload.count)]
        ))
        placeholders.append(ISOBMFFILocEntry(
            itemID: linearThumbnailID, constructionMethod: 0, dataReferenceIndex: 0,
            extents: [(0, payload.linearThumbnailHEVC.count)]
        ))
        placeholders.append(ISOBMFFILocEntry(
            itemID: styleMetadataID, constructionMethod: 1, dataReferenceIndex: 0,
            extents: [(styleMetadataOffset, payload.stylePropertyList.count)]
        ))
        let preliminaryMeta = buildMeta(placeholders)
        var prefixByteCount = 0
        for box in top {
            if box.boxStart == mdat.boxStart { break }
            prefixByteCount += box.boxStart == meta.boxStart ? preliminaryMeta.count : box.size
        }
        let newMdatDataStart = prefixByteCount + 8
        let fileDelta = newMdatDataStart - mdat.dataStart
        let sourceMdatPayload = source.subdata(in: mdat.dataStart..<mdat.dataEnd)
        var finalLocations = ilocEntries.map { entry -> ISOBMFFILocEntry in
            let extents = entry.extents.map { extent -> (offset: Int, length: Int) in
                let shouldShift = entry.constructionMethod == 0
                    && extent.offset >= mdat.dataStart
                    && extent.offset < mdat.dataEnd
                return (extent.offset + (shouldShift ? fileDelta : 0), extent.length)
            }
            return ISOBMFFILocEntry(
                itemID: entry.itemID,
                constructionMethod: entry.constructionMethod,
                dataReferenceIndex: entry.dataReferenceIndex,
                extents: extents
            )
        }
        var appendedMdat = Data()
        for tileID in deltaTileIDs {
            let offset = newMdatDataStart + sourceMdatPayload.count + appendedMdat.count
            appendedMdat.append(payload.styleDeltaHEVC)
            finalLocations.append(ISOBMFFILocEntry(
                itemID: tileID, constructionMethod: 0, dataReferenceIndex: 0,
                extents: [(offset, payload.styleDeltaHEVC.count)]
            ))
        }
        finalLocations.append(ISOBMFFILocEntry(
            itemID: deltaGridID, constructionMethod: 1, dataReferenceIndex: 0,
            extents: [(deltaGridOffset, deltaGridPayload.count)]
        ))
        let linearOffset = newMdatDataStart + sourceMdatPayload.count + appendedMdat.count
        appendedMdat.append(payload.linearThumbnailHEVC)
        finalLocations.append(ISOBMFFILocEntry(
            itemID: linearThumbnailID, constructionMethod: 0, dataReferenceIndex: 0,
            extents: [(linearOffset, payload.linearThumbnailHEVC.count)]
        ))
        finalLocations.append(ISOBMFFILocEntry(
            itemID: styleMetadataID, constructionMethod: 1, dataReferenceIndex: 0,
            extents: [(styleMetadataOffset, payload.stylePropertyList.count)]
        ))
        let finalMeta = buildMeta(finalLocations)
        guard finalMeta.count == preliminaryMeta.count else {
            throw CLIError.invalidContainer("Photographic Styles meta layout was not size stable")
        }
        var finalMdatPayload = sourceMdatPayload
        finalMdatPayload.append(appendedMdat)
        let finalMdat = makeBox("mdat", payload: finalMdatPayload)
        var output = Data()
        for box in top {
            if box.boxStart == meta.boxStart {
                output.append(finalMeta)
            } else if box.boxStart == mdat.boxStart {
                output.append(finalMdat)
            } else {
                output.append(source.subdata(in: box.boxStart..<box.boxStart + box.size))
            }
        }
        try output.write(to: outputURL, options: .atomic)
        let outputPrefix = finalMdatPayload.prefix(sourceMdatPayload.count)
        let sourceMdatSHA = sha256Hex(sourceMdatPayload)
        let outputPrefixSHA = sha256Hex(Data(outputPrefix))
        guard sourceMdatSHA == outputPrefixSHA else {
            throw CLIError.invalidContainer("base/HDR Gain Map mdat payload changed while adding Styles")
        }
        return GraphWriteResult(
            primaryItemID: primaryID,
            gainMapItemID: gainMapID,
            toneMapItemID: tmapID,
            styleDeltaItemID: deltaGridID,
            linearThumbnailItemID: linearThumbnailID,
            styleMetadataItemID: styleMetadataID,
            originalMdatPayloadSHA256: sourceMdatSHA,
            outputOriginalMdatPrefixSHA256: outputPrefixSHA,
            itemCount: rawInfes.count,
            propertyCount: isobmffBoxes(in: ipcoPayload, start: 0, end: ipcoPayload.count).count
        )
    }

    private static func convert(inputURL: URL, outputURL: URL, options: Options) throws {
        guard options.features.photographicStyles else {
            throw CLIError.invalidContainer("Photographic Styles pipeline invoked without its capability flag")
        }
        options.eventHandler?(.phaseChanged(.readingSource))
        let parent = outputURL.deletingLastPathComponent()
        try ensureDirectory(parent, fileManager: .default)
        let featureInputURL = siblingScratchURL(
            for: outputURL,
            label: "apple-input",
            pathExtension: "heic"
        )
        let sharedSemanticDirectory = outputURL.deletingLastPathComponent()
            .appendingPathComponent(
                ".\(outputURL.lastPathComponent).shared-semantics-\(UUID().uuidString)",
                isDirectory: true
            )
        defer { try? FileManager.default.removeItem(at: featureInputURL) }
        defer { try? FileManager.default.removeItem(at: sharedSemanticDirectory) }

        var portraitUnavailableReason: String?
        var portraitWritten = false
        var portraitSemanticFusion: [String: Any]?
        var portraitSemanticAnalysis: AppleSemanticSceneAnalysis?
        var portraitManifestURL: URL?
        let photoIdentifier = UUID().uuidString.uppercased()
        if options.features.portrait {
            do {
                let outcome = try PortraitConversionPipeline.convertWithOutcome(
                    inputURL: inputURL,
                    outputURL: featureInputURL,
                    mode: .on,
                    photoIdentifier: photoIdentifier,
                    includesPhotographicStylesSemantics: true,
                    semanticOutputDirectory: sharedSemanticDirectory,
                    writeSemanticPNGEvidence: options.debugRootURL != nil,
                    eventHandler: AppleFeatureEventForwarder.preparationHandler(
                        options.eventHandler
                    )
                )
                portraitWritten = outcome.written
                portraitSemanticFusion = outcome.semanticFusion
                portraitSemanticAnalysis = outcome.semanticAnalysis
                portraitManifestURL = outcome.manifestURL
                if !portraitWritten {
                    portraitUnavailableReason = "required OPPO portrait depth bundle is unavailable"
                }
            } catch {
                let sourceExtension = inputURL.pathExtension.lowercased()
                if sourceExtension == "jpg" || sourceExtension == "jpeg" {
                    // JPEG enters this product path only through the validated
                    // src.image portrait bridge. Never fall back to the generic
                    // HEIC converter after that bridge fails.
                    throw error
                }
                portraitUnavailableReason = String(describing: error)
            }
        }

        if let portraitUnavailableReason {
            options.eventHandler?(.warning(ConversionWarning(
                code: .portraitUnavailable,
                messageKey: .warningPortraitUnavailable,
                diagnostics: portraitUnavailableReason
            )))
        }

        if !portraitWritten {
            _ = try XDRemuxProductCore.convert(
                inputURL: inputURL,
                outputURL: featureInputURL,
                familyPreference: options.family,
                debugRootURL: options.debugRootURL,
                oppoCompatibility: options.oppoCompatibility,
                inputProcessingBranch: options.inputProcessingBranch,
                oppoCameraTail: options.oppoCameraTail,
                tmapFormat: options.tmapFormat,
                eventHandler: AppleFeatureEventForwarder.preparationHandler(
                    options.eventHandler
                )
            )
        }

        options.eventHandler?(.phaseChanged(.generatingPhotographicStyles))
        options.eventHandler?(.phaseChanged(.writingContainer))
        try augmentPhotographicStyles(
            sourceURL: inputURL,
            standardHDRURL: featureInputURL,
            outputURL: outputURL,
            portraitRequested: options.features.portrait,
            portraitWritten: portraitWritten,
            portraitUnavailableReason: portraitUnavailableReason,
            portraitSemanticFusion: portraitSemanticFusion,
            portraitSemanticAnalysis: portraitSemanticAnalysis,
            portraitSemanticEvidenceDirectory: portraitWritten ? sharedSemanticDirectory : nil,
            preferredPhotoIdentifier: photoIdentifier,
            debugRootURL: options.debugRootURL
        )
        options.eventHandler?(.phaseChanged(.verifyingOutput))
        if let portraitManifestURL, FileManager.default.fileExists(atPath: portraitManifestURL.path) {
            let destination = outputURL.deletingPathExtension()
                .appendingPathExtension("portrait-manifest.json")
            try? FileManager.default.removeItem(at: destination)
            try FileManager.default.moveItem(at: portraitManifestURL, to: destination)
            options.eventHandler?(.diagnostic("portrait manifest=\(destination.path)"))
        }
    }

    private static func augmentPhotographicStyles(
        sourceURL: URL,
        standardHDRURL: URL,
        outputURL: URL,
        portraitRequested: Bool,
        portraitWritten: Bool,
        portraitUnavailableReason: String?,
        portraitSemanticFusion: [String: Any]?,
        portraitSemanticAnalysis: AppleSemanticSceneAnalysis?,
        portraitSemanticEvidenceDirectory: URL?,
        preferredPhotoIdentifier: String,
        debugRootURL: URL?
    ) throws {
        let augmentStartedAt = CFAbsoluteTimeGetCurrent()
        let runToken = UUID().uuidString.uppercased()
        let persistEvidence = debugRootURL != nil
        let evidenceContainer: URL
        if let debugRootURL {
            evidenceContainer = debugRootURL
                .appendingPathComponent(sourceURL.deletingPathExtension().lastPathComponent, isDirectory: true)
                .appendingPathComponent("photographic-styles", isDirectory: true)
        } else {
            evidenceContainer = FileManager.default.temporaryDirectory
                .appendingPathComponent("xdremux-photographic-styles-\(runToken)", isDirectory: true)
        }
        defer {
            if !persistEvidence {
                try? FileManager.default.removeItem(at: evidenceContainer)
            }
        }
        let evidenceDirectory = evidenceContainer
            .appendingPathComponent("runs", isDirectory: true)
            .appendingPathComponent(runToken, isDirectory: true)
        try FileManager.default.createDirectory(
            at: evidenceDirectory,
            withIntermediateDirectories: true
        )
        if let portraitSemanticFusion {
            let fusionData = try JSONSerialization.data(
                withJSONObject: portraitSemanticFusion,
                options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
            )
            try fusionData.write(
                to: evidenceDirectory.appendingPathComponent("portrait-semantic-fusion.json"),
                options: .atomic
            )
        }
        let semanticDirectory = evidenceDirectory.appendingPathComponent("semantics", isDirectory: true)
        try FileManager.default.copyItem(
            at: standardHDRURL,
            to: evidenceDirectory.appendingPathComponent("base-hdr-before-semantics.heic")
        )
        let analysis: AppleSemanticSceneAnalysis
        let semanticAnalysisSource: String
        if portraitWritten,
           let portraitSemanticAnalysis,
           let portraitSemanticEvidenceDirectory {
            try AppleSemanticSceneAnalyzer.copyEvidence(
                from: portraitSemanticEvidenceDirectory,
                to: semanticDirectory
            )
            analysis = portraitSemanticAnalysis
            semanticAnalysisSource = "portrait_shared"
            print("Vision semantic analysis source=portrait-shared; skipped duplicate Styles analysis")
        } else {
            analysis = try AppleSemanticSceneAnalyzer.analyze(
                imageURL: standardHDRURL,
                outputDirectory: semanticDirectory,
                profile: .styleHuman,
                writePNGEvidence: persistEvidence
            )
            semanticAnalysisSource = "styles"
        }
        let semanticStageSeconds = CFAbsoluteTimeGetCurrent() - augmentStartedAt
        let existingPhotoIdentifier: String? = CGImageSourceCreateWithURL(
            standardHDRURL as CFURL, nil
        ).flatMap { source in
            guard let properties = CGImageSourceCopyPropertiesAtIndex(
                source, 0, nil
            ) as? [CFString: Any],
                  let dictionary = properties[
                      kCGImagePropertyMakerAppleDictionary
                  ] as? NSDictionary else { return nil }
            return dictionary["43"] as? String ?? dictionary[43] as? String
        }
        let photoIdentifier = existingPhotoIdentifier ?? preferredPhotoIdentifier
        let scaffoldURL = outputURL.deletingLastPathComponent().appendingPathComponent(
            ".\(outputURL.lastPathComponent).semantic-scaffold-\(runToken).heic"
        )
        let semanticMergedURL = outputURL.deletingLastPathComponent().appendingPathComponent(
            ".\(outputURL.lastPathComponent).semantic-merged-\(runToken).heic"
        )
        defer { try? FileManager.default.removeItem(at: scaffoldURL) }
        defer { try? FileManager.default.removeItem(at: semanticMergedURL) }
        let semanticWriteProfile: AppleSemanticWriteProfile = portraitWritten
            ? .portraitAndStyles
            : analysis.nativeStyleWriteProfile
        let featureGraphURL: URL
        if portraitWritten {
            // The Portrait writer already authored the same six Vision resources and
            // disparity. Reusing that graph prevents duplicate semantic auxiliaries.
            featureGraphURL = standardHDRURL
        } else {
            try AppleSemanticScaffoldBuilder.write(
                sourceHDRURL: standardHDRURL,
                outputURL: scaffoldURL,
                analysis: analysis,
                profile: semanticWriteProfile,
                photoIdentifier: photoIdentifier,
                preserveOriginalBaseAndGain: false
            )
            try FileManager.default.copyItem(
                at: scaffoldURL,
                to: evidenceDirectory.appendingPathComponent("semantic-scaffold-before-styles.heic")
            )
            try mergeSemanticAuxiliaryGraph(
                sourceHDRURL: standardHDRURL,
                semanticScaffoldURL: scaffoldURL,
                outputURL: semanticMergedURL,
                profile: semanticWriteProfile
            )
            try FileManager.default.copyItem(
                at: semanticMergedURL,
                to: evidenceDirectory.appendingPathComponent("semantic-merged-base-gain-preserved.heic")
            )
            featureGraphURL = semanticMergedURL
        }
        let styleDirectory = evidenceDirectory.appendingPathComponent("styles", isDirectory: true)
        let stylePayloadStartedAt = CFAbsoluteTimeGetCurrent()
        let stylePayload = try buildStylePayload(
            sourceURL: sourceURL,
            standardHDRURL: featureGraphURL,
            semantics: analysis,
            portraitWritten: portraitWritten,
            outputDirectory: styleDirectory,
            photoIdentifier: photoIdentifier
        )
        let stylePayloadSeconds = CFAbsoluteTimeGetCurrent() - stylePayloadStartedAt
        let graphStartedAt = CFAbsoluteTimeGetCurrent()
        let graph = try writeIncrementalStylesGraph(
            sourceURL: featureGraphURL,
            outputURL: outputURL,
            payload: stylePayload
        )
        let validation = try validatePhotographicStylesOutput(
            outputURL,
            expectsPortrait: portraitWritten,
            prevalidatedStylePropertyList: stylePayload.stylePropertyList
        )
        let graphAndValidationSeconds = CFAbsoluteTimeGetCurrent() - graphStartedAt
        let totalSeconds = CFAbsoluteTimeGetCurrent() - augmentStartedAt
        print(String(
            format: "styles pipeline semanticSource=%@ semantic=%.3fs payload=%.3fs graph+validation=%.3fs total=%.3fs",
            semanticAnalysisSource,
            semanticStageSeconds,
            stylePayloadSeconds,
            graphAndValidationSeconds,
            totalSeconds
        ))

        func matteSummary(_ matte: AppleSemanticMatte?) -> [String: Any] {
            guard let matte else { return ["available": false] }
            return [
                "available": true,
                "requestClass": matte.provenance.requestClass,
                "attributeName": matte.provenance.attributeName,
                "revision": matte.provenance.revision,
                "inputSHA256": matte.provenance.inputSHA256,
                "width": matte.width,
                "height": matte.height,
                "pixelFormat": matte.provenance.pixelFormat,
                "orientation": matte.provenance.orientation,
                "orientationTransform": matte.provenance.orientationTransform,
                "fallback": matte.provenance.fallback,
                "minimum": matte.statistics.minimum,
                "maximum": matte.statistics.maximum,
                "mean": matte.statistics.mean,
                "coverage": matte.statistics.coverage,
                "rawSHA256": sha256Hex(matte.pixels),
            ]
        }
        let outputData = validation.outputData
        let contaminationReport = validation.contaminationReport
        let contaminationReportData = try JSONSerialization.data(
            withJSONObject: contaminationReport,
            options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        )
        try contaminationReportData.write(
            to: evidenceDirectory.appendingPathComponent("donor-contamination.json"),
            options: .atomic
        )
        let graphManifest: [String: Any] = [
            "primaryItemID": graph.primaryItemID,
            "gainMapItemID": graph.gainMapItemID,
            "toneMapItemID": graph.toneMapItemID,
            "styleDeltaItemID": graph.styleDeltaItemID,
            "linearThumbnailItemID": graph.linearThumbnailItemID,
            "styleMetadataItemID": graph.styleMetadataItemID,
            "itemCount": graph.itemCount,
            "propertyCount": graph.propertyCount,
            "primaryAndGainEncodedOnce": graph.originalMdatPayloadSHA256 == graph.outputOriginalMdatPrefixSHA256,
            "originalMdatPayloadSHA256": graph.originalMdatPayloadSHA256,
            "outputOriginalMdatPrefixSHA256": graph.outputOriginalMdatPrefixSHA256,
        ]
        let configuration: [String: Any] = [
            "applePhotographicStyles": true,
            "applePortraitRequested": portraitRequested,
            "applePortraitWritten": portraitWritten,
            "applePortraitUnavailableReason": portraitUnavailableReason.map { $0 as Any } ?? NSNull(),
            "styleProfile": "iPhone18,1/23F84-zero-residual",
            "debugRoot": debugRootURL.map { $0.path as Any } ?? NSNull(),
        ]
        let manifest: [String: Any] = [
            "schema": "xdremux-apple-feature-conversion-v1",
            "runIdentifier": runToken,
            "input": [
                "path": sourceURL.path,
                "sha256": sha256Hex(try Data(contentsOf: sourceURL, options: [.mappedIfSafe])),
            ],
            "output": [
                "path": outputURL.path,
                "sha256": sha256Hex(outputData),
                "byteCount": outputData.count,
            ],
            "configuration": configuration,
            "photoIdentifier": photoIdentifier,
            "semanticWriteProfile": [
                "kind": semanticWriteProfile.kind.rawValue,
                "roles": semanticWriteProfile.orderedRoles.map(\.rawValue),
                "nativeEvidence": "style sky-only; style human PEM+skin+sky; portrait family PEM+skin+hair+teeth+glasses",
            ],
            "semanticAnalysisSource": semanticAnalysisSource,
            "timingsSeconds": [
                "semantic": semanticStageSeconds,
                "stylePayload": stylePayloadSeconds,
                "graphAndValidation": graphAndValidationSeconds,
                "total": totalSeconds,
            ],
            "semantics": [
                "person": matteSummary(analysis.person),
                "skin": matteSummary(analysis.skin),
                "hair": matteSummary(analysis.hair),
                "teeth": matteSummary(analysis.teeth),
                "glasses": matteSummary(analysis.glasses),
                "sky": matteSummary(analysis.sky),
            ],
            "portraitSemanticFusion": portraitSemanticFusion.map { $0 as Any } ?? NSNull(),
            "stylePayloadManifestSHA256": sha256Hex(stylePayload.manifestJSON),
            "heifGraph": graphManifest,
            "donorPolicy": [
                "shellCopied": false,
                "scenePayloadCopied": false,
                "styleDataSource": "deterministic complete identity derived from verified CMImaging layout",
                "linearThumbnailSource": "same-input coherent HDR reconstruction",
                "styleDeltaSource": "profile-scoped neutral protocol tuning",
            ],
            "donorContamination": contaminationReport,
        ]
        let manifestData = try JSONSerialization.data(
            withJSONObject: manifest,
            options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        )
        try manifestData.write(
            to: evidenceDirectory.appendingPathComponent("manifest.json"),
            options: .atomic
        )
        try stylePayload.manifestJSON.write(
            to: evidenceDirectory.appendingPathComponent("style-payload-manifest.json"),
            options: .atomic
        )
        let latest: [String: Any] = [
            "runIdentifier": runToken,
            "manifest": evidenceDirectory.appendingPathComponent("manifest.json").path,
            "outputSHA256": sha256Hex(outputData),
        ]
        if persistEvidence {
            try JSONSerialization.data(
                withJSONObject: latest,
                options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
            ).write(to: evidenceContainer.appendingPathComponent("latest.json"), options: .atomic)
        }
    }

    private struct StylesValidationResult {
        let outputData: Data
        let contaminationReport: [String: Any]
    }

    private static func validatePhotographicStylesOutput(
        _ outputURL: URL,
        expectsPortrait: Bool,
        prevalidatedStylePropertyList: Data? = nil
    ) throws -> StylesValidationResult {
        let data = try Data(contentsOf: outputURL, options: [.mappedIfSafe])
        let top = isobmffBoxes(in: data, start: 0, end: data.count)
        guard let meta = top.first(where: { $0.type == "meta" }) else {
            throw CLIError.invalidContainer("Styles validation: meta is missing")
        }
        let children = isobmffBoxes(in: data, start: meta.dataStart + 4, end: meta.dataEnd)
        guard let iinf = children.first(where: { $0.type == "iinf" }),
              let iloc = children.first(where: { $0.type == "iloc" }),
              let pitm = children.first(where: { $0.type == "pitm" }),
              let iref = children.first(where: { $0.type == "iref" }) else {
            throw CLIError.invalidContainer("Styles validation: required item graph boxes are missing")
        }
        let idat = children.first(where: { $0.type == "idat" })
        let primaryID = parseISOBMFFPITM(data, pitm)
        let items = parseISOBMFFItemInfos(data, iinf).items
        let locations = try parseISOBMFFILoc(data, iloc)
        let locationByID = Dictionary(uniqueKeysWithValues: locations.map { ($0.itemID, $0) })
        let refs = parseISOBMFFIRefs(data, iref).refs
        guard let tmapID = items.first(where: { $0.type == "tmap" })?.itemID,
              let styleMetadataID = items.first(where: { item in
                  item.type == "uri " && item.rawInfe.range(
                      of: Data("tag:apple.com,2023:photo:metadata:styles".utf8)
                  ) != nil
              })?.itemID,
              let styleLocation = locationByID[styleMetadataID],
              refs.contains(where: {
                  $0.type == "cdsc" && $0.from == styleMetadataID
                      && Set($0.to) == Set([primaryID, tmapID])
              }) else {
            throw CLIError.invalidContainer("Styles validation: style uri item or cdsc reference is missing")
        }
        let styleData = try itemPayload(in: data, entry: styleLocation, idat: idat)
        guard styleData.starts(with: Data("bplist00".utf8)),
              let object = try PropertyListSerialization.propertyList(
                  from: styleData, options: [], format: nil
              ) as? [String: Any],
              (object["0"] as? NSNumber)?.intValue == 15,
              let coefficients = object["1"] as? Data,
              coefficients.count == 51_840,
              object["2"] as? Bool == true,
              (object["3"] as? Data)?.count == 516,
              (object["c"] as? Data)?.count == 2_048,
              (object["d"] as? Data)?.count == 2_048 else {
            throw CLIError.invalidContainer("Styles validation: binary plist contract is incomplete")
        }
        if prevalidatedStylePropertyList != styleData {
            let parserDirectory = FileManager.default.temporaryDirectory
                .appendingPathComponent("xdremux-style-parser-\(UUID().uuidString)", isDirectory: true)
            try ensureDirectory(parserDirectory, fileManager: .default)
            defer { try? FileManager.default.removeItem(at: parserDirectory) }
            _ = try validateWithSemanticStyleProperties(
                stylePropertyList: styleData,
                expectedStyleData: coefficients,
                outputDirectory: parserDirectory
            )
        }
        let deltaGrid = refs.first(where: { ref in
            ref.type == "dimg" && ref.to.count == 30
                && items.first(where: { $0.itemID == ref.from })?.type == "grid"
        })
        guard let deltaGrid,
              refs.contains(where: {
                  $0.type == "auxl" && $0.from == deltaGrid.from
                      && Set($0.to) == Set([primaryID, tmapID])
              }),
              data.range(of: Data("tag:apple.com,2023:photo:aux:styledeltamap".utf8)) != nil,
              data.range(of: Data("tag:apple.com,2023:photo:aux:linearthumbnail".utf8)) != nil else {
            throw CLIError.invalidContainer("Styles validation: auxiliary item graph is incomplete")
        }
        let linearCandidates = refs.filter { ref in
            ref.type == "auxl" && Set(ref.to) == Set([primaryID, tmapID])
                && items.first(where: { $0.itemID == ref.from })?.type == "hvc1"
        }
        guard !linearCandidates.isEmpty else {
            throw CLIError.invalidContainer("Styles validation: Linear Thumbnail auxl is missing")
        }
        try verifyImageIOISOGainMap(outputURL)
        guard let imageSource = CGImageSourceCreateWithURL(outputURL as CFURL, nil),
              CGImageSourceCreateImageAtIndex(imageSource, 0, nil) != nil,
              CGImageSourceCopyAuxiliaryDataInfoAtIndex(
                  imageSource, 0, kCGImageAuxiliaryDataTypeSemanticSegmentationSkyMatte
              ) != nil else {
            throw CLIError.invalidContainer(
                "Styles validation: primary or native style sky matte is not decodable"
            )
        }
        if expectsPortrait {
            guard PortraitConversionPipeline.isValidOutput(outputURL) else {
                throw CLIError.invalidContainer("Styles validation: combined Portrait resources are incomplete")
            }
        } else {
            let person = CGImageSourceCopyAuxiliaryDataInfoAtIndex(
                imageSource, 0, kCGImageAuxiliaryDataTypePortraitEffectsMatte
            )
            let skin = CGImageSourceCopyAuxiliaryDataInfoAtIndex(
                imageSource, 0, kCGImageAuxiliaryDataTypeSemanticSegmentationSkinMatte
            )
            let hasUnexpectedFullPortraitRole = [
                kCGImageAuxiliaryDataTypeSemanticSegmentationHairMatte,
                kCGImageAuxiliaryDataTypeSemanticSegmentationTeethMatte,
                kCGImageAuxiliaryDataTypeSemanticSegmentationGlassesMatte,
            ].contains { type in
                CGImageSourceCopyAuxiliaryDataInfoAtIndex(imageSource, 0, type) != nil
            }
            guard (person == nil) == (skin == nil), !hasUnexpectedFullPortraitRole else {
                throw CLIError.invalidContainer(
                    "Styles validation: styles-only semantics must be sky-only or PEM+skin+sky"
                )
            }
        }
        let contamination = try donorContaminationScan(data: data, items: items, locations: locations, idat: idat)
        guard contamination.matches.isEmpty else {
            throw CLIError.invalidContainer(
                "donor contamination scanner matched: \(contamination.matches.joined(separator: ", "))"
            )
        }
        return StylesValidationResult(
            outputData: data,
            contaminationReport: donorContaminationReport(
                data: data,
                matches: contamination.matches,
                scannedItemCount: contamination.scannedItemCount
            )
        )
    }

    private static func donorContaminationScan(
        data: Data,
        items: [ISOBMFFItemInfo],
        locations: [ISOBMFFILocEntry],
        idat: ISOBMFFBox?
    ) throws -> (matches: [String], scannedItemCount: Int) {
        let knownPayloadSHA256: Set<String> = [
            // Persisted research-corpus donor scene resources. Constants are hashes only;
            // the product never opens or references the donor files themselves.
            "d3d468a711a21a591198aee1ef575309719256acf2b0d66468e621521687c551",
            "7b78d1cd56c175d4bbc0d5cbead9108a95086e8dfd6d3021f3041b6f875ac1ec",
            "ebe6edffebdf31be41591f6d43dad6621ae3103a229039dda2eea91607462787",
            "24288d4bc7c68ff6d75df51948da4d2fe0fa129847b46cd171dc5d818b462ae2",
            "732c0893d0e76e2d67cc15a56f13420ca7b4b050fa7f936e9a6f268c6dc1b983",
            "a0907e4e23ebb919cf817e55ea845a0e2a4c5a19cd617c4adad9b8dfa0162e58",
            "3f82ff9619e55b889bcadb78b8a5705ac87d393c5590d29ee75ebc91c1d98e6e",
            "166af88ec34efa4a188a003201c6afacbc7725a579036e7d9ab1adc5a20b56ed",
            "348f108bd9735c138c5047ddd11170b1fa50c92c899f196a112cf31645623fb8",
        ]
        var matches: [String] = []
        let itemByID = Dictionary(uniqueKeysWithValues: items.map { ($0.itemID, $0) })
        for location in locations {
            let bytes = try itemPayload(in: data, entry: location, idat: idat)
            let digest = sha256Hex(bytes)
            if knownPayloadSHA256.contains(digest) {
                let type = itemByID[location.itemID]?.type ?? "unknown"
                matches.append("item \(location.itemID) \(type) sha256=\(digest)")
            }
            if itemByID[location.itemID]?.type == "uri ",
               let object = try? PropertyListSerialization.propertyList(
                   from: bytes, options: [], format: nil
               ) as? [String: Any] {
                for key in ["1", "3", "c", "d"] {
                    if let blob = object[key] as? Data {
                        let blobDigest = sha256Hex(blob)
                        if knownPayloadSHA256.contains(blobDigest) {
                            matches.append("style key \(key) sha256=\(blobDigest)")
                        }
                    }
                }
            }
        }
        for identifier in [
            "8E9F338B-51CA-4903-888E-6CEAF5EC8C50",
        ] where data.range(of: Data(identifier.utf8)) != nil {
            matches.append("known donor PhotoIdentifier \(identifier)")
        }
        return (matches.sorted(), locations.count)
    }

    static func donorContaminationReport(for outputURL: URL) throws -> [String: Any] {
        let data = try Data(contentsOf: outputURL, options: [.mappedIfSafe])
        let top = isobmffBoxes(in: data, start: 0, end: data.count)
        guard let meta = top.first(where: { $0.type == "meta" }) else {
            throw CLIError.invalidContainer("donor scanner cannot locate meta")
        }
        let children = isobmffBoxes(in: data, start: meta.dataStart + 4, end: meta.dataEnd)
        guard let iinf = children.first(where: { $0.type == "iinf" }),
              let iloc = children.first(where: { $0.type == "iloc" }) else {
            throw CLIError.invalidContainer("donor scanner cannot locate item tables")
        }
        let result = try donorContaminationScan(
            data: data,
            items: parseISOBMFFItemInfos(data, iinf).items,
            locations: parseISOBMFFILoc(data, iloc),
            idat: children.first(where: { $0.type == "idat" })
        )
        return donorContaminationReport(
            data: data,
            matches: result.matches,
            scannedItemCount: result.scannedItemCount
        )
    }

    private static func donorContaminationReport(
        data: Data,
        matches: [String],
        scannedItemCount: Int
    ) -> [String: Any] {
        [
            "schema": "xdremux-donor-contamination-scan-v1",
            "passed": matches.isEmpty,
            "knownPayloadSHA256Count": 9,
            "knownPhotoIdentifierCount": 1,
            "scannedItemCount": scannedItemCount,
            "matches": matches,
            "outputSHA256": sha256Hex(data),
        ]
    }
}
