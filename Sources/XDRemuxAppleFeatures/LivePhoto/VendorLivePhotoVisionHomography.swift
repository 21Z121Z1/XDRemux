import CoreGraphics
import Foundation
import ImageIO
import Vision

/// Public Vision primitive used by vendor-specific Motion Photo geometry analyzers.
///
/// Vision defines the returned registration as the transform that morphs the floating/target image
/// into the reference image. XDRemux deliberately keeps that convention explicit here rather than
/// relabeling the matrix as an Apple Live Photo Track 4 or Track 5 transform. Those metadata formats
/// have additional coordinate/reference conventions that must be validated independently.
public struct VendorLivePhotoVisionHomography: Sendable, Equatable {
    public let floatingToReference: [Double]

    public init(floatingToReference: [Double]) {
        self.floatingToReference = floatingToReference
    }
}

public enum VendorLivePhotoVisionHomographyEstimator {
    public enum Error: Swift.Error, LocalizedError {
        case noAlignmentObservation
        case invalidHomography

        public var errorDescription: String? {
            switch self {
            case .noAlignmentObservation:
                return "Vision did not produce a homographic alignment observation."
            case .invalidHomography:
                return "Vision produced a non-finite or degenerate homography."
            }
        }
    }

    /// Estimates a projective transform from `floatingImage` into `referenceImage`.
    ///
    /// The returned nine values are row-major. No Apple metadata normalization, inversion, reference
    /// dimension conversion, or orientation compensation is applied after Vision's registration.
    public static func estimate(
        referenceImage: CGImage,
        floatingImage: CGImage,
        referenceOrientation: CGImagePropertyOrientation = .up,
        floatingOrientation: CGImagePropertyOrientation = .up
    ) throws -> VendorLivePhotoVisionHomography {
        let request = VNHomographicImageRegistrationRequest(
            targetedCGImage: floatingImage,
            orientation: floatingOrientation,
            options: [:]
        )
        let handler = VNImageRequestHandler(
            cgImage: referenceImage,
            orientation: referenceOrientation,
            options: [:]
        )
        try handler.perform([request])
        guard let observation = request.results?.first as? VNImageHomographicAlignmentObservation else {
            throw Error.noAlignmentObservation
        }

        let transform = observation.warpTransform
        // simd_float3x3 is column-major. Reorder by rows so the serialized Swift array is unambiguous.
        var matrix = [
            Double(transform[0][0]), Double(transform[1][0]), Double(transform[2][0]),
            Double(transform[0][1]), Double(transform[1][1]), Double(transform[2][1]),
            Double(transform[0][2]), Double(transform[1][2]), Double(transform[2][2]),
        ]
        guard matrix.allSatisfy(\.isFinite),
              abs(matrix[8]) > 1e-12 else {
            throw Error.invalidHomography
        }

        // Homographies are scale invariant. Normalize the representation only; this does not alter
        // the geometric mapping and makes diagnostics/comparisons deterministic.
        let normalization = matrix[8]
        matrix = matrix.map { $0 / normalization }
        guard matrix.allSatisfy(\.isFinite) else {
            throw Error.invalidHomography
        }
        return VendorLivePhotoVisionHomography(floatingToReference: matrix)
    }
}
