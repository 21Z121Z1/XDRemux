import Foundation
@preconcurrency import AVFoundation
import CoreMedia
import CryptoKit
import ImageIO
import XCTest
import XDRemuxCore
@testable import XDRemuxAppleFeatures

/// Production gate for the real samples supplied for the Motion Photo implementation review.
///
/// The test is intentionally skipped when no private fixture root is configured. Once
/// `XDREMUX_MOTION_PHOTO_FIXTURE_ROOT` is present, every named fixture is mandatory and the gate
/// performs both characterization and full HEIC+MOV -> PHLivePhoto validation on macOS.
final class UploadedMotionPhotoFixtureGateTests: XCTestCase {
    private struct FixtureSpec: Sendable {
        let filename: String
        let sha256: String
        let sourceKind: MotionPhotoSourceKind
        let stillEnd: Int64
        let videoStart: Int64
        let videoEnd: Int64
        let presentationTimestampUs: Int64
        let streamCount: Int
        let primaryVideoEnd: Int64?
        let expectsGainMap: Bool
    }

    private let fixtures: [FixtureSpec] = [
        // OPPO ColorOS 15 — one embedded HEVC Motion Photo stream, Ultra HDR still.
        .init(
            filename: "motion-photo/oppo/coloros15-ultrahdr-01.jpg",
            sha256: "83a4f9f3c978f541e1255bff3bd89cffe0da182aef5558c1d9d081c41f4cdb01",
            sourceKind: .oppoLivePhoto,
            stillEnd: 5_212_915,
            videoStart: 5_212_915,
            videoEnd: 15_165_684,
            presentationTimestampUs: 1_469_600,
            streamCount: 1,
            primaryVideoEnd: nil,
            expectsGainMap: true
        ),
        .init(
            filename: "motion-photo/oppo/coloros15-ultrahdr-02.jpg",
            sha256: "3f5cc79c1cf26f18acf22522964e7b8e009bf35b36c4c509d7618b1fd7cd6707",
            sourceKind: .oppoLivePhoto,
            stillEnd: 4_610_334,
            videoStart: 4_610_334,
            videoEnd: 13_359_471,
            presentationTimestampUs: 1_433_190,
            streamCount: 1,
            primaryVideoEnd: nil,
            expectsGainMap: true
        ),
        .init(
            filename: "motion-photo/oppo/coloros15-ultrahdr-03.jpg",
            sha256: "20afbcfb3f6fbcd7ea7b2ca306b8208dbfd10eaeb7a9fb91cf86a5a9b21c3920",
            sourceKind: .oppoLivePhoto,
            stillEnd: 19_365_654,
            videoStart: 19_365_654,
            videoEnd: 30_680_658,
            presentationTimestampUs: 1_666_600,
            streamCount: 1,
            primaryVideoEnd: nil,
            expectsGainMap: true
        ),

        // OPPO ColorOS 16 — the Android MotionPhoto resource contains two concatenated BMFF
        // streams. Stream 1 is the Apple paired video; Stream 2 is vendor preview data.
        .init(
            filename: "motion-photo/oppo/coloros16-dualstream-ultrahdr-01.jpg",
            sha256: "5b555b0fffcec9ffb64a082a0532822431b59fc0490b677cc557e9810b764e70",
            sourceKind: .oppoLivePhoto,
            stillEnd: 6_809_684,
            videoStart: 6_809_684,
            videoEnd: 24_929_781,
            presentationTimestampUs: 1_533_287,
            streamCount: 2,
            primaryVideoEnd: 23_211_122,
            expectsGainMap: true
        ),
        .init(
            filename: "motion-photo/oppo/coloros16-dualstream-ultrahdr-02.jpg",
            sha256: "15c19972c3328da9c4bfb8ad9134f92764c6c51827853f8118d5d2d986e967ff",
            sourceKind: .oppoLivePhoto,
            stillEnd: 13_591_436,
            videoStart: 13_591_436,
            videoEnd: 29_199_130,
            presentationTimestampUs: 1_298_732,
            streamCount: 2,
            primaryVideoEnd: 27_234_826,
            expectsGainMap: true
        ),

        // Xiaomi Android Motion Photo V1, Ultra HDR JPEG static resource.
        .init(
            filename: "motion-photo/xiaomi/android-v1-ultrahdr-01.jpg",
            sha256: "18f5d5b9243dec290626b446f6812d7bf41399bdc66d7feb794e562a9ffca4dc",
            sourceKind: .androidMotionPhotoV1,
            stillEnd: 9_541_876,
            videoStart: 9_541_876,
            videoEnd: 10_550_148,
            presentationTimestampUs: 430_574,
            streamCount: 1,
            primaryVideoEnd: nil,
            expectsGainMap: true
        ),

        // Samsung JPEG Motion Photo V1. Samsung keeps another BMFF-looking vendor region inside
        // the static resource; the semantic Container directory selects the later true video start.
        .init(
            filename: "motion-photo/samsung/jpeg-ultrahdr-01.jpg",
            sha256: "d95c3bfe772d681c3b7b4c33ab39f6a9da46517b3e88209fe263843dfa49cfa4",
            sourceKind: .androidMotionPhotoV1,
            stillEnd: 2_689_001,
            videoStart: 2_689_001,
            videoEnd: 6_842_570,
            presentationTimestampUs: 1_573_888,
            streamCount: 1,
            primaryVideoEnd: nil,
            expectsGainMap: true
        ),
        .init(
            filename: "motion-photo/samsung/jpeg-ultrahdr-02.jpg",
            sha256: "c9e97669689fcc975f3d511cc15274b047c6b340d12c434fd04ceaa249bfee9b",
            sourceKind: .androidMotionPhotoV1,
            stillEnd: 2_690_459,
            videoStart: 2_690_459,
            videoEnd: 3_752_096,
            presentationTimestampUs: 1_585_246,
            streamCount: 1,
            primaryVideoEnd: nil,
            expectsGainMap: true
        ),

        // Samsung HEIF Motion Photo V1. The video is the mpvd payload only; trailing sefd is vendor
        // data and must never be copied into the Apple paired MOV. R002/R003 are byte-identical
        // duplicates supplied in the same archive and are retained in the gate deliberately.
        .init(
            filename: "motion-photo/samsung/heif-ultrahdr-01.heic",
            sha256: "06eb244bc69ae464bd7b0a60b769f4fc3429dc543451481f5331586a7536b8d0",
            sourceKind: .androidHeifMotionPhotoV1,
            stillEnd: 1_232_154,
            videoStart: 1_232_162,
            videoEnd: 5_181_667,
            presentationTimestampUs: 1_540_401,
            streamCount: 1,
            primaryVideoEnd: nil,
            expectsGainMap: true
        ),
        .init(
            filename: "motion-photo/samsung/heif-ultrahdr-01-duplicate-r002.heic",
            sha256: "06eb244bc69ae464bd7b0a60b769f4fc3429dc543451481f5331586a7536b8d0",
            sourceKind: .androidHeifMotionPhotoV1,
            stillEnd: 1_232_154,
            videoStart: 1_232_162,
            videoEnd: 5_181_667,
            presentationTimestampUs: 1_540_401,
            streamCount: 1,
            primaryVideoEnd: nil,
            expectsGainMap: true
        ),
        .init(
            filename: "motion-photo/samsung/heif-ultrahdr-02.heic",
            sha256: "d33f502276f0d8e8a0f49c9f5674ed1728812f7432f355a5a3325007fc780f1f",
            sourceKind: .androidHeifMotionPhotoV1,
            stillEnd: 1_217_171,
            videoStart: 1_217_179,
            videoEnd: 5_586_957,
            presentationTimestampUs: 2_518_658,
            streamCount: 1,
            primaryVideoEnd: nil,
            expectsGainMap: true
        ),
        .init(
            filename: "motion-photo/samsung/heif-ultrahdr-02-duplicate-r003.heic",
            sha256: "d33f502276f0d8e8a0f49c9f5674ed1728812f7432f355a5a3325007fc780f1f",
            sourceKind: .androidHeifMotionPhotoV1,
            stillEnd: 1_217_171,
            videoStart: 1_217_179,
            videoEnd: 5_586_957,
            presentationTimestampUs: 2_518_658,
            streamCount: 1,
            primaryVideoEnd: nil,
            expectsGainMap: true
        ),

        // vivo standard Motion Photo V1. These two supplied samples are SDR still resources rather
        // than Ultra HDR; they still exercise HEVC + AAC passthrough and generic vendor neutrality.
        .init(
            filename: "motion-photo/vivo/android-v1-sdr-01.jpg",
            sha256: "f71104787d3ce236e5543a71cfc50f8208fd9acbaeef057178350dfbacecd277",
            sourceKind: .androidMotionPhotoV1,
            stillEnd: 3_307_962,
            videoStart: 3_307_962,
            videoEnd: 6_031_584,
            presentationTimestampUs: 1_333_944,
            streamCount: 1,
            primaryVideoEnd: nil,
            expectsGainMap: false
        ),
        .init(
            filename: "motion-photo/vivo/android-v1-sdr-02.jpg",
            sha256: "7a00f4a63b51abfde5d1a93bc08053b3f4f28222b2234212da030ab8ed12d321",
            sourceKind: .androidMotionPhotoV1,
            stillEnd: 3_036_474,
            videoStart: 3_036_474,
            videoEnd: 9_638_904,
            presentationTimestampUs: 838_055,
            streamCount: 1,
            primaryVideoEnd: nil,
            expectsGainMap: false
        ),
    ]

