import Foundation
import XCTest
import XDRemuxCore
@testable import XDRemuxCLI

final class BatchCoordinatorTests: XCTestCase {
    func testOutputURLPreservesRelativeDirectoryStructure() {
        let inputRoot = URL(fileURLWithPath: "/input", isDirectory: true)
        let outputRoot = URL(fileURLWithPath: "/output", isDirectory: true)

        XCTAssertEqual(
            BatchCoordinator.outputURL(
                input: inputRoot.appendingPathComponent("album-a/IMG_001.heic"),
                inputRoot: inputRoot,
                outputRoot: outputRoot
            ).path,
            "/output/album-a/IMG_001.heic"
        )
        XCTAssertEqual(
            BatchCoordinator.outputURL(
                input: inputRoot.appendingPathComponent("album-b/IMG_001.heic"),
                inputRoot: inputRoot,
                outputRoot: outputRoot
            ).path,
            "/output/album-b/IMG_001.heic"
        )
    }

    func testInputEnumerationExcludesNestedOutputTree() throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-enumeration-test-\(UUID().uuidString)", isDirectory: true)
        let outputDirectory = directory.appendingPathComponent("converted", isDirectory: true)
        try FileManager.default.createDirectory(at: outputDirectory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let source = directory.appendingPathComponent("album/source.heic")
        let oldOutput = outputDirectory.appendingPathComponent("album/source.heic")
        try FileManager.default.createDirectory(
            at: source.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try FileManager.default.createDirectory(
            at: oldOutput.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try Data().write(to: source)
        try Data().write(to: oldOutput)

        let inputs = try XDRemuxCommand.enumerateInputs(
            root: directory,
            glob: "*.heic",
            excluding: outputDirectory
        )

        XCTAssertEqual(
            inputs.map { $0.resolvingSymlinksInPath().path },
            [source.resolvingSymlinksInPath().path]
        )
    }

    func testValidOutputIsSkippedAndInvalidOutputIsRegenerated() {
        let items = [
            BatchWorkItem(
                inputURL: URL(fileURLWithPath: "/input/valid.heic"),
                outputURL: URL(fileURLWithPath: "/output/valid.heic")
            ),
            BatchWorkItem(
                inputURL: URL(fileURLWithPath: "/input/invalid.heic"),
                outputURL: URL(fileURLWithPath: "/output/invalid.heic")
            ),
        ]
        let state = ValidityState(validPaths: [items[0].outputURL.path])
        let reporter = quietReporter()
        reporter.beginBatch(total: items.count, jobs: 2, mode: .standard)

        let result = BatchCoordinator.run(
            items: items,
            jobs: 2,
            overwrite: false,
            diagnosticsAvailable: false,
            reporter: reporter,
            isValid: { state.contains($0.outputURL.path) },
            convert: { item in state.insert(item.outputURL.path) }
        )
        reporter.finish()

        XCTAssertEqual(result.skipped, 1)
        XCTAssertEqual(result.converted, 1)
        XCTAssertTrue(result.failures.isEmpty)
        XCTAssertEqual(state.convertedPaths, [items[1].outputURL.path])
    }

    func testOverwriteRegeneratesAValidOutput() {
        let item = BatchWorkItem(
            inputURL: URL(fileURLWithPath: "/input/valid.heic"),
            outputURL: URL(fileURLWithPath: "/output/valid.heic")
        )
        let state = ValidityState(validPaths: [item.outputURL.path])
        let reporter = quietReporter()
        reporter.beginBatch(total: 1, jobs: 1, mode: .standard)

        let result = BatchCoordinator.run(
            items: [item],
            jobs: 1,
            overwrite: true,
            diagnosticsAvailable: false,
            reporter: reporter,
            isValid: { state.contains($0.outputURL.path) },
            convert: { converted in state.insert(converted.outputURL.path) }
        )
        reporter.finish()

        XCTAssertEqual(result.converted, 1)
        XCTAssertEqual(result.skipped, 0)
        XCTAssertEqual(state.convertedPaths, [item.outputURL.path])
    }

    func testOneFailureDoesNotStopConcurrentBatchAndCountsRemainConsistent() {
        let items = (0..<20).map { index in
            BatchWorkItem(
                inputURL: URL(fileURLWithPath: "/input/\(index).heic"),
                outputURL: URL(fileURLWithPath: "/output/\(index).heic")
            )
        }
        let state = ValidityState(validPaths: [])
        let reporter = quietReporter()
        reporter.beginBatch(total: items.count, jobs: 4, mode: .standard)

        let result = BatchCoordinator.run(
            items: items,
            jobs: 4,
            overwrite: false,
            diagnosticsAvailable: true,
            reporter: reporter,
            isValid: { state.contains($0.outputURL.path) },
            convert: { item in
                if item.inputURL.lastPathComponent == "7.heic" {
                    throw XDRemuxError.invalidContainer("broken test container")
                }
                state.insert(item.outputURL.path)
            }
        )
        reporter.finish()

        XCTAssertEqual(result.converted, 19)
        XCTAssertEqual(result.skipped, 0)
        XCTAssertEqual(result.failures.count, 1)
        XCTAssertEqual(result.failures[0].errorCode, "internal_container_error")
        XCTAssertTrue(result.failures[0].diagnosticsAvailable)
    }

    func testAtomicConversionReplacesOutputAndCleansTemporaryFile() throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-batch-test-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let input = directory.appendingPathComponent("input.heic")
        let output = directory.appendingPathComponent("nested/output.heic")
        try Data("input".utf8).write(to: input)
        try FileManager.default.createDirectory(
            at: output.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try Data("old".utf8).write(to: output)

        try BatchCoordinator.convertAtomically(
            item: BatchWorkItem(inputURL: input, outputURL: output)
        ) { _, temporary in
            try Data("new".utf8).write(to: temporary)
        }

        XCTAssertEqual(try Data(contentsOf: output), Data("new".utf8))
        let siblings = try FileManager.default.contentsOfDirectory(
            at: output.deletingLastPathComponent(),
            includingPropertiesForKeys: nil
        )
        XCTAssertFalse(siblings.contains { $0.lastPathComponent.contains("xdremux-batch") })
    }

    func testAtomicValidationFailureKeepsPreviousOutputAndCleansTemporaryFile() throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-batch-validation-test-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let input = directory.appendingPathComponent("input.heic")
        let output = directory.appendingPathComponent("output.heic")
        try Data("input".utf8).write(to: input)
        try Data("old".utf8).write(to: output)

        XCTAssertThrowsError(try BatchCoordinator.convertAtomically(
            item: BatchWorkItem(inputURL: input, outputURL: output),
            validateTemporary: { _ in false }
        ) { _, temporary in
            try Data("invalid".utf8).write(to: temporary)
        })

        XCTAssertEqual(try Data(contentsOf: output), Data("old".utf8))
        let siblings = try FileManager.default.contentsOfDirectory(
            at: directory,
            includingPropertiesForKeys: nil
        )
        XCTAssertFalse(siblings.contains { $0.lastPathComponent.contains("xdremux-batch") })
    }

    func testFailureReportUsesStableSchemaAndIsRemovedAfterCleanRun() throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-report-test-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let failure = BatchFailureRecord(
            input: "/input/a.heic",
            output: "/output/a.heic",
            errorCode: "source_gain_map_missing",
            diagnosticsAvailable: false
        )

        let reportURL = try XCTUnwrap(BatchCoordinator.writeFailureReport(
            [failure],
            outputDirectory: directory
        ))
        let object = try JSONSerialization.jsonObject(with: Data(contentsOf: reportURL))
        let report = try XCTUnwrap(object as? [String: Any])
        XCTAssertEqual(report["schema_version"] as? Int, 1)
        let failures = try XCTUnwrap(report["failures"] as? [[String: Any]])
        XCTAssertEqual(failures[0]["error_code"] as? String, "source_gain_map_missing")
        XCTAssertEqual(failures[0]["diagnostics_available"] as? Bool, false)

        XCTAssertNil(try BatchCoordinator.writeFailureReport([], outputDirectory: directory))
        XCTAssertFalse(FileManager.default.fileExists(atPath: reportURL.path))
    }

    private func quietReporter() -> CLIReporter {
        var options = OutputOptions()
        options.verbosity = .quiet
        return CLIReporter(
            options: options,
            localizer: Localizer(requested: .english, environment: [:], preferredLanguages: []),
            isTTY: false,
            environment: [:],
            stdout: OutputWriter { _ in },
            stderr: OutputWriter { _ in }
        )
    }
}

private final class ValidityState: @unchecked Sendable {
    private let lock = NSLock()
    private var paths: Set<String>
    private var converted: Set<String> = []

    init(validPaths: Set<String>) {
        paths = validPaths
    }

    func contains(_ path: String) -> Bool {
        lock.withLock { paths.contains(path) }
    }

    func insert(_ path: String) {
        lock.withLock {
            paths.insert(path)
            converted.insert(path)
        }
    }

    var convertedPaths: Set<String> { lock.withLock { converted } }
}
