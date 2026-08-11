import Foundation
import XCTest
import XDRemuxCore
@testable import XDRemuxAppleFeatures

/// Optional characterization + end-to-end gates for the real OPPO samples used while developing
/// LivePhotoToolbox. CI runs these after unpacking the private fixture archive when the files are
/// present. Normal open-source test runs skip them rather than depending on private media.
final class RealMotionPhotoFixtureTests: XCTestCase {
    func testColorOS15RealFixture() async throws {
        let source = try requireFixture(named: "IMG20250425184722.jpg")
        let asset = try XCTUnwrap(OppoMotionPhotoParser.parse(url: source))

        XCTAssertEqual(asset.sourceKind, .oppoLivePhoto)
        XCTAssertEqual(asset.stillResourceRange.upperBound, 1_905_360)
        XCTAssertEqual(asset.videoResourceRange.lowerBound, 1_905_360)
        XCTAssertEqual(asset.presentationTimestampUs, 1_265_580)
        XCTAssertEqual(asset.presentationSource, .androidXMP)
        XCTAssertEqual(asset.vendorMetadata?.streamCount, 1)

        try await assertEndToEndConversion(source: source)
    }

    func testColorOS16RealFixture() async throws {
        let source = try requireFixture(named: "IMG20250901181353.jpg")
        let asset = try XCTUnwrap(OppoMotionPhotoParser.parse(url: source))

        XCTAssertEqual(asset.sourceKind, .oppoLivePhoto)
        XCTAssertEqual(asset.stillResourceRange.upperBound, 8_273_274)
        XCTAssertEqual(asset.videoResourceRange.lowerBound, 8_273_274)
        XCTAssertEqual(asset.presentationTimestampUs, 1_634_640)
        XCTAssertEqual(asset.presentationSource, .androidXMP)
        XCTAssertGreaterThanOrEqual(asset.vendorMetadata?.streamCount ?? 0, 2)

        let primary = try OppoMotionPhotoStreamResolver.primaryVideoRange(for: asset)
        XCTAssertEqual(primary.lowerBound, 8_273_274)
        XCTAssertEqual(primary.upperBound, 15_283_688)

        try await assertEndToEndConversion(source: source)
    }

    private func assertEndToEndConversion(source: URL) async throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-real-motion-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        let output = directory.appendingPathComponent(source.deletingPathExtension().lastPathComponent)
            .appendingPathExtension("heic")
        let result = try await AppleLivePhotoConversionEngine.convertAsync(
            inputURL: source,
            outputImageURL: output,
            requirePhotoKitValidation: true
        )
        XCTAssertTrue(FileManager.default.fileExists(atPath: result.imageURL.path))
        XCTAssertTrue(FileManager.default.fileExists(atPath: result.videoURL.path))
        XCTAssertEqual(
            AppleLivePhotoStillWriter.assetIdentifier(in: result.imageURL),
            result.assetIdentifier
        )
        let videoIdentifier = await AppleLivePhotoVideoWriter.contentIdentifier(in: result.videoURL)
        XCTAssertEqual(videoIdentifier, result.assetIdentifier)
    }

    private func requireFixture(named filename: String) throws -> URL {
        guard let rootPath = ProcessInfo.processInfo.environment["XDREMUX_MOTION_PHOTO_FIXTURE_ROOT"],
              !rootPath.isEmpty else {
            throw XCTSkip("XDREMUX_MOTION_PHOTO_FIXTURE_ROOT is not configured")
        }
        let root = URL(fileURLWithPath: rootPath).standardizedFileURL
        if root.lastPathComponent == filename,
           FileManager.default.fileExists(atPath: root.path) {
            return root
        }
        let keys: [URLResourceKey] = [.isRegularFileKey]
        guard let enumerator = FileManager.default.enumerator(
            at: root,
            includingPropertiesForKeys: keys,
            options: [.skipsHiddenFiles]
        ) else {
            throw XCTSkip("cannot enumerate private Motion Photo fixture root")
        }
        for case let url as URL in enumerator where url.lastPathComponent == filename {
            if (try? url.resourceValues(forKeys: [.isRegularFileKey]).isRegularFile) == true {
                return url
            }
        }
        throw XCTSkip("private Motion Photo fixture \(filename) is not present")
    }
}
