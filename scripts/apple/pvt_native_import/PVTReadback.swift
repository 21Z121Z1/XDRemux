import Foundation
import UIKit
import UniformTypeIdentifiers

// Shares one complete file representation with Apple's system activity. There
// is no PhotoKit creation request, resource-list import or PHLivePhoto assembly.
@main
final class AppDelegate: UIResponder, UIApplicationDelegate {
    var window: UIWindow?
    private let statusLabel = UILabel()

    func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {
        let window = UIWindow(frame: UIScreen.main.bounds)
        let controller = UIViewController()
        controller.view.backgroundColor = .white
        window.rootViewController = controller
        window.makeKeyAndVisible()
        self.window = window
        let arguments = CommandLine.arguments
        let index = arguments.firstIndex(of: "--phase")
        let phase = index.flatMap { arguments.indices.contains($0 + 1) ? arguments[$0 + 1] : nil } ?? "after"
        if phase == "share" {
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
        return true
    }

    @objc private func sharePackage() {
        let package = ReadbackCore.documents.appendingPathComponent("capture.pvt", isDirectory: true)
        var isDirectory: ObjCBool = false
        guard FileManager.default.fileExists(atPath: package.path, isDirectory: &isDirectory), isDirectory.boolValue else {
            finishShare(activity: nil, completed: false, error: ReadbackCore.failure("whole PVT transport missing"))
            return
        }
        let identifier = UTType.livePhoto.identifier
        let provider = NSItemProvider()
        provider.suggestedName = "capture.pvt"
        provider.registerFileRepresentation(forTypeIdentifier: identifier, fileOptions: [], visibility: .all) { completion in
            do {
                let hashes = try ReadbackCore.packageHashes(package)
                let facts: [String: Any] = ["type_identifier": identifier, "whole_package_url": package.path,
                                           "representation": "file", "is_in_place": false,
                                           "member_sha256": hashes]
                try ReadbackCore.write(facts, phase: "share-transfer")
                completion(package, false, nil)
            } catch { completion(nil, false, error) }
            return nil
        }
        let configuration = UIActivityItemsConfiguration(itemProviders: [provider])
        let activity = UIActivityViewController(activityItemsConfiguration: configuration)
        activity.completionWithItemsHandler = { [weak self] type, completed, _, error in
            self?.finishShare(activity: type, completed: completed, error: error)
        }
        window?.rootViewController?.present(activity, animated: true)
    }

    private func finishShare(activity: UIActivity.ActivityType?, completed: Bool, error: Error?) {
        var facts: [String: Any] = ["status": completed ? "completed" : "not-completed",
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
