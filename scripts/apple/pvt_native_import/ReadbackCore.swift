import AVFoundation
import CryptoKit
import Foundation
import ImageIO
import Photos

// Native observations only. Publication policy and import admission stay in
// the Python probe; this helper never creates or alters a Photos asset.
enum ReadbackCore {
    static var documents: URL {
        FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
    }

    static func write(_ report: [String: Any], phase: String) throws {
        let data = try JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys])
        try data.write(to: documents.appendingPathComponent(phase + ".json"), options: .atomic)
    }

    static func packageHashes(_ package: URL) throws -> [String: String] {
        let entries = try FileManager.default.contentsOfDirectory(at: package,
            includingPropertiesForKeys: [.isRegularFileKey, .isSymbolicLinkKey])
        var hashes: [String: String] = [:]
        for entry in entries {
            let values = try entry.resourceValues(forKeys: [.isRegularFileKey, .isSymbolicLinkKey])
            guard values.isRegularFile == true, values.isSymbolicLink != true else {
                throw failure("package representation contains a non-regular member")
            }
            let bytes = try Data(contentsOf: entry)
            hashes[entry.lastPathComponent] = SHA256.hash(data: bytes).map { String(format: "%02x", $0) }.joined()
        }
        return hashes
    }

    static func collect(phase: String) -> [String: Any] {
        var report: [String: Any] = ["phase": phase, "read_only_photokit": true]
        report["authorization"] = PHPhotoLibrary.authorizationStatus(for: .readWrite).rawValue
        guard PHPhotoLibrary.authorizationStatus(for: .readWrite) == .authorized else {
            report["status"] = "blocked-photos-authorization"
            return report
        }
        let baselineURL = documents.appendingPathComponent("baseline.json")
        var baselineIDs: [String]?
        if let data = try? Data(contentsOf: baselineURL),
            let object = try? JSONSerialization.jsonObject(with: data),
            let baseline = object as? [String: Any] {
            baselineIDs = baseline["asset_identifiers"] as? [String]
        }
        let deadline = Date().addingTimeInterval(15)
        var assets = PHAsset.fetchAssets(with: nil)
        var attempts = 0
        while true {
            attempts += 1
            var identifiers: [String] = []
            assets.enumerateObjects { asset, _, _ in identifiers.append(asset.localIdentifier) }
            report["asset_count"] = assets.count
            report["asset_identifiers"] = identifiers.sorted()
            if phase == "baseline" { report["status"] = "baseline"; return report }
            guard let baselineIDs else { report["status"] = "missing-baseline-identifiers"; return report }
            let added = Set(identifiers).subtracting(baselineIDs)
            let removed = Set(baselineIDs).subtracting(identifiers)
            report["added_asset_identifiers"] = added.sorted()
            report["removed_asset_identifiers"] = removed.sorted()
            report["settlement_attempts"] = attempts
            if added.count > 1 || !removed.isEmpty {
                report["status"] = "unexpected-asset-delta"; return report
            }
            if let identifier = added.first,
                let asset = PHAsset.fetchAssets(withLocalIdentifiers: [identifier], options: nil).firstObject,
                asset.mediaSubtypes.contains(.photoLive) {
                return addingResourceFacts(asset, to: report)
            }
            guard Date() < deadline else { report["status"] = "live-asset-settlement-timeout"; return report }
            Thread.sleep(forTimeInterval: 0.2)
            assets = PHAsset.fetchAssets(with: nil)
        }
    }

    private static func addingResourceFacts(_ asset: PHAsset, to initial: [String: Any]) -> [String: Any] {
        var report = initial
        report["asset_local_identifier"] = asset.localIdentifier
        report["media_type"] = asset.mediaType.rawValue
        report["media_subtypes"] = asset.mediaSubtypes.rawValue
        report["photo_live"] = asset.mediaSubtypes.contains(.photoLive)
        report["pixel_width"] = asset.pixelWidth
        report["pixel_height"] = asset.pixelHeight
        var resources: [[String: Any]] = []
        do {
            let directory = documents.appendingPathComponent("readback", isDirectory: true)
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            for (index, resource) in PHAssetResource.assetResources(for: asset).enumerated() {
                resources.append(try resourceFacts(resource, index: index, directory: directory))
            }
            report["status"] = "readback-complete"
        } catch {
            report["status"] = "readback-error"
            report["error"] = errorFacts(error)
        }
        report["resources"] = resources
        return report
    }

    private static func resourceFacts(_ resource: PHAssetResource, index: Int, directory: URL) throws -> [String: Any] {
        let suffix = resource.type == .pairedVideo ? "mov" : "bin"
        let url = directory.appendingPathComponent("resource-\(index).\(suffix)")
        var facts: [String: Any] = [:]
        facts["type"] = resource.type.rawValue
        facts["original_filename"] = resource.originalFilename
        facts["uti"] = resource.uniformTypeIdentifier
        facts["exported_name"] = url.lastPathComponent
        facts["is_original_photo"] = resource.type == .photo
        facts["is_original_paired_video"] = resource.type == .pairedVideo
        try export(resource, to: url)
        let bytes = try Data(contentsOf: url)
        facts["bytes"] = bytes.count
        facts["sha256"] = SHA256.hash(data: bytes).map { String(format: "%02x", $0) }.joined()
        if resource.type == .photo,
            let source = CGImageSourceCreateWithData(bytes as CFData, nil),
            let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [String: Any],
            let maker = properties[kCGImagePropertyMakerAppleDictionary as String] as? [String: Any] {
            facts["maker_apple_17"] = maker["17"] ?? NSNull()
        }
        if resource.type == .pairedVideo {
            let video = AVURLAsset(url: url)
            let metadata = video.metadata(forFormat: .quickTimeMetadata)
            facts["content_identifiers"] = metadata.filter { $0.identifier == .quickTimeMetadataContentIdentifier }.compactMap { $0.stringValue }
            facts["still_image_time_samples"] = try stillTimeSamples(video)
        }
        return facts
    }

    private static func stillTimeSamples(_ video: AVAsset) throws -> [[String: Any]] {
        var samples: [[String: Any]] = []
        for track in video.tracks(withMediaType: .metadata) {
            let reader = try AVAssetReader(asset: video)
            let output = AVAssetReaderTrackOutput(track: track, outputSettings: nil)
            guard reader.canAdd(output) else { throw failure("metadata track output unavailable") }
            reader.add(output)
            let adaptor = AVAssetReaderOutputMetadataAdaptor(assetReaderTrackOutput: output)
            guard reader.startReading() else { throw reader.error ?? failure("metadata reader failed") }
            while let group = adaptor.nextTimedMetadataGroup() {
                for item in group.items where item.identifier?.rawValue == "mdta/com.apple.quicktime.still-image-time" {
                    var sample: [String: Any] = [:]
                    sample["value"] = group.timeRange.start.value
                    sample["timescale"] = group.timeRange.start.timescale
                    sample["epoch"] = group.timeRange.start.epoch
                    if let value = item.numberValue { sample["sample_value"] = value }
                    else { sample["sample_value"] = NSNull() }
                    samples.append(sample)
                }
            }
            guard reader.status == .completed else { throw reader.error ?? failure("metadata reader did not complete") }
        }
        return samples
    }

    private static func export(_ resource: PHAssetResource, to url: URL) throws {
        guard FileManager.default.createFile(atPath: url.path, contents: nil) else { throw failure("export file unavailable") }
        let handle = try FileHandle(forWritingTo: url)
        defer { try? handle.close() }
        let options = PHAssetResourceRequestOptions()
        options.isNetworkAccessAllowed = false
        let semaphore = DispatchSemaphore(value: 0)
        let lock = NSLock()
        var receivedError: Error?
        let identifier = PHAssetResourceManager.default().requestData(for: resource, options: options, dataReceivedHandler: { data in
            lock.lock(); defer { lock.unlock() }
            do { try handle.write(contentsOf: data) } catch { receivedError = error }
        }, completionHandler: { error in
            lock.lock(); if let error { receivedError = error }; lock.unlock()
            semaphore.signal()
        })
        guard semaphore.wait(timeout: .now() + 30) == .success else {
            PHAssetResourceManager.default().cancelDataRequest(identifier)
            throw failure("resource export timed out")
        }
        lock.lock(); let error = receivedError; lock.unlock()
        if let error { throw error }
    }

    static func errorFacts(_ error: Error) -> [String: Any] {
        let native = error as NSError
        return ["domain": native.domain, "code": native.code, "description": String(describing: error)]
    }

    static func failure(_ message: String) -> NSError {
        NSError(domain: "PVTReadback", code: 1, userInfo: [NSLocalizedDescriptionKey: message])
    }
}
