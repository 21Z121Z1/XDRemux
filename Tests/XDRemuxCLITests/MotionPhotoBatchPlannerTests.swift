import Foundation
import XCTest
@testable import XDRemuxCLI

final class MotionPhotoBatchPlannerTests: XCTestCase {
    func testDuplicateBasenamesPreserveRelativeDirectoriesAcrossSubsetRuns() throws {
        let root = URL(fileURLWithPath: "/tmp/xdremux-input", isDirectory: true)
        let output = URL(fileURLWithPath: "/tmp/xdremux-output", isDirectory: true)
        let a = root.appendingPathComponent("A/IMG.jpg")
        let b = root.appendingPathComponent("B/IMG.jpg")

        let aOutput = MotionPhotoBatchPlanner.outputImageURL(
            for: a,
            inputRootURL: root,
            outputDirectoryURL: output
        )
        let bOutput = MotionPhotoBatchPlanner.outputImageURL(
            for: b,
            inputRootURL: root,
            outputDirectoryURL: output
        )
        let bSubsetOutput = MotionPhotoBatchPlanner.outputImageURL(
            for: b,
            inputRootURL: root,
            outputDirectoryURL: output
        )

        XCTAssertEqual(aOutput.path, output.appendingPathComponent("A/IMG.live.heic").path)
        XCTAssertEqual(bOutput.path, output.appendingPathComponent("B/IMG.live.heic").path)
        XCTAssertEqual(bOutput, bSubsetOutput)
    }

    func testJPEGOutputDoesNotUseSiblingSourceHEICName() {
        let root = URL(fileURLWithPath: "/tmp/xdremux-input", isDirectory: true)
        let output = root
        let jpeg = root.appendingPathComponent("A/IMG.jpg")
        let siblingHEIC = root.appendingPathComponent("A/IMG.heic")
        let planned = MotionPhotoBatchPlanner.outputImageURL(
            for: jpeg,
            inputRootURL: root,
            outputDirectoryURL: output
        )
        XCTAssertNotEqual(planned.path, siblingHEIC.path)
        XCTAssertEqual(planned.path, root.appendingPathComponent("A/IMG.live.heic").path)
    }

    func testAbsoluteInputRootDoesNotLeakIntoOutputName() {
        let firstRoot = URL(fileURLWithPath: "/tmp/xdremux-root-one", isDirectory: true)
        let secondRoot = URL(fileURLWithPath: "/tmp/xdremux-root-two", isDirectory: true)
        let output = URL(fileURLWithPath: "/tmp/xdremux-output", isDirectory: true)
        let first = firstRoot.appendingPathComponent("A/IMG.jpg")
        let second = secondRoot.appendingPathComponent("A/IMG.jpg")

        let firstOutput = MotionPhotoBatchPlanner.outputImageURL(
            for: first,
            inputRootURL: firstRoot,
            outputDirectoryURL: output
        )
        let secondOutput = MotionPhotoBatchPlanner.outputImageURL(
            for: second,
            inputRootURL: secondRoot,
            outputDirectoryURL: output
        )

        XCTAssertEqual(firstOutput.path, output.appendingPathComponent("A/IMG.live.heic").path)
        XCTAssertEqual(secondOutput, firstOutput)
    }

    func testHEIFMotionPhotoUsesReadableLiveFilenameInsideRelativeDirectory() {
        let root = URL(fileURLWithPath: "/tmp/xdremux-input", isDirectory: true)
        let output = URL(fileURLWithPath: "/tmp/xdremux-output", isDirectory: true)
        let input = root.appendingPathComponent("Trips/IMG.heic")
        let planned = MotionPhotoBatchPlanner.outputImageURL(
            for: input,
            inputRootURL: root,
            outputDirectoryURL: output
        )
        XCTAssertEqual(planned.path, output.appendingPathComponent("Trips/IMG.live.heic").path)
    }

    func testPlannerRejectsDuplicateDestination() {
        let first = URL(fileURLWithPath: "/tmp/a.jpg")
        let second = URL(fileURLWithPath: "/tmp/b.jpg")
        let output = URL(fileURLWithPath: "/tmp/result.heic")
        XCTAssertThrowsError(
            try MotionPhotoBatchPlanner.validateUnique([
                (input: first, output: output),
                (input: second, output: output),
            ])
        )
    }
}
