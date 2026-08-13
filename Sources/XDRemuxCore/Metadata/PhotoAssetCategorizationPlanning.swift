import CryptoKit
import Foundation

/// Platform-specific code supplies the proof that an image and video form one Apple Live Photo.
/// XDRemuxCore deliberately does not depend on AVFoundation/Photos just to organize files.
public typealias LivePhotoPairValidator = (_ imageURL: URL, _ videoURL: URL) -> Bool

public extension PhotoCategorizationEngine {
    /// Asset-aware standalone categorization.
    ///
    /// Images remain the discovery roots. A sibling MOV is claimed only when the supplied validator
    /// proves that it belongs to the image. Once claimed, all resources in the asset share one
    /// classification, directory, and collision sequence, so a Live Photo can never be split across
    /// `name.*` and `name (2).*` merely because one resource collided first.
    static func makePlan(
        inputs: [URL],
        outputDirectory: URL?,
        livePhotoPairValidator: LivePhotoPairValidator,
        fileManager: FileManager = .default
    ) throws -> PhotoCategorizationPlan {
        let primaryImages = try collectAssetPrimaryImages(
            inputs: inputs,
            excluding: outputDirectory,
            fileManager: fileManager
        )
        var reserved: [String: (url: URL, digest: String)] = [:]
        var items: [PhotoCategorizationItem] = []

        for primaryImage in primaryImages {
            let asset = resolvedAsset(
                for: primaryImage,
                pairValidator: livePhotoPairValidator,
                fileManager: fileManager
            )
            let classification = classify(asset: asset)
            let destinationRoot = outputDirectory ?? primaryImage.deletingLastPathComponent()
            let destinationDirectory = PhotoFolderProjection.relativeDirectoryComponents(for: classification)
                .reduce(destinationRoot) { partial, component in
                    partial.appendingPathComponent(component, isDirectory: true)
                }
            let resourceDigests = try Dictionary(
                uniqueKeysWithValues: asset.resources.map { resource in
                    (resource.url.standardizedFileURL.path, try categorizationSHA256(resource.url))
                }
            )

            var sequence = 1
            while true {
                let candidates = asset.resources.map { resource in
                    (
                        resource: resource,
                        destination: sequencedCategorizationDestination(
                            directory: destinationDirectory,
                            sourceName: resource.url.lastPathComponent,
                            sequence: sequence
                        )
                    )
                }

                var dispositions: [String: PhotoCategorizationDisposition] = [:]
                var conflict = false
                for candidate in candidates {
                    let sourcePath = candidate.resource.url.standardizedFileURL.path
                    let digest = resourceDigests[sourcePath]!
                    let destinationKey = candidate.destination.standardizedFileURL.path
                    if let prior = reserved[destinationKey] {
                        if prior.digest == digest {
                            dispositions[sourcePath] = .duplicate
                        } else {
                            conflict = true
                            break
                        }
                    } else if fileManager.fileExists(atPath: candidate.destination.path) {
                        if try categorizationFilesMatch(candidate.resource.url, candidate.destination) {
                            dispositions[sourcePath] = .duplicate
                        } else {
                            conflict = true
                            break
                        }
                    } else {
                        dispositions[sourcePath] = .copy
                    }
                }

                if conflict {
                    sequence += 1
                    continue
                }

                for candidate in candidates {
                    let sourcePath = candidate.resource.url.standardizedFileURL.path
                    let digest = resourceDigests[sourcePath]!
                    let disposition = dispositions[sourcePath] ?? .copy
                    let destinationKey = candidate.destination.standardizedFileURL.path
                    if reserved[destinationKey] == nil {
                        reserved[destinationKey] = (candidate.destination, digest)
                    }
                    items.append(
                        PhotoCategorizationItem(
                            sourceURL: candidate.resource.url,
                            destinationURL: candidate.destination,
                            classification: classification,
                            disposition: disposition
                        )
                    )
                }
                break
            }
        }

        return PhotoCategorizationPlan(outputDirectory: outputDirectory, items: items)
    }

    private static func resolvedAsset(
        for primaryImage: URL,
        pairValidator: LivePhotoPairValidator,
        fileManager: FileManager
    ) -> PhotoAsset {
        if let videoURL = categorizationCompanionVideo(for: primaryImage, fileManager: fileManager),
           pairValidator(primaryImage, videoURL) {
            return .livePhoto(imageURL: primaryImage, videoURL: videoURL)
        }
        if inferredAssetType(at: primaryImage) == .livePhoto {
            return PhotoAsset(
                id: primaryImage.standardizedFileURL.path,
                type: .livePhoto,
                resources: [PhotoResource(url: primaryImage, role: .primaryImage)]
            )
        }
        return .staticPhoto(primaryImage)
    }

