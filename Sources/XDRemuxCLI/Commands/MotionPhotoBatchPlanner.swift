import Foundation

/// Stable Motion Photo batch naming based on the source path below the batch input root.
/// Preserving the relative directory structure makes duplicate basenames unambiguous without
/// introducing opaque hashed namespaces into user-visible filenames.
enum MotionPhotoBatchPlanner {
    static func outputImageURL(
        for inputURL: URL,
        inputRootURL: URL,
        outputDirectoryURL: URL
    ) -> URL {
        let source = inputURL.resolvingSymlinksInPath().standardizedFileURL
        let root = inputRootURL.resolvingSymlinksInPath().standardizedFileURL
        let prefix = root.path.hasSuffix("/") ? root.path : root.path + "/"
        let relativePath: String
        if source.path.hasPrefix(prefix) {
            relativePath = String(source.path.dropFirst(prefix.count)).precomposedStringWithCanonicalMapping
        } else {
            // Batch discovery normally guarantees that inputs are below inputRootURL. Keep the
            // fallback readable and let validateUnique() reject any resulting collision.
            relativePath = source.lastPathComponent.precomposedStringWithCanonicalMapping
        }

        let components = relativePath.split(separator: "/").map(String.init)
        var directory = outputDirectoryURL
        for component in components.dropLast() {
            directory.appendPathComponent(component, isDirectory: true)
        }

        let ext = source.pathExtension.lowercased()
        let sourceStem = source.deletingPathExtension().lastPathComponent
        let stem = (ext == "heic" || ext == "heif") ? "\(sourceStem).live" : sourceStem
        return directory.appendingPathComponent(stem).appendingPathExtension("heic")
    }

    static func validateUnique(_ items: [(input: URL, output: URL)]) throws {
        var owners: [String: String] = [:]
        for item in items {
            let outputPath = item.output.standardizedFileURL.path
            let inputPath = item.input.resolvingSymlinksInPath().standardizedFileURL.path
            if let prior = owners[outputPath], prior != inputPath {
                throw PlannerError.collision(first: prior, second: inputPath, output: outputPath)
            }
            owners[outputPath] = inputPath
        }
    }

    enum PlannerError: LocalizedError {
        case collision(first: String, second: String, output: String)

        var errorDescription: String? {
            switch self {
            case let .collision(first, second, output):
                return "Motion Photo output collision: \(first) and \(second) -> \(output)"
            }
        }
    }
}
