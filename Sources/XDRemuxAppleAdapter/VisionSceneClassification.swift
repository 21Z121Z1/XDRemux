import CoreGraphics
import Foundation
import ImageIO
import Vision

/// One factual Vision classification observation. The adapter reports the
/// framework identifier and confidence verbatim; XDRemux semantic grouping is
/// Rust-owned product policy.
struct VisionClassificationObservationFacts: Encodable {
    let identifier: String
    let confidence: Double
}

/// Factual `VNClassifyImageRequest` output for the Rust-owned Photographic
/// Styles scene policy. No XDRemux categories or thresholds live in Swift.
struct VisionSceneClassificationFacts: Encodable {
    let observations: [VisionClassificationObservationFacts]
    let requestClass: String
    let revision: Int

    enum CodingKeys: String, CodingKey {
        case observations
        case requestClass = "request_class"
        case revision
    }
}

func classifyVisionScene(inputPath: String) throws -> VisionSceneClassificationFacts {
    let inputURL = URL(fileURLWithPath: inputPath)
    guard let source = CGImageSourceCreateWithURL(inputURL as CFURL, nil),
          let image = CGImageSourceCreateImageAtIndex(source, 0, nil) else {
        throw NSError(
            domain: "XDRemuxAppleAdapter",
            code: 30,
            userInfo: [NSLocalizedDescriptionKey: "Vision cannot decode input \(inputPath)"]
        )
    }
    let request = VNClassifyImageRequest()
    let handler = VNImageRequestHandler(cgImage: image, options: [:])
    try handler.perform([request])
    let observations = request.results ?? []
    return VisionSceneClassificationFacts(
        observations: observations.map {
            VisionClassificationObservationFacts(
                identifier: $0.identifier,
                confidence: Double($0.confidence)
            )
        },
        requestClass: "VNClassifyImageRequest",
        revision: request.revision
    )
}
