import CryptoKit
import Foundation

/// Stable Motion Photo batch naming. A destination is a pure function of the canonical batch root
/// plus the source path relative to that root, so batch membership/order cannot remap a source and
/// two different input roots cannot silently target the same persisted Live Photo name.
enum MotionPhotoBatchPlanner {
    static func outputImageURL(
        for inputURL: URL,
        inputRootURL: URL,
        outputDirectoryURL: URL
    ) -> URL {
        let source = inputURL.resolvingSymlinksInPath().standardizedFileURL
        let root = inputRootURL.resolvingSymlinksInPath().standardizedFileURL
        let rootIdentity = root.path.precomposedStringWithCanonicalMapping
        let relativePath: String
        let prefix = root.path.hasSuffix("/") ? root.path : root.path + "/"
        if source.path.hasPrefix(prefix) {
            relativePath = String(source.path.dropFirst(prefix.count)).precomposedStringWithCanonicalMapping
        } else {
            relativePath = source.path.precomposedStringWithCanonicalMapping
        }

        // Root namespace + relative path is equivalent to a canonical source identity while keeping
        // the intent explicit. This prevents two independent input trees with the same A/IMG.jpg
        // layout from overwriting each other when they share an output directory.
        let identity = rootIdentity + "\u{0}" + relativePath
        let digest = SHA256.hash(data: Data(identity.utf8))
        // 128 bits keeps the filename compact while making accidental persisted-name collisions
        // negligible. validateUnique() still fails closed if a collision is ever observed.
        let token = digest.prefix(16).map { String(format: "%02x", $0) }.joined()
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
                return "stable Motion Photo output collision: \(first) and \(second) -> \(output)"
            }
        }
    }
}
