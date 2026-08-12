import Foundation
import XCTest
@testable import XDRemuxAppleFeatures

/// Actions-only harness for producing a real-device validation pair without repeating the expensive
/// PhotoKit lifecycle check for every artifact. The strict real-fixture gate runs immediately before
/// this harness and validates the same production converter through PhotoKit. This test still calls
/// the production conversion engine and therefore retains its structural, metadata, gain-map,
/// compressed-video/audio passthrough, and transactional validation.
final class LivePhotoDeviceValidationBundleTests: XCTestCase {
    func testGeneratePairUsingProductionConverterWithoutRepeatedPhotoKitLoad() async throws {
        let environment = ProcessInfo.processInfo.environment
        guard let sourcePath = environment["XDREMUX_DEVICE_VALIDATION_SOURCE"],
              !sourcePath.isEmpty,
              let outputPath = environment["XDREMUX_DEVICE_VALIDATION_OUTPUT"],
              !outputPath.isEmpty else {
            throw XCTSkip("device-validation source/output environment is not configured")
        }

        let sourceURL = URL(fileURLWithPath: sourcePath).standardizedFileURL
        let outputImageURL = URL(fileURLWithPath: outputPath).standardizedFileURL
        guard FileManager.default.fileExists(atPath: sourceURL.path) else {
            XCTFail("device-validation source does not exist: \(sourceURL.path)")
            return
        }

        let result = try await AppleLivePhotoConversionEngine.convertAsync(
            inputURL: sourceURL,
            outputImageURL: outputImageURL,
            requirePhotoKitValidation: false
        )
        let expectedVideoURL = AppleLivePhotoConversionEngine.companionVideoURL(for: outputImageURL)

        XCTAssertEqual(result.imageURL.standardizedFileURL, outputImageURL)
        XCTAssertEqual(result.videoURL.standardizedFileURL, expectedVideoURL.standardizedFileURL)
        XCTAssertTrue(FileManager.default.fileExists(atPath: result.imageURL.path))
        XCTAssertTrue(FileManager.default.fileExists(atPath: result.videoURL.path))
        XCTAssertTrue(
            AppleLivePhotoValidator.isValidPair(
                imageURL: result.imageURL,
                videoURL: result.videoURL
            ),
            "production converter did not publish a structurally valid Live Photo pair"
        )
        XCTAssertEqual(
            AppleLivePhotoStillWriter.assetIdentifier(in: result.imageURL),
            result.assetIdentifier,
            "published still lost the conversion asset identifier"
        )
        XCTAssertEqual(
            await AppleLivePhotoVideoWriter.contentIdentifier(in: result.videoURL),
            result.assetIdentifier,
            "published MOV lost the conversion asset identifier"
        )
    }
}
