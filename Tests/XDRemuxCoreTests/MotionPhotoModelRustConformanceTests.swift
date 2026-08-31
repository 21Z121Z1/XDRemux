import Foundation
import XCTest
@testable import XDRemuxCore

final class MotionPhotoModelRustConformanceTests: XCTestCase {
    func testPresentationSourceRawValuesMatchRust() throws {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
        process.arguments = [
            "cargo", "run", "--quiet", "--locked", "-p", "xdremux-motion-photo",
            "--example", "motion_photo_conformance", "--", "sources",
        ]
        process.currentDirectoryURL = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
        let stdout = Pipe()
        let stderr = Pipe()
        process.standardOutput = stdout
        process.standardError = stderr
        try process.run()
        process.waitUntilExit()

        let output = stdout.fileHandleForReading.readDataToEndOfFile()
        let error = String(
            data: stderr.fileHandleForReading.readDataToEndOfFile(),
            encoding: .utf8
        ) ?? ""
        XCTAssertEqual(process.terminationStatus, 0, "Rust Motion Photo model oracle failed: \(error)")

        let object = try JSONSerialization.jsonObject(with: output)
        let dictionary = try XCTUnwrap(object as? [String: Any])
        let rustSources = try XCTUnwrap(dictionary["presentationSources"] as? [String])
        let swiftSources = [
            MotionPhotoPresentationSource.androidXMP.rawValue,
            MotionPhotoPresentationSource.legacyMicroVideoXMP.rawValue,
            MotionPhotoPresentationSource.oppoCoverFrame.rawValue,
            MotionPhotoPresentationSource.timelineFallback.rawValue,
        ]
        XCTAssertEqual(rustSources, swiftSources)
    }
}
