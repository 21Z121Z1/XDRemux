import Foundation

/// User-facing Motion Photo naming policy.
///
/// Preserve the source basename whenever possible. Sequence suffixes are introduced only when the
/// destination namespace has a real collision. Source identity remains in checkpoint/provenance;
/// it is deliberately not exposed in the user's filename.
enum MotionPhotoBatchPlanner {
    static func outputImageURL(
        for inputURL: URL,
        inputRootURL _: URL,
        outputDirectoryURL: URL
    ) -> URL {
        let source = inputURL.resolvingSymlinksInPath().standardizedFileURL
        let stem = source.deletingPathExtension().lastPathComponent
        return outputDirectoryURL
            .appendingPathComponent(stem)
            .appendingPathExtension("heic")
    }

    static func numberedOutputImageURL(base: URL, sequence: Int) -> URL {
        guard sequence > 1 else { return base }
        let stem = base.deletingPathExtension().lastPathComponent
        return base.deletingLastPathComponent()
            .appendingPathComponent("\(stem) (\(sequence))")
            .appendingPathExtension(base.pathExtension)
    }

    static func companionVideoURL(for imageURL: URL) -> URL {
        imageURL.deletingPathExtension().appendingPathExtension("mov")
    }

    /// Reserves one HEIC+MOV namespace atomically. `candidateBelongsToSource` lets durable
    /// provenance keep an existing name on rerun, while unrelated pre-existing files count as real
    /// collisions and receive the next sequence number.
    static func reserveOutputImageURL(
        for inputURL: URL,
        inputRootURL: URL,
        outputDirectoryURL: URL,
        reservedPaths: inout Set<String>,
        candidateBelongsToSource: (URL, URL) -> Bool = { _, _ in false },
        fileExists: (URL) -> Bool = { FileManager.default.fileExists(atPath: $0.path) }
    ) -> URL {
        let sourcePath = inputURL.resolvingSymlinksInPath().standardizedFileURL.path
        let base = outputImageURL(
            for: inputURL,
            inputRootURL: inputRootURL,
            outputDirectoryURL: outputDirectoryURL
        )
        var sequence = 1

        while true {
            let image = numberedOutputImageURL(base: base, sequence: sequence)
            let video = companionVideoURL(for: image)
            let imagePath = image.standardizedFileURL.path
            let videoPath = video.standardizedFileURL.path
            let plannedConflict = reservedPaths.contains(imagePath) || reservedPaths.contains(videoPath)
            let sourceConflict = imagePath == sourcePath || videoPath == sourcePath
            let belongsToSource = candidateBelongsToSource(image, video)
            let filesystemConflict = !belongsToSource && (fileExists(image) || fileExists(video))

            if !plannedConflict && !sourceConflict && !filesystemConflict {
                reservedPaths.insert(imagePath)
                reservedPaths.insert(videoPath)
                return image
            }
            sequence += 1
        }
    }
}
