import CryptoKit
import Foundation

/// Stable Motion Photo batch naming. A destination is a pure function of the source path relative
/// to the batch root, so adding/removing another input cannot remap an existing source onto a
/// different Live Photo pair.
enum MotionPhotoBatchPlanner {
    static func outputImageURL(
        for inputURL: URL,
        inputRootURL: URL,
        outputDirectoryURL: URL
    ) -> URL {
        let source = inputURL.standardizedFileURL
        let root = inputRootURL.standardizedFileURL
        let relativePath: String
        let prefix = root.path.hasSuffix("/") ? root.path : root.path + "/"
        if source.path.hasPrefix(prefix) {
            relativePath = String(source.path.dropFirst(prefix.count)).precomposedStringWithCanonicalMapping
        } else {
            relativePath = source.path.precomposedStringWithCanonicalMapping
        }

        let digest = SHA256.hash(data: Data(relativePath.utf8))
        let token = digest.prefix(8).map { String(format: "%02x", $0) }.joined()
        let ext = source.pathExtension.lowercased()
        let sourceStem = source.deletingPathExtension().lastPathComponent
        let stem = (ext == "heic" || ext == "heif") ? "\(sourceStem).live" : sourceStem
        return outputDirectoryURL
            .appendingPathComponent("\(stem)~\(token)")
            .appendingPathExtension("heic")
    }

    static func validateUnique(_ items: [(input: URL, output: URL)]) throws {
        var owners: [String: String] = [:]
        for item in items {
            let outputPath = item.output.standardizedFileURL.path
            let inputPath = item.input.standardizedFileURL.path
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
                return "stable Motion Photo output collision: \(first) and \(second) -> \(output)"
            }
        }
    }
}
