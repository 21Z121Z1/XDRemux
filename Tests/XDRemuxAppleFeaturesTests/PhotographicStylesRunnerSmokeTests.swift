import Foundation
import XCTest
import XDRemuxCore
@testable import XDRemuxAppleFeatures

final class PhotographicStylesRunnerSmokeTests: XCTestCase {
    func testColorOS16MotionPhotoStillGeneratesPhotographicStyles() throws {
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
        try? FileManager.default.removeItem(at: outputRoot)
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

        let sourceData = try Data(contentsOf: fixtureURL, options: [.mappedIfSafe])
        let stillLower = try XCTUnwrap(Int(exactly: asset.stillResourceRange.lowerBound))
        let stillUpper = try XCTUnwrap(Int(exactly: asset.stillResourceRange.upperBound))
        guard stillLower >= 0, stillUpper > stillLower, stillUpper <= sourceData.count else {
            XCTFail("Motion Photo parser returned an invalid static-resource range")
            return
        }

        let baseJPEGURL = outputRoot.appendingPathComponent("coloros16-base.jpg")
        let baseHEICURL = outputRoot.appendingPathComponent("coloros16-base.heic")
        let stylesHEICURL = outputRoot.appendingPathComponent("coloros16-base-styles.heic")
        let diagnosticsURL = outputRoot.appendingPathComponent("diagnostics", isDirectory: true)
        let validationURL = outputRoot.appendingPathComponent("validation.json")

        try sourceData.subdata(in: stillLower..<stillUpper).write(to: baseJPEGURL, options: .atomic)

        try AppleLivePhotoStillWriter.write(
            stillInputURL: baseJPEGURL,
            outputURL: baseHEICURL,
            assetIdentifier: "xdremux-macos26-photographic-styles-smoke",
            lossyCompressionQuality: 1.0
        )
        XCTAssertTrue(
            AppleLivePhotoStillWriter.hasGainMap(baseHEICURL),
            "ImageIO failed to preserve the ColorOS 16 Ultra HDR gain map"
        )
        XCTAssertTrue(
            AppleFeatureConversionEngine.hasValidISOGainMap(baseHEICURL),
            "materialized base HEIC is not an ImageIO-readable ISO gain-map image"
        )

        let configuration = ConversionConfiguration(
            debugDirectory: diagnosticsURL,
            skipExisting: false,
            applePhotographicStyles: true,
            appleStyleDataProducer: .constrainedSolver
        )
        try AppleFeatureConversionEngine.convert(
            inputURL: baseHEICURL,
            outputURL: stylesHEICURL,
            configuration: configuration
        )

        let report = try AppleFeatureConversionEngine.validationReport(
            for: stylesHEICURL,
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

        let summary: [String: Any] = [
            "fixture": fixtureURL.lastPathComponent,
            "fixtureBytes": sourceData.count,
            "stillResourceBytes": stillUpper - stillLower,
            "baseJPEG": baseJPEGURL.path,
            "baseHEIC": baseHEICURL.path,
            "stylesHEIC": stylesHEICURL.path,
            "validation": validationURL.path,
            "passed": report["passed"] as? Bool ?? false,
            "semanticStyleProperties": report["semanticStyleProperties"] as? Bool ?? false,
            "styleDataLength": (report["styleDataLength"] as? NSNumber)?.intValue ?? -1,
        ]
        let summaryData = try JSONSerialization.data(
            withJSONObject: summary,
            options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        )
        try summaryData.write(
            to: outputRoot.appendingPathComponent("smoke-summary.json"),
            options: .atomic
        )
        FileHandle.standardOutput.write(summaryData)
        FileHandle.standardOutput.write(Data("\n".utf8))
    }
}
