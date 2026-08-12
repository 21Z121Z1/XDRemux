import Darwin
import Foundation

/// Crash-recoverable commit protocol for the HEIC+MOV resources that form one Live Photo.
///
/// A filesystem cannot atomically rename two independent files as one operation. This transaction
/// therefore keeps a durable, same-volume journal and can either finish a fully installed valid pair
/// or restore the previous pair after SIGKILL, power loss, or another abrupt process termination.
enum LivePhotoPairTransaction {
    static let journalDirectoryName = ".xdremux-live-photo-transactions"
    static let lockFileName = ".xdremux-live-photo-transactions.lock"
    static let schemaVersion = 1

    enum State: String, Codable {
        case prepared
        case originalsBackedUp = "originals_backed_up"
        case imageInstalled = "image_installed"
        case pairInstalled = "pair_installed"
        case committed
    }

    struct Manifest: Codable {
        let schemaVersion: Int
        let transactionID: String
        var state: State
        let finalImage: String
        let finalVideo: String
        let temporaryImage: String
        let temporaryVideo: String
        let backupImage: String
        let backupVideo: String
        let hadImage: Bool
        let hadVideo: Bool
    }

    typealias PairValidator = (URL, URL) -> Bool

    static func recover(
        in directoryURL: URL,
        validatePair: PairValidator? = nil
    ) throws {
        let directory = directoryURL.standardizedFileURL
        guard FileManager.default.fileExists(atPath: directory.path) else { return }
        try withDirectoryLock(directory) {
            try recoverLocked(in: directory, validatePair: validatePair)
        }
    }

    static func commit(
        temporaryImageURL: URL,
        temporaryVideoURL: URL,
        finalImageURL: URL,
        finalVideoURL: URL,
        validatePair: PairValidator? = nil
    ) throws {
        let temporaryImage = temporaryImageURL.standardizedFileURL
        let temporaryVideo = temporaryVideoURL.standardizedFileURL
        let finalImage = finalImageURL.standardizedFileURL
        let finalVideo = finalVideoURL.standardizedFileURL
        let directory = finalImage.deletingLastPathComponent()
        let directoryPath = directory.path

        guard finalVideo.deletingLastPathComponent().path == directoryPath,
              temporaryImage.deletingLastPathComponent().path == directoryPath,
              temporaryVideo.deletingLastPathComponent().path == directoryPath else {
            throw AppleLivePhotoError.transactionFailed(
                "Live Photo transaction resources must be on the destination directory/filesystem"
            )
        }
        guard FileManager.default.fileExists(atPath: temporaryImage.path),
              FileManager.default.fileExists(atPath: temporaryVideo.path) else {
            throw AppleLivePhotoError.transactionFailed("validated Live Photo temporary pair is incomplete")
        }

        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        try withDirectoryLock(directory) {
            try recoverLocked(in: directory, validatePair: validatePair)

            let transactionID = UUID().uuidString.lowercased()
            let imageBackupName = ".\(finalImage.lastPathComponent).\(transactionID).backup"
            let videoBackupName = ".\(finalVideo.lastPathComponent).\(transactionID).backup"
            var manifest = Manifest(
                schemaVersion: schemaVersion,
                transactionID: transactionID,
                state: .prepared,
                finalImage: finalImage.lastPathComponent,
                finalVideo: finalVideo.lastPathComponent,
                temporaryImage: temporaryImage.lastPathComponent,
                temporaryVideo: temporaryVideo.lastPathComponent,
                backupImage: imageBackupName,
                backupVideo: videoBackupName,
                hadImage: FileManager.default.fileExists(atPath: finalImage.path),
                hadVideo: FileManager.default.fileExists(atPath: finalVideo.path)
            )

            try synchronizeFile(temporaryImage)
            try synchronizeFile(temporaryVideo)
            var journalURL = try writeManifest(manifest, in: directory)

            do {
                if manifest.hadImage {
                    try FileManager.default.moveItem(
                        at: finalImage,
                        to: directory.appendingPathComponent(imageBackupName)
                    )
                }
                if manifest.hadVideo {
                    try FileManager.default.moveItem(
                        at: finalVideo,
                        to: directory.appendingPathComponent(videoBackupName)
                    )
                }
                manifest.state = .originalsBackedUp
                journalURL = try writeManifest(manifest, in: directory)

                try FileManager.default.moveItem(at: temporaryImage, to: finalImage)
                manifest.state = .imageInstalled
                journalURL = try writeManifest(manifest, in: directory)

                try FileManager.default.moveItem(at: temporaryVideo, to: finalVideo)
                manifest.state = .pairInstalled
                journalURL = try writeManifest(manifest, in: directory)
                // Ensure the installed resource names are durable before publishing the commit point.
                try synchronizeDirectory(directory)

                // AppleLivePhotoConversionEngine fully validates the temporary pair before commit.
                // A same-directory rename does not mutate file contents. Keep this transaction layer
                // synchronous: callers may provide a genuinely synchronous validator when useful.
                if let validatePair, !validatePair(finalImage, finalVideo) {
                    throw AppleLivePhotoError.transactionFailed(
                        "installed Live Photo pair failed final validation"
                    )
                }

                // This durable marker is the commit point. Backup cleanup happens only afterwards.
                // If cleanup itself is interrupted, recovery must continue cleanup and never roll
                // one resource back independently of the other.
                manifest.state = .committed
                journalURL = try writeManifest(manifest, in: directory)
                try cleanupCommitted(manifest, journalURL: journalURL, in: directory)
            } catch {
                do {
                    if manifest.state == .committed {
                        try cleanupCommitted(manifest, journalURL: journalURL, in: directory)
                    } else {
                        try rollback(manifest, journalURL: journalURL, in: directory)
                    }
                } catch {
                    // Keep the durable journal when cleanup/rollback itself cannot finish. The next
                    // run can retry from the last persisted transaction state.
                }
                throw error
            }
        }
    }

