import Foundation
import XCTest
@testable import XDRemuxCLI

final class MotionPhotoBatchCheckpointTests: XCTestCase {
    func testRoundTripsPairOutputsAndCheapInputSignature() throws {
        let directory = try makeDirectory("roundtrip")
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
            signature: signature,
            assetIdentifier: "ASSET-1"
        )
        try writer.close()

        let state = try MotionPhotoBatchCheckpoint.load(url: checkpoint)
        let item = try XCTUnwrap(state[input.standardizedFileURL.path])
        XCTAssertEqual(item.status, .success)
        XCTAssertTrue(item.matchesSignature(signature))
        XCTAssertTrue(item.matchesOutputs(imageURL: image, videoURL: video))
        XCTAssertEqual(item.assetIdentifier, "ASSET-1")
    }

    func testPR18CheckpointRecordMigratesWithoutDigestDependency() throws {
        let directory = try makeDirectory("migration")
        defer { try? FileManager.default.removeItem(at: directory) }
        let input = directory.appendingPathComponent("source.jpg")
        let image = directory.appendingPathComponent("source.heic")
        let video = directory.appendingPathComponent("source.mov")
        let checkpoint = directory.appendingPathComponent("checkpoint.jsonl")
        let record: [String: Any] = [
            "kind": "item",
            "inputPath": input.standardizedFileURL.path,
            "sourceRelativePath": "source.jpg",
            "outputImagePath": image.standardizedFileURL.path,
            "outputVideoPath": video.standardizedFileURL.path,
            "status": "success",
            "inputSize": 123,
            "inputMtimeNs": 456,
            "inputSHA256": String(repeating: "ab", count: 32),
            "assetIdentifier": "ASSET-OLD",
            "error": NSNull(),
        ]
        let data = try JSONSerialization.data(withJSONObject: record, options: [.sortedKeys])
        try data.write(to: checkpoint)
        try FileHandle(forWritingTo: checkpoint).close()

        let state = try MotionPhotoBatchCheckpoint.load(url: checkpoint)
        let item = try XCTUnwrap(state[input.standardizedFileURL.path])
        XCTAssertEqual(item.inputSize, 123)
        XCTAssertEqual(item.inputMtimeNs, 456)
        XCTAssertEqual(item.assetIdentifier, "ASSET-OLD")
    }

    func testMalformedRecordIsIgnored() throws {
        let directory = try makeDirectory("malformed")
        defer { try? FileManager.default.removeItem(at: directory) }
        let checkpoint = directory.appendingPathComponent("checkpoint.jsonl")
        let input = directory.appendingPathComponent("source.jpg").standardizedFileURL.path
        let contents = """
        {"kind":"item","inputPath":"\(input)","unexpected":true}
        {"kind":"item"
        """
        try Data(contents.utf8).write(to: checkpoint)
        XCTAssertEqual(try MotionPhotoBatchCheckpoint.load(url: checkpoint).count, 0)
    }

    func testChangedSourceMetadataDoesNotResumeAsDone() throws {
        let directory = try makeDirectory("change")
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
            signature: before,
            assetIdentifier: "ASSET-1"
        )
        try writer.close()

        try Data("after-with-different-size".utf8).write(to: input, options: .atomic)
        let after = try MotionPhotoBatchCheckpoint.signature(for: input)
        let item = try XCTUnwrap(try MotionPhotoBatchCheckpoint.load(url: checkpoint)[input.standardizedFileURL.path])
        XCTAssertFalse(item.matchesSignature(after))
    }

    func testMtimeChangeInvalidatesSameSizeSource() throws {
        let directory = try makeDirectory("mtime")
        defer { try? FileManager.default.removeItem(at: directory) }
        let input = directory.appendingPathComponent("source.jpg")
        try Data("AAAA".utf8).write(to: input)
        let before = try MotionPhotoBatchCheckpoint.signature(for: input)
        let future = Date(timeIntervalSince1970: Double(before.mtimeNs) / 1_000_000_000 + 10)
        try FileManager.default.setAttributes([.modificationDate: future], ofItemAtPath: input.path)
        let after = try MotionPhotoBatchCheckpoint.signature(for: input)

        XCTAssertEqual(before.size, after.size)
        XCTAssertNotEqual(before.mtimeNs, after.mtimeNs)
        XCTAssertNotEqual(before, after)
    }

    private func makeDirectory(_ suffix: String) throws -> URL {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-motion-checkpoint-\(suffix)-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        return directory
    }
}
