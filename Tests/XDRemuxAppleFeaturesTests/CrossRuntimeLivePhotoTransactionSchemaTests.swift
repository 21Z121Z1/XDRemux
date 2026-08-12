import Foundation
import XCTest
@testable import XDRemuxAppleFeatures

final class CrossRuntimeLivePhotoTransactionSchemaTests: XCTestCase {
    func testSwiftRecoversCanonicalPythonCommittedJournal() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(
            "xdremux-python-journal-\(UUID().uuidString)",
            isDirectory: true
        )
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        let journalDirectory = directory.appendingPathComponent(
            LivePhotoPairTransaction.journalDirectoryName,
            isDirectory: true
        )
        try FileManager.default.createDirectory(at: journalDirectory, withIntermediateDirectories: true)

        let transactionID = "feed-face"
        let image = directory.appendingPathComponent("photo.heic")
        let video = directory.appendingPathComponent("photo.mov")
        let imageBackup = directory.appendingPathComponent(".photo.heic.feed-face.backup")
        let videoBackup = directory.appendingPathComponent(".photo.mov.feed-face.backup")
        try Data("new-image".utf8).write(to: image)
        try Data("new-video".utf8).write(to: video)
        try Data("old-image".utf8).write(to: imageBackup)
        try Data("old-video".utf8).write(to: videoBackup)

        // Exact camelCase wire names emitted by xdremux_py.live_photo_transaction.
        let object: [String: Any] = [
            "schemaVersion": LivePhotoPairTransaction.schemaVersion,
            "transactionID": transactionID,
            "state": "committed",
            "finalImage": image.lastPathComponent,
            "finalVideo": video.lastPathComponent,
            "temporaryImage": ".photo.feed-face.tmp.heic",
            "temporaryVideo": ".photo.feed-face.tmp.mov",
            "backupImage": imageBackup.lastPathComponent,
            "backupVideo": videoBackup.lastPathComponent,
            "hadImage": true,
            "hadVideo": true,
        ]
        let data = try JSONSerialization.data(withJSONObject: object, options: [.sortedKeys])
        try data.write(
            to: journalDirectory
                .appendingPathComponent(transactionID)
                .appendingPathExtension("json")
        )

        try LivePhotoPairTransaction.recover(in: directory, validatePair: { _, _ in false })

        XCTAssertEqual(try Data(contentsOf: image), Data("new-image".utf8))
        XCTAssertEqual(try Data(contentsOf: video), Data("new-video".utf8))
        XCTAssertFalse(FileManager.default.fileExists(atPath: imageBackup.path))
        XCTAssertFalse(FileManager.default.fileExists(atPath: videoBackup.path))
        XCTAssertFalse(
            FileManager.default.fileExists(
                atPath: journalDirectory
                    .appendingPathComponent(transactionID)
                    .appendingPathExtension("json")
                    .path
            )
        )
    }
}
