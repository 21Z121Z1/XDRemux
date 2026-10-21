import AVFoundation
import CryptoKit
import Foundation
import ImageIO
import Photos
import UIKit

// Read-only PhotoKit consumer. Import is exclusively simctl addmedia <whole.pvt>.
// No PHAssetChangeRequest, performChanges, resource creation or pairing mutation.
@main
final class AppDelegate: UIResponder, UIApplicationDelegate {
    var window: UIWindow?

    func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {
        let window = UIWindow(frame: UIScreen.main.bounds)
        let controller = UIViewController()
        controller.view.backgroundColor = .white
        window.rootViewController = controller
        window.makeKeyAndVisible()
        self.window = window
        DispatchQueue.global(qos: .userInitiated).async { self.report() }
        return true
    }

    nonisolated private func report() {
        let arguments = CommandLine.arguments
        let phase = arguments.firstIndex(of: "--phase").flatMap { arguments.indices.contains($0 + 1) ? arguments[$0 + 1] : nil } ?? "after"
        var report: [String: Any] = ["phase": phase, "read_only_photokit": true,
            "authorization": PHPhotoLibrary.authorizationStatus(for: .readWrite).rawValue]
        defer {
            let documents = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
            let url = documents.appendingPathComponent("\(phase).json")
            do { try JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys]).write(to: url, options: .atomic) }
            catch { NSLog("PVT_READBACK_REPORT_WRITE_FAILED: %@", String(describing: error)) }
        }
        guard PHPhotoLibrary.authorizationStatus(for: .readWrite) == .authorized else {
            report["status"] = "blocked-photos-authorization"
            return
        }
        let assets = PHAsset.fetchAssets(with: nil)
        report["asset_count"] = assets.count
        var assetIDs: [String] = []
        assets.enumerateObjects { asset, _, _ in assetIDs.append(asset.localIdentifier) }
        report["asset_identifiers"] = assetIDs.sorted()
        if phase == "baseline" { report["status"] = "baseline"; return }
        let documents = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        guard let baselineData = try? Data(contentsOf: documents.appendingPathComponent("baseline.json")),
            let object = try? JSONSerialization.jsonObject(with: baselineData),
            let baseline = object as? [String: Any],
            let baselineIDs = baseline["asset_identifiers"] as? [String] else {
            report["status"] = "missing-baseline-identifiers"; return
        }
        let added = Set(assetIDs).subtracting(baselineIDs)
        let removed = Set(baselineIDs).subtracting(assetIDs)
        report["added_asset_identifiers"] = added.sorted()
        report["removed_asset_identifiers"] = removed.sorted()
        guard added.count == 1, removed.isEmpty,
            let identifier = added.first,
            let asset = PHAsset.fetchAssets(withLocalIdentifiers: [identifier], options: nil).firstObject else {
            report["status"] = "unexpected-asset-delta"; return
        }
        report["asset_local_identifier"] = asset.localIdentifier
        report["media_type"] = asset.mediaType.rawValue
        report["media_subtypes"] = asset.mediaSubtypes.rawValue
        report["photo_live"] = asset.mediaSubtypes.contains(.photoLive)
        report["pixel_width"] = asset.pixelWidth
        report["pixel_height"] = asset.pixelHeight
        let directory = documents.appendingPathComponent("readback", isDirectory: true)
        var resources: [[String: Any]] = []
        do {
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            for (index, resource) in PHAssetResource.assetResources(for: asset).enumerated() {
                let suffix = resource.type == .pairedVideo ? "mov" : "bin"
                let url = directory.appendingPathComponent("resource-\(index).\(suffix)")
                var facts: [String: Any] = ["type": resource.type.rawValue, "original_filename": resource.originalFilename,
                    "uti": resource.uniformTypeIdentifier, "exported_name": url.lastPathComponent,
                    "is_original_photo": resource.type == .photo, "is_original_paired_video": resource.type == .pairedVideo]
                defer { resources.append(facts) }
                try export(resource, to: url)
                let bytes = try Data(contentsOf: url)
                facts["bytes"] = bytes.count
                facts["sha256"] = SHA256.hash(data: bytes).map { String(format: "%02x", $0) }.joined()
                if resource.type == .photo {
                    if let source = CGImageSourceCreateWithData(bytes as CFData, nil),
                        let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [String: Any],
                        let maker = properties[kCGImagePropertyMakerAppleDictionary as String] as? [String: Any] {
                        facts["maker_apple_17"] = maker["17"] ?? NSNull()
                    }
                } else if resource.type == .pairedVideo {
                    let video = AVURLAsset(url: url)
                    let identifiers = video.metadata(forFormat: .quickTimeMetadata).filter {
                        $0.identifier == .quickTimeMetadataContentIdentifier
                    }.compactMap { $0.stringValue }
                    facts["content_identifiers"] = identifiers
                    var covers: [[String: Any]] = []
                    for track in video.tracks(withMediaType: .metadata) {
                        let reader = try AVAssetReader(asset: video)
                        let output = AVAssetReaderTrackOutput(track: track, outputSettings: nil)
                        guard reader.canAdd(output) else { throw failure("metadata track output unavailable") }
                        reader.add(output)
                        let adaptor = AVAssetReaderOutputMetadataAdaptor(assetReaderTrackOutput: output)
                        guard reader.startReading() else { throw reader.error ?? failure("metadata reader failed") }
                        while let group = adaptor.nextTimedMetadataGroup() {
                            for item in group.items where item.identifier?.rawValue == "mdta/com.apple.quicktime.still-image-time" {
                                covers.append(["value": group.timeRange.start.value, "timescale": group.timeRange.start.timescale,
                                    "epoch": group.timeRange.start.epoch, "sample_value": item.numberValue.map { $0 as Any } ?? NSNull()])
                            }
                        }
                        guard reader.status == .completed else { throw reader.error ?? failure("metadata reader did not complete") }
                    }
                    facts["still_image_time_samples"] = covers
                }
            }
            report["status"] = "readback-complete"
        } catch {
            report["status"] = "readback-error"
            let native = error as NSError
            report["error"] = ["domain": native.domain, "code": native.code, "description": String(describing: error)]
        }
        report["resources"] = resources
    }

    nonisolated private func export(_ resource: PHAssetResource, to url: URL) throws {
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

    nonisolated private func failure(_ message: String) -> NSError {
        NSError(domain: "PVTReadback", code: 1, userInfo: [NSLocalizedDescriptionKey: message])
    }
}
