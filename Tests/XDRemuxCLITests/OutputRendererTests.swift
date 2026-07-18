import Foundation
import XCTest
import XDRemuxCore
@testable import XDRemuxCLI

final class OutputRendererTests: XCTestCase {
    func testTTYUsesANSIAndFinishRestoresCursor() {
        let capture = OutputCapture()
        let reporter = makeReporter(capture: capture, isTTY: true)
        let input = URL(fileURLWithPath: "/tmp/input.heic")
        let output = URL(fileURLWithPath: "/tmp/output.heic")
        reporter.beginSingle(
            input: input,
            output: output,
            mode: .standard,
            phases: CLIReporter.plannedPhases(for: ConversionOptions())
        )
        reporter.handleSingle(.phaseChanged(.readingSource), input: input, output: output)
        reporter.finish()

        XCTAssertTrue(capture.stderr.contains("\u{001B}[?25l"))
        XCTAssertTrue(capture.stderr.contains("\u{001B}[2K"))
        XCTAssertTrue(capture.stderr.hasSuffix("\u{001B}[?25h"))
    }

    func testNonTTYAndCIDoNotUseANSI() {
        for configuration in [(false, [String: String]()), (true, ["CI": "true"])] {
            let capture = OutputCapture()
            let reporter = makeReporter(
                capture: capture,
                isTTY: configuration.0,
                environment: configuration.1
            )
            let input = URL(fileURLWithPath: "/tmp/input.heic")
            let output = URL(fileURLWithPath: "/tmp/output.heic")
            reporter.beginSingle(
                input: input,
                output: output,
                mode: .standard,
                phases: CLIReporter.plannedPhases(for: ConversionOptions())
            )
            reporter.handleSingle(.phaseChanged(.readingSource), input: input, output: output)
            reporter.finish()
            XCTAssertFalse(capture.stderr.contains("\u{001B}"))
        }
    }

    func testQuietSuppressesProgressButKeepsFinalResult() {
        let capture = OutputCapture()
        var options = OutputOptions()
        options.verbosity = .quiet
        let reporter = makeReporter(capture: capture, options: options, isTTY: false)
        let input = URL(fileURLWithPath: "/tmp/input.heic")
        let output = URL(fileURLWithPath: "/tmp/output.heic")
        reporter.beginSingle(input: input, output: output, mode: .standard, phases: [.readingSource])
        reporter.handleSingle(.phaseChanged(.readingSource), input: input, output: output)
        reporter.handleSingle(
            .completed(ConversionResult(
                input: InputSource(url: input),
                output: OutputDestination(url: output)
            )),
            input: input,
            output: output
        )
        reporter.finish()

        XCTAssertFalse(capture.stderr.contains("Reading source"))
        XCTAssertTrue(capture.stderr.contains("Completed: /tmp/output.heic"))
    }

    func testDefaultBatchDoesNotPrintEverySuccessfulFileButVerboseDoes() {
        XCTAssertFalse(batchOutput(verbosity: .normal).contains("Converted a.heic"))
        XCTAssertTrue(batchOutput(verbosity: .verbose).contains("Converted a.heic"))
    }

    func testDefaultHundredFileBatchHasBoundedLineOutput() {
        let capture = OutputCapture()
        let reporter = makeReporter(capture: capture, isTTY: false)
        reporter.beginBatch(total: 100, jobs: 4, mode: .standard)
        for index in 0..<100 {
            let input = URL(fileURLWithPath: "/tmp/file-\(index).heic")
            reporter.batchFileStarted(input)
            reporter.batchFileFinished(
                input: input,
                output: URL(fileURLWithPath: "/tmp/out/file-\(index).heic"),
                outcome: .converted
            )
        }
        reporter.completeBatch(failureReportURL: nil)
        reporter.finish()

        let lines = capture.stderr.split(separator: "\n")
        XCTAssertLessThanOrEqual(lines.count, 25)
        XCTAssertFalse(capture.stderr.contains("Converted file-"))
    }

