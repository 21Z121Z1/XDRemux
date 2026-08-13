import Foundation
import XCTest
@testable import XDRemuxCLI

final class MotionPhotoBatchPlannerTests: XCTestCase {
    func testPreferredOutputPreservesUserBasenameForJPEGAndHEIC() {
        let root = URL(fileURLWithPath: "/tmp/xdremux-input", isDirectory: true)
        let output = URL(fileURLWithPath: "/tmp/xdremux-output", isDirectory: true)

        for name in ["IMG.jpg", "IMG.heic", "IMG.heif"] {
            let planned = MotionPhotoBatchPlanner.outputImageURL(
                for: root.appendingPathComponent(name),
                inputRootURL: root,
                outputDirectoryURL: output
            )
            XCTAssertEqual(planned.lastPathComponent, "IMG.heic")
        }
    }

    func testDuplicateBasenamesReceiveSequenceOnlyAtCollision() {
        let root = URL(fileURLWithPath: "/tmp/xdremux-input", isDirectory: true)
        let output = URL(fileURLWithPath: "/tmp/xdremux-output", isDirectory: true)
        let a = root.appendingPathComponent("A/IMG.jpg")
        let b = root.appendingPathComponent("B/IMG.jpg")
        var reserved = Set<String>()

        let aOutput = MotionPhotoBatchPlanner.reserveOutputImageURL(
            for: a,
            inputRootURL: root,
            outputDirectoryURL: output,
            reservedPaths: &reserved,
            fileExists: { _ in false }
        )
        let bOutput = MotionPhotoBatchPlanner.reserveOutputImageURL(
            for: b,
            inputRootURL: root,
            outputDirectoryURL: output,
            reservedPaths: &reserved,
            fileExists: { _ in false }
        )

        XCTAssertEqual(aOutput.lastPathComponent, "IMG.heic")
        XCTAssertEqual(bOutput.lastPathComponent, "IMG (2).heic")
        XCTAssertTrue(reserved.contains(output.appendingPathComponent("IMG.mov").standardizedFileURL.path))
        XCTAssertTrue(reserved.contains(output.appendingPathComponent("IMG (2).mov").standardizedFileURL.path))
    }

    func testHEICSourceInSameDirectoryUsesSequenceInsteadOfLiveMarker() {
        let root = URL(fileURLWithPath: "/tmp/xdremux-input", isDirectory: true)
        let input = root.appendingPathComponent("IMG.heic")
        var reserved = Set<String>()
        let planned = MotionPhotoBatchPlanner.reserveOutputImageURL(
            for: input,
            inputRootURL: root,
            outputDirectoryURL: root,
            reservedPaths: &reserved,
            fileExists: { _ in false }
        )
        XCTAssertEqual(planned.lastPathComponent, "IMG (2).heic")
    }

    func testForeignExistingPairSequencesButProvenanceCanKeepOriginalName() {
        let root = URL(fileURLWithPath: "/tmp/xdremux-input", isDirectory: true)
        let output = URL(fileURLWithPath: "/tmp/xdremux-output", isDirectory: true)
        let input = root.appendingPathComponent("IMG.jpg")
        let original = output.appendingPathComponent("IMG.heic")
        var foreignReserved = Set<String>()

        let foreign = MotionPhotoBatchPlanner.reserveOutputImageURL(
            for: input,
            inputRootURL: root,
            outputDirectoryURL: output,
            reservedPaths: &foreignReserved,
            fileExists: { $0.lastPathComponent == "IMG.heic" || $0.lastPathComponent == "IMG.mov" }
        )
        XCTAssertEqual(foreign.lastPathComponent, "IMG (2).heic")

        var ownedReserved = Set<String>()
        let owned = MotionPhotoBatchPlanner.reserveOutputImageURL(
            for: input,
            inputRootURL: root,
            outputDirectoryURL: output,
            reservedPaths: &ownedReserved,
            candidateBelongsToSource: { image, video in
                image == original && video == output.appendingPathComponent("IMG.mov")
            },
            fileExists: { _ in true }
        )
        XCTAssertEqual(owned.lastPathComponent, "IMG.heic")
    }
}
