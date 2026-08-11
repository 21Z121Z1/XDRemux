import XCTest
import XDRemuxCore
@testable import XDRemuxAppleFeatures

final class OppoLivePhotoAlignmentTests: XCTestCase {
    func testColorOS16AppliesEISCompensationAfterIdentityVendorMatrices() throws {
        let identity: [Double] = [1, 0, 0, 0, 1, 0, 0, 0, 1]
        let metadata = OppoMotionPhotoMetadata(
            coverFramePtsUs: 1_000_000,
            version: 1,
            matrixCount: 0,
            photoCropMatrix: identity,
            photoEisMatrix: identity,
            videoWidth: 1728,
            videoHeight: 1296
        )
        let matrix = try XCTUnwrap(OppoLivePhotoAlignment.transformMatrix(for: metadata))
        XCTAssertEqual(matrix[0], 0.90, accuracy: 1e-12)
        XCTAssertEqual(matrix[4], 0.90, accuracy: 1e-12)
        XCTAssertEqual(matrix[8], 1.0, accuracy: 1e-12)
        XCTAssertEqual(OppoLivePhotoAlignment.referenceDimensions(for: metadata), [1728, 1296])
    }

    func testColorOS15UsesClosestCoverFrameAndInvertsMatrix() throws {
        let metadata = OppoMotionPhotoMetadata(
            coverFramePtsUs: 1_100,
            version: 0,
            matrixCount: 2,
            matrices: [
                "1000": [2, 0, 0, 0, 2, 0, 0, 0, 1],
                "2000": [4, 0, 0, 0, 4, 0, 0, 0, 1],
            ]
        )
        let matrix = try XCTUnwrap(OppoLivePhotoAlignment.transformMatrix(for: metadata))
        XCTAssertEqual(matrix[0], 0.5, accuracy: 1e-12)
        XCTAssertEqual(matrix[4], 0.5, accuracy: 1e-12)
        XCTAssertEqual(matrix[8], 1.0, accuracy: 1e-12)
    }

    func testColorOS15WithNoMatricesNeedsNoTransform() {
        let metadata = OppoMotionPhotoMetadata(coverFramePtsUs: 1_000, version: 0, matrixCount: 0)
        XCTAssertNil(OppoLivePhotoAlignment.transformMatrix(for: metadata))
    }
}
