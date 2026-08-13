import Foundation
import XCTest
@testable import XDRemuxCore

final class PhotoClassificationContractTests: XCTestCase {
    private struct ContractCase: Decodable {
        let name: String
        let userComment: String?
        let assetType: String
        let captureModes: [String]
        let primaryCaptureMode: String?
        let folder: String
        let metadataStatus: String
        let recognizedFlags: UInt64
        let knownUnmappedFlags: UInt64
        let unknownFlags: UInt64
        let tags: [String]

        enum CodingKeys: String, CodingKey {
            case name
            case userComment = "user_comment"
            case assetType = "asset_type"
            case captureModes = "capture_modes"
            case primaryCaptureMode = "primary_capture_mode"
            case folder
            case metadataStatus = "metadata_status"
            case recognizedFlags = "recognized_flags"
            case knownUnmappedFlags = "known_unmapped_flags"
            case unknownFlags = "unknown_flags"
            case tags
        }
    }

    func testCanonicalGoldenContract() throws {
        let fixtureURL = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .appendingPathComponent("fixtures/photo_classification_cases.json")
        let cases = try JSONDecoder().decode([ContractCase].self, from: Data(contentsOf: fixtureURL))

        for item in cases {
            guard let assetType = PhotoAssetType(rawValue: item.assetType) else {
                return XCTFail("unknown asset type in fixture: \(item.assetType)")
            }
            let classification = PhotoCategorizationEngine.classify(
                userComment: item.userComment,
                assetType: assetType
            )
            let contract = PhotoCategorizationEngine.classificationContract(for: classification)
            XCTAssertEqual(contract.assetType, item.assetType, item.name)
            XCTAssertEqual(contract.captureModes, item.captureModes, item.name)
            XCTAssertEqual(contract.primaryCaptureMode, item.primaryCaptureMode, item.name)
            XCTAssertEqual(contract.folder, item.folder, item.name)
            XCTAssertEqual(contract.metadataStatus, item.metadataStatus, item.name)
            XCTAssertEqual(contract.recognizedFlags, item.recognizedFlags, item.name)
            XCTAssertEqual(contract.knownUnmappedFlags, item.knownUnmappedFlags, item.name)
            XCTAssertEqual(contract.unknownFlags, item.unknownFlags, item.name)
            XCTAssertEqual(contract.tags, item.tags, item.name)
        }
    }

    func testMultiBitFlagsAreLosslessButFolderProjectionIsStable() {
        let classification = PhotoCategorizationEngine.classify(userComment: "oplus_18")
        XCTAssertEqual(classification.captureModes, Set([.portrait, .beauty]))
        XCTAssertEqual(classification.mode, .portrait)
        XCTAssertFalse(classification.tags.contains("capture.normal"))
        XCTAssertTrue(classification.tags.contains("capture.portrait"))
        XCTAssertTrue(classification.tags.contains("capture.beauty"))
    }

    func testUnknownFlagsAreIndependentFromMetadataReadStatus() {
        let known = PhotoCategorizationEngine.classify(userComment: "oplus_262144")
        XCTAssertEqual(known.knownUnmappedFlags, 262144)
        XCTAssertEqual(known.unknownFlags, 0)
        XCTAssertEqual(known.metadataStatus, .ok)
        XCTAssertEqual(known.mode, .normal)

        let unknown = PhotoCategorizationEngine.classify(userComment: "oplus_17179869184")
        XCTAssertEqual(unknown.knownUnmappedFlags, 0)
        XCTAssertEqual(unknown.unknownFlags, 17179869184)
        XCTAssertEqual(unknown.metadataStatus, .ok)
        XCTAssertNil(unknown.mode)
        XCTAssertEqual(unknown.status, .unknownFlags)
    }

    func testFolderProjectionSeparatesAssetTypeFromCaptureTags() {
        let classification = PhotoCategorizationEngine.classify(userComment: "oplus_18")
        XCTAssertEqual(
            PhotoFolderProjection.relativeDirectory(for: classification),
            "静态照片/人像"
        )
        XCTAssertEqual(
            PhotoFolderProjection.relativeDirectory(for: classification, assetType: .livePhoto),
            "实况照片/人像"
        )
        XCTAssertEqual(PhotoFolderProjection.layoutVersion, "asset-type-v1")
    }

    func testPhotoAssetKeepsLivePhotoResourcesTogether() {
        let image = URL(fileURLWithPath: "/tmp/IMG.heic")
        let video = URL(fileURLWithPath: "/tmp/IMG.mov")
        let asset = PhotoAsset.livePhoto(imageURL: image, videoURL: video, id: "asset-id")
        XCTAssertEqual(asset.type, .livePhoto)
        XCTAssertEqual(asset.primaryImageURL, image)
        XCTAssertEqual(asset.resources.map(\.role), [.primaryImage, .pairedVideo])
    }

    func testCapabilitiesRequireCompleteManifestEntryNames() throws {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-classification-capabilities-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }

        let complete = root.appendingPathComponent("complete.heic")
        let completePayload = "oplus_18 {\"name\":\"local.uhdr.gainmap.data\"} "
            + "{\"name\":\"rear.depth\"} {\"name\":\"rear.depth.config\"}"
        try Data(completePayload.utf8).write(to: complete)
        let classification = PhotoCategorizationEngine.classify(at: complete, assetType: .staticPhoto)
        XCTAssertEqual(classification.capabilities, Set([.proXDR, .gainMap, .hdr, .depth]))

        let configOnly = root.appendingPathComponent("config-only.heic")
        try Data("oplus_18 {\"name\":\"rear.depth.config\"}".utf8).write(to: configOnly)
        let configClassification = PhotoCategorizationEngine.classify(
            at: configOnly,
            assetType: .staticPhoto
        )
        XCTAssertFalse(configClassification.capabilities.contains(.depth))
    }
}
