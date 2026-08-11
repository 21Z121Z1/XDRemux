import Foundation
import XCTest
@testable import XDRemuxCLI

final class MotionPhotoBatchCheckpointTests: XCTestCase {
    func testRoundTripsPairOutputsAndInputSignature() throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-motion-checkpoint-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        let input = directory.appendingPathComponent("source.jpg")
        try Data("source".utf8).write(to: input)
        let image = directory.appendingPathComponent("source.heic")
        let video = directory.appendingPathComponent("source.mov")
        let checkpoint = directory.appendingPathComponent("checkpoint.jsonl")
        let signature = try MotionPhotoBatchCheckpoint.signature(for: input)

        let writer = try MotionPhotoBatchCheckpoint.Writer(url: checkpoint)
        try writer.append(
            inputURL: input,
            outputImageURL: image,
            outputVideoURL: video,
            status: .success,
            signature: signature
        )
        try writer.close()

        let state = try MotionPhotoBatchCheckpoint.load(url: checkpoint)
        let item = try XCTUnwrap(state[input.standardizedFileURL.path])
        XCTAssertEqual(item.status, .success)
        XCTAssertTrue(item.matchesSignature(signature))
        XCTAssertTrue(item.matchesOutputs(imageURL: image, videoURL: video))
    }

    func testChangedInputSignatureDoesNotResumeAsDone() throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-motion-checkpoint-change-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        let input = directory.appendingPathComponent("source.jpg")
        try Data("before".utf8).write(to: input)
        let image = directory.appendingPathComponent("source.heic")
        let video = directory.appendingPathComponent("source.mov")
        let checkpoint = directory.appendingPathComponent("checkpoint.jsonl")
        let before = try MotionPhotoBatchCheckpoint.signature(for: input)

        let writer = try MotionPhotoBatchCheckpoint.Writer(url: checkpoint)
        try writer.append(
            inputURL: input,
            outputImageURL: image,
            outputVideoURL: video,
            status: .success,
            signature: before
        )
        try writer.close()

        try Data("after-with-different-size".utf8).write(to: input, options: .atomic)
        let after = try MotionPhotoBatchCheckpoint.signature(for: input)
        let state = try MotionPhotoBatchCheckpoint.load(url: checkpoint)
        let item = try XCTUnwrap(state[input.standardizedFileURL.path])
        XCTAssertFalse(item.matchesSignature(after))
    }
}
