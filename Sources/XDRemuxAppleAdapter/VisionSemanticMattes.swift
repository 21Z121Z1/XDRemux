import CoreGraphics
import CoreVideo
import Foundation
import ImageIO
import Vision

struct VisionSemanticMaskFacts: Encodable {
    let role: String
    let featureName: String?
    let requestClass: String
    let revision: Int
    let width: Int
    let height: Int
    let pixelFormat: UInt32

    enum CodingKeys: String, CodingKey {
        case role
        case featureName = "feature_name"
        case requestClass = "request_class"
        case revision
        case width
        case height
        case pixelFormat = "pixel_format"
    }
}

private let supportedSemanticRoles: Set<String> = [
    "person",
    "skin",
    "hair",
    "teeth",
    "glasses",
    "sky",
]

// Primitive contract: XDRemux exposes one person-segmentation primitive and
// that primitive always asks Vision for its accurate result. This is not an
// adaptive product policy or user-selectable quality setting. If multiple
// quality levels ever become product-visible, the versioned Rust-owned adapter
// request must carry that selection instead of adding Swift-side heuristics.
private let personSegmentationPrimitiveQuality: VNGeneratePersonSegmentationRequest.QualityLevel = .accurate

private func writeSemanticMask(
    _ observation: VNPixelBufferObservation,
    role: String,
    requestClass: String,
    revision: Int,
    outputDirectory: URL
) throws -> VisionSemanticMaskFacts {
    let pixelBuffer = observation.pixelBuffer
    let pixelFormat = CVPixelBufferGetPixelFormatType(pixelBuffer)
    guard pixelFormat == kCVPixelFormatType_OneComponent8 else {
        throw NSError(
            domain: "XDRemuxAppleAdapter",
            code: 1,
            userInfo: [NSLocalizedDescriptionKey: "Vision \(role) mask is not L008"]
        )
    }

    CVPixelBufferLockBaseAddress(pixelBuffer, .readOnly)
    defer { CVPixelBufferUnlockBaseAddress(pixelBuffer, .readOnly) }
    guard let baseAddress = CVPixelBufferGetBaseAddress(pixelBuffer) else {
        throw NSError(
            domain: "XDRemuxAppleAdapter",
            code: 2,
            userInfo: [NSLocalizedDescriptionKey: "Vision \(role) mask has no readable base address"]
        )
    }

    let width = CVPixelBufferGetWidth(pixelBuffer)
    let height = CVPixelBufferGetHeight(pixelBuffer)
    let bytesPerRow = CVPixelBufferGetBytesPerRow(pixelBuffer)
    guard width > 0, height > 0 else {
        throw NSError(
            domain: "XDRemuxAppleAdapter",
            code: 3,
            userInfo: [NSLocalizedDescriptionKey: "Vision \(role) mask has invalid geometry"]
        )
    }

    let source = baseAddress.assumingMemoryBound(to: UInt8.self)
    var raw = Data(count: width * height)
    raw.withUnsafeMutableBytes { destinationBytes in
        guard let destination = destinationBytes.bindMemory(to: UInt8.self).baseAddress else {
            return
        }
        for y in 0..<height {
            destination
                .advanced(by: y * width)
                .update(from: source.advanced(by: y * bytesPerRow), count: width)
        }
    }
    try raw.write(
        to: outputDirectory.appendingPathComponent("\(role).l8"),
        options: .atomic
    )

    return VisionSemanticMaskFacts(
        role: role,
        featureName: observation.featureName,
        requestClass: requestClass,
        revision: revision,
        width: width,
        height: height,
        pixelFormat: pixelFormat
    )
}

