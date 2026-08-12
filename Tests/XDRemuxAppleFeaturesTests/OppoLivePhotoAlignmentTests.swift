import XCTest
import XDRemuxCore
@testable import XDRemuxAppleFeatures

final class OppoLivePhotoAlignmentTests: XCTestCase {
    func testColorOS16AppliesLegacyEISCompensationWhenNoFactorExists() throws {
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

    func testColorOS16UsesReciprocalPhotoEisCropFactor() throws {
        let metadata = OppoMotionPhotoMetadata(
            version: 1,
            photoEisCropFactor: [1.11, 1.12]
        )
        let matrix = try XCTUnwrap(OppoLivePhotoAlignment.transformMatrix(for: metadata))
        XCTAssertEqual(matrix[0], 1.0 / 1.11, accuracy: 1e-12)
        XCTAssertEqual(matrix[4], 1.0 / 1.12, accuracy: 1e-12)
        XCTAssertEqual(matrix[8], 1.0, accuracy: 1e-12)
    }

    func testColorOS16AcceptsLegacyDirectScaleBelowOne() throws {
        let metadata = OppoMotionPhotoMetadata(
            version: 1,
            eisCropFactor: [0.91, 0.92]
        )
        let matrix = try XCTUnwrap(OppoLivePhotoAlignment.transformMatrix(for: metadata))
        XCTAssertEqual(matrix[0], 0.91, accuracy: 1e-12)
        XCTAssertEqual(matrix[4], 0.92, accuracy: 1e-12)
    }

    func testPhotoEisCropFactorWinsOverLegacyEisFactor() throws {
        let metadata = OppoMotionPhotoMetadata(
            version: 1,
            photoEisCropFactor: [1.11, 1.11],
            eisCropFactor: [0.8, 0.8]
        )
        let scale = OppoLivePhotoAlignment.colorOS16EISCompensationScale(for: metadata)
        XCTAssertEqual(scale.x, 1.0 / 1.11, accuracy: 1e-12)
        XCTAssertEqual(scale.y, 1.0 / 1.11, accuracy: 1e-12)
    }

    func testWriterCompatibilityReferenceDimensionsPreferExplicitAnalysisSpace() {
        let metadata = OppoMotionPhotoMetadata(
            version: 1,
            videoWidth: 1728,
            videoHeight: 1296
        )
        let transform: [Double] = [0.9, 0, 0, 0, 0.9, 0, 0, 0, 1]
        XCTAssertEqual(
            AppleLivePhotoVideoWriter.transformReferenceDimensions(
                transform: transform,
                requestedStillImageDimensions: [4096, 3072],
                oppoMetadata: metadata
            ),
            [4096, 3072]
        )
        XCTAssertEqual(
            AppleLivePhotoVideoWriter.transformReferenceDimensions(
                transform: transform,
                requestedStillImageDimensions: nil,
                oppoMetadata: metadata
            ),
            [1728, 1296]
        )
        XCTAssertNil(
            AppleLivePhotoVideoWriter.transformReferenceDimensions(
                transform: nil,
                requestedStillImageDimensions: [4096, 3072],
                oppoMetadata: metadata
            )
        )
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