    func testPublicHelpOmitsDeveloperOptions() {
        let publicCapture = OutputCapture()
        XCTAssertEqual(XDRemuxCommand.run(
            arguments: ["--help", "--language", "en"],
            mode: .production,
            environment: [:],
            preferredLanguages: [],
            isTTY: false,
            stdout: publicCapture.stdoutWriter,
            stderr: publicCapture.stderrWriter
        ), 0)
        XCTAssertFalse(publicCapture.stdout.contains("--family"))
        XCTAssertFalse(publicCapture.stdout.contains("validate-apple"))

        let developerCapture = OutputCapture()
        XCTAssertEqual(XDRemuxCommand.run(
            arguments: ["--help", "--language", "en"],
            mode: .developer,
            environment: [:],
            preferredLanguages: [],
            isTTY: false,
            stdout: developerCapture.stdoutWriter,
            stderr: developerCapture.stderrWriter
        ), 0)
        XCTAssertTrue(developerCapture.stdout.contains("--family"))
        XCTAssertTrue(developerCapture.stdout.contains("validate-apple"))
    }

    func testFailureTemporarilyClearsAndRestoresTTYProgress() {
        let capture = OutputCapture()
        let reporter = makeReporter(capture: capture, isTTY: true)
        let input = URL(fileURLWithPath: "/tmp/a.heic")
        reporter.beginBatch(total: 2, jobs: 1, mode: .standard)
        reporter.batchFileStarted(input)
        reporter.batchFileFinished(
            input: input,
            output: URL(fileURLWithPath: "/tmp/out/a.heic"),
            outcome: .failed(ConversionFailure(
                code: .sourceNotFound,
                userSummaryKey: .errorSourceNotFound,
                recoverySuggestionKey: .recoveryCheckSource,
                diagnostics: "missing"
            ))
        )
        reporter.finish()

        let failureRange = capture.stderr.range(of: "Failed a.heic")
        XCTAssertNotNil(failureRange)
        XCTAssertTrue(capture.stderr.contains("\u{001B}[2K"))
        XCTAssertTrue(capture.stderr.hasSuffix("\u{001B}[?25h"))
    }

    func testDefaultWarningsAreDeduplicatedByFileAndCode() {
        let capture = OutputCapture()
        let reporter = makeReporter(capture: capture, isTTY: false)
        let input = URL(fileURLWithPath: "/tmp/a.heic")
        let output = URL(fileURLWithPath: "/tmp/out.heic")
        reporter.beginSingle(input: input, output: output, mode: .combined, phases: [])
        for diagnostics in ["first helper", "second helper"] {
            reporter.handleSingle(
                .warning(ConversionWarning(
                    code: .privateBridgeFallback,
                    messageKey: .warningPrivateBridgeFallback,
                    diagnostics: diagnostics
                )),
                input: input,
                output: output
            )
        }
        reporter.finish()

        XCTAssertEqual(capture.stderr.components(separatedBy: "Warning:").count - 1, 1)
    }

    func testPlannedPhaseOrderAndApplePhaseCounts() {
        let standard = CLIReporter.plannedPhases(for: ConversionOptions())
        XCTAssertEqual(standard, [
            .readingSource,
            .extractingGainMap,
            .reconstructingHDR,
            .writingContainer,
            .verifyingOutput,
        ])

        var styles = ConversionOptions()
        styles.appleFeatures = AppleFeatureOptions(photographicStyles: true)
        XCTAssertEqual(CLIReporter.plannedPhases(for: styles).count, 6)
        XCTAssertEqual(
            CLIReporter.plannedPhases(for: styles)[3],
            .generatingPhotographicStyles
        )

        var combined = ConversionOptions()
        combined.appleFeatures = AppleFeatureOptions(photographicStyles: true, portrait: true)
        XCTAssertEqual(CLIReporter.plannedPhases(for: combined).count, 7)
        XCTAssertEqual(CLIReporter.plannedPhases(for: combined)[3], .generatingPortraitResources)
        XCTAssertEqual(
            CLIReporter.plannedPhases(for: combined)[4],
            .generatingPhotographicStyles
        )
    }

    func testJSONLFieldNamesAndErrorCodesAreLanguageIndependent() throws {
        let english = runMissingInputJSONL(language: .english)
        let chinese = runMissingInputJSONL(language: .simplifiedChinese)
        let englishRecord = try XCTUnwrap(english.last)
        let chineseRecord = try XCTUnwrap(chinese.last)

        XCTAssertEqual(Set(englishRecord.keys), Set(chineseRecord.keys))
        XCTAssertEqual(englishRecord["event"] as? String, "conversion_failed")
        XCTAssertEqual(chineseRecord["event"] as? String, "conversion_failed")
        XCTAssertEqual(englishRecord["error_code"] as? String, "source_not_found")
        XCTAssertEqual(chineseRecord["error_code"] as? String, "source_not_found")
        XCTAssertNotEqual(englishRecord["message"] as? String, chineseRecord["message"] as? String)
    }

