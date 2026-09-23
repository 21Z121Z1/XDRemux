// Public-framework structural/synthetic evidence only; not Photos or ANE evidence.
import CoreML
import CryptoKit
import Foundation

private enum ProbeError: Error {
    case contract(String)
}

@main
struct CoreMLProbe {
    static func main() {
        do {
            guard CommandLine.arguments.count == 2 else {
                throw ProbeError.contract("usage: CoreMLProbe package.mlpackage")
            }
            let package = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
            let compiled = try MLModel.compileModel(at: package)
            defer { try? FileManager.default.removeItem(at: compiled) }
            let configuration = MLModelConfiguration()
            configuration.computeUnits = .cpuOnly
            let model = try MLModel(contentsOf: compiled, configuration: configuration)
            let expectedInputs: [String: [Int]] = [
                "features": [1, 9, 256, 256], "metadata": [1, 16], "metadata_mask": [1, 16]
            ]
            let expectedOutputs: [String: [Int]] = [
                "key1": [1, 34560], "key1_log_variance": [1, 240],
                "gtc": [1, 516], "light_maps": [1, 2048], "scalars": [1, 6]
            ]
            guard Set(model.modelDescription.inputDescriptionsByName.keys) == Set(expectedInputs.keys),
                  Set(model.modelDescription.outputDescriptionsByName.keys) == Set(expectedOutputs.keys) else {
                throw ProbeError.contract("model feature names differ from the preserved contract")
            }
            var values: [String: Any] = [:]
            for (name, shape) in expectedInputs {
                guard let constraint = model.modelDescription.inputDescriptionsByName[name]?.multiArrayConstraint,
                      constraint.shape.map({ $0.intValue }) == shape else {
                    throw ProbeError.contract("input shape differs: \(name)")
                }
                let array = try MLMultiArray(shape: constraint.shape, dataType: constraint.dataType)
                for index in 0..<array.count { array[index] = NSNumber(value: 0) }
                values[name] = array
            }
            let result = try model.prediction(from: MLDictionaryFeatureProvider(dictionary: values))
            var elements = 0
            for (name, shape) in expectedOutputs {
                guard let array = result.featureValue(for: name)?.multiArrayValue,
                      array.shape.map({ $0.intValue }) == shape else {
                    throw ProbeError.contract("output shape differs: \(name)")
                }
                for index in 0..<array.count {
                    guard array[index].doubleValue.isFinite else {
                        throw ProbeError.contract("non-finite output: \(name)[\(index)]")
                    }
                }
                elements += array.count
            }
            var identities: [String: String] = [:]
            for name in ["Manifest.json", "Data/com.apple.CoreML/model.mlmodel",
                         "Data/com.apple.CoreML/weights/weight.bin"] {
                let data = try Data(contentsOf: package.appendingPathComponent(name))
                identities[name] = SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
            }
            try emit([
                "schema": 1, "passed": true,
                "evidenceLevel": "CoreML-compile-and-synthetic-forward",
                "computeUnits": "cpuOnly", "ANEPlacementValidated": false,
                "nativeStyleResponseValidated": false, "photosEditingValidated": false,
                "inputs": expectedInputs, "outputs": expectedOutputs, "finiteOutputElements": elements,
                "packageSHA256": identities,
                "os": ProcessInfo.processInfo.operatingSystemVersionString
            ])
        } catch {
            try? emit(["schema": 1, "passed": false, "error": String(describing: error),
                       "nativeStyleResponseValidated": false, "photosEditingValidated": false])
            exit(1)
        }
    }

    private static func emit(_ value: [String: Any]) throws {
        var data = try JSONSerialization.data(withJSONObject: value, options: [.prettyPrinted, .sortedKeys])
        data.append(0x0A)
        FileHandle.standardOutput.write(data)
    }
}
