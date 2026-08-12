import Foundation
import XCTest
@testable import XDRemuxAppleFeatures

final class LivePhotoPairTransactionTests: XCTestCase {
    func testCommitReplacesBothResourcesAndCleansJournal() throws {
        let directory = try makeDirectory("commit")
        defer { try? FileManager.default.removeItem(at: directory) }
        let image = directory.appendingPathComponent("photo.heic")
        let video = directory.appendingPathComponent("photo.mov")
        let tempImage = directory.appendingPathComponent(".photo.tx.tmp.heic")
        let tempVideo = directory.appendingPathComponent(".photo.tx.tmp.mov")
        try Data("old-image".utf8).write(to: image)
        try Data("old-video".utf8).write(to: video)
        try Data("new-image".utf8).write(to: tempImage)
        try Data("new-video".utf8).write(to: tempVideo)

        try LivePhotoPairTransaction.commit(
            temporaryImageURL: tempImage,
            temporaryVideoURL: tempVideo,
            finalImageURL: image,
            finalVideoURL: video,
            validatePair: { _, _ in true }
        )

        XCTAssertEqual(try Data(contentsOf: image), Data("new-image".utf8))
        XCTAssertEqual(try Data(contentsOf: video), Data("new-video".utf8))
        let journalDirectory = directory.appendingPathComponent(
            LivePhotoPairTransaction.journalDirectoryName,
            isDirectory: true
        )
        let journals = (try? FileManager.default.contentsOfDirectory(at: journalDirectory, includingPropertiesForKeys: nil)) ?? []
        XCTAssertFalse(journals.contains { $0.pathExtension == "json" })
    }

    func testCommitRejectsTemporaryPairOutsideDestinationDirectory() throws {
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
            try LivePhotoPairTransaction.commit(
                temporaryImageURL: tempImage,
                temporaryVideoURL: tempVideo,
                finalImageURL: image,
                finalVideoURL: video,
                validatePair: { _, _ in true }
            )
        )
    }

    func testRecoveryRestoresOriginalsAfterImageWasInstalled() throws {
        let directory = try makeDirectory("recovery-image")
        defer { try? FileManager.default.removeItem(at: directory) }
        let id = "abc123"
        let image = directory.appendingPathComponent("photo.heic")
        let video = directory.appendingPathComponent("photo.mov")
        let tempImage = directory.appendingPathComponent(".photo.abc123.tmp.heic")
        let tempVideo = directory.appendingPathComponent(".photo.abc123.tmp.mov")
        let imageBackup = directory.appendingPathComponent(".photo.heic.abc123.backup")
        let videoBackup = directory.appendingPathComponent(".photo.mov.abc123.backup")
        try Data("new-image".utf8).write(to: image)
        try Data("new-video".utf8).write(to: tempVideo)
        try Data("old-image".utf8).write(to: imageBackup)
        try Data("old-video".utf8).write(to: videoBackup)

        let manifest = LivePhotoPairTransaction.Manifest(
            schemaVersion: LivePhotoPairTransaction.schemaVersion,
            transactionID: id,
            state: .imageInstalled,
            finalImage: image.lastPathComponent,
            finalVideo: video.lastPathComponent,
            temporaryImage: tempImage.lastPathComponent,
            temporaryVideo: tempVideo.lastPathComponent,
            backupImage: imageBackup.lastPathComponent,
            backupVideo: videoBackup.lastPathComponent,
            hadImage: true,
            hadVideo: true
        )
        _ = try LivePhotoPairTransaction.writeManifest(manifest, in: directory)

        try LivePhotoPairTransaction.recover(in: directory, validatePair: { _, _ in false })

        XCTAssertEqual(try Data(contentsOf: image), Data("old-image".utf8))
        XCTAssertEqual(try Data(contentsOf: video), Data("old-video".utf8))
        XCTAssertFalse(FileManager.default.fileExists(atPath: tempVideo.path))
    }

    func testRecoveryInfersRenameThatHappenedBeforeStateUpdate() throws {
        let directory = try makeDirectory("recovery-window")
        defer { try? FileManager.default.removeItem(at: directory) }
        let id = "deadbeef"
        let image = directory.appendingPathComponent("photo.heic")
        let video = directory.appendingPathComponent("photo.mov")
        let tempImage = directory.appendingPathComponent(".photo.deadbeef.tmp.heic")
        let tempVideo = directory.appendingPathComponent(".photo.deadbeef.tmp.mov")
        try Data("new-image".utf8).write(to: image)
        try Data("new-video".utf8).write(to: tempVideo)

        let manifest = LivePhotoPairTransaction.Manifest(
            schemaVersion: LivePhotoPairTransaction.schemaVersion,
            transactionID: id,
            state: .originalsBackedUp,
            finalImage: image.lastPathComponent,
            finalVideo: video.lastPathComponent,
            temporaryImage: tempImage.lastPathComponent,
            temporaryVideo: tempVideo.lastPathComponent,
            backupImage: ".photo.heic.deadbeef.backup",
            backupVideo: ".photo.mov.deadbeef.backup",
            hadImage: false,
            hadVideo: false
        )
        _ = try LivePhotoPairTransaction.writeManifest(manifest, in: directory)

        try LivePhotoPairTransaction.recover(in: directory, validatePair: { _, _ in false })

        XCTAssertFalse(FileManager.default.fileExists(atPath: image.path))
        XCTAssertFalse(FileManager.default.fileExists(atPath: video.path))
        XCTAssertFalse(FileManager.default.fileExists(atPath: tempVideo.path))
    }

    func testPairInstalledRecoveryFinalizesValidPair() throws {
        let directory = try makeDirectory("recovery-finalize")
        defer { try? FileManager.default.removeItem(at: directory) }
        let id = "feedface"
        let image = directory.appendingPathComponent("photo.heic")
        let video = directory.appendingPathComponent("photo.mov")
        let imageBackup = directory.appendingPathComponent(".photo.heic.feedface.backup")
        let videoBackup = directory.appendingPathComponent(".photo.mov.feedface.backup")
        try Data("new-image".utf8).write(to: image)
        try Data("new-video".utf8).write(to: video)
        try Data("old-image".utf8).write(to: imageBackup)
        try Data("old-video".utf8).write(to: videoBackup)

        let manifest = LivePhotoPairTransaction.Manifest(
            schemaVersion: LivePhotoPairTransaction.schemaVersion,
            transactionID: id,
            state: .pairInstalled,
            finalImage: image.lastPathComponent,
            finalVideo: video.lastPathComponent,
            temporaryImage: ".photo.feedface.tmp.heic",
            temporaryVideo: ".photo.feedface.tmp.mov",
            backupImage: imageBackup.lastPathComponent,
            backupVideo: videoBackup.lastPathComponent,
            hadImage: true,
            hadVideo: true
        )
        _ = try LivePhotoPairTransaction.writeManifest(manifest, in: directory)

        try LivePhotoPairTransaction.recover(in: directory, validatePair: { candidateImage, candidateVideo in
            candidateImage == image && candidateVideo == video
        })

        XCTAssertEqual(try Data(contentsOf: image), Data("new-image".utf8))
        XCTAssertEqual(try Data(contentsOf: video), Data("new-video".utf8))
        XCTAssertFalse(FileManager.default.fileExists(atPath: imageBackup.path))
        XCTAssertFalse(FileManager.default.fileExists(atPath: videoBackup.path))
    }

    private func makeDirectory(_ suffix: String) throws -> URL {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(
            "xdremux-live-photo-transaction-\(suffix)-\(UUID().uuidString)",
            isDirectory: true
        )
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        return directory
    }
}
