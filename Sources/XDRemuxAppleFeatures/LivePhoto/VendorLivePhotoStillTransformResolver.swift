import Foundation
import CoreMedia
import XDRemuxCore

struct VendorLivePhotoStillTransformResolution: Sendable, Equatable {
    let transform: AppleLivePhotoStillTransform?
    let diagnostics: [String]
}

enum VendorLivePhotoStillTransformResolver {
    struct Request {
        let asset: MotionPhotoAsset
        let geometryPlan: VendorLivePhotoGeometryPlan?
        let primaryVideoURL: URL
        let auxiliaryVideoURL: URL?
        let stillImageURL: URL
        let stillImageTime: CMTime
    }

    static func resolve(_ request: Request) async -> VendorLivePhotoStillTransformResolution {
        if request.geometryPlan?.kind == .colorOS16,
           let auxiliaryVideoURL = request.auxiliaryVideoURL,
           let referenceDimensions = request.asset.vendorMetadata.flatMap(
            OppoLivePhotoAlignment.referenceDimensions
           ) {
            do {
                let alignment = try await ColorOS16VisionCoverAlignmentAnalyzer.analyze(
                    primaryVideoURL: request.primaryVideoURL,
                    auxiliaryVideoURL: auxiliaryVideoURL,
                    stillImageURL: request.stillImageURL,
                    stillImageTime: request.stillImageTime,
                    referenceDimensions: referenceDimensions
                )
                return VendorLivePhotoStillTransformResolution(
                    transform: alignment.transform,
                    diagnostics: [alignment.diagnostic]
                )
            } catch {
                let fallback = metadataTransform(request.asset.vendorMetadata)
                let fallbackDescription = fallback == nil
                    ? "no Track5 transform"
                    : "the OPPO metadata fallback"
                let reason = String(describing: error)
                return VendorLivePhotoStillTransformResolution(
                    transform: fallback,
                    diagnostics: [
                        "Vision Stream 1/Stream 2 cover alignment was rejected "
                            + "(\(reason)); used \(fallbackDescription)."
                    ]
                )
            }
        }

        return VendorLivePhotoStillTransformResolution(
            transform: metadataTransform(request.asset.vendorMetadata),
            diagnostics: []
        )
    }

    private static func metadataTransform(
        _ metadata: OppoMotionPhotoMetadata?
    ) -> AppleLivePhotoStillTransform? {
        guard let metadata,
              let matrix = OppoLivePhotoAlignment.transformMatrix(for: metadata),
              let referenceDimensions = OppoLivePhotoAlignment.referenceDimensions(for: metadata) else {
            return nil
        }
        return AppleLivePhotoStillTransform(
            matrix: matrix,
            referenceDimensions: referenceDimensions,
            source: .oppoMetadata
        )
    }
}
