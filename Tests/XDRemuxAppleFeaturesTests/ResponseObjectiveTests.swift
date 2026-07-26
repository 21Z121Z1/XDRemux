import Foundation
import XCTest
import XDRemuxCore
@testable import XDRemuxAppleFeatures

// Stage A (response-v6) pure-function contracts.  Plan and envelope
// provenance: docs/plans/active/
// apple-styles-editor-response-optimization-20260726.md.
final class ResponseObjectiveTests: XCTestCase {
    private typealias Producer = ConstrainedPolynomialStyleDataProducer

    private func uniformRaster(
        red: Float, green: Float, blue: Float, width: Int = 64, height: Int = 64
    ) -> [Float] {
        var rgb = [Float]()
        rgb.reserveCapacity(width * height * 3)
        for _ in 0..<(width * height) {
            rgb.append(red)
            rgb.append(green)
            rgb.append(blue)
        }
        return rgb
    }

    func testHueWrapCrossesPlusMinus180() {
        XCTAssertEqual(Producer.wrappedDegrees(-179 - 179), 2, accuracy: 1e-9)
        XCTAssertEqual(Producer.wrappedDegrees(179 - -179), -2, accuracy: 1e-9)
        XCTAssertEqual(Producer.wrappedDegrees(10 - 4), 6, accuracy: 1e-9)
    }

    func testGrayRasterHasNoEligibleROI() {
        let sample = Producer.responseMetricSample(
            rgb8: uniformRaster(red: 128, green: 128, blue: 128),
            width: 64,
            height: 64,
            mask: nil
        )
        XCTAssertEqual(sample.roiKind, "none")
        XCTAssertEqual(sample.roiPixelCount, 0)
    }

    func testWarmPatchFallsBackToWarmROIWithSkinToneHue() {
        let sample = Producer.responseMetricSample(
            rgb8: uniformRaster(red: 200, green: 150, blue: 120),
            width: 64,
            height: 64,
            mask: nil
        )
        XCTAssertEqual(sample.roiKind, "warm-fallback")
        XCTAssertEqual(sample.roiPixelCount, 64 * 64)
        XCTAssertGreaterThan(sample.hueDegrees, 20)
        XCTAssertLessThan(sample.hueDegrees, 60)
        XCTAssertGreaterThan(sample.rgRatio, 1)
    }

    func testSkinMaskSelectsOnlyMaskedPixels() {
        let width = 64
        let height = 64
        // Left half warm, right half strong green: without the mask the green
        // side would dominate the warm ROI away.
        var rgb = [Float]()
        rgb.reserveCapacity(width * height * 3)
        for y in 0..<height {
            _ = y
            for x in 0..<width {
                if x < width / 2 {
                    rgb += [200, 150, 120]
                } else {
                    rgb += [40, 220, 40]
                }
            }
        }
        var maskSamples = [UInt8](repeating: 0, count: 32 * 32)
        for y in 0..<32 {
            for x in 0..<16 {
                maskSamples[y * 32 + x] = 255
            }
        }
        let masked = Producer.responseMetricSample(
            rgb8: rgb,
            width: width,
            height: height,
            mask: Producer.ResponseSkinMask(width: 32, height: 32, samples: maskSamples)
        )
        XCTAssertEqual(masked.roiKind, "skin-mask")
        XCTAssertEqual(masked.roiPixelCount, width / 2 * height)
        XCTAssertGreaterThan(masked.hueDegrees, 20)
        XCTAssertLessThan(masked.hueDegrees, 60)
    }

    func testHingeMatchesDocumentedIdentityViolation() {
        // Documented identity defect: hueDelta -7.261615deg against the native
        // lower bound -1.702725deg with a 0.30deg margin.
        let plus = Producer.ResponseMetricSample(
            hueDegrees: -7.261615, rgRatio: 1.30, roiPixelCount: 10_000, roiKind: "skin-mask"
        )
        let mid = Producer.ResponseMetricSample(
            hueDegrees: 0, rgRatio: 1.29, roiPixelCount: 10_000, roiKind: "skin-mask"
        )
        let state = Producer.responseObjectiveState(plus: plus, mid: mid)
        XCTAssertEqual(state.hueDeltaDegrees, -7.261615, accuracy: 1e-9)
        XCTAssertEqual(
            state.hueViolationDegrees,
            (-1.702725 + 0.30) - (-7.261615),
            accuracy: 1e-9
        )
        XCTAssertEqual(state.rgDelta, 0.01, accuracy: 1e-9)
        XCTAssertEqual(state.rgViolation, 0, accuracy: 1e-9)
        XCTAssertEqual(state.hingeScore, state.hueViolationDegrees, accuracy: 1e-9)
    }

    func testInsideEnvelopeHasZeroViolationAndNoneROIDisablesTerms() {
        let inside = Producer.responseObjectiveState(
            plus: Producer.ResponseMetricSample(
                hueDegrees: 3, rgRatio: 1.31, roiPixelCount: 5_000, roiKind: "skin-mask"
            ),
            mid: Producer.ResponseMetricSample(
                hueDegrees: 0, rgRatio: 1.30, roiPixelCount: 5_000, roiKind: "skin-mask"
            )
        )
        XCTAssertEqual(inside.hueViolationDegrees, 0)
        XCTAssertEqual(inside.rgViolation, 0)

        let none = Producer.responseObjectiveState(
            plus: Producer.ResponseMetricSample(
                hueDegrees: -30, rgRatio: 2, roiPixelCount: 0, roiKind: "none"
            ),
            mid: Producer.ResponseMetricSample(
                hueDegrees: 0, rgRatio: 1, roiPixelCount: 0, roiKind: "none"
            )
        )
        XCTAssertEqual(none.roiKind, "none")
        XCTAssertEqual(none.hingeScore, 0)
    }

    func testVanishedROIInheritsIdentityViolations() {
        let identity = Producer.responseObjectiveState(
            plus: Producer.ResponseMetricSample(
                hueDegrees: -7.261615, rgRatio: 1.30, roiPixelCount: 10_000, roiKind: "skin-mask"
            ),
            mid: Producer.ResponseMetricSample(
                hueDegrees: 0, rgRatio: 1.29, roiPixelCount: 10_000, roiKind: "skin-mask"
            )
        )
        let vanished = Producer.responseObjectiveState(
            plus: Producer.ResponseMetricSample(
                hueDegrees: 0, rgRatio: 0, roiPixelCount: 0, roiKind: "none"
            ),
            mid: Producer.ResponseMetricSample(
                hueDegrees: 0, rgRatio: 0, roiPixelCount: 0, roiKind: "none"
            )
        )
        let substituted = Producer.substitutingVanishedROI(vanished, identity: identity)
        XCTAssertEqual(substituted.roiKind, "roi-vanished")
        XCTAssertEqual(substituted.hingeScore, identity.hingeScore, accuracy: 1e-12)
        XCTAssertEqual(substituted.roiPixelCount, 0)

        // Without an identity control the state passes through untouched, and
        // a measured state is never substituted.
        XCTAssertEqual(
            Producer.substitutingVanishedROI(vanished, identity: nil).roiKind, "none"
        )
        XCTAssertEqual(
            Producer.substitutingVanishedROI(identity, identity: identity).roiKind, "skin-mask"
        )
    }
}
