import Foundation
import UIKit
import UniformTypeIdentifiers

// Shares one complete file representation with Apple's system activity. There
// is no PhotoKit creation request, resource-list import or PHLivePhoto assembly.
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
            button.addTarget(self, action: #selector(sharePackage), for: .touchUpInside)
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

    @objc private func sharePackage() {
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
            present(UIActivityViewController(activityItems: [image], applicationActivities: nil))
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
            present(UIActivityViewController(activityItems: [package], applicationActivities: nil))
            return
        }
        guard let fileTypeIdentifier = nativeBundleFacts["url_content_type"] as? String,
              let registeredTypes = nativeBundleFacts["contents_provider_registered_types"] as? [String],
              registeredTypes.contains(fileTypeIdentifier),
              nativeBundleFacts["package_is_directory"] as? Bool == true,
              nativeBundleFacts["package_is_package"] as? Bool == true else {
            finishShare(activity: nil, completed: false,
                        error: ReadbackCore.failure("native PVT bundle file type is not established"))
            return
        }
        let provider = NSItemProvider()
        provider.suggestedName = "capture.pvt"
        provider.registerFileRepresentation(forTypeIdentifier: fileTypeIdentifier, fileOptions: [], visibility: .all) { completion in
            do {
                let hashes = try ReadbackCore.packageHashes(package)
                let facts: [String: Any] = ["type_identifier": fileTypeIdentifier, "share_phase": "share", "whole_package_url": package.path,
                                           "representation": "file", "is_in_place": false,
                                           "member_sha256": hashes, "native_bundle_facts": nativeBundleFacts]
                try ReadbackCore.write(facts, phase: "share-transfer")
                completion(package, false, nil)
            } catch { completion(nil, false, error) }
            return nil
        }
        let configuration = UIActivityItemsConfiguration(itemProviders: [provider])
        present(UIActivityViewController(activityItemsConfiguration: configuration))
    }

    private func present(_ activity: UIActivityViewController) {
        activity.completionWithItemsHandler = { [weak self] type, completed, _, error in
            self?.finishShare(activity: type, completed: completed, error: error)
        }
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
