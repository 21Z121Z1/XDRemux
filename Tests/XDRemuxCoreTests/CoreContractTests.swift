import Foundation
import XCTest
@testable import XDRemuxCore

final class CoreContractTests: XCTestCase {
    func testConversionDefaultsMatchLegacyCLI() {
        let configuration = ConversionConfiguration()

        XCTAssertEqual(configuration.family, .auto)
        XCTAssertEqual(configuration.oppoCompatibility, .off)
        XCTAssertEqual(configuration.inputProcessingBranch, .hybrid)
        XCTAssertEqual(configuration.oppoCameraTail, .preserveWithoutPrivateHDR)
        XCTAssertEqual(configuration.tmapFormat, .imageIO)
        XCTAssertEqual(configuration.fileNameSuffix, "_iso")
        XCTAssertTrue(configuration.skipExisting)
        XCTAssertFalse(configuration.appleFeaturesEnabled)
    }

    func testOutputTargetResolvesReplacementAndExplicitFile() {
        let source = InputSource(url: URL(fileURLWithPath: "/tmp/input.heic"))
        let explicit = URL(fileURLWithPath: "/tmp/output.heic")

        XCTAssertEqual(OutputTarget.replaceInput.destination(for: source).url, source.url)
        XCTAssertEqual(OutputTarget.file(explicit).destination(for: source).url, explicit)
    }

    func testISOBMFFBoxParserAcceptsValidBoxAndRejectsTruncation() {
        let valid = makeBox("free", payload: Data([1, 2, 3, 4]))
        let boxes = isobmffBoxes(in: valid, start: 0, end: valid.count)

        XCTAssertEqual(boxes.count, 1)
        XCTAssertEqual(boxes[0].type, "free")
        XCTAssertEqual(boxes[0].size, 12)
        XCTAssertEqual(boxes[0].dataStart, 8)
        XCTAssertEqual(boxes[0].dataEnd, 12)

        var oversized = Data([0, 0, 0, 20])
        oversized.append(Data("free".utf8))
        XCTAssertTrue(isobmffBoxes(in: oversized, start: 0, end: oversized.count).isEmpty)
        XCTAssertTrue(isobmffBoxes(in: valid, start: valid.count - 4, end: valid.count).isEmpty)
    }

    func testScratchNameIsBoundedAndSiblingScoped() {
        let output = URL(fileURLWithPath: "/tmp/" + String(repeating: "x", count: 240) + ".heic")
        let scratch = siblingScratchURL(for: output, label: "portrait-private", pathExtension: "heic")

        XCTAssertEqual(scratch.deletingLastPathComponent(), output.deletingLastPathComponent())
        XCTAssertLessThan(scratch.lastPathComponent.utf8.count, 255)
    }

    func testInvalidIntermediateDoesNotCreateOutput() throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-core-test-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let input = directory.appendingPathComponent("invalid.heic")
        let output = directory.appendingPathComponent("output.heic")
        try Data("not a HEIC".utf8).write(to: input, options: .atomic)

        XCTAssertThrowsError(
            try ISOHDRWriter.writeWithPreserveReencode(
                intermediateURL: input,
                outputURL: output
            )
        )
        XCTAssertFalse(FileManager.default.fileExists(atPath: output.path))
    }

    func testLegacyErrorTextIsStable() {
        let url = URL(fileURLWithPath: "/tmp/input.heic")
        XCTAssertEqual(
            XDRemuxError.inputNotFound(url).description,
            "input not found: /tmp/input.heic"
        )
        XCTAssertEqual(
            XDRemuxError.invalidValue(option: "--jobs", value: "0").description,
            "invalid value for --jobs: 0"
        )
    }

    func testMissingInputEmitsStructuredEventsInOrder() {
        let recorder = EventRecorder()
        let input = URL(fileURLWithPath: "/tmp/xdremux-core-does-not-exist.heic")
        let output = URL(fileURLWithPath: "/tmp/xdremux-core-output.heic")
        let configuration = ConversionConfiguration(eventHandler: { event in
            switch event {
            case .started: recorder.append("started")
            case .phaseChanged(let phase): recorder.append(phase.rawValue)
            case .failed(let failure): recorder.append("failed:\(failure.code.rawValue)")
            default: break
            }
        })

        XCTAssertThrowsError(try ConversionEngine.convert(
            inputURL: input,
            outputURL: output,
            config: configuration
        ))
        XCTAssertEqual(recorder.values, ["started", "reading_source", "failed:source_not_found"])
    }

    func testFailureCodesRemainStableEnglishIdentifiers() {
        XCTAssertEqual(FailureCode.sourceNotFound.rawValue, "source_not_found")
        XCTAssertEqual(FailureCode.sourceGainMapMissing.rawValue, "source_gain_map_missing")
        XCTAssertEqual(FailureCode.outputVerificationFailed.rawValue, "output_verification_failed")
        XCTAssertEqual(FailureCode.internalContainerError.rawValue, "internal_container_error")
        XCTAssertEqual(WarningCode.portraitUnavailable.rawValue, "portrait_unavailable")
    }

    func testCancellationDoesNotEmitFailureEvent() {
        let recorder = EventRecorder()
        let cancellation = ConversionCancellation()
        cancellation.cancel()
        let input = URL(fileURLWithPath: "/tmp/xdremux-cancelled-input.heic")
        let configuration = ConversionConfiguration(
            eventHandler: { event in
                switch event {
                case .started: recorder.append("started")
                case .failed: recorder.append("failed")
                default: break
                }
            },
            cancellation: cancellation
        )

        XCTAssertThrowsError(try ConversionEngine.convert(
            inputURL: input,
            outputURL: input,
            config: configuration
        )) { error in
            XCTAssertTrue(error is CancellationError)
        }
        XCTAssertEqual(recorder.values, ["started"])
    }
}

private final class EventRecorder: @unchecked Sendable {
    private let lock = NSLock()
    private var storage: [String] = []

    func append(_ value: String) {
        lock.lock()
        storage.append(value)
        lock.unlock()
    }

    var values: [String] {
        lock.lock()
        defer { lock.unlock() }
        return storage
    }
}
