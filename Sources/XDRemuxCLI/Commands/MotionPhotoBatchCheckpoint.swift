import Foundation

/// Small resume checkpoint for the Motion Photo batch pass.
///
/// This state is local to the Swift CLI. It records enough source metadata and output identity to
/// avoid repeating completed work, but it is not a cross-runtime provenance database.
enum MotionPhotoBatchCheckpoint {
    enum Status: String, Codable {
        case success
        case failure
        case skippedExisting = "skipped_existing"
    }

    struct FileSignature: Equatable {
        let size: Int64
        let mtimeNs: Int64
    }

    struct Item: Codable {
        let kind: String
        let inputPath: String
        let outputImagePath: String
        let outputVideoPath: String
        let status: Status
        let inputSize: Int64?
        let inputMtimeNs: Int64?
        let assetIdentifier: String?
        let error: String?

        func matchesSignature(_ signature: FileSignature?) -> Bool {
            guard let signature, let inputSize, let inputMtimeNs else { return false }
            return inputSize == signature.size && inputMtimeNs == signature.mtimeNs
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
            self.schemaVersion = 1
            self.createdAt = createdAt
        }
    }

    static func resolvedURL(for command: BatchCommand) -> URL {
        if let requested = command.checkpointURL {
            let parent = requested.deletingLastPathComponent()
            return parent.appendingPathComponent("\(requested.lastPathComponent).motion-photo")
        }
        return command.outputDirURL.appendingPathComponent(".xdremux-motion-photo-checkpoint.jsonl")
    }

    static func signature(for url: URL) throws -> FileSignature {
        let attributes = try FileManager.default.attributesOfItem(atPath: url.path)
        let size = (attributes[.size] as? NSNumber)?.int64Value ?? 0
        let modified = (attributes[.modificationDate] as? Date)?.timeIntervalSince1970 ?? 0
        return FileSignature(size: size, mtimeNs: Int64(modified * 1_000_000_000))
    }

    static func load(url: URL) throws -> [String: Item] {
        guard FileManager.default.fileExists(atPath: url.path) else { return [:] }
        let data = try Data(contentsOf: url, options: [.mappedIfSafe])
        guard !data.isEmpty else { return [:] }
        let decoder = JSONDecoder()
        var state: [String: Item] = [:]
        for line in data.split(separator: 0x0a) where !line.isEmpty {
            guard let object = try? JSONSerialization.jsonObject(with: Data(line)) as? [String: Any],
                  object["kind"] as? String == "item",
                  let item = try? decoder.decode(Item.self, from: Data(line)) else {
                // Truncated or older foreign records simply cause the source to be rebuilt.
                continue
            }
            state[item.inputPath] = item
        }
        return state
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
                try appendEncodable(Header(createdAt: ISO8601DateFormatter().string(from: Date())))
            }
        }

        func append(
            inputURL: URL,
            inputRootURL _: URL? = nil,
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
                    outputImagePath: outputImageURL.standardizedFileURL.path,
                    outputVideoPath: outputVideoURL.standardizedFileURL.path,
                    status: status,
                    inputSize: signature?.size,
                    inputMtimeNs: signature?.mtimeNs,
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
        }

        deinit {
            try? close()
        }
    }
}
