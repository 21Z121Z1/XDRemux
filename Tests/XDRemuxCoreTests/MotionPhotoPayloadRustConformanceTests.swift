import Foundation
import XCTest
@testable import XDRemuxCore

final class MotionPhotoPayloadRustConformanceTests: XCTestCase {
    private func temporaryDirectory() throws -> URL {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-payload-rust-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        return url
    }

    private func runRust(
        source: URL,
        range: MotionPhotoByteRange,
        destination: URL,
        maxBytes: Int64,
        bufferSize: Int
    ) throws -> [String: Any] {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
        process.arguments = [
            "cargo", "run", "--quiet", "--locked", "-p", "xdremux-motion-photo",
            "--example", "payload_conformance", "--",
            source.path,
            String(range.lowerBound),
            String(range.upperBound),
            destination.path,
            String(maxBytes),
            String(bufferSize),
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
        XCTAssertEqual(process.terminationStatus, 0, "Rust payload oracle failed: \(error)")
        let object = try JSONSerialization.jsonObject(with: output)
        return try XCTUnwrap(object as? [String: Any])
    }

    private func assertSwiftError(
        _ expected: MotionPhotoParsingError,
        range: MotionPhotoByteRange,
        source: URL,
        destination: URL,
        maxBytes: Int64,
        bufferSize: Int,
        file: StaticString = #filePath,
        line: UInt = #line
    ) {
        XCTAssertThrowsError(
            try MotionPhotoPayloadExtractor.copy(
                range: range,
                from: source,
                to: destination,
                maxBytes: maxBytes,
                bufferSize: bufferSize
            ),
            file: file,
            line: line
        ) { error in
            XCTAssertEqual(error as? MotionPhotoParsingError, expected, file: file, line: line)
        }
    }

    private func assertRustError(
        _ expectedCode: String,
        result: [String: Any],
        file: StaticString = #filePath,
        line: UInt = #line
    ) {
        XCTAssertEqual(result["status"] as? String, "error", file: file, line: line)
        XCTAssertEqual(result["kind"] as? String, "motionPhoto", file: file, line: line)
        XCTAssertEqual(result["code"] as? String, expectedCode, file: file, line: line)
    }

    func testSwiftAndRustPayloadExtractionContractsMatch() throws {
        let root = try temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }

        let bytes = Data((0..<4096).map { UInt8($0 & 0xff) })
        let source = root.appendingPathComponent("source.bin")
        try bytes.write(to: source)

        let range = try MotionPhotoByteRange(lowerBound: 17, upperBound: 3031)
        let swiftDestination = root.appendingPathComponent("swift/nested/output.bin")
        let rustDestination = root.appendingPathComponent("rust/nested/output.bin")
        try FileManager.default.createDirectory(
            at: swiftDestination.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try FileManager.default.createDirectory(
            at: rustDestination.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try Data("stale swift".utf8).write(to: swiftDestination)
        try Data("stale rust".utf8).write(to: rustDestination)

        try MotionPhotoPayloadExtractor.copy(
            range: range,
            from: source,
            to: swiftDestination,
            maxBytes: 4096,
            bufferSize: 97
        )
        let rustSuccess = try runRust(
            source: source,
            range: range,
            destination: rustDestination,
            maxBytes: 4096,
            bufferSize: 97
        )
        XCTAssertEqual(rustSuccess["status"] as? String, "ok")
        let swiftBytes = try Data(contentsOf: swiftDestination)
        let rustBytes = try Data(contentsOf: rustDestination)
        XCTAssertEqual(swiftBytes, rustBytes)
        XCTAssertEqual(swiftBytes, bytes.subdata(in: 17..<3031))

        let emptyRange = try MotionPhotoByteRange(lowerBound: 222, upperBound: 222)
        let swiftEmpty = root.appendingPathComponent("swift-empty.bin")
        let rustEmpty = root.appendingPathComponent("rust-empty.bin")
        try MotionPhotoPayloadExtractor.copy(
            range: emptyRange,
            from: source,
            to: swiftEmpty,
            maxBytes: 1,
            bufferSize: 1
        )
        let rustEmptyResult = try runRust(
            source: source,
            range: emptyRange,
            destination: rustEmpty,
            maxBytes: 1,
            bufferSize: 1
        )
        XCTAssertEqual(rustEmptyResult["status"] as? String, "ok")
        XCTAssertEqual(try Data(contentsOf: swiftEmpty), Data())
        XCTAssertEqual(try Data(contentsOf: rustEmpty), Data())

        let preserved = Data("keep me".utf8)
        let swiftLimit = root.appendingPathComponent("swift-limit.bin")
        let rustLimit = root.appendingPathComponent("rust-limit.bin")
        try preserved.write(to: swiftLimit)
        try preserved.write(to: rustLimit)
        assertSwiftError(
            .payloadTooLarge,
            range: range,
            source: source,
            destination: swiftLimit,
            maxBytes: range.length - 1,
            bufferSize: 64
        )
        assertRustError(
            "payloadTooLarge",
            result: try runRust(
                source: source,
                range: range,
                destination: rustLimit,
                maxBytes: range.length - 1,
                bufferSize: 64
            )
        )
        XCTAssertEqual(try Data(contentsOf: swiftLimit), preserved)
        XCTAssertEqual(try Data(contentsOf: rustLimit), preserved)

        let swiftZero = root.appendingPathComponent("swift-zero.bin")
        let rustZero = root.appendingPathComponent("rust-zero.bin")
        try preserved.write(to: swiftZero)
        try preserved.write(to: rustZero)
        assertSwiftError(
            .invalidByteRange,
            range: range,
            source: source,
            destination: swiftZero,
            maxBytes: 4096,
            bufferSize: 0
        )
        assertRustError(
            "invalidByteRange",
            result: try runRust(
                source: source,
                range: range,
                destination: rustZero,
                maxBytes: 4096,
                bufferSize: 0
            )
        )
        XCTAssertEqual(try Data(contentsOf: swiftZero), preserved)
        XCTAssertEqual(try Data(contentsOf: rustZero), preserved)

        let invalidRange = try MotionPhotoByteRange(lowerBound: 4000, upperBound: 5000)
        let swiftOutOfFile = root.appendingPathComponent("swift-out-of-file.bin")
        let rustOutOfFile = root.appendingPathComponent("rust-out-of-file.bin")
        try preserved.write(to: swiftOutOfFile)
        try preserved.write(to: rustOutOfFile)
        assertSwiftError(
            .invalidByteRange,
            range: invalidRange,
            source: source,
            destination: swiftOutOfFile,
            maxBytes: 4096,
            bufferSize: 64
        )
        assertRustError(
            "invalidByteRange",
            result: try runRust(
                source: source,
                range: invalidRange,
                destination: rustOutOfFile,
                maxBytes: 4096,
                bufferSize: 64
            )
        )
        XCTAssertEqual(try Data(contentsOf: swiftOutOfFile), preserved)
        XCTAssertEqual(try Data(contentsOf: rustOutOfFile), preserved)

        let swiftSame = root.appendingPathComponent("swift-same.bin")
        let rustSame = root.appendingPathComponent("rust-same.bin")
        try Data(repeating: 0x88, count: 16).write(to: swiftSame)
        try Data(repeating: 0x88, count: 16).write(to: rustSame)
        let fullRange = try MotionPhotoByteRange(lowerBound: 0, upperBound: 16)
        assertSwiftError(
            .invalidByteRange,
            range: fullRange,
            source: swiftSame,
            destination: swiftSame,
            maxBytes: 1024,
            bufferSize: 8
        )
        assertRustError(
            "invalidByteRange",
            result: try runRust(
                source: rustSame,
                range: fullRange,
                destination: rustSame,
                maxBytes: 1024,
                bufferSize: 8
            )
        )
        XCTAssertFalse(FileManager.default.fileExists(atPath: swiftSame.path))
        XCTAssertFalse(FileManager.default.fileExists(atPath: rustSame.path))
    }
}
