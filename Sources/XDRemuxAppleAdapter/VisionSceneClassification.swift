import CoreGraphics
import Foundation
import ImageIO
import Vision

/// Factual Vision output for the Rust-owned Photographic Styles scene policy.
/// The adapter reports confidence buckets only; it never chooses the Styles
/// scene type or applies product thresholds.
struct VisionSceneClassificationFacts: Encodable {
    let food: Double
    let sunset: Double
    let indoor: Double
    let outdoor: Double
    let requestClass: String
    let revision: Int

    enum CodingKeys: String, CodingKey {
        case food
        case sunset
        case indoor
        case outdoor
        case requestClass = "request_class"
        case revision
    }
}

private func maximumConfidence(
    _ observations: [VNClassificationObservation],
    identifiers: Set<String>
) -> Double {
    observations
        .filter { identifiers.contains($0.identifier) }
        .map { Double($0.confidence) }
        .max() ?? 0
}

func classifyVisionScene(inputPath: String) throws -> VisionSceneClassificationFacts {
    let inputURL = URL(fileURLWithPath: inputPath)
    guard let source = CGImageSourceCreateWithURL(inputURL as CFURL, nil),
          let image = CGImageSourceCreateImageAtIndex(source, 0, nil) else {
        throw NSError(
            domain: "XDRemuxAppleAdapter",
            code: 30,
            userInfo: [NSLocalizedDescriptionKey: "Vision cannot decode input (inputPath)"]
        )
    }
    let request = VNClassifyImageRequest()
    let handler = VNImageRequestHandler(cgImage: image, options: [:])
    try handler.perform([request])
    let observations = request.results ?? []
    return VisionSceneClassificationFacts(
        food: maximumConfidence(observations, identifiers: ["food", "meal", "dish"]),
        sunset: maximumConfidence(observations, identifiers: ["sunset", "sunrise", "dusk"]),
        indoor: maximumConfidence(observations, identifiers: ["indoor", "interior", "room"]),
        outdoor: maximumConfidence(observations, identifiers: ["outdoor"]),
        requestClass: "VNClassifyImageRequest",
        revision: request.revision
    )
}