    private static func categorizationCompanionVideo(
        for imageURL: URL,
        fileManager: FileManager
    ) -> URL? {
        let directory = imageURL.deletingLastPathComponent()
        let stem = imageURL.deletingPathExtension().lastPathComponent
        guard let entries = try? fileManager.contentsOfDirectory(
            at: directory,
            includingPropertiesForKeys: [.isRegularFileKey],
            options: [.skipsHiddenFiles]
        ) else { return nil }
        return entries
            .filter {
                $0.pathExtension.lowercased() == "mov"
                    && $0.deletingPathExtension().lastPathComponent == stem
            }
            .sorted { $0.lastPathComponent < $1.lastPathComponent }
            .first
    }

    private static func collectAssetPrimaryImages(
        inputs: [URL],
        excluding outputDirectory: URL?,
        fileManager: FileManager
    ) throws -> [URL] {
        let supportedExtensions = Set(["heic", "heif", "jpg", "jpeg"])
        let inPlaceSkipRoots = PhotoFolderProjection.rootFolderNames
            .union(Set(OppoCaptureMode.allCases.map(\.folderName)))
        let excludedPath = outputDirectory?.standardizedFileURL.path
        var collected: [String: URL] = [:]

        for input in inputs {
            var isDirectory: ObjCBool = false
            guard fileManager.fileExists(atPath: input.path, isDirectory: &isDirectory) else {
                throw CocoaError(.fileNoSuchFile)
            }
            let candidates: [URL]
            if isDirectory.boolValue {
                guard let enumerator = fileManager.enumerator(
                    at: input,
                    includingPropertiesForKeys: [.isRegularFileKey],
                    options: [.skipsHiddenFiles]
                ) else { continue }
                candidates = enumerator.compactMap { $0 as? URL }
            } else {
                candidates = [input]
            }

            for candidate in candidates {
                let standardized = candidate.standardizedFileURL
                let path = standardized.path
                if let excludedPath,
                   (path == excludedPath || path.hasPrefix(excludedPath + "/")) {
                    continue
                }
                guard supportedExtensions.contains(candidate.pathExtension.lowercased()) else { continue }
                let values = try? candidate.resourceValues(forKeys: [.isRegularFileKey])
                guard values?.isRegularFile != false else { continue }

                if isDirectory.boolValue {
                    let rootPath = input.standardizedFileURL.path
                    if path.hasPrefix(rootPath + "/") {
                        let relative = String(path.dropFirst(rootPath.count + 1))
                        if let first = relative.split(separator: "/").first,
                           inPlaceSkipRoots.contains(String(first)) {
                            continue
                        }
                    }
                }
                collected[path] = candidate
            }
        }
        return collected.keys.sorted().compactMap { collected[$0] }
    }
}

private func sequencedCategorizationDestination(
    directory: URL,
    sourceName: String,
    sequence: Int
) -> URL {
    guard sequence > 1 else { return directory.appendingPathComponent(sourceName) }
    let source = URL(fileURLWithPath: sourceName)
    let ext = source.pathExtension
    let stem = source.deletingPathExtension().lastPathComponent
    let name = ext.isEmpty ? "\(stem) (\(sequence))" : "\(stem) (\(sequence)).\(ext)"
    return directory.appendingPathComponent(name)
}

private func categorizationFilesMatch(_ lhs: URL, _ rhs: URL) throws -> Bool {
    let leftAttributes = try FileManager.default.attributesOfItem(atPath: lhs.path)
    let rightAttributes = try FileManager.default.attributesOfItem(atPath: rhs.path)
    if (leftAttributes[.size] as? NSNumber)?.uint64Value
        != (rightAttributes[.size] as? NSNumber)?.uint64Value {
        return false
    }
    return try categorizationSHA256(lhs) == categorizationSHA256(rhs)
}

private func categorizationSHA256(_ url: URL) throws -> String {
    let handle = try FileHandle(forReadingFrom: url)
    defer { try? handle.close() }
    var hasher = SHA256()
    while true {
        let chunk = try handle.read(upToCount: 1024 * 1024) ?? Data()
        if chunk.isEmpty { break }
        hasher.update(data: chunk)
    }
    return hasher.finalize().map { String(format: "%02x", $0) }.joined()
}
