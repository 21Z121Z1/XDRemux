import Foundation
import CoreGraphics
import XCTest
import XDRemuxCore
@testable import XDRemuxAppleFeatures

final class ColorOS16VisionCoverAlignmentTests: XCTestCase {
    func testReferenceMappingScalesTranslationIntoDeclaredAnalysisSpace() throws {
        let mapped = try XCTUnwrap(
            ColorOS16VisionCoverAlignmentAnalyzer.mapToReferenceDimensions(
                [1, 0, 100, 0, 1, 50, 0, 0, 1],
                floatingSize: CGSize(width: 1_600, height: 1_200),
                referenceSize: CGSize(width: 1_600, height: 1_200),
                outputReferenceDimensions: [1_920, 1_440]
            )
        )
        XCTAssertEqual(mapped[0], 1, accuracy: 1e-12)
        XCTAssertEqual(mapped[4], 1, accuracy: 1e-12)
        XCTAssertEqual(mapped[2], 120, accuracy: 1e-12)
        XCTAssertEqual(mapped[5], 60, accuracy: 1e-12)
        XCTAssertEqual(mapped[8], 1, accuracy: 1e-12)
    }

    func testTrajectoryAgreementRejectsDifferentCoverGeometry() {
        let still: [Double] = [0.902, -0.0046, 108.7, 0.0046, 0.9005, 81.3, 0, 0, 1]
        let matchingStream2: [Double] = [0.9055, -0.0016, 105.3, 0.0052, 0.9041, 80.2, 0, 0, 1]
        let conflictingStream2: [Double] = [1.15, 0, -180, 0, 1.15, 190, 0, 0, 1]

        XCTAssertTrue(
            ColorOS16VisionCoverAlignmentAnalyzer.matricesAgree(
                still,
                matchingStream2,
                referenceDimensions: [1_920, 1_440]
            )
        )
        XCTAssertFalse(
            ColorOS16VisionCoverAlignmentAnalyzer.matricesAgree(
                still,
                conflictingStream2,
                referenceDimensions: [1_920, 1_440]
            )
        )
    }

    func testStillTransformRejectsInvalidMatrixOrReferenceDimensions() {
        XCTAssertNil(AppleLivePhotoStillTransform(
            matrix: [1, 0, 0],
            referenceDimensions: [1_920, 1_440],
            source: .colorOS16VisionTrajectory
        ))
        XCTAssertNil(AppleLivePhotoStillTransform(
            matrix: [1, 0, 0, 0, 1, 0, 0, 0, 1],
            referenceDimensions: [1_920, 0],
            source: .colorOS16VisionTrajectory
        ))
    }

    func testVerifiedColorOS16FixtureWritesVisionCoverTransformWithoutReencoding() async throws {
        guard let path = ProcessInfo.processInfo.environment["XDREMUX_VISION_COVER_FIXTURE"],
              !path.isEmpty else {
            throw XCTSkip("XDREMUX_VISION_COVER_FIXTURE is not configured")
        }
        let inputURL = URL(fileURLWithPath: path)
        XCTAssertTrue(FileManager.default.fileExists(atPath: inputURL.path))

        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-vision-cover-functional-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let outputHEIC = directory.appendingPathComponent("vision-cover.heic")

        let result = try await AppleLivePhotoConversionEngine.convertAsync(
            inputURL: inputURL,
            outputImageURL: outputHEIC,
            requirePhotoKitValidation: true
        )
        let report = try await AppleLivePhotoValidator.validate(
            imageURL: result.imageURL,
            videoURL: result.videoURL,
            expectedAssetIdentifier: result.assetIdentifier,
            expectedStillImageTime: result.stillImageTime,
            requirePhotoKitLoad: true
        )

        let matrix = try XCTUnwrap(report.stillImageTransform)
        let expected: [Double] = [
            0.9019638784, -0.0046428046, 108.7131985,
            0.0045975180, 0.9004663028, 81.3306202,
            0.0000005604, 0.0000003985, 1
        ]
        for index in [0, 1, 3, 4, 6, 7, 8] {
            XCTAssertEqual(matrix[index], expected[index], accuracy: 0.002, "matrix[\(index)]")
        }
        XCTAssertEqual(matrix[2], expected[2], accuracy: 3, "matrix[2]")
        XCTAssertEqual(matrix[5], expected[5], accuracy: 3, "matrix[5]")
        XCTAssertEqual(report.stillImageTransformReferenceDimensions ?? [], [1_920, 1_440])
        XCTAssertTrue(report.vitalityTransformLimitingAllowed)
        XCTAssertTrue(result.diagnostics.contains { $0.contains("Vision Track5 cover alignment accepted") })
    }
}
