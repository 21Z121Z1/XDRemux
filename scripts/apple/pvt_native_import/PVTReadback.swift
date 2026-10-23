import Foundation
import UIKit
import Photos
import UniformTypeIdentifiers

// Loads one complete PVT through Photos' native file reader and hands its result
// to the system activity. No PhotoKit creation or resource-list assembly occurs.
@main
final class AppDelegate: UIResponder, UIApplicationDelegate {
    func application(_ application: UIApplication, configurationForConnecting session: UISceneSession,
                     options: UIScene.ConnectionOptions) -> UISceneConfiguration {
        let configuration = UISceneConfiguration(name: "PVT audit", sessionRole: session.role)
        configuration.delegateClass = PVTSceneDelegate.self
        return configuration
    }
}

final class PVTSceneDelegate: UIResponder, UIWindowSceneDelegate {
    var window: UIWindow?
    private let statusLabel = UILabel()
    private var sharePhase = "share"

    func scene(_ scene: UIScene, willConnectTo session: UISceneSession, options: UIScene.ConnectionOptions) {
        guard let windowScene = scene as? UIWindowScene else { return }
        let window = UIWindow(windowScene: windowScene)
        let controller = UIViewController()
        controller.view.backgroundColor = .white
        window.rootViewController = controller
        window.makeKeyAndVisible()
        self.window = window
        let arguments = CommandLine.arguments
        let index = arguments.firstIndex(of: "--phase")
        let phase = index.flatMap { arguments.indices.contains($0 + 1) ? arguments[$0 + 1] : nil } ?? "after"
        if ["share", "share-control", "share-url"].contains(phase) {
            sharePhase = phase
            let button = UIButton(type: .system)
            button.setTitle("Share PVT", for: .normal)
            button.accessibilityIdentifier = "share-pvt"
            button.frame = CGRect(x: 20, y: 100, width: 250, height: 60)
            button.addTarget(self, action: #selector(sharePackage(_:)), for: .touchUpInside)
            controller.view.addSubview(button)
            statusLabel.accessibilityIdentifier = "share-result"
            statusLabel.text = "pending"
            statusLabel.frame = CGRect(x: 20, y: 170, width: 350, height: 100)
            statusLabel.numberOfLines = 0
            controller.view.addSubview(statusLabel)
        } else {
            DispatchQueue.global(qos: .userInitiated).async {
                let report = ReadbackCore.collect(phase: phase)
                do { try ReadbackCore.write(report, phase: phase) }
                catch { NSLog("PVT_READBACK_REPORT_WRITE_FAILED: %@", String(describing: error)) }
            }
        }
    }

    @objc private func sharePackage(_ sender: UIButton) {
        let package = ReadbackCore.documents.appendingPathComponent("capture.pvt", isDirectory: true)
        var isDirectory: ObjCBool = false
        guard FileManager.default.fileExists(atPath: package.path, isDirectory: &isDirectory), isDirectory.boolValue else {
            finishShare(activity: nil, completed: false, error: ReadbackCore.failure("whole PVT transport missing"))
            return
        }
        let identifier = UTType.livePhoto.identifier
        // A previous availability control must not supply a later callback's
        // input receipt if the provider was never asked for its representation.
        let transfer = ReadbackCore.documents.appendingPathComponent("share-transfer.json")
        do {
            if FileManager.default.fileExists(atPath: transfer.path) { try FileManager.default.removeItem(at: transfer) }
        } catch {
            finishShare(activity: nil, completed: false, error: error)
            return
        }
        let nativeBundleFacts: [String: Any]
        do {
            let values = try package.resourceValues(forKeys: [.contentTypeKey, .isPackageKey, .isDirectoryKey])
            var facts: [String: Any] = ["live_photo_type": identifier,
                                       "live_photo_readable_types": PHLivePhoto.readableTypeIdentifiersForItemProvider,
                                       "live_photo_conforms_to_image": UTType.livePhoto.conforms(to: .image),
                                       "live_photo_conforms_to_package": UTType.livePhoto.conforms(to: .package),
                                       "package_is_directory": values.isDirectory == true,
                                       "package_is_package": values.isPackage == true,
                                       "contents_provider_registered_types": NSItemProvider(contentsOf: package)?.registeredTypeIdentifiers ?? []]
            facts["extension_type"] = UTType(filenameExtension: "pvt")?.identifier ?? "<not-declared>"
            facts["url_content_type"] = values.contentType?.identifier ?? "<not-declared>"
            nativeBundleFacts = facts
            try ReadbackCore.write(facts, phase: "share-types")
        } catch {
            finishShare(activity: nil, completed: false, error: error)
            return
        }
        if sharePhase == "share-control" {
            guard let image = UIImage(contentsOfFile: ReadbackCore.documents.appendingPathComponent("control.jpg").path) else {
                finishShare(activity: nil, completed: false, error: ReadbackCore.failure("synthetic control missing"))
                return
            }
            // Availability control only: XCTest never selects an activity here.
            present(UIActivityViewController(activityItems: [image], applicationActivities: nil), from: sender)
            return
        }
        if sharePhase == "share-url" {
            do {
                let facts: [String: Any] = ["representation": "whole-package-url", "share_phase": "share-url", "whole_package_url": package.path,
                                           "is_in_place": false, "member_sha256": try ReadbackCore.packageHashes(package)]
                try ReadbackCore.write(facts, phase: "share-transfer")
            } catch {
                finishShare(activity: nil, completed: false, error: error)
                return
            }
            present(UIActivityViewController(activityItems: [package], applicationActivities: nil), from: sender)
            return
        }
        let readablePackageTypes = PHLivePhoto.readableTypeIdentifiersForItemProvider.filter {
            UTType($0)?.conforms(to: .package) == true
        }
        guard let nativeFileType = nativeBundleFacts["url_content_type"] as? String,
              let registeredTypes = nativeBundleFacts["contents_provider_registered_types"] as? [String],
              registeredTypes.contains(nativeFileType), readablePackageTypes.count == 1,
              nativeBundleFacts["package_is_directory"] as? Bool == true,
              nativeBundleFacts["package_is_package"] as? Bool == true else {
            finishShare(activity: nil, completed: false,
                        error: ReadbackCore.failure("native whole-PVT reader/file types are not established"))
            return
        }
        let fileTypeIdentifier = readablePackageTypes[0]
        let members: [String: String]
        do { members = try ReadbackCore.packageHashes(package) }
        catch { finishShare(activity: nil, completed: false, error: error); return }
        // Foundation's filesystem alias is not necessarily a type that Photos
        // reads. Ask Photos to decode the exact whole-directory file promise;
        // never extract members or assemble a PHLivePhoto from resource URLs.
        let decoder = wholePackageProvider(package, typeIdentifier: fileTypeIdentifier)
        _ = decoder.loadObject(ofClass: PHLivePhoto.self) { [weak self] (object: NSItemProviderReading?, error: Error?) in
            DispatchQueue.main.async {
                guard let self else { return }
                var facts = nativeBundleFacts
                guard let photo = object as? PHLivePhoto,
                      photo.size.width.isFinite, photo.size.height.isFinite,
                      photo.size.width > 0, photo.size.height > 0 else {
                    facts["whole_package_decode"] = ["status": "failed", "type_identifier": fileTypeIdentifier,
                        "error": error.map(ReadbackCore.errorFacts) ?? ["description": "native whole-PVT decoder returned no Live Photo"]]
                    try? ReadbackCore.write(facts, phase: "share-types")
                    self.finishShare(activity: nil, completed: false,
                                     error: error ?? ReadbackCore.failure("native whole-PVT decode failed"))
                    return
                }
                facts["file_representation_conforms_to_package"] = UTType(fileTypeIdentifier)?.conforms(to: .package) == true
                facts["whole_package_decode"] = ["status": "loaded", "type_identifier": fileTypeIdentifier,
                                                  "member_sha256": members,
                                                  "width": photo.size.width, "height": photo.size.height,
                                                  "object_class": NSStringFromClass(type(of: photo))]
                do {
                    guard try ReadbackCore.packageHashes(package) == members else {
                        throw ReadbackCore.failure("native whole-PVT decode changed the input package")
                    }
                    try ReadbackCore.write(facts, phase: "share-types")
                    let transfer: [String: Any] = ["type_identifier": UTType.livePhoto.identifier,
                        "share_phase": "share", "whole_package_url": package.path,
                        "representation": "whole-package-native-live-photo", "is_in_place": false,
                        "native_object_class": NSStringFromClass(type(of: photo)),
                        "native_object_is_live_photo": photo.isKind(of: PHLivePhoto.self),
                        "member_sha256": members, "native_bundle_facts": facts]
                    try ReadbackCore.write(transfer, phase: "share-transfer")
                } catch { self.finishShare(activity: nil, completed: false, error: error); return }
                // UIKit's save activity consumes the native Live Photo value.
                // This value came only from the opaque whole-PVT reader above.
                self.present(UIActivityViewController(activityItems: [photo], applicationActivities: nil), from: sender)
            }
        }
    }

    private func wholePackageProvider(_ package: URL, typeIdentifier: String) -> NSItemProvider {
        let provider = NSItemProvider()
        provider.suggestedName = "capture.pvt"
        provider.registerFileRepresentation(forTypeIdentifier: typeIdentifier, fileOptions: [], visibility: .all) { completion in
            do {
                _ = try ReadbackCore.packageHashes(package)
                completion(package, false, nil)
            } catch { completion(nil, false, error) }
            return nil
        }
        return provider
    }

    private func present(_ activity: UIActivityViewController, from sourceView: UIView) {
        activity.completionWithItemsHandler = { [weak self] type, completed, _, error in
            self?.finishShare(activity: type, completed: completed, error: error)
        }
        activity.popoverPresentationController?.sourceView = sourceView
        activity.popoverPresentationController?.sourceRect = sourceView.bounds
        window?.rootViewController?.present(activity, animated: true)
    }

    private func finishShare(activity: UIActivity.ActivityType?, completed: Bool, error: Error?) {
        var facts: [String: Any] = ["status": completed ? "completed" : "not-completed",
                                   "share_phase": sharePhase,
                                   "activity_completed": completed,
                                   "is_save_to_camera_roll": activity == .saveToCameraRoll]
        if let activity { facts["activity_type"] = activity.rawValue }
        else { facts["activity_type"] = NSNull() }
        if let error { facts["error"] = ReadbackCore.errorFacts(error) }
        do {
            try ReadbackCore.write(facts, phase: "share")
            statusLabel.text = completed ? "completed" : "not-completed"
        } catch {
            statusLabel.text = "report-write-failed"
            NSLog("PVT_SHARE_REPORT_WRITE_FAILED: %@", String(describing: error))
        }
    }
}
