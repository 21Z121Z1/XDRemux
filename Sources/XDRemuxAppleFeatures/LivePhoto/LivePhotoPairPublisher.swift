import Foundation

/// Publishes a validated HEIC+MOV Live Photo pair without a durable transaction protocol.
///
/// Both temporary resources must live beside the destination so each rename stays on one
/// filesystem. Ordinary publication failures roll back in-process. If the process is terminated
/// between the two renames, the next conversion treats the pair as derived output: stale artifacts
/// and any incomplete/invalid final pair are removed and the source is converted again.
enum LivePhotoPairPublisher {
    typealias PairValidator = (URL, URL) -> Bool

    static func reconcile(
        finalImageURL: URL,
        finalVideoURL: URL,
        validatePair: PairValidator
    ) throws {
        let image = finalImageURL.standardizedFileURL
        let video = finalVideoURL.standardizedFileURL
        let directory = image.deletingLastPathComponent()
        guard video.deletingLastPathComponent().path == directory.path else {
            throw AppleLivePhotoError.transactionFailed("Live Photo resources must share one destination directory")
        }

        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        try removeStaleArtifacts(for: image, video: video, in: directory)

        let manager = FileManager.default
        let imageExists = manager.fileExists(atPath: image.path)
        let videoExists = manager.fileExists(atPath: video.path)
        guard imageExists || videoExists else { return }
        guard imageExists, videoExists, validatePair(image, video) else {
            try removeIfPresent(image)
            try removeIfPresent(video)
            return
        }
    }

    static func publish(
        temporaryImageURL: URL,
        temporaryVideoURL: URL,
        finalImageURL: URL,
        finalVideoURL: URL
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
                "Live Photo publication resources must be on the destination directory/filesystem"
            )
        }
        guard FileManager.default.fileExists(atPath: temporaryImage.path),
              FileManager.default.fileExists(atPath: temporaryVideo.path) else {
            throw AppleLivePhotoError.transactionFailed("validated Live Photo temporary pair is incomplete")
        }

        let manager = FileManager.default
        let backupID = UUID().uuidString.lowercased()
        let imageBackup = directory.appendingPathComponent(".\(finalImage.lastPathComponent).\(backupID).backup")
        let videoBackup = directory.appendingPathComponent(".\(finalVideo.lastPathComponent).\(backupID).backup")
        let hadImage = manager.fileExists(atPath: finalImage.path)
        let hadVideo = manager.fileExists(atPath: finalVideo.path)
        var imageInstalled = false
        var videoInstalled = false

        do {
            if hadImage { try manager.moveItem(at: finalImage, to: imageBackup) }
            if hadVideo { try manager.moveItem(at: finalVideo, to: videoBackup) }
            try manager.moveItem(at: temporaryImage, to: finalImage)
            imageInstalled = true
            try manager.moveItem(at: temporaryVideo, to: finalVideo)
            videoInstalled = true
        } catch {
            if imageInstalled { try? manager.removeItem(at: finalImage) }
            if videoInstalled { try? manager.removeItem(at: finalVideo) }
            if hadImage, manager.fileExists(atPath: imageBackup.path) {
                try? manager.moveItem(at: imageBackup, to: finalImage)
            }
            if hadVideo, manager.fileExists(atPath: videoBackup.path) {
                try? manager.moveItem(at: videoBackup, to: finalVideo)
            }
            throw AppleLivePhotoError.transactionFailed("Live Photo pair publication failed: \(error.localizedDescription)")
        }

        // Publication is complete once both renames succeeded. Backup cleanup must not turn a valid
        // new pair into a rollback path; stale backups are harmless and reconcile() removes them.
        try? removeIfPresent(imageBackup)
        try? removeIfPresent(videoBackup)
    }

    private static func removeStaleArtifacts(for image: URL, video: URL, in directory: URL) throws {
        let manager = FileManager.default

        // One-time cleanup for users upgrading from the journal-based publisher. No journal state is
        // interpreted: outputs are derived and will be validated or rebuilt below.
        try removeIfPresent(directory.appendingPathComponent(".xdremux-live-photo-transactions", isDirectory: true))
        try removeIfPresent(directory.appendingPathComponent(".xdremux-live-photo-transactions.lock"))

        let stem = image.deletingPathExtension().lastPathComponent
        let entries = try manager.contentsOfDirectory(
            at: directory,
            includingPropertiesForKeys: nil,
            options: [.skipsSubdirectoryDescendants]
        )
        for entry in entries {
            let name = entry.lastPathComponent
            let isBackup =
                (name.hasPrefix(".\(image.lastPathComponent).") || name.hasPrefix(".\(video.lastPathComponent)."))
                && name.hasSuffix(".backup")
            let isTemporary = name.hasPrefix(".\(stem).")
                && (name.hasSuffix(".tmp.heic") || name.hasSuffix(".tmp.mov"))
            if isBackup || isTemporary {
                try removeIfPresent(entry)
            }
        }
    }

    private static func removeIfPresent(_ url: URL) throws {
        if FileManager.default.fileExists(atPath: url.path) {
            try FileManager.default.removeItem(at: url)
        }
    }
}
