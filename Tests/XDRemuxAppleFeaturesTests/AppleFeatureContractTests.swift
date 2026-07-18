import Foundation
import XCTest
import XDRemuxCore
@testable import XDRemuxAppleFeatures

final class AppleFeatureContractTests: XCTestCase {
    func testPortraitCoreSelfTestRemainsByteStable() throws {
        let report = try AppleFeatureConversionEngine.portraitSelfTestReport()

        XCTAssertEqual(report["passed"] as? Bool, true)
        XCTAssertEqual(report["byteStableRoundTrip"] as? Bool, true)
        XCTAssertEqual(report["malformedLengthRejected"] as? Bool, true)
        XCTAssertEqual(report["duplicateRecordRejected"] as? Bool, true)
    }

    func testAppleAndOppoModesRemainMutuallyExclusive() {
        let input = URL(fileURLWithPath: "/tmp/missing.heic")
        let configuration = ConversionConfiguration(
            oppoCompatibility: .auto,
            applePhotographicStyles: true
        )
        let request = ConversionRequest(
            input: InputSource(url: input),
            output: OutputDestination(url: input),
            configuration: configuration
        )

        XCTAssertThrowsError(try AppleFeatureConversionEngine.convert(request)) { error in
            XCTAssertEqual(
                String(describing: error),
                "invalid value for --apple-photographic-styles: cannot be combined with OPPO-compatible output"
            )
        }
    }

    func testPreparationEventsDoNotPublishFinalWriterPhases() throws {
        let recorder = AppleEventRecorder()
        let handler = try XCTUnwrap(AppleFeatureEventForwarder.preparationHandler { event in
            if case .phaseChanged(let phase) = event {
                recorder.append(phase)
            }
        })

        for phase in ConversionPhase.allCases {
            handler(.phaseChanged(phase))
        }

        XCTAssertEqual(recorder.phases, [
            .extractingGainMap,
            .reconstructingHDR,
            .generatingPhotographicStyles,
            .generatingPortraitResources,
        ])
    }

    func testPrebuiltHelperRunnerSeparatesProtocolAndDiagnostics() throws {
        let result = try AppleNativeToolchain.run(
            URL(fileURLWithPath: "/bin/sh"),
            arguments: [
                "-c",
                "printf '{\"schema\":\"xdremux-test-helper-v1\",\"event\":\"warning\"}\\n'; printf 'diagnostic\\n' >&2",
            ],
            timeout: 2
        )

        XCTAssertEqual(result.status, 0)
        XCTAssertEqual(
            try JSONSerialization.jsonObject(with: result.stdout) as? [String: String],
            ["schema": "xdremux-test-helper-v1", "event": "warning"]
        )
        XCTAssertEqual(String(data: result.stderr, encoding: .utf8), "diagnostic\n")
    }

    func testPrebuiltHelperRunnerPreservesErrorStatus() throws {
        let result = try AppleNativeToolchain.run(
            URL(fileURLWithPath: "/bin/sh"),
            arguments: ["-c", "printf 'failed\\n' >&2; exit 17"],
            timeout: 2
        )

        XCTAssertEqual(result.status, 17)
        XCTAssertTrue(result.stdout.isEmpty)
        XCTAssertEqual(String(data: result.stderr, encoding: .utf8), "failed\n")
    }

    func testPrebuiltHelperRunnerSupportsCancellation() {
        let cancellation = ConversionCancellation()
        DispatchQueue.global().asyncAfter(deadline: .now() + 0.1) {
            cancellation.cancel()
        }

        XCTAssertThrowsError(try AppleNativeToolchain.run(
            URL(fileURLWithPath: "/bin/sh"),
            arguments: ["-c", "sleep 5"],
            timeout: 10,
            cancellation: cancellation
        )) { error in
            XCTAssertTrue(error is CancellationError)
        }
    }

    func testHelperLocatorUsesPrebuiltProducts() throws {
        let buildDirectory = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
            .appendingPathComponent(".build/debug", isDirectory: true)
        setenv("XDREMUX_HELPER_DIRECTORY", buildDirectory.path, 1)
        defer { unsetenv("XDREMUX_HELPER_DIRECTORY") }

        XCTAssertEqual(
            try AppleNativeToolchain.semanticExecutable().lastPathComponent,
            "XDRemuxSemanticHelper"
        )
        XCTAssertEqual(
            try AppleNativeToolchain.hevcEncoderExecutable().lastPathComponent,
            "XDRemuxHEVCEncoderHelper"
        )
        XCTAssertEqual(
            try AppleNativeToolchain.stylePropertiesProbeExecutable().lastPathComponent,
            "XDRemuxStyleValidationHelper"
        )
    }
}

private final class AppleEventRecorder: @unchecked Sendable {
    private let lock = NSLock()
    private var storage: [ConversionPhase] = []

    func append(_ phase: ConversionPhase) {
        lock.lock()
        storage.append(phase)
        lock.unlock()
    }

    var phases: [ConversionPhase] {
        lock.lock()
        defer { lock.unlock() }
        return storage
    }
}
