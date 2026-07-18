import Foundation
import XDRemuxCore

package enum AppleSemanticSceneAnalyzer {
    static func analyze(
        imageURL: URL,
        outputDirectory: URL,
        orientationOverride: UInt32? = nil
    ) throws -> AppleSemanticSceneAnalysis {
        try FileManager.default.createDirectory(at: outputDirectory, withIntermediateDirectories: true)
        let executable = try AppleNativeToolchain.semanticExecutable()
        var arguments = [imageURL.path, outputDirectory.path]
        if let orientationOverride {
            arguments += ["--orientation", String(orientationOverride)]
        }
        let result = try AppleNativeToolchain.run(executable, arguments: arguments)
        guard result.status == 0 else {
            let stderr = String(data: result.stderr, encoding: .utf8) ?? ""
            let stdout = String(data: result.stdout, encoding: .utf8) ?? ""
            throw CLIError.invalidContainer(
                "Apple semantic capability unavailable: \([stderr, stdout].filter { !$0.isEmpty }.joined(separator: " "))"
            )
        }
        try result.stdout.write(
            to: outputDirectory.appendingPathComponent("manifest.json"),
            options: .atomic
        )
        guard let object = try JSONSerialization.jsonObject(with: result.stdout) as? [String: Any],
              object["ok"] as? Bool == true,
              let maskRows = object["masks"] as? [[String: Any]] else {
            throw CLIError.invalidContainer("Apple semantic helper returned an invalid manifest")
        }
        var mattes: [String: AppleSemanticMatte] = [:]
        for row in maskRows {
            guard let name = row["name"] as? String,
                  let rawPath = row["raw_output"] as? String,
                  let width = (row["width"] as? NSNumber)?.intValue,
                  let height = (row["height"] as? NSNumber)?.intValue,
                  let serializedBytesPerRow = (row["serialized_bytes_per_row"] as? NSNumber)?.intValue,
                  let minimum = (row["minimum"] as? NSNumber)?.uint8Value,
                  let maximum = (row["maximum"] as? NSNumber)?.uint8Value,
                  let mean = (row["mean"] as? NSNumber)?.doubleValue,
                  let coverage = (row["coverage"] as? NSNumber)?.doubleValue,
                  let requestClass = row["request_class"] as? String,
                  let revision = (row["revision"] as? NSNumber)?.intValue,
                  let inputSHA256 = row["input_sha256"] as? String,
                  let pixelFormat = row["pixel_format"] as? String,
                  let orientation = (row["orientation"] as? NSNumber)?.uint32Value,
                  let orientationTransform = row["orientation_transform"] as? String,
                  let fallback = row["fallback"] as? Bool else {
                throw CLIError.invalidContainer("incomplete semantic manifest row")
            }
            let attributeName = row["feature_name"] as? String ?? name
            let pixels = try Data(contentsOf: URL(fileURLWithPath: rawPath))
            guard serializedBytesPerRow == width, pixels.count == width * height else {
                throw CLIError.invalidContainer(
                    "\(name) semantic matte has invalid L008 geometry or data length"
                )
            }
            mattes[name] = AppleSemanticMatte(
                pixels: pixels,
                width: width,
                height: height,
                bytesPerRow: serializedBytesPerRow,
                statistics: SemanticStatistics(
                    minimum: minimum,
                    maximum: maximum,
                    mean: mean,
                    coverage: coverage
                ),
                provenance: SemanticProvenance(
                    requestClass: requestClass,
                    attributeName: attributeName,
                    revision: revision,
                    inputSHA256: inputSHA256,
                    width: width,
                    height: height,
                    pixelFormat: pixelFormat,
                    orientation: orientation,
                    orientationTransform: orientationTransform,
                    fallback: fallback
                )
            )
        }
        for required in ["portrait", "skin", "hair", "facial_hair", "teeth", "glasses", "sky"] {
            guard mattes[required] != nil else {
                throw CLIError.invalidContainer("Vision returned no \(required) semantic resource")
            }
        }
        return AppleSemanticSceneAnalysis(
            person: mattes["portrait"],
            skin: mattes["skin"],
            hair: mattes["hair"],
            facialHair: mattes["facial_hair"],
            teeth: mattes["teeth"],
            glasses: mattes["glasses"],
            sky: mattes["sky"]
        )
    }
}
