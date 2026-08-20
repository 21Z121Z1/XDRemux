import CoreImage
import CoreML
import Foundation
import ImageIO
import XDRemuxCore

package enum UniversalPhotographicStyleCoreMLPredictor {
    package struct Prediction {
        package let styleData: Data
        package let gtcData: Data
        package let lightMapCData: Data
        package let lightMapDData: Data
        package let scalars: [String: Double]
        package let uncertainty: Double
        package let timing: [String: Double]
    }

    private static let inputSize = 256
    private static let channelCount = 9
    private static let metadataCount = 16
    private static let gridLong = 12
    private static let gridShort = 9
    private static let key1ValueCount = 12 * 12 * 8 * 10 * 3
    private static let uncertaintyValueCount = 8 * 10 * 3
    private static let gtcValueCount = 516
    private static let lightMapValueCount = 2 * 32 * 32
    private static let scalarNames = [
        "TagH", "IOriginalRangeMin", "IOriginalRangeMax", "IGain", "Tag4", "Tag5",
    ]
    private static let modelLock = NSLock()
    private static var models: [String: MLModel] = [:]

    package static func predict(
        modelURL: URL,
        styledURL: URL,
        metadataURL: URL
    ) throws -> Prediction {
        let startedAt = CFAbsoluteTimeGetCurrent()
        let modelStartedAt = CFAbsoluteTimeGetCurrent()
        let model = try cachedModel(at: modelURL)
        let modelSeconds = CFAbsoluteTimeGetCurrent() - modelStartedAt
        let preprocessStartedAt = CFAbsoluteTimeGetCurrent()
        let styled = try fittedRGB(at: styledURL)
        let features = try makeFeatures(from: styled.rgba)
        let metadata = try makeMetadata(
            at: metadataURL,
            displayWidth: styled.displayWidth,
            displayHeight: styled.displayHeight
        )
        let preprocessSeconds = CFAbsoluteTimeGetCurrent() - preprocessStartedAt
        let inferenceStartedAt = CFAbsoluteTimeGetCurrent()
        let provider = try MLDictionaryFeatureProvider(dictionary: [
            "features": MLFeatureValue(multiArray: features),
            "metadata": MLFeatureValue(multiArray: metadata.values),
            "metadata_mask": MLFeatureValue(multiArray: metadata.mask),
        ])
        let output = try model.prediction(from: provider)
        let key1 = try floatValues(
            output.featureValue(for: "key1")?.multiArrayValue,
            expectedCount: key1ValueCount,
            label: "key1"
        )
        let logVariance = try floatValues(
            output.featureValue(for: "key1_log_variance")?.multiArrayValue,
            expectedCount: uncertaintyValueCount,
            label: "key1_log_variance"
        )
        let gtc = try floatValues(
            output.featureValue(for: "gtc")?.multiArrayValue,
            expectedCount: gtcValueCount,
            label: "gtc"
        )
        let lightMaps = try floatValues(
            output.featureValue(for: "light_maps")?.multiArrayValue,
            expectedCount: lightMapValueCount,
            label: "light_maps"
        )
        let scalarValues = try floatValues(
            output.featureValue(for: "scalars")?.multiArrayValue,
            expectedCount: scalarNames.count,
            label: "scalars"
        )
        let inferenceSeconds = CFAbsoluteTimeGetCurrent() - inferenceStartedAt
        let serializationStartedAt = CFAbsoluteTimeGetCurrent()
        let styleData = try serializeKey1(
            key1,
            displayWidth: styled.displayWidth,
            displayHeight: styled.displayHeight
        )
        let gtcData = Data(gtc.map { value in
            UInt8(max(0, min(255, Int((Double(value) * 255).rounded()))))
        })
        let lightMapCData = float16Data(lightMaps[0..<(32 * 32)])
        let lightMapDData = float16Data(lightMaps[(32 * 32)..<(2 * 32 * 32)])
        let scalars = Dictionary(
            uniqueKeysWithValues: zip(scalarNames, scalarValues).map {
                ($0.0, Double($0.1))
            }
        )
        let uncertainty = logVariance.reduce(0.0) { partial, value in
            partial + exp(Double(value))
        } / Double(logVariance.count)
        guard uncertainty.isFinite else {
            throw CLIError.invalidContainer(
                "universal Photographic Style uncertainty is non-finite"
            )
        }
        let serializationSeconds = CFAbsoluteTimeGetCurrent() - serializationStartedAt
        return Prediction(
            styleData: styleData,
            gtcData: gtcData,
            lightMapCData: lightMapCData,
            lightMapDData: lightMapDData,
            scalars: scalars,
            uncertainty: uncertainty,
            timing: [
                "modelLoadSeconds": modelSeconds,
                "preprocessSeconds": preprocessSeconds,
                "inferenceSeconds": inferenceSeconds,
                "serializationSeconds": serializationSeconds,
                "totalSeconds": CFAbsoluteTimeGetCurrent() - startedAt,
            ]
        )
    }

    package static func prepare(modelURL: URL) throws {
        let model = try cachedModel(at: modelURL)
        let features = try zeroArray(shape: [1, channelCount, inputSize, inputSize])
        let metadata = try zeroArray(shape: [1, metadataCount])
        let metadataMask = try zeroArray(shape: [1, metadataCount])
        let provider = try MLDictionaryFeatureProvider(dictionary: [
            "features": MLFeatureValue(multiArray: features),
            "metadata": MLFeatureValue(multiArray: metadata),
            "metadata_mask": MLFeatureValue(multiArray: metadataMask),
        ])
        _ = try model.prediction(from: provider)
    }

    private static func zeroArray(shape: [Int]) throws -> MLMultiArray {
        let array = try MLMultiArray(shape: shape.map(NSNumber.init), dataType: .float32)
        memset(array.dataPointer, 0, array.count * MemoryLayout<Float>.size)
        return array
    }

    private static func makeFeatures(from rgba: Data) throws -> MLMultiArray {
        let array = try zeroArray(shape: [1, channelCount, inputSize, inputSize])
        let destination = array.dataPointer.bindMemory(
            to: Float.self,
            capacity: channelCount * inputSize * inputSize
        )
        let pixelCount = inputSize * inputSize
        var luma = [Float](repeating: 0, count: pixelCount)
        rgba.withUnsafeBytes { raw in
            let source = raw.bindMemory(to: UInt8.self)
            for pixel in 0..<pixelCount {
                let offset = pixel * 4
                let red = Float(source[offset]) / 255
                let green = Float(source[offset + 1]) / 255
                let blue = Float(source[offset + 2]) / 255
                let y = 0.2126 * red + 0.7152 * green + 0.0722 * blue
                destination[pixel] = red
                destination[pixelCount + pixel] = green
                destination[2 * pixelCount + pixel] = blue
                destination[3 * pixelCount + pixel] = y
                destination[4 * pixelCount + pixel] = (blue - y) * 0.5
                destination[5 * pixelCount + pixel] = (red - y) * 0.5
                destination[6 * pixelCount + pixel] = Float(
                    log1p(15 * Double(y)) / log(16)
                )
                luma[pixel] = y
            }
        }
        for pixel in 0..<pixelCount {
            let x = pixel % inputSize
            let y = pixel / inputSize
            destination[7 * pixelCount + pixel] = x == 0
                ? 0 : luma[pixel] - luma[pixel - 1]
            destination[8 * pixelCount + pixel] = y == 0
                ? 0 : luma[pixel] - luma[pixel - inputSize]
        }
        return array
    }

    private static func makeMetadata(
        at url: URL,
        displayWidth: Int,
        displayHeight: Int
    ) throws -> (values: MLMultiArray, mask: MLMultiArray) {
        let values = try zeroArray(shape: [1, metadataCount])
        let mask = try zeroArray(shape: [1, metadataCount])
        let destination = values.dataPointer.bindMemory(to: Float.self, capacity: metadataCount)
        let observed = mask.dataPointer.bindMemory(to: Float.self, capacity: metadataCount)
        func set(_ index: Int, _ value: Double?) {
            guard let value, value.isFinite else { return }
            destination[index] = Float(value)
            observed[index] = 1
        }
        set(0, log2(Double(displayWidth)))
        set(1, log2(Double(displayHeight)))
        set(2, Double(displayWidth) / Double(displayHeight))
        guard let source = CGImageSourceCreateWithURL(url as CFURL, nil),
              let properties = CGImageSourceCopyPropertiesAtIndex(
                  source, 0, nil
              ) as? [CFString: Any] else {
            set(12, 0); set(13, 0); set(15, 0)
            return (values, mask)
        }
        let exif = properties[kCGImagePropertyExifDictionary] as? [CFString: Any] ?? [:]
        let tiff = properties[kCGImagePropertyTIFFDictionary] as? [CFString: Any] ?? [:]
        let maker = properties[kCGImagePropertyMakerAppleDictionary] as? NSDictionary
        if let orientation = number(properties[kCGImagePropertyOrientation]) {
            set(3, orientation / 8)
        }
        if let exposure = number(exif[kCGImagePropertyExifExposureTime]), exposure > 0 {
            set(4, log2(exposure))
        }
        if let aperture = number(exif[kCGImagePropertyExifFNumber]), aperture > 0 {
            set(5, log2(aperture))
        }
        if let iso = firstNumber(exif[kCGImagePropertyExifISOSpeedRatings]), iso > 0 {
            set(6, log2(iso))
        }
        if let focal = number(exif[kCGImagePropertyExifFocalLength]) {
            set(7, focal / 20)
        }
        if let temperature = number(makerValue(maker, key: 45)), temperature > 0 {
            set(8, log2(temperature))
        }
        set(9, number(makerValue(maker, key: 48)))
        if let software = leadingNumber(tiff[kCGImagePropertyTIFFSoftware]) {
            set(10, software / 30)
        }
        // PLIST Tag0 is not surfaced by CGImageSource properties. Protocol 15
        // is the target schema used by the research writer and the default
        // generic-image inference profile.
        set(11, log2(16) / 16)
        // Presence flags are always observed. This checkpoint disables them
        // internally because the native training split contained no positives.
        set(12, 0)
        set(13, url.pathExtension.lowercased() == "dng" ? 1 : 0)
        set(15, 0)
        return (values, mask)
    }

    private static func number(_ value: Any?) -> Double? {
        if let number = value as? NSNumber { return number.doubleValue }
        if let text = value as? String {
            return Double(text.split(separator: " ").first ?? "")
        }
        return nil
    }

    private static func makerValue(_ maker: NSDictionary?, key: Int) -> Any? {
        maker?.object(forKey: key) ?? maker?.object(forKey: String(key))
    }

    private static func firstNumber(_ value: Any?) -> Double? {
        if let values = value as? [NSNumber] { return values.first?.doubleValue }
        return number(value)
    }

    private static func leadingNumber(_ value: Any?) -> Double? {
        guard let text = value as? String else { return number(value) }
        return Double(text.split(separator: " ").first ?? "")
    }

    private static func floatValues(
        _ array: MLMultiArray?,
        expectedCount: Int,
        label: String
    ) throws -> [Float] {
        guard let array, array.count == expectedCount else {
            throw CLIError.invalidContainer(
                "universal Photographic Style \(label) output has an invalid shape"
            )
        }
        var values = [Float](repeating: 0, count: expectedCount)
        switch array.dataType {
        case .float16:
            let source = array.dataPointer.bindMemory(to: UInt16.self, capacity: expectedCount)
            for index in values.indices {
                values[index] = Float(Float16(bitPattern: source[index]))
            }
        case .float32:
            let source = array.dataPointer.bindMemory(to: Float.self, capacity: expectedCount)
            values.withUnsafeMutableBufferPointer { destination in
                destination.baseAddress?.update(from: source, count: expectedCount)
            }
        case .double:
            let source = array.dataPointer.bindMemory(to: Double.self, capacity: expectedCount)
            for index in values.indices { values[index] = Float(source[index]) }
        default:
            throw CLIError.invalidContainer(
                "universal Photographic Style \(label) output is not floating point"
            )
        }
        guard values.allSatisfy(\.isFinite) else {
            throw CLIError.invalidContainer(
                "universal Photographic Style \(label) output is non-finite"
            )
        }
        return values
    }

    private static func serializeKey1(
        _ values: [Float],
        displayWidth: Int,
        displayHeight: Int
    ) throws -> Data {
        let landscape = displayWidth >= displayHeight
        let widthSlots = landscape ? gridLong : gridShort
        let heightSlots = landscape ? gridShort : gridLong
        var styleData = Data()
        styleData.reserveCapacity(widthSlots * heightSlots * 8 * 10 * 3 * 2)
        for x in 0..<widthSlots {
            for y in 0..<heightSlots {
                for plane in 0..<8 {
                    for polynomial in 0..<10 {
                        for outputChannel in 0..<3 {
                            let index = (((y * gridLong + x) * 8 + plane) * 10
                                + polynomial) * 3 + outputChannel
                            var bits = Float16(values[index]).bitPattern.littleEndian
                            withUnsafeBytes(of: &bits) { styleData.append(contentsOf: $0) }
                        }
                    }
                }
            }
        }
        _ = try AppleStyleDataLayout.validate(styleData)
        return styleData
    }

    private static func float16Data(_ values: ArraySlice<Float>) -> Data {
        var result = Data()
        result.reserveCapacity(values.count * 2)
        for value in values {
            var bits = Float16(value).bitPattern.littleEndian
            withUnsafeBytes(of: &bits) { result.append(contentsOf: $0) }
        }
        return result
    }

    private static func cachedModel(at url: URL) throws -> MLModel {
        let key = url.resolvingSymlinksInPath().path
        modelLock.lock()
        defer { modelLock.unlock() }
        if let model = models[key] { return model }
        let configuration = MLModelConfiguration()
        configuration.computeUnits = .all
        let compiledURL = url.pathExtension == "mlmodelc"
            ? url : try MLModel.compileModel(at: url)
        let model = try MLModel(contentsOf: compiledURL, configuration: configuration)
        models[key] = model
        return model
    }

    private struct FittedRGB {
        let rgba: Data
        let displayWidth: Int
        let displayHeight: Int
    }

    private static func fittedRGB(at url: URL) throws -> FittedRGB {
        guard let image = CIImage(
            contentsOf: url,
            options: [.applyOrientationProperty: true]
        ) else {
            throw CLIError.invalidContainer(
                "universal Photographic Style model cannot decode \(url.lastPathComponent)"
            )
        }
        let extent = image.extent.integral
        let width = max(1, Int(extent.width))
        let height = max(1, Int(extent.height))
        let scale = min(
            Double(inputSize) / Double(width),
            Double(inputSize) / Double(height)
        )
        let fittedWidth = max(1, Int((Double(width) * scale).rounded()))
        let fittedHeight = max(1, Int((Double(height) * scale).rounded()))
        let originNormalized = image.transformed(
            by: CGAffineTransform(translationX: -extent.origin.x, y: -extent.origin.y)
        )
        let scaled = originNormalized.applyingFilter(
            "CILanczosScaleTransform",
            parameters: [
                kCIInputScaleKey: Double(fittedHeight) / Double(height),
                kCIInputAspectRatioKey: (
                    Double(fittedWidth) / Double(width)
                ) / (Double(fittedHeight) / Double(height)),
            ]
        ).cropped(to: CGRect(x: 0, y: 0, width: fittedWidth, height: fittedHeight))
        let translated = scaled.transformed(
            by: CGAffineTransform(
                translationX: CGFloat((inputSize - fittedWidth) / 2),
                y: CGFloat((inputSize - fittedHeight) / 2)
            )
        )
        let bounds = CGRect(x: 0, y: 0, width: inputSize, height: inputSize)
        let fitted = translated.composited(over: CIImage(color: .black).cropped(to: bounds))
        var rgba = Data(count: inputSize * inputSize * 4)
        let context = CIContext(options: [
            .cacheIntermediates: false,
            .workingColorSpace: NSNull(),
            .outputColorSpace: NSNull(),
        ])
        rgba.withUnsafeMutableBytes { raw in
            guard let baseAddress = raw.baseAddress else { return }
            context.render(
                fitted,
                toBitmap: baseAddress,
                rowBytes: inputSize * 4,
                bounds: bounds,
                format: .RGBA8,
                colorSpace: nil
            )
        }
        return FittedRGB(rgba: rgba, displayWidth: width, displayHeight: height)
    }
}
