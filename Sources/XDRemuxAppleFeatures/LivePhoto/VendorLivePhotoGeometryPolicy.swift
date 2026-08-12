import Foundation
import ImageIO
import XDRemuxCore

/// Vendor families for which XDRemux has explicit Live Photo geometry evidence.
///
/// Keep this list intentionally narrow. A generic Android Motion Photo is not opted into geometry
/// metadata just because its container happens to have multiple BMFF-looking regions.
enum VendorLivePhotoGeometryKind: String, Sendable, Equatable {
    case colorOS16
    case samsung
}

struct VendorLivePhotoGeometryPlan: Sendable, Equatable {
    let kind: VendorLivePhotoGeometryKind
    let streamLayout: MotionPhotoVideoStreamLayout
    let stillRasterDimensions: [Float]?
}

enum VendorLivePhotoGeometryPolicy {
    static func plan(
        for asset: MotionPhotoAsset,
        stillResourceURL: URL
    ) throws -> VendorLivePhotoGeometryPlan? {
        let layout = try MotionPhotoVideoStreamLayoutResolver.resolve(for: asset)

        if asset.sourceKind == .oppoLivePhoto,
           let metadata = asset.vendorMetadata,
           metadata.version >= 1,
           metadata.streamCount >= 2,
           !layout.auxiliaryGeometry.isEmpty {
            return VendorLivePhotoGeometryPlan(
                kind: .colorOS16,
                streamLayout: layout,
                stillRasterDimensions: AppleLivePhotoStillWriter.pixelDimensions(in: stillResourceURL)
            )
        }

        if isSamsungMotionPhoto(asset: asset, stillResourceURL: stillResourceURL) {
            return VendorLivePhotoGeometryPlan(
                kind: .samsung,
                streamLayout: layout,
                stillRasterDimensions: AppleLivePhotoStillWriter.pixelDimensions(in: stillResourceURL)
            )
        }

        return nil
    }

    static func isSamsungMotionPhoto(
        asset: MotionPhotoAsset,
        stillResourceURL: URL
    ) -> Bool {
        guard asset.sourceKind == .androidMotionPhotoV1
                || asset.sourceKind == .androidHeifMotionPhotoV1 else {
            return false
        }
        guard let source = CGImageSourceCreateWithURL(
            stillResourceURL as CFURL,
            [kCGImageSourceShouldCache: false] as CFDictionary
        ),
        let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [String: Any] else {
            return false
        }

        if isSamsungMake(properties[kCGImagePropertyTIFFMake as String] as? String) {
            return true
        }
        if let tiff = properties[kCGImagePropertyTIFFDictionary as String] as? [String: Any],
           isSamsungMake(tiff[kCGImagePropertyTIFFMake as String] as? String) {
            return true
        }
        return false
    }

    static func isSamsungMake(_ make: String?) -> Bool {
        guard let make else { return false }
        return make.localizedCaseInsensitiveContains("samsung")
    }
}
