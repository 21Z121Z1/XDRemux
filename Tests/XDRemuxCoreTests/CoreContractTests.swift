import Foundation
import CoreImage
import XCTest
@testable import XDRemuxCore

final class CoreContractTests: XCTestCase {
    func testPhotographicStylesResolveUnspecifiedProducerToProductionSolver() {
        XCTAssertEqual(
            AppleStyleDataProducerMode.unspecified.resolvedForPhotographicStyles,
            .constrainedSolver
        )
        XCTAssertEqual(
            AppleStyleDataProducerMode.identityFallback.resolvedForPhotographicStyles,
            .identityFallback
        )
    }

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
        XCTAssertEqual(configuration.appleStyleDataProducer, .unspecified)
        XCTAssertNil(configuration.appleStylesRawDNGURL)
    }

    func testCoreImageRAWOrientationReorderKeepsStableDimensions() throws {
        var source = Data(count: 2 * 1 * 8)
        source.withUnsafeMutableBytes { raw in
            let values = raw.bindMemory(to: UInt16.self)
            values[0] = 1; values[1] = 2; values[2] = 3; values[3] = 4
            values[4] = 5; values[5] = 6; values[6] = 7; values[7] = 8
        }
        let oriented = try CoreImageRAW.orientedRGBA16(source, width: 2, height: 1, orientation: 6)
        XCTAssertEqual(oriented.width, 1)
        XCTAssertEqual(oriented.height, 2)
        XCTAssertEqual(oriented.data.count, source.count)
    }

    func testCoreImageRAWInvalidDNGFailsClosedAndCacheIdentityBindsInputs() throws {
        let invalidURL = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-invalid-raw-\(UUID().uuidString).dng")
        defer { try? FileManager.default.removeItem(at: invalidURL) }
        try Data("not a DNG".utf8).write(to: invalidURL, options: .atomic)
        XCTAssertThrowsError(
            try CoreImageRAW.decode(dngURL: invalidURL, targetWidth: 8, targetHeight: 8)
        )

        let first = CoreImageRAW.cacheKey(dngSHA256: "dng-a", embeddedPreviewSHA256: "preview-a")
        XCTAssertNotEqual(
            first,
            CoreImageRAW.cacheKey(dngSHA256: "dng-b", embeddedPreviewSHA256: "preview-a")
        )
        XCTAssertNotEqual(
            first,
            CoreImageRAW.cacheKey(dngSHA256: "dng-a", embeddedPreviewSHA256: "preview-b")
        )
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

    func testAppleTmapPayloadUsesImageIOCanonicalRationals() {
        let ratio = 4.926108360290527
        let floats = [
            1.0, 1.0, 1.0, 1.0,
            ratio, ratio, ratio,
            1.0, 1.0, 1.0,
            0.0, 0.0, 0.0,
            0.0, 0.0, 0.0,
            1.0, ratio, ratio, 0.0,
        ]
        XCTAssertEqual(
            makeAppleTmapPayload(infoFloats: floats).map { String(format: "%02x", $0) }.joined(),
            "000000000040000000000000000100933a9300400000000000000000000100933a9300400000000000010000000100000000000000010000000000000001"
        )
    }

    func testEncodingQualityPolicyAcceptsOnlyFiniteUnitIntervalValues() {
        XCTAssertEqual(
            EncodingQualityPolicy.value(
                environmentKey: "QUALITY",
                defaultValue: 0.8,
                environment: ["QUALITY": "0.925"]
            ),
            0.925
        )
        for invalid in ["-0.1", "0", "1.1", "nan", "inf", "not-a-number"] {
            XCTAssertEqual(
                EncodingQualityPolicy.value(
                    environmentKey: "QUALITY",
                    defaultValue: 0.8,
                    environment: ["QUALITY": invalid]
                ),
                0.8
            )
        }
    }

    func testEncodingQualityPolicyAcceptsOnlyAllowedIntegerValues() {
        XCTAssertEqual(
            EncodingQualityPolicy.integer(
                environmentKey: "TILE",
                defaultValue: 512,
                allowedValues: [256, 512, 1024],
                environment: ["TILE": "1024"]
            ),
            1024
        )
        for invalid in ["128", "768", "not-a-number"] {
            XCTAssertEqual(
                EncodingQualityPolicy.integer(
                    environmentKey: "TILE",
                    defaultValue: 512,
                    allowedValues: [256, 512, 1024],
                    environment: ["TILE": invalid]
                ),
                512
            )
        }
    }

    func testTmapGeometryUsesDisplayDimensionsForQuarterTurnOrientation() throws {
        let primary = makeIspeBox(width: 4080, height: 3064)
        let quarterTurn = Data([0, 0, 0, 9, 0x69, 0x72, 0x6f, 0x74, 1])
        XCTAssertEqual(
            try makeImageIOCanonicalTmapIspeBox(primaryIspe: primary, irot: quarterTurn),
            makeIspeBox(width: 3064, height: 4080)
        )
        let upright = Data([0, 0, 0, 9, 0x69, 0x72, 0x6f, 0x74, 0])
        XCTAssertEqual(
            try makeImageIOCanonicalTmapIspeBox(primaryIspe: primary, irot: upright),
            primary
        )
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
}
