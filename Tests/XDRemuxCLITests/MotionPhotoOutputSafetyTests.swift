import CryptoKit
import Foundation
import XCTest
import XDRemuxAppleFeatures
@testable import XDRemuxCLI

final class MotionPhotoOutputSafetyTests: XCTestCase {
    private let fixtureName = "motion-photo/oppo/coloros16-dualstream-ultrahdr-01.jpg"

    func testImplicitPlannerPreservesForeignSameBasenamePairAndConvertedOutputUsesNumberedNamespace() throws {
        let sourceFixture = try fixtureURL()
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-motion-output-safety-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        let source = directory.appendingPathComponent(fixtureName)
        try FileManager.default.copyItem(at: sourceFixture, to: source)
        let stem = source.deletingPathExtension().lastPathComponent
        let foreignImage = directory.appendingPathComponent(stem).appendingPathExtension("heic")
        let foreignVideo = directory.appendingPathComponent(stem).appendingPathExtension("mov")
        try Data("foreign-heic-do-not-touch".utf8).write(to: foreignImage, options: .atomic)
        try Data("foreign-mov-do-not-touch".utf8).write(to: foreignVideo, options: .atomic)
        let imageDigest = try digest(foreignImage)
        let videoDigest = try digest(foreignVideo)

        var reserved = Set<String>()
        let outputImage = MotionPhotoBatchPlanner.reserveOutputImageURL(
            for: source,
            inputRootURL: directory,
            outputDirectoryURL: directory,
            reservedPaths: &reserved
        )
        let outputVideo = AppleLivePhotoConversionEngine.companionVideoURL(for: outputImage)
        XCTAssertEqual(outputImage.lastPathComponent, "\(stem) (2).heic")
        XCTAssertEqual(outputVideo.lastPathComponent, "\(stem) (2).mov")

        _ = try AppleLivePhotoConversionEngine.convert(
            inputURL: source,
            outputImageURL: outputImage,
            requirePhotoKitValidation: false
        )

        XCTAssertEqual(try digest(foreignImage), imageDigest)
        XCTAssertEqual(try digest(foreignVideo), videoDigest)
        XCTAssertTrue(FileManager.default.fileExists(atPath: outputImage.path))
        XCTAssertTrue(FileManager.default.fileExists(atPath: outputVideo.path))
        XCTAssertTrue(AppleLivePhotoValidator.isValidPair(imageURL: outputImage, videoURL: outputVideo))
    }

    func testExplicitConvertRefusesExistingForeignOutputAndPreservesBytes() throws {
        let sourceFixture = try fixtureURL()
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-motion-explicit-output-safety-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        let source = directory.appendingPathComponent(fixtureName)
        try FileManager.default.copyItem(at: sourceFixture, to: source)
        let outputImage = directory.appendingPathComponent("user-owned.heic")
        let outputVideo = directory.appendingPathComponent("user-owned.mov")
        try Data("foreign-explicit-heic".utf8).write(to: outputImage, options: .atomic)
        try Data("foreign-explicit-mov".utf8).write(to: outputVideo, options: .atomic)
        let imageDigest = try digest(outputImage)
        let videoDigest = try digest(outputVideo)

        XCTAssertThrowsError(try MotionPhotoCLIIntegration.handleIfNeeded([
            "convert", "--input", source.path, "--output", outputImage.path,
        ]))
        XCTAssertEqual(try digest(outputImage), imageDigest)
        XCTAssertEqual(try digest(outputVideo), videoDigest)
    }

    private func fixtureURL() throws -> URL {
        let root = URL(fileURLWithPath: FileManager.default.currentDirectoryPath, isDirectory: true)
        let fixture = root.appendingPathComponent("fixtures").appendingPathComponent(fixtureName)
        guard FileManager.default.fileExists(atPath: fixture.path) else {
            throw XCTSkip("repository Motion Photo fixture is unavailable: \(fixture.path)")
        }
        return fixture
    }

    private func digest(_ url: URL) throws -> SHA256.Digest {
        SHA256.hash(data: try Data(contentsOf: url))
    }
}