    func testJSONFormatProducesOneValidDocument() throws {
        let capture = OutputCapture()
        let status = XDRemuxCommand.run(
            arguments: [
                "convert", "--input", "/tmp/xdremux-does-not-exist.heic",
                "--format", "json",
            ],
            mode: .production,
            environment: [:],
            preferredLanguages: ["en"],
            isTTY: false,
            stdout: capture.stdoutWriter,
            stderr: capture.stderrWriter
        )

        XCTAssertEqual(status, 3)
        let object = try JSONSerialization.jsonObject(with: Data(capture.stdout.utf8))
        let document = try XCTUnwrap(object as? [String: Any])
        XCTAssertEqual(document["schema_version"] as? Int, 1)
        XCTAssertNotNil(document["events"] as? [[String: Any]])
        XCTAssertTrue(capture.stderr.isEmpty)
    }

    func testExplicitInPlaceOutputIsNotRejectedAsAnExistingOutput() throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-in-place-cli-test-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let input = directory.appendingPathComponent("invalid.heic")
        try Data("not a HEIC".utf8).write(to: input)
        let capture = OutputCapture()

        let status = XDRemuxCommand.run(
            arguments: [
                "convert", "--input", input.path, "--output", input.path,
                "--quiet", "--language", "en",
            ],
            mode: .production,
            environment: [:],
            preferredLanguages: [],
            isTTY: false,
            stdout: capture.stdoutWriter,
            stderr: capture.stderrWriter
        )

        XCTAssertNotEqual(status, 4)
        XCTAssertFalse(capture.stderr.contains("output file could not be written"))
    }

    private func batchOutput(verbosity: OutputVerbosity) -> String {
        let capture = OutputCapture()
        var options = OutputOptions()
        options.verbosity = verbosity
        let reporter = makeReporter(capture: capture, options: options, isTTY: false)
        let input = URL(fileURLWithPath: "/tmp/a.heic")
        reporter.beginBatch(total: 1, jobs: 1, mode: .standard)
        reporter.batchFileStarted(input)
        reporter.batchFileFinished(
            input: input,
            output: URL(fileURLWithPath: "/tmp/out/a.heic"),
            outcome: .converted
        )
        reporter.completeBatch(failureReportURL: nil)
        reporter.finish()
        return capture.stderr
    }

    private func runMissingInputJSONL(language: OutputLanguage) -> [[String: Any]] {
        let capture = OutputCapture()
        _ = XDRemuxCommand.run(
            arguments: [
                "convert", "--input", "/tmp/xdremux-does-not-exist.heic",
                "--format", "jsonl", "--language", language.rawValue,
            ],
            mode: .production,
            environment: [:],
            preferredLanguages: [],
            isTTY: false,
            stdout: capture.stdoutWriter,
            stderr: capture.stderrWriter
        )
        return capture.stdout.split(separator: "\n").compactMap { line in
            (try? JSONSerialization.jsonObject(with: Data(line.utf8))) as? [String: Any]
        }
    }

    private func makeReporter(
        capture: OutputCapture,
        options: OutputOptions = OutputOptions(),
        isTTY: Bool,
        environment: [String: String] = [:]
    ) -> CLIReporter {
        CLIReporter(
            options: options,
            localizer: Localizer(requested: .english, environment: [:], preferredLanguages: []),
            isTTY: isTTY,
            environment: environment,
            stdout: capture.stdoutWriter,
            stderr: capture.stderrWriter
        )
    }
}

final class OutputCapture: @unchecked Sendable {
    private let lock = NSLock()
    private var stdoutStorage = ""
    private var stderrStorage = ""

    lazy var stdoutWriter = OutputWriter { [weak self] text in
        self?.lock.withLock { self?.stdoutStorage += text }
    }
    lazy var stderrWriter = OutputWriter { [weak self] text in
        self?.lock.withLock { self?.stderrStorage += text }
    }

    var stdout: String { lock.withLock { stdoutStorage } }
    var stderr: String { lock.withLock { stderrStorage } }
}

extension NSLocking {
    fileprivate func withLock<T>(_ body: () throws -> T) rethrows -> T {
        lock()
        defer { unlock() }
        return try body()
    }
}
