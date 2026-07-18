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

    func testSharedSemanticEvidenceRelocatesManifestPaths() throws {
        let fileManager = FileManager.default
        let root = fileManager.temporaryDirectory
            .appendingPathComponent("xdremux-semantic-evidence-\(UUID().uuidString)")
        let source = root.appendingPathComponent("source", isDirectory: true)
        let destination = root.appendingPathComponent("destination", isDirectory: true)
        defer { try? fileManager.removeItem(at: root) }
        try fileManager.createDirectory(at: source, withIntermediateDirectories: true)

        let rawURL = source.appendingPathComponent("portrait.l8")
        let pngURL = source.appendingPathComponent("portrait.png")
        try Data([0, 255]).write(to: rawURL)
        try Data([1, 2, 3]).write(to: pngURL)
        let manifest: [String: Any] = [
            "ok": true,
            "masks": [[
                "name": "portrait",
                "raw_output": rawURL.path,
                "output": pngURL.path,
            ]],
        ]
        try JSONSerialization.data(withJSONObject: manifest).write(
            to: source.appendingPathComponent("manifest.json")
        )

        try AppleSemanticSceneAnalyzer.copyEvidence(from: source, to: destination)

        let copiedManifest = try XCTUnwrap(
            try JSONSerialization.jsonObject(
                with: Data(contentsOf: destination.appendingPathComponent("manifest.json"))
            ) as? [String: Any]
        )
        let rows = try XCTUnwrap(copiedManifest["masks"] as? [[String: Any]])
        let row = try XCTUnwrap(rows.first)
        XCTAssertEqual(row["raw_output"] as? String, destination.appendingPathComponent("portrait.l8").path)
        XCTAssertEqual(row["output"] as? String, destination.appendingPathComponent("portrait.png").path)
        XCTAssertTrue(fileManager.fileExists(atPath: try XCTUnwrap(row["raw_output"] as? String)))
        XCTAssertTrue(fileManager.fileExists(atPath: try XCTUnwrap(row["output"] as? String)))
    }

    func testStyleDistributionPercentilesRemainEquivalent() throws {
        let values = (0..<200).map(Float.init) + [.nan, .infinity]
        let distribution = ApplePhotographicStylesPipeline.distribution(values)

        let expected: [String: Double] = [
            "blackPoint": 0.995,
            "highKey": 189.05,
            "p02": 3.98,
            "p10": 19.9,
            "p25": 49.75,
            "p50": 99.5,
            "p75": 149.25,
            "p98": 195.02,
            "whitePoint": 198.005,
        ]
        for (key, value) in expected {
            XCTAssertEqual(try XCTUnwrap(distribution[key]), value, accuracy: 0.000_001)
        }
    }

    func testSegmentedStyleSamplesPreserveRasterOrderInOnePass() throws {
        func matte(_ pixels: [UInt8]) -> AppleSemanticMatte {
            AppleSemanticMatte(
                pixels: Data(pixels),
                width: 2,
                height: 2,
                bytesPerRow: 2,
                statistics: SemanticStatistics(
                    minimum: pixels.min() ?? 0,
                    maximum: pixels.max() ?? 0,
                    mean: 0,
                    coverage: 0
                ),
                provenance: SemanticProvenance(
                    requestClass: "test",
                    attributeName: "test",
                    revision: 1,
                    inputSHA256: "test",
                    width: 2,
                    height: 2,
                    pixelFormat: "L008",
                    orientation: 1,
                    orientationTransform: "identity",
                    fallback: false
                )
            )
        }
        let tone = (0..<8).map(Float.init)
        let hdr = (100..<108).map(Float.init)
        var rgb: [Float] = []
        for pixel in 0..<8 {
            rgb.append(Float(pixel * 3))
            rgb.append(Float(pixel * 3 + 1))
            rgb.append(Float(pixel * 3 + 2))
        }
        let samples = ApplePhotographicStylesPipeline.selectedStyleSamples(
            toneLuma: tone,
            hdrLuma: hdr,
            toneLinearRGB: rgb,
            width: 4,
            height: 2,
            person: matte([255, 0, 0, 255]),
            skin: matte([0, 255, 255, 0])
        )

        XCTAssertEqual(samples["personTone"], [0, 1, 6, 7])
        XCTAssertEqual(samples["personHDR"], [100, 101, 106, 107])
        XCTAssertEqual(samples["skinTone"], [2, 3, 4, 5])
        XCTAssertEqual(samples["skinHDR"], [102, 103, 104, 105])
        XCTAssertEqual(samples["skinRed"], [6, 9, 12, 15])
        XCTAssertEqual(samples["skinGreen"], [7, 10, 13, 16])
        XCTAssertEqual(samples["skinBlue"], [8, 11, 14, 17])
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
