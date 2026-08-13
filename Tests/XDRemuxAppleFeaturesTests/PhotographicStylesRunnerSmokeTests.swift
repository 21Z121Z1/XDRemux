import Foundation
import XCTest
import XDRemuxCore
@testable import XDRemuxAppleFeatures

final class PhotographicStylesRunnerSmokeTests: XCTestCase {
    func testColorOS16MotionPhotoConvertsToLivePhotoWithPhotographicStyles() throws {
        let environment = ProcessInfo.processInfo.environment
        let repositoryRoot = URL(
            fileURLWithPath: environment["GITHUB_WORKSPACE"] ?? FileManager.default.currentDirectoryPath,
            isDirectory: true
        )
        let fixtureURL = URL(
            fileURLWithPath: environment["XDREMUX_STYLE_RUNNER_FIXTURE"]
                ?? repositoryRoot
                    .appendingPathComponent("fixtures/IMG20260801190843_ColorOS_16.jpg")
                    .path
        ).standardizedFileURL
        let outputRoot = URL(
            fileURLWithPath: environment["XDREMUX_STYLE_RUNNER_OUTPUT"]
                ?? FileManager.default.temporaryDirectory
                    .appendingPathComponent("xdremux-macos26-photographic-styles-smoke")
                    .path,
            isDirectory: true
        ).standardizedFileURL

        XCTAssertTrue(FileManager.default.fileExists(atPath: fixtureURL.path), fixtureURL.path)
        try FileManager.default.createDirectory(at: outputRoot, withIntermediateDirectories: true)

        let asset = try XCTUnwrap(
            OppoMotionPhotoParser.parse(url: fixtureURL),
            "ColorOS 16 fixture must remain an OPPO Motion Photo"
        )
        XCTAssertEqual(asset.sourceKind, .oppoLivePhoto)
        XCTAssertEqual(asset.stillResourceRange.lowerBound, 0)
        XCTAssertEqual(
            asset.stillResourceRange.upperBound,
            13_591_436,
            "fixture static-resource boundary changed"
        )

        let outputImageURL = outputRoot.appendingPathComponent("coloros16-live-styles.heic")
        let outputVideoURL = AppleLivePhotoConversionEngine.companionVideoURL(for: outputImageURL)
        let validationURL = outputRoot.appendingPathComponent("validation.json")
        let summaryURL = outputRoot.appendingPathComponent("smoke-summary.json")
        for stale in [outputImageURL, outputVideoURL, validationURL, summaryURL] {
            try? FileManager.default.removeItem(at: stale)
        }

        // Keep this smoke gate deterministic and focused on hosted-runner capability. The full
        // constrained solver is exercised separately after the combined graph itself is green.
        let stylesConfiguration = ConversionConfiguration(
            skipExisting: false,
            applePhotographicStyles: true,
            appleStyleDataProducer: .identityFallback
        )
        let result = try AppleLivePhotoConversionEngine.convert(
            inputURL: fixtureURL,
            outputImageURL: outputImageURL,
            requirePhotoKitValidation: true,
            photographicStylesConfiguration: stylesConfiguration
        )

        XCTAssertEqual(result.imageURL, outputImageURL)
        XCTAssertEqual(result.videoURL, outputVideoURL)
        XCTAssertEqual(result.sourceKind, .oppoLivePhoto)
        XCTAssertEqual(
            AppleLivePhotoStillWriter.assetIdentifier(in: outputImageURL),
            result.assetIdentifier,
            "Photographic Styles rewrite changed the Live Photo asset identifier"
        )
        XCTAssertTrue(
            AppleLivePhotoValidator.isValidPair(imageURL: outputImageURL, videoURL: outputVideoURL),
            "combined output is not a structurally valid Apple Live Photo pair"
        )
        XCTAssertTrue(
            AppleLivePhotoStillWriter.hasGainMap(outputImageURL),
            "combined output lost the Ultra HDR gain map"
        )

        let report = try AppleFeatureConversionEngine.validationReport(
            for: outputImageURL,
            expectsPortrait: false
        )
        XCTAssertEqual(report["passed"] as? Bool, true)
        XCTAssertEqual(report["semanticStyleProperties"] as? Bool, true)
        XCTAssertEqual((report["styleDataLength"] as? NSNumber)?.intValue, 51_840)

        let reportData = try JSONSerialization.data(
            withJSONObject: report,
            options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        )
        try reportData.write(to: validationURL, options: .atomic)

        let imageBytes = (try? outputImageURL.resourceValues(forKeys: [.fileSizeKey]).fileSize) ?? -1
        let videoBytes = (try? outputVideoURL.resourceValues(forKeys: [.fileSizeKey]).fileSize) ?? -1
        let summary: [String: Any] = [
            "fixture": fixtureURL.lastPathComponent,
            "producer": "identity-fallback",
            "livePhotoPairValid": true,
            "photoKitValidationRequired": true,
            "assetIdentifierPreserved": true,
            "gainMapPreserved": true,
            "photographicStylesPassed": report["passed"] as? Bool ?? false,
            "semanticStyleProperties": report["semanticStyleProperties"] as? Bool ?? false,
            "styleDataLength": (report["styleDataLength"] as? NSNumber)?.intValue ?? -1,
            "image": outputImageURL.lastPathComponent,
            "imageBytes": imageBytes,
            "video": outputVideoURL.lastPathComponent,
            "videoBytes": videoBytes,
        ]
        let summaryData = try JSONSerialization.data(
            withJSONObject: summary,
            options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        )
        try summaryData.write(to: summaryURL, options: .atomic)
        FileHandle.standardOutput.write(summaryData)
        FileHandle.standardOutput.write(Data("\n".utf8))
    }
}