    private static func recoverLocked(
        in directory: URL,
        validatePair: PairValidator?
    ) throws {
        let journalDirectory = directory.appendingPathComponent(journalDirectoryName, isDirectory: true)
        guard FileManager.default.fileExists(atPath: journalDirectory.path) else { return }
        let journals = try FileManager.default.contentsOfDirectory(
            at: journalDirectory,
            includingPropertiesForKeys: nil,
            options: [.skipsHiddenFiles]
        )
        .filter { $0.pathExtension.lowercased() == "json" }
        .sorted { $0.lastPathComponent < $1.lastPathComponent }

        for journalURL in journals {
            let data = try Data(contentsOf: journalURL)
            var manifest = try JSONDecoder().decode(Manifest.self, from: data)
            guard manifest.schemaVersion == schemaVersion else {
                throw AppleLivePhotoError.transactionFailed(
                    "unsupported Live Photo transaction schema \(manifest.schemaVersion)"
                )
            }

            if manifest.state == .committed {
                try cleanupCommitted(manifest, journalURL: journalURL, in: directory)
                continue
            }

            let finalImage = try safeChild(manifest.finalImage, of: directory)
            let finalVideo = try safeChild(manifest.finalVideo, of: directory)
            if manifest.state == .pairInstalled,
               FileManager.default.fileExists(atPath: finalImage.path),
               FileManager.default.fileExists(atPath: finalVideo.path),
               let validatePair,
               validatePair(finalImage, finalVideo) {
                manifest.state = .committed
                let committedJournalURL = try writeManifest(manifest, in: directory)
                try cleanupCommitted(manifest, journalURL: committedJournalURL, in: directory)
                continue
            }

            // No synchronous validator means fail closed. A pair-installed journal is still an
            // uncommitted transaction until the committed marker is durable.
            try rollback(manifest, journalURL: journalURL, in: directory)
        }
    }

    private static func rollback(
        _ manifest: Manifest,
        journalURL: URL,
        in directory: URL
    ) throws {
        let manager = FileManager.default
        let finalImage = try safeChild(manifest.finalImage, of: directory)
        let finalVideo = try safeChild(manifest.finalVideo, of: directory)
        let temporaryImage = try safeChild(manifest.temporaryImage, of: directory)
        let temporaryVideo = try safeChild(manifest.temporaryVideo, of: directory)
        let imageBackup = try safeChild(manifest.backupImage, of: directory)
        let videoBackup = try safeChild(manifest.backupVideo, of: directory)

        if manager.fileExists(atPath: imageBackup.path) {
            try removeIfPresent(finalImage)
            try manager.moveItem(at: imageBackup, to: finalImage)
        } else if !manifest.hadImage,
                  !manager.fileExists(atPath: temporaryImage.path),
                  manager.fileExists(atPath: finalImage.path) {
            // A crash can occur after the temp->final rename and before its state update.
            try removeIfPresent(finalImage)
        }

        if manager.fileExists(atPath: videoBackup.path) {
            try removeIfPresent(finalVideo)
            try manager.moveItem(at: videoBackup, to: finalVideo)
        } else if !manifest.hadVideo,
                  !manager.fileExists(atPath: temporaryVideo.path),
                  manager.fileExists(atPath: finalVideo.path) {
            try removeIfPresent(finalVideo)
        }

        try removeIfPresent(temporaryImage)
        try removeIfPresent(temporaryVideo)
        try removeIfPresent(imageBackup)
        try removeIfPresent(videoBackup)
        try synchronizeDirectory(directory)
        try removeIfPresent(journalURL)
        try synchronizeDirectory(journalURL.deletingLastPathComponent())
    }