func generateVisionSemanticMattes(
    inputPath: String,
    outputPath: String,
    roles: [String],
    orientationOverride: UInt32?
) throws -> [VisionSemanticMaskFacts] {
    let selectedRoles = Set(roles)
    guard !selectedRoles.isEmpty,
          selectedRoles.count == roles.count,
          selectedRoles.isSubset(of: supportedSemanticRoles) else {
        throw NSError(
            domain: "XDRemuxAppleAdapter",
            code: 4,
            userInfo: [NSLocalizedDescriptionKey: "semantic roles are empty, duplicated, or unsupported"]
        )
    }

    let inputURL = URL(fileURLWithPath: inputPath)
    guard let source = CGImageSourceCreateWithURL(inputURL as CFURL, nil),
          let image = CGImageSourceCreateImageAtIndex(source, 0, nil) else {
        throw NSError(
            domain: "XDRemuxAppleAdapter",
            code: 5,
            userInfo: [NSLocalizedDescriptionKey: "Vision cannot decode input \(inputPath)"]
        )
    }
    let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any]
    let metadataOrientation = (properties?[kCGImagePropertyOrientation] as? NSNumber)?.uint32Value ?? 1
    let orientationRaw = orientationOverride ?? metadataOrientation
    guard let orientation = CGImagePropertyOrientation(rawValue: orientationRaw),
          (1...8).contains(orientationRaw) else {
        throw NSError(
            domain: "XDRemuxAppleAdapter",
            code: 6,
            userInfo: [NSLocalizedDescriptionKey: "Vision orientation must be 1 through 8"]
        )
    }

    let outputDirectory = URL(fileURLWithPath: outputPath, isDirectory: true)
    try FileManager.default.createDirectory(
        at: outputDirectory,
        withIntermediateDirectories: true
    )

    let humanRoles: Set<String> = ["skin", "hair", "teeth"]
    let needsHumanAttributes = !selectedRoles.isDisjoint(with: humanRoles)
    let humanAttributesRequest: VNRequest? = needsHumanAttributes
        ? (NSClassFromString("VNGenerateHumanAttributesSegmentationRequest") as? VNRequest.Type)?.init()
        : nil
    if needsHumanAttributes && humanAttributesRequest == nil {
        throw NSError(
            domain: "XDRemuxAppleAdapter",
            code: 7,
            userInfo: [NSLocalizedDescriptionKey: "VNGenerateHumanAttributesSegmentationRequest is unavailable"]
        )
    }

    let personRequest: VNGeneratePersonSegmentationRequest? = selectedRoles.contains("person")
        ? VNGeneratePersonSegmentationRequest()
        : nil
    personRequest?.qualityLevel = personSegmentationPrimitiveQuality
    personRequest?.outputPixelFormat = kCVPixelFormatType_OneComponent8

    let glassesRequest: VNRequest? = selectedRoles.contains("glasses")
        ? (NSClassFromString("VNGenerateGlassesSegmentationRequest") as? VNRequest.Type)?.init()
        : nil
    if selectedRoles.contains("glasses") && glassesRequest == nil {
        throw NSError(
            domain: "XDRemuxAppleAdapter",
            code: 8,
            userInfo: [NSLocalizedDescriptionKey: "VNGenerateGlassesSegmentationRequest is unavailable"]
        )
    }

    let skyRequest: VNRequest? = selectedRoles.contains("sky")
        ? (NSClassFromString("VNGenerateSkySegmentationRequest") as? VNRequest.Type)?.init()
        : nil
    if selectedRoles.contains("sky") && skyRequest == nil {
        throw NSError(
            domain: "XDRemuxAppleAdapter",
            code: 9,
            userInfo: [NSLocalizedDescriptionKey: "VNGenerateSkySegmentationRequest is unavailable"]
        )
    }

    let requests = [humanAttributesRequest, personRequest, glassesRequest, skyRequest].compactMap { $0 }
    let handler = VNImageRequestHandler(cgImage: image, orientation: orientation, options: [:])
    try handler.perform(requests)

    var masks: [VisionSemanticMaskFacts] = []
    if let humanAttributesRequest {
        let observations = humanAttributesRequest.results?
            .compactMap { $0 as? VNPixelBufferObservation } ?? []
        let byFeatureName = Dictionary(
            uniqueKeysWithValues: observations.compactMap { observation in
                observation.featureName.map { ($0, observation) }
            }
        )
        for (featureName, role) in [
            ("human_attribute_skin", "skin"),
            ("human_attribute_hair", "hair"),
            ("human_attribute_teeth", "teeth"),
        ] where selectedRoles.contains(role) {
            guard let observation = byFeatureName[featureName] else {
                throw NSError(
                    domain: "XDRemuxAppleAdapter",
                    code: 10,
                    userInfo: [NSLocalizedDescriptionKey: "Vision returned no \(featureName) observation"]
                )
            }
            masks.append(
                try writeSemanticMask(
                    observation,
                    role: role,
                    requestClass: "VNGenerateHumanAttributesSegmentationRequest",
                    revision: humanAttributesRequest.revision,
                    outputDirectory: outputDirectory
                )
            )
        }
    }

    if let personRequest {
        guard let observation = personRequest.results?.first else {
            throw NSError(
                domain: "XDRemuxAppleAdapter",
                code: 11,
                userInfo: [NSLocalizedDescriptionKey: "Vision returned no person observation"]
            )
        }
        masks.append(
            try writeSemanticMask(
                observation,
                role: "person",
                requestClass: "VNGeneratePersonSegmentationRequest",
                revision: personRequest.revision,
                outputDirectory: outputDirectory
            )
        )
    }

    if let glassesRequest {
        guard let observation = glassesRequest.results?.first as? VNPixelBufferObservation else {
            throw NSError(
                domain: "XDRemuxAppleAdapter",
                code: 12,
                userInfo: [NSLocalizedDescriptionKey: "Vision returned no glasses observation"]
            )
        }
        masks.append(
            try writeSemanticMask(
                observation,
                role: "glasses",
                requestClass: "VNGenerateGlassesSegmentationRequest",
                revision: glassesRequest.revision,
                outputDirectory: outputDirectory
            )
        )
    }

    if let skyRequest {
        guard let observation = skyRequest.results?.first as? VNPixelBufferObservation else {
            throw NSError(
                domain: "XDRemuxAppleAdapter",
                code: 13,
                userInfo: [NSLocalizedDescriptionKey: "Vision returned no sky observation"]
            )
        }
        masks.append(
            try writeSemanticMask(
                observation,
                role: "sky",
                requestClass: "VNGenerateSkySegmentationRequest",
                revision: skyRequest.revision,
                outputDirectory: outputDirectory
            )
        )
    }

    return masks.sorted { $0.role < $1.role }
}
