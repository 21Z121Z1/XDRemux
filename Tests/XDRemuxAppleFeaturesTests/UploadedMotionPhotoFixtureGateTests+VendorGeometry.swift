import Foundation
import XCTest
import XDRemuxCore
@testable import XDRemuxAppleFeatures

/// Vendor-geometry assertions intentionally share the strict fixture-test name prefix so the
/// existing `swift test --filter UploadedMotionPhotoFixtureGateTests` CI lane executes them.
final class UploadedMotionPhotoFixtureGateTestsVendorGeometry: XCTestCase {
    private struct ExpectedPlan {
        let relativePath: String
        let kind: VendorLivePhotoGeometryKind?
        let auxiliaryCount: Int
    }

    func testStrictFixturesGateVendorGeometryScopeAndStreamRoles() throws {
        guard let rootPath = ProcessInfo.processInfo.environment["XDREMUX_MOTION_PHOTO_FIXTURE_ROOT"],
              !rootPath.isEmpty else {
            throw XCTSkip("XDREMUX_MOTION_PHOTO_FIXTURE_ROOT is not configured")
        }
        let root = URL(fileURLWithPath: rootPath, isDirectory: true)
        let expectations: [ExpectedPlan] = [
            .init(relativePath: "IMG20260710191114_ColorOS_16.jpg", kind: .colorOS16, auxiliaryCount: 1),
            .init(relativePath: "IMG20260801190843_ColorOS_16.jpg", kind: .colorOS16, auxiliaryCount: 1),
            .init(relativePath: "20260312_135609..heic", kind: .samsung, auxiliaryCount: 0),
            .init(relativePath: "20260312_135610..heic", kind: .samsung, auxiliaryCount: 0),
            .init(relativePath: "20260312_135625..jpg", kind: .samsung, auxiliaryCount: 0),
            .init(relativePath: "20260312_135627..jpg", kind: .samsung, auxiliaryCount: 0),
            .init(relativePath: "IMG20250502131605.jpg", kind: nil, auxiliaryCount: 0),
            .init(relativePath: "IMG20250502131608.jpg", kind: nil, auxiliaryCount: 0),
            .init(relativePath: "IMG20250819170327.jpg", kind: nil, auxiliaryCount: 0),
        ]

        for expected in expectations {
            let inputURL = root.appendingPathComponent(expected.relativePath)
            XCTAssertTrue(FileManager.default.fileExists(atPath: inputURL.path), expected.relativePath)
            let asset = try XCTUnwrap(
                OppoMotionPhotoParser.parse(url: inputURL),
                "fixture did not parse: \(expected.relativePath)"
            )

            let scratch = FileManager.default.temporaryDirectory
                .appendingPathComponent("xdremux-vendor-geometry-fixture-\(UUID().uuidString)", isDirectory: true)
            try FileManager.default.createDirectory(at: scratch, withIntermediateDirectories: true)
            defer { try? FileManager.default.removeItem(at: scratch) }
            let stillExtension = asset.sourceKind == .androidHeifMotionPhotoV1 ? "heic" : "jpg"
            let stillURL = scratch.appendingPathComponent("still.\(stillExtension)")
            try MotionPhotoPayloadExtractor.copy(
                range: asset.stillResourceRange,
                from: inputURL,
                to: stillURL
            )

            let plan = try VendorLivePhotoGeometryPolicy.plan(for: asset, stillResourceURL: stillURL)
            XCTAssertEqual(plan?.kind, expected.kind, expected.relativePath)
            XCTAssertEqual(plan?.streamLayout.auxiliaryGeometry.count ?? 0, expected.auxiliaryCount, expected.relativePath)

            if expected.kind != nil {
                XCTAssertNotNil(plan?.stillReferenceDimensions, expected.relativePath)
            }
            if expected.kind == .samsung {
                XCTAssertEqual(plan?.streamLayout.primary.range, asset.videoResourceRange, expected.relativePath)
            }
        }
    }

    func testSamsungMakeGateIsCaseInsensitiveButNarrow() {
        XCTAssertTrue(VendorLivePhotoGeometryPolicy.isSamsungMake("SAMSUNG"))
        XCTAssertTrue(VendorLivePhotoGeometryPolicy.isSamsungMake("Samsung Electronics"))
        XCTAssertFalse(VendorLivePhotoGeometryPolicy.isSamsungMake("OPPO"))
        XCTAssertFalse(VendorLivePhotoGeometryPolicy.isSamsungMake(nil))
    }
}
