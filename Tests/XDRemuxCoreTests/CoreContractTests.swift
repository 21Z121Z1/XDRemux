import Foundation
import CoreImage
import XCTest
@testable import XDRemuxCore

final class CoreContractTests: XCTestCase {
    func testOppoCaptureModeContractMatrix() throws {
        struct ContractCase: Decodable {
            let userComment: String?
            let mode: String?
            let folder: String?
            let status: String

            enum CodingKeys: String, CodingKey {
                case userComment = "user_comment"
                case mode, folder, status
            }
        }
        let fixtureURL = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .appendingPathComponent("fixtures/oppo_capture_mode_cases.json")
        let cases = try JSONDecoder().decode([ContractCase].self, from: Data(contentsOf: fixtureURL))
        for item in cases {
            let classification = PhotoCategorizationEngine.classify(userComment: item.userComment)
            XCTAssertEqual(classification.mode?.rawValue, item.mode, "comment: \(item.userComment ?? "nil")")
            XCTAssertEqual(classification.mode?.folderName, item.folder)
            XCTAssertEqual(classification.status.rawValue, item.status)
        }
    }

    func testPhotoCategorizationPlansFoldersRootAndStableDuplicates() throws {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-categorize-\(UUID().uuidString)", isDirectory: true)
        let input = root.appendingPathComponent("input", isDirectory: true)
        let nested = input.appendingPathComponent("nested", isDirectory: true)
        let output = input.appendingPathComponent("categorized", isDirectory: true)
        try FileManager.default.createDirectory(at: nested, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }

        let portrait = input.appendingPathComponent("same.heic")
        let secondPortrait = nested.appendingPathComponent("same.heic")
        let unclassified = input.appendingPathComponent("plain.jpg")
        try Data("header-oplus_18-tail".utf8).write(to: portrait)
        try Data("different-oplus_18-tail".utf8).write(to: secondPortrait)
        try Data("no user comment".utf8).write(to: unclassified)

        let plan = try PhotoCategorizationEngine.makePlan(inputs: [input], outputDirectory: output)
        XCTAssertEqual(plan.items.count, 3)
        XCTAssertEqual(plan.items[0].destinationURL.deletingLastPathComponent().lastPathComponent, "人像")
        XCTAssertEqual(plan.items[1].destinationURL.lastPathComponent, "plain.jpg")
        let unclassifiedDirectory = output
            .appendingPathComponent("静态照片", isDirectory: true)
            .appendingPathComponent("未分类", isDirectory: true)
        XCTAssertEqual(
            plan.items[1].destinationURL.deletingLastPathComponent().standardizedFileURL.path,
            unclassifiedDirectory.standardizedFileURL.path
        )
        XCTAssertEqual(plan.items[2].destinationURL.lastPathComponent, "same (2).heic")

        let result = PhotoCategorizationEngine.execute(plan, jobs: 2)
        XCTAssertEqual(result.copiedCount, 3)
        let repeated = try PhotoCategorizationEngine.makePlan(inputs: [input], outputDirectory: output)
        XCTAssertEqual(repeated.items.count, 3)
        XCTAssertTrue(repeated.items.allSatisfy { $0.disposition == .duplicate })
    }

    func testPhotoCategorizationKeepsValidatedLivePhotoResourcesTogether() throws {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-categorize-live-pair-\(UUID().uuidString)", isDirectory: true)
        let input = root.appendingPathComponent("input", isDirectory: true)
        let output = root.appendingPathComponent("output", isDirectory: true)
        try FileManager.default.createDirectory(at: input, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }

        let image = input.appendingPathComponent("pair.heic")
        let video = input.appendingPathComponent("pair.mov")
        try Data("oplus_18".utf8).write(to: image)
        try Data("paired-video".utf8).write(to: video)

        let occupied = output
            .appendingPathComponent("实况照片", isDirectory: true)
            .appendingPathComponent("人像", isDirectory: true)
        try FileManager.default.createDirectory(at: occupied, withIntermediateDirectories: true)
        try Data("foreign-image".utf8).write(to: occupied.appendingPathComponent("pair.heic"))

        let paired = try PhotoCategorizationEngine.makePlan(
            inputs: [input],
            outputDirectory: output,
            livePhotoPairValidator: { candidateImage, candidateVideo in
                candidateImage.lastPathComponent == "pair.heic"
                    && candidateVideo.lastPathComponent == "pair.mov"
            }
        )
        XCTAssertEqual(paired.items.count, 2)
        XCTAssertTrue(paired.items.allSatisfy { $0.classification.assetType == .livePhoto })
        XCTAssertEqual(
            Set(paired.items.map(\.destinationURL.lastPathComponent)),
            Set(["pair (2).heic", "pair (2).mov"])
        )
        XCTAssertTrue(paired.items.allSatisfy {
            $0.destinationURL.path.contains("实况照片/人像/")
        })

        let rejected = try PhotoCategorizationEngine.makePlan(
            inputs: [input],
            outputDirectory: output,
            livePhotoPairValidator: { _, _ in false }
        )
        XCTAssertEqual(rejected.items.count, 1)
        XCTAssertEqual(
            rejected.items[0].sourceURL.resolvingSymlinksInPath().standardizedFileURL.path,
            image.resolvingSymlinksInPath().standardizedFileURL.path
        )
        XCTAssertEqual(rejected.items[0].classification.assetType, .staticPhoto)
    }