    func testAllSuppliedFixturesCharacterizeAndLoadAsAppleLivePhotos() async throws {
        let root = try fixtureRoot()
        let index = try fixtureIndex(root: root)
        let outputRoot = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-uploaded-motion-gate-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: outputRoot, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: outputRoot) }

        for spec in fixtures {
            guard let source = index[spec.filename] else {
                XCTFail("required private Motion Photo fixture is missing: \(spec.filename)")
                continue
            }
            let beforeHash = try sha256(source)
            XCTAssertEqual(beforeHash, spec.sha256, "fixture identity changed: \(spec.filename)")

            let asset = try XCTUnwrap(
                OppoMotionPhotoParser.parse(url: source),
                "parser rejected \(spec.filename)"
            )
            XCTAssertEqual(asset.sourceKind, spec.sourceKind, spec.filename)
            XCTAssertEqual(asset.stillResourceRange.lowerBound, 0, spec.filename)
            XCTAssertEqual(asset.stillResourceRange.upperBound, spec.stillEnd, spec.filename)
            XCTAssertEqual(asset.videoResourceRange.lowerBound, spec.videoStart, spec.filename)
            XCTAssertEqual(asset.videoResourceRange.upperBound, spec.videoEnd, spec.filename)
            XCTAssertEqual(asset.presentationTimestampUs, spec.presentationTimestampUs, spec.filename)
            XCTAssertEqual(asset.vendorMetadata?.streamCount ?? 1, spec.streamCount, spec.filename)

            let primaryVideo = try OppoMotionPhotoStreamResolver.primaryVideoRange(for: asset)
            XCTAssertEqual(primaryVideo.lowerBound, spec.videoStart, spec.filename)
            XCTAssertEqual(
                primaryVideo.upperBound,
                spec.primaryVideoEnd ?? spec.videoEnd,
                spec.filename
            )

            let output = outputRoot
                .appendingPathComponent(sanitizedStem(spec.filename))
                .appendingPathExtension("heic")
            let result = try await AppleLivePhotoConversionEngine.convertAsync(
                inputURL: source,
                outputImageURL: output,
                requirePhotoKitValidation: true
            )

            XCTAssertEqual(result.sourceKind, spec.sourceKind, spec.filename)
            XCTAssertTrue(FileManager.default.fileExists(atPath: result.imageURL.path), spec.filename)
            XCTAssertTrue(FileManager.default.fileExists(atPath: result.videoURL.path), spec.filename)
            XCTAssertEqual(
                AppleLivePhotoStillWriter.hasGainMap(result.imageURL),
                spec.expectsGainMap,
                "gain-map state changed: \(spec.filename)"
            )
            XCTAssertTrue(
                AppleLivePhotoValidator.isValidPair(
                    imageURL: result.imageURL,
                    videoURL: result.videoURL
                ),
                "structural Live Photo validation failed: \(spec.filename)"
            )
            XCTAssertFalse(
                outputContainsStaleMotionPhotoXMP(result.imageURL),
                "stale Android Motion Photo XMP survived conversion: \(spec.filename)"
            )

            let afterHash = try sha256(source)
            XCTAssertEqual(afterHash, beforeHash, "conversion modified source fixture: \(spec.filename)")
        }
    }

    private func fixtureRoot() throws -> URL {
        guard let path = ProcessInfo.processInfo.environment["XDREMUX_MOTION_PHOTO_FIXTURE_ROOT"],
              !path.isEmpty else {
            throw XCTSkip("XDREMUX_MOTION_PHOTO_FIXTURE_ROOT is not configured")
        }
        let root = URL(fileURLWithPath: path).standardizedFileURL
        var isDirectory: ObjCBool = false
        guard FileManager.default.fileExists(atPath: root.path, isDirectory: &isDirectory),
              isDirectory.boolValue else {
            XCTFail("XDREMUX_MOTION_PHOTO_FIXTURE_ROOT is not a readable directory")
            throw AppleLivePhotoError.pairValidationFailed("private fixture root is invalid")
        }
        return root
    }

    private func fixtureIndex(root: URL) throws -> [String: URL] {
        let expectedNames = Set(fixtures.map(\.filename))
        guard let enumerator = FileManager.default.enumerator(
            at: root,
            includingPropertiesForKeys: [.isRegularFileKey],
            options: [.skipsHiddenFiles]
        ) else {
            return [:]
        }
        var result: [String: URL] = [:]
        for case let url as URL in enumerator where expectedNames.contains(url.lastPathComponent) {
            if (try? url.resourceValues(forKeys: [.isRegularFileKey]).isRegularFile) == true {
                result[url.lastPathComponent] = url
            }
        }
        return result
    }

    private func sha256(_ url: URL) throws -> String {
        let handle = try FileHandle(forReadingFrom: url)
        defer { try? handle.close() }
        var hasher = SHA256()
        while let data = try handle.read(upToCount: 1 << 20), !data.isEmpty {
            hasher.update(data: data)
        }
        return hasher.finalize().map { String(format: "%02x", $0) }.joined()
    }

    private func sanitizedStem(_ filename: String) -> String {
        filename.replacingOccurrences(of: "/", with: "_")
    }

    private func outputContainsStaleMotionPhotoXMP(_ imageURL: URL) -> Bool {
        guard let source = CGImageSourceCreateWithURL(
            imageURL as CFURL,
            [kCGImageSourceShouldCache: false] as CFDictionary
        ),
        let metadata = CGImageSourceCopyMetadataAtIndex(source, 0, nil),
        let xmp = CGImageMetadataCreateXMPData(metadata, nil),
        let text = String(data: xmp as Data, encoding: .utf8) else {
            return false
        }
        return text.contains("MotionPhoto")
            || text.contains("MicroVideo")
            || text.contains("GContainer")
            || text.contains("Container:Directory")
    }
}
