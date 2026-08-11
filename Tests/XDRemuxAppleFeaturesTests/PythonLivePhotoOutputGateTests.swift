import Foundation
import CoreMedia
import XCTest
import XDRemuxCore
@testable import XDRemuxAppleFeatures

/// Apple-platform acceptance oracle for resource pairs produced entirely by the Python runtime.
/// The Python conversion step must not import or invoke Apple frameworks. This test runs only
/// afterwards on macOS and asks the existing structural validator + PhotoKit to consume the pair.
final class PythonLivePhotoOutputGateTests: XCTestCase {
    private struct Manifest: Decodable {
        let fixtures: [Entry]
    }

    private struct Entry: Decodable {
        let sourceFilename: String
        let sourcePath: String
        let sourceKind: String
        let outputImagePath: String
        let outputVideoPath: String
        let contentIdentifier: String
        let stillImageTimeSeconds: Double
        let expectsGainMap: Bool
    }

    func testPurePythonOutputsLoadAsAppleLivePhotos() async throws {
        guard let manifestPath = ProcessInfo.processInfo.environment["XDREMUX_PYTHON_LIVE_PHOTO_MANIFEST"],
              !manifestPath.isEmpty else {
            throw XCTSkip("XDREMUX_PYTHON_LIVE_PHOTO_MANIFEST is not configured")
        }
        let manifestURL = URL(fileURLWithPath: manifestPath)
        let manifest = try JSONDecoder().decode(Manifest.self, from: Data(contentsOf: manifestURL))
        XCTAssertEqual(manifest.fixtures.count, 14, "strict gate must cover every supplied fixture")

        for entry in manifest.fixtures {
            let sourceURL = URL(fileURLWithPath: entry.sourcePath)
            let imageURL = URL(fileURLWithPath: entry.outputImagePath)
            let videoURL = URL(fileURLWithPath: entry.outputVideoPath)
            let sourceAsset = try XCTUnwrap(
                OppoMotionPhotoParser.parse(url: sourceURL),
                "Swift reference parser rejected source fixture: \(entry.sourceFilename)"
            )
            XCTAssertEqual(sourceAsset.sourceKind.rawValue, entry.sourceKind, entry.sourceFilename)
            let expectsTransform = sourceAsset.vendorMetadata
                .flatMap(OppoLivePhotoAlignment.transformMatrix) != nil
            let expectedTime = CMTime(
                seconds: entry.stillImageTimeSeconds,
                preferredTimescale: 1_000_000
            )

            let report = try await AppleLivePhotoValidator.validate(
                imageURL: imageURL,
                videoURL: videoURL,
                expectedAssetIdentifier: entry.contentIdentifier,
                expectedStillImageTime: expectedTime,
                sourceHadGainMap: entry.expectsGainMap,
                expectsOppoTransform: expectsTransform,
                requirePhotoKitLoad: true
            )
            XCTAssertEqual(report.assetIdentifier, entry.contentIdentifier, entry.sourceFilename)
            XCTAssertEqual(report.hasGainMap, entry.expectsGainMap, entry.sourceFilename)
            XCTAssertEqual(report.hasTransform, expectsTransform, entry.sourceFilename)
        }
    }
}
