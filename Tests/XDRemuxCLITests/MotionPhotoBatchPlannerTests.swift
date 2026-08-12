import Foundation
import XCTest
@testable import XDRemuxCLI

final class MotionPhotoBatchPlannerTests: XCTestCase {
    func testDuplicateBasenamesRemainDistinctAndStableAcrossSubsetRuns() throws {
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

        XCTAssertNotEqual(aOutput, bOutput)
        XCTAssertEqual(bOutput, bSubsetOutput)
        XCTAssertTrue(aOutput.lastPathComponent.hasPrefix("IMG~"))
        XCTAssertTrue(bOutput.lastPathComponent.hasPrefix("IMG~"))
        XCTAssertEqual(aOutput.pathExtension, "heic")
    }

    func testSameRelativePathInDifferentInputRootsCannotAliasSharedOutputDirectory() {
        let firstRoot = URL(fileURLWithPath: "/tmp/xdremux-root-one", isDirectory: true)
        let secondRoot = URL(fileURLWithPath: "/tmp/xdremux-root-two", isDirectory: true)
        let output = URL(fileURLWithPath: "/tmp/xdremux-shared-output", isDirectory: true)
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

        XCTAssertNotEqual(firstOutput, secondOutput)
    }

    func testHEIFMotionPhotoUsesSeparateLiveNamespace() {
        let root = URL(fileURLWithPath: "/tmp/xdremux-input", isDirectory: true)
        let output = URL(fileURLWithPath: "/tmp/xdremux-output", isDirectory: true)
        let input = root.appendingPathComponent("IMG.heic")
        let planned = MotionPhotoBatchPlanner.outputImageURL(
            for: input,
            inputRootURL: root,
            outputDirectoryURL: output
        )
        XCTAssertTrue(planned.lastPathComponent.hasPrefix("IMG.live~"))
        XCTAssertEqual(planned.pathExtension, "heic")
    }

    func testPlannerRejectsDuplicateDestinationInsteadOfRenumberingByOrder() {
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
