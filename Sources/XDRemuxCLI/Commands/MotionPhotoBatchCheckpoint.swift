import CryptoKit
import Foundation

/// Durable sidecar state for the Motion Photo pre-pass. Besides retry status, this is the
/// provenance record that proves a specific source produced a specific HEIC+MOV pair.
/// The wire format intentionally matches xdremux_py.live_photo_batch.
enum MotionPhotoBatchCheckpoint {
    enum Status: String, Codable {
        case success
        case failure
        case skippedExisting = "skipped_existing"
    }

    struct FileSignature: Equatable {
        let size: Int64
        let mtimeNs: Int64
        let sha256: String
    }

    struct Item: Codable {
        let kind: String
        let inputPath: String
        let sourceRelativePath: String?
        let outputImagePath: String
        let outputVideoPath: String
        let status: Status
        let inputSize: Int64?
        let inputMtimeNs: Int64?
        let inputSHA256: String?
        let assetIdentifier: String?
        let error: String?

        /// Old schema-1 entries intentionally fail this check: size/mtime alone are not strong
        /// enough provenance to allow --skip-existing to claim a pair for the current source.
        func matchesSignature(_ signature: FileSignature?) -> Bool {
            guard let signature,
                  let inputSize,
                  let inputSHA256 else { return false }
            return inputSize == signature.size && inputSHA256 == signature.sha256
        }

        func matchesOutputs(imageURL: URL, videoURL: URL) -> Bool {
            outputImagePath == imageURL.standardizedFileURL.path
                && outputVideoPath == videoURL.standardizedFileURL.path
        }
    }

    struct Header: Codable {
        let kind: String
        let schemaVersion: Int
        let createdAt: String

        init(createdAt: String) {
            self.kind = "header"
            self.schemaVersion = 2
            self.createdAt = createdAt
        }
    }

    static func resolvedURL(for command: BatchCommand) -> URL {
        if let requested = command.checkpointURL {
            let parent = requested.deletingLastPathComponent()
            let name = requested.lastPathComponent
            return parent.appendingPathComponent("\(name).motion-photo")
        }
        return command.outputDirURL.appendingPathComponent(".xdremux-motion-photo-checkpoint.jsonl")
    }

    static func signature(for url: URL) throws -> FileSignature {
        let attributes = try FileManager.default.attributesOfItem(atPath: url.path)
        let size = (attributes[.size] as? NSNumber)?.int64Value ?? 0
        let modified = (attributes[.modificationDate] as? Date)?.timeIntervalSince1970 ?? 0

        let handle = try FileHandle(forReadingFrom: url)
        defer { try? handle.close() }
        var hasher = SHA256()
        while let data = try handle.read(upToCount: 1024 * 1024), !data.isEmpty {
            hasher.update(data: data)
        }
        let digest = hasher.finalize().map { String(format: "%02x", $0) }.joined()
        return FileSignature(
            size: size,
            mtimeNs: Int64(modified * 1_000_000_000),
            sha256: digest
        )
    }

    static func relativeSourcePath(inputURL: URL, inputRootURL: URL) -> String {
        let input = inputURL.standardizedFileURL.path
        let root = inputRootURL.standardizedFileURL.path
        let prefix = root.hasSuffix("/") ? root : root + "/"
        if input.hasPrefix(prefix) {
            return String(input.dropFirst(prefix.count)).precomposedStringWithCanonicalMapping
        }
        return input.precomposedStringWithCanonicalMapping
    }

    static func load(url: URL) throws -> [String: Item] {
        guard FileManager.default.fileExists(atPath: url.path) else { return [:] }
        let data = try Data(contentsOf: url, options: [.mappedIfSafe])
        guard !data.isEmpty else { return [:] }
        let decoder = JSONDecoder()
        var state: [String: Item] = [:]
        for line in data.split(separator: 0x0a) where !line.isEmpty {
            guard let object = try? JSONSerialization.jsonObject(with: Data(line)) as? [String: Any],
                  object["kind"] as? String == "item" else {
                // A crash can leave a truncated final JSONL record. Ignoring it is fail-closed:
                // the input will be rebuilt rather than incorrectly authorized for reuse.
                continue
            }
            guard let item = try? decoder.decode(Item.self, from: Data(line)) else {
                // Foreign/unsupported item schemas also cannot authorize reuse. This makes the
                // checkpoint robust to mixed runtime versions without turning corruption into a
                // batch-wide availability failure.
                continue
            }
            state[item.inputPath] = item
        }
        return state
    }

    static func reset(url: URL) throws {
        if FileManager.default.fileExists(atPath: url.path) {
            try FileManager.default.removeItem(at: url)
        }
    }

    final class Writer: @unchecked Sendable {
        private let lock = NSLock()
        private var handle: FileHandle?
        private let encoder = JSONEncoder()

        init(url: URL) throws {
            try FileManager.default.createDirectory(
                at: url.deletingLastPathComponent(),
                withIntermediateDirectories: true
            )
            if !FileManager.default.fileExists(atPath: url.path) {
                guard FileManager.default.createFile(atPath: url.path, contents: nil) else {
                    throw CocoaError(.fileWriteUnknown)
                }
            }
            let handle = try FileHandle(forWritingTo: url)
            try handle.seekToEnd()
            self.handle = handle

            let attributes = try FileManager.default.attributesOfItem(atPath: url.path)
            if (attributes[.size] as? NSNumber)?.int64Value == 0 {
                try appendEncodable(
                    Header(createdAt: ISO8601DateFormatter().string(from: Date()))
                )
            }
        }

        func append(
            inputURL: URL,
            inputRootURL: URL? = nil,
            outputImageURL: URL,
            outputVideoURL: URL,
            status: Status,
            signature: FileSignature?,
            assetIdentifier: String? = nil,
            error: String? = nil
        ) throws {
            try appendEncodable(
                Item(
                    kind: "item",
                    inputPath: inputURL.standardizedFileURL.path,
                    sourceRelativePath: inputRootURL.map {
                        MotionPhotoBatchCheckpoint.relativeSourcePath(
                            inputURL: inputURL,
                            inputRootURL: $0
                        )
                    },
                    outputImagePath: outputImageURL.standardizedFileURL.path,
                    outputVideoPath: outputVideoURL.standardizedFileURL.path,
                    status: status,
                    inputSize: signature?.size,
                    inputMtimeNs: signature?.mtimeNs,
                    inputSHA256: signature?.sha256,
                    assetIdentifier: assetIdentifier,
                    error: error
                )
            )
        }

        func close() throws {
            lock.lock()
            defer { lock.unlock() }
            guard let handle else { return }
            try handle.synchronize()
            try handle.close()
            self.handle = nil
        }

        private func appendEncodable<T: Encodable>(_ value: T) throws {
            let data = try encoder.encode(value)
            lock.lock()
            defer { lock.unlock() }
            guard let handle else { throw CocoaError(.fileWriteUnknown) }
            try handle.write(contentsOf: data)
            try handle.write(contentsOf: Data([0x0a]))
            try handle.synchronize()
        }

        deinit {
            try? close()
        }
    }
}