    func testPhotoCategorizationReadsTIFFUserCommentBytes() throws {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-categorize-tiff-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let source = root.appendingPathComponent("comment.jpg")
        try makeTIFFUserComment("Oplus_4096").write(to: source)

        let classification = PhotoCategorizationEngine.classify(at: source)
        XCTAssertEqual(classification.mode, .enhancedText)
        XCTAssertEqual(classification.status, .categorized)
    }

    func testCategorizationResultCountsMalformedCommentsAsIssuesAfterCopy() {
        let source = URL(fileURLWithPath: "/tmp/malformed.jpg")
        let classification = PhotoCategorizationEngine.classify(userComment: "not-an-oppo-comment")
        let item = PhotoCategorizationItem(
            sourceURL: source,
            destinationURL: URL(fileURLWithPath: "/tmp/output/malformed.jpg"),
            classification: classification,
            disposition: .copied
        )
        let result = PhotoCategorizationResult(items: [item])

        XCTAssertEqual(result.rootCount, 1)
        XCTAssertEqual(result.copiedCount, 1)
        XCTAssertEqual(result.issueCount, 1)
    }

    func testSourceDirectoryCategorizationExcludesCreatedModeTrees() throws {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-categorize-source-root-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let source = root.appendingPathComponent("portrait.heic")
        try Data("oplus_18".utf8).write(to: source)

        let initial = try PhotoCategorizationEngine.makePlan(inputs: [root], outputDirectory: nil)
        XCTAssertEqual(initial.items.first?.destinationURL.lastPathComponent, "portrait.heic")
        XCTAssertEqual(
            initial.items.first?.destinationURL.deletingLastPathComponent().lastPathComponent,
            "人像"
        )
        XCTAssertEqual(PhotoCategorizationEngine.execute(initial).copiedCount, 1)

        let repeated = try PhotoCategorizationEngine.makePlan(inputs: [root], outputDirectory: nil)
        XCTAssertEqual(repeated.items.count, 1)
        XCTAssertEqual(repeated.items.first?.disposition, .duplicate)
        XCTAssertFalse(repeated.items.contains { $0.destinationURL.path.contains("人像/人像") })
    }
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

    private func makeTIFFUserComment(_ comment: String) -> Data {
        let payload = Data("ASCII\0\0\0\(comment)".utf8)
        var data = Data([0x49, 0x49, 0x2a, 0x00, 0x08, 0x00, 0x00, 0x00])
        data.append(contentsOf: [0x01, 0x00])
        data.append(contentsOf: [0x69, 0x87, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0x1a, 0x00, 0x00, 0x00])
        data.append(contentsOf: [0x00, 0x00, 0x00, 0x00])
        data.append(contentsOf: [0x01, 0x00])
        data.append(contentsOf: [0x86, 0x92, 0x07, 0x00])
        let count = UInt32(payload.count)
        data.append(contentsOf: [
            UInt8(count & 0xff), UInt8((count >> 8) & 0xff),
            UInt8((count >> 16) & 0xff), UInt8((count >> 24) & 0xff),
            0x2c, 0x00, 0x00, 0x00,
        ])
        data.append(contentsOf: [0x00, 0x00, 0x00, 0x00])
        data.append(payload)
        return data
    }
}
