import Foundation

public enum OppoMotionPhotoStreamResolver {
    /// Returns the primary OPPO Live Photo video stream. ColorOS 16 commonly stores two
    /// concatenated BMFF streams; Stream 1 is the Apple paired movie and Stream 2 is retained only
    /// as an auxiliary geometry source. The shared layout resolver owns the topology so downstream
    /// code does not need another OPPO-only interpretation of the same bytes.
    public static func primaryVideoRange(for asset: MotionPhotoAsset) throws -> MotionPhotoByteRange {
        try MotionPhotoVideoStreamLayoutResolver.resolve(for: asset).primary.range
    }

    /// Returns analysis-only auxiliary video resources when the vendor layout is proven.
    /// Current real fixtures expose one such stream for ColorOS 16 and none for Samsung.
    public static func auxiliaryGeometryVideoRanges(
        for asset: MotionPhotoAsset
    ) throws -> [MotionPhotoByteRange] {
        try MotionPhotoVideoStreamLayoutResolver.resolve(for: asset)
            .auxiliaryGeometry
            .map(\.range)
    }
}