    private static func cleanupCommitted(
        _ manifest: Manifest,
        journalURL: URL,
        in directory: URL
    ) throws {
        // Cleanup is idempotent. The committed journal is deliberately removed last so any crash
        // in the middle can only re-enter cleanup; it can never trigger rollback of half the pair.
        try removeIfPresent(try safeChild(manifest.backupImage, of: directory))
        try removeIfPresent(try safeChild(manifest.backupVideo, of: directory))
        try removeIfPresent(try safeChild(manifest.temporaryImage, of: directory))
        try removeIfPresent(try safeChild(manifest.temporaryVideo, of: directory))
        try synchronizeDirectory(directory)
        try removeIfPresent(journalURL)
        try synchronizeDirectory(journalURL.deletingLastPathComponent())
    }

    @discardableResult
    static func writeManifest(_ manifest: Manifest, in directory: URL) throws -> URL {
        let journalDirectory = directory.appendingPathComponent(journalDirectoryName, isDirectory: true)
        try FileManager.default.createDirectory(at: journalDirectory, withIntermediateDirectories: true)
        let journalURL = journalDirectory
            .appendingPathComponent(manifest.transactionID)
            .appendingPathExtension("json")
        let data = try JSONEncoder().encode(manifest)
        // Foundation's atomic write creates the replacement in the destination directory, so the
        // manifest transition itself never depends on a cross-volume rename.
        try data.write(to: journalURL, options: [.atomic])
        try synchronizeFile(journalURL)
        try synchronizeDirectory(journalDirectory)
        return journalURL
    }

    private static func safeChild(_ name: String, of directory: URL) throws -> URL {
        guard !name.isEmpty,
              name != ".",
              name != "..",
              !name.contains("/"),
              !name.contains(":") else {
            throw AppleLivePhotoError.transactionFailed("unsafe Live Photo transaction path")
        }
        return directory.appendingPathComponent(name)
    }

    private static func removeIfPresent(_ url: URL) throws {
        if FileManager.default.fileExists(atPath: url.path) {
            try FileManager.default.removeItem(at: url)
        }
    }

    private static func synchronizeFile(_ url: URL) throws {
        let handle = try FileHandle(forReadingFrom: url)
        defer { try? handle.close() }
        try handle.synchronize()
    }

    private static func synchronizeDirectory(_ url: URL) throws {
        let descriptor = url.path.withCString { path in
            Darwin.open(path, O_RDONLY)
        }
        guard descriptor >= 0 else {
            throw AppleLivePhotoError.transactionFailed("could not open Live Photo transaction directory for sync")
        }
        defer { Darwin.close(descriptor) }
        guard Darwin.fsync(descriptor) == 0 else {
            throw AppleLivePhotoError.transactionFailed("could not synchronize Live Photo transaction directory")
        }
    }

    private static func withDirectoryLock<T>(
        _ directory: URL,
        _ body: () throws -> T
    ) throws -> T {
        let lockURL = directory.appendingPathComponent(lockFileName)
        let descriptor = lockURL.path.withCString { path in
            Darwin.open(path, O_CREAT | O_RDWR, mode_t(0o600))
        }
        guard descriptor >= 0 else {
            throw AppleLivePhotoError.transactionFailed("could not open Live Photo transaction lock")
        }
        defer { Darwin.close(descriptor) }
        guard Darwin.lockf(descriptor, F_LOCK, 0) == 0 else {
            throw AppleLivePhotoError.transactionFailed("could not acquire Live Photo transaction lock")
        }
        defer { _ = Darwin.lockf(descriptor, F_ULOCK, 0) }
        return try body()
    }
}
