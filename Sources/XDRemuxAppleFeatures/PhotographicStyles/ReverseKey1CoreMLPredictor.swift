import CoreImage
import CoreML
import Foundation
import XDRemuxCore

package enum ReverseKey1CoreMLPredictor {
    package struct Prediction {
        package let styleData: Data
        package let displayWidth: Int
        package let displayHeight: Int
        package let timing: [String: Double]
    }

    private static let inputSize = 256
    private static let channelCount = 12
    private static let gridLong = 12
    private static let gridShort = 9
    private static let outputValueCount = 12 * 12 * 8 * 10 * 3
    private static let modelLock = NSLock()
    private static var models: [String: MLModel] = [:]

    package static func predict(
        modelURL: URL,
        styledURL: URL,
        unstyledURL: URL
    ) throws -> Prediction {
        let startedAt = CFAbsoluteTimeGetCurrent()
        let modelStartedAt = CFAbsoluteTimeGetCurrent()
        let model = try cachedModel(at: modelURL)
        let modelSeconds = CFAbsoluteTimeGetCurrent() - modelStartedAt
        let preprocessStartedAt = CFAbsoluteTimeGetCurrent()
        let styled = try fittedRGB(at: styledURL)
        let unstyled = try fittedRGB(at: unstyledURL)
        let features = try MLMultiArray(
            shape: [1, channelCount, inputSize, inputSize] as [NSNumber],
            dataType: .float32
        )
        let pointer = features.dataPointer.bindMemory(
            to: Float.self,
            capacity: channelCount * inputSize * inputSize
        )
        let pixelCount = inputSize * inputSize
        for pixel in 0..<pixelCount {
            let source = pixel * 4
            let red = Float(styled.rgba[source]) / 255
            let green = Float(styled.rgba[source + 1]) / 255
            let blue = Float(styled.rgba[source + 2]) / 255
            let unstyledRed = Float(unstyled.rgba[source]) / 255
            let unstyledGreen = Float(unstyled.rgba[source + 1]) / 255
            let unstyledBlue = Float(unstyled.rgba[source + 2]) / 255
            let differenceRed = red - unstyledRed
            let differenceGreen = green - unstyledGreen
            let differenceBlue = blue - unstyledBlue
            pointer[pixel] = red
            pointer[pixelCount + pixel] = green
            pointer[2 * pixelCount + pixel] = blue
            pointer[3 * pixelCount + pixel] = unstyledRed
            pointer[4 * pixelCount + pixel] = unstyledGreen
            pointer[5 * pixelCount + pixel] = unstyledBlue
            pointer[6 * pixelCount + pixel] = differenceRed
            pointer[7 * pixelCount + pixel] = differenceGreen
            pointer[8 * pixelCount + pixel] = differenceBlue
            pointer[9 * pixelCount + pixel] = 0.2126 * differenceRed
                + 0.7152 * differenceGreen + 0.0722 * differenceBlue
            pointer[10 * pixelCount + pixel] = -0.114572 * differenceRed
                - 0.385428 * differenceGreen + 0.5 * differenceBlue
            pointer[11 * pixelCount + pixel] = 0.5 * differenceRed
                - 0.454153 * differenceGreen - 0.045847 * differenceBlue
        }
        let preprocessSeconds = CFAbsoluteTimeGetCurrent() - preprocessStartedAt
        let inferenceStartedAt = CFAbsoluteTimeGetCurrent()
        let provider = try MLDictionaryFeatureProvider(dictionary: [
            "features": MLFeatureValue(multiArray: features),
        ])
        let output = try model.prediction(from: provider)
        guard let array = output.featureValue(for: "key1")?.multiArrayValue,
              array.count == outputValueCount else {
            throw CLIError.invalidContainer(
                "ReverseKey1Net Core ML output has an invalid shape"
            )
        }
        let inferenceSeconds = CFAbsoluteTimeGetCurrent() - inferenceStartedAt
        let serializationStartedAt = CFAbsoluteTimeGetCurrent()
        var values = [Float](repeating: 0, count: outputValueCount)
        switch array.dataType {
        case .float16:
            let source = array.dataPointer.bindMemory(
                to: UInt16.self,
                capacity: outputValueCount
            )
            for index in values.indices {
                values[index] = Float(Float16(bitPattern: source[index]))
            }
        case .float32:
            let source = array.dataPointer.bindMemory(
                to: Float.self,
                capacity: outputValueCount
            )
            values.withUnsafeMutableBufferPointer { destination in
                destination.baseAddress?.update(from: source, count: outputValueCount)
            }
        case .double:
            let source = array.dataPointer.bindMemory(
                to: Double.self,
                capacity: outputValueCount
            )
            for index in values.indices { values[index] = Float(source[index]) }
        default:
            throw CLIError.invalidContainer(
                "ReverseKey1Net Core ML output type is not floating point"
            )
        }
        let landscape = styled.displayWidth >= styled.displayHeight
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
        let serializationSeconds = CFAbsoluteTimeGetCurrent() - serializationStartedAt
        return Prediction(
            styleData: styleData,
            displayWidth: styled.displayWidth,
            displayHeight: styled.displayHeight,
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
        let features = try MLMultiArray(
            shape: [1, channelCount, inputSize, inputSize] as [NSNumber],
            dataType: .float32
        )
        memset(features.dataPointer, 0, features.count * MemoryLayout<Float>.size)
        let provider = try MLDictionaryFeatureProvider(dictionary: [
            "features": MLFeatureValue(multiArray: features),
        ])
        _ = try model.prediction(from: provider)
    }

    private static func cachedModel(at url: URL) throws -> MLModel {
        let key = url.resolvingSymlinksInPath().path
        modelLock.lock()
        defer { modelLock.unlock() }
        if let model = models[key] { return model }
        let configuration = MLModelConfiguration()
        configuration.computeUnits = .all
        let compiledURL: URL
        if url.pathExtension == "mlmodelc" {
            compiledURL = url
        } else {
            compiledURL = try MLModel.compileModel(at: url)
        }
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
                "ReverseKey1Net cannot decode \(url.lastPathComponent)"
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
        let fitted = translated.composited(
            over: CIImage(color: .black).cropped(to: bounds)
        )
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
