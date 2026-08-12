import Foundation
import XCTest
@testable import XDRemuxAppleFeatures

final class LivePhotoPairPublisherTests: XCTestCase {
    func testPublishReplacesBothResources() throws {
        let directory = try makeDirectory("publish")
        defer { try? FileManager.default.removeItem(at: directory) }
        let image = directory.appendingPathComponent("photo.heic")
        let video = directory.appendingPathComponent("photo.mov")
        let tempImage = directory.appendingPathComponent(".photo.tx.tmp.heic")
        let tempVideo = directory.appendingPathComponent(".photo.tx.tmp.mov")
        try Data("old-image".utf8).write(to: image)
        try Data("old-video".utf8).write(to: video)
        try Data("new-image".utf8).write(to: tempImage)
        try Data("new-video".utf8).write(to: tempVideo)

        try LivePhotoPairPublisher.publish(
            temporaryImageURL: tempImage,
            temporaryVideoURL: tempVideo,
            finalImageURL: image,
            finalVideoURL: video
        )

        XCTAssertEqual(try Data(contentsOf: image), Data("new-image".utf8))
        XCTAssertEqual(try Data(contentsOf: video), Data("new-video".utf8))
        XCTAssertFalse(FileManager.default.fileExists(atPath: tempImage.path))
        XCTAssertFalse(FileManager.default.fileExists(atPath: tempVideo.path))
        XCTAssertTrue((try FileManager.default.contentsOfDirectory(atPath: directory.path)).allSatisfy {
            !$0.hasSuffix(".backup")
        })
    }

    func testPublishRejectsTemporaryPairOutsideDestinationDirectory() throws {
        let directory = try makeDirectory("destination")
        let other = try makeDirectory("other")
        defer {
            try? FileManager.default.removeItem(at: directory)
            try? FileManager.default.removeItem(at: other)
        }
        let image = directory.appendingPathComponent("photo.heic")
        let video = directory.appendingPathComponent("photo.mov")
        let tempImage = other.appendingPathComponent("photo.tmp.heic")
        let tempVideo = other.appendingPathComponent("photo.tmp.mov")
        try Data("image".utf8).write(to: tempImage)
        try Data("video".utf8).write(to: tempVideo)

        XCTAssertThrowsError(
            try LivePhotoPairPublisher.publish(
                temporaryImageURL: tempImage,
                temporaryVideoURL: tempVideo,
                finalImageURL: image,
                finalVideoURL: video
            )
        )
    }

    func testReconcileRemovesIncompletePairAndStaleArtifacts() throws {
        let directory = try makeDirectory("reconcile-incomplete")
        defer { try? FileManager.default.removeItem(at: directory) }
        let image = directory.appendingPathComponent("photo.heic")
        let video = directory.appendingPathComponent("photo.mov")
        let staleTemp = directory.appendingPathComponent(".photo.deadbeef.tmp.mov")
        let staleBackup = directory.appendingPathComponent(".photo.heic.deadbeef.backup")
        let legacyJournal = directory.appendingPathComponent(".xdremux-live-photo-transactions", isDirectory: true)
        try Data("partial-image".utf8).write(to: image)
        try Data("temp".utf8).write(to: staleTemp)
        try Data("backup".utf8).write(to: staleBackup)
        try FileManager.default.createDirectory(at: legacyJournal, withIntermediateDirectories: true)
        try Data("{}".utf8).write(to: legacyJournal.appendingPathComponent("old.json"))

        try LivePhotoPairPublisher.reconcile(
            finalImageURL: image,
            finalVideoURL: video,
            validatePair: { _, _ in false }
        )

        XCTAssertFalse(FileManager.default.fileExists(atPath: image.path))
        XCTAssertFalse(FileManager.default.fileExists(atPath: video.path))
        XCTAssertFalse(FileManager.default.fileExists(atPath: staleTemp.path))
        XCTAssertFalse(FileManager.default.fileExists(atPath: staleBackup.path))
        XCTAssertFalse(FileManager.default.fileExists(atPath: legacyJournal.path))
    }

    func testReconcileKeepsValidPair() throws {
        let directory = try makeDirectory("reconcile-valid")
        defer { try? FileManager.default.removeItem(at: directory) }
        let image = directory.appendingPathComponent("photo.heic")
        let video = directory.appendingPathComponent("photo.mov")
        try Data("image".utf8).write(to: image)
        try Data("video".utf8).write(to: video)

        try LivePhotoPairPublisher.reconcile(
            finalImageURL: image,
            finalVideoURL: video,
            validatePair: { candidateImage, candidateVideo in
                candidateImage == image && candidateVideo == video
            }
        )

        XCTAssertEqual(try Data(contentsOf: image), Data("image".utf8))
        XCTAssertEqual(try Data(contentsOf: video), Data("video".utf8))
    }

    func testReconcileRemovesInvalidCompletePair() throws {
        let directory = try makeDirectory("reconcile-invalid")
        defer { try? FileManager.default.removeItem(at: directory) }
        let image = directory.appendingPathComponent("photo.heic")
        let video = directory.appendingPathComponent("photo.mov")
        try Data("image".utf8).write(to: image)
        try Data("video".utf8).write(to: video)

        try LivePhotoPairPublisher.reconcile(
            finalImageURL: image,
            finalVideoURL: video,
            validatePair: { _, _ in false }
        )

        XCTAssertFalse(FileManager.default.fileExists(atPath: image.path))
        XCTAssertFalse(FileManager.default.fileExists(atPath: video.path))
    }

    private func makeDirectory(_ suffix: String) throws -> URL {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(
            "xdremux-live-photo-publisher-\(suffix)-\(UUID().uuidString)",
            isDirectory: true
        )
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        return directory
    }
}
