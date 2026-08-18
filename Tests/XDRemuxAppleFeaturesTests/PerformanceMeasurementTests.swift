import Foundation
import XCTest
@testable import XDRemuxAppleFeatures

final class PerformanceMeasurementTests: XCTestCase {
    private struct Fixture {
        let current: [Float]
        let target: [Float]
        let perturbed: [[Float]]
        let steps: [Double]
    }

    private static let fixture: Fixture = {
        // Large enough to expose the former full-raster Jacobian allocation regime, while keeping
        // the benchmark practical on shared CI runners. Inputs are constructed outside measure().
        let width = 768
        let height = 768
        let valueCount = width * height * 3
        var current = [Float](repeating: 0, count: valueCount)
        var target = [Float](repeating: 0, count: valueCount)
        for index in 0..<valueCount {
            let base = Float((index * 17 + 13) % 251) / 10
            current[index] = base
            target[index] = base + Float((index * 7 + 3) % 19 - 9) * 0.05
        }
        let steps = (0..<12).map { 0.01 + Double($0) * 0.0005 }
        var perturbed: [[Float]] = []
        perturbed.reserveCapacity(12)
        for parameter in 0..<12 {
            let step = Float(steps[parameter])
            var raster = current
            for index in raster.indices {
                let feature = Float(((index + 1) * (parameter + 3)) % 23 - 11) / 11
                raster[index] += step * feature
            }
            perturbed.append(raster)
        }
        return Fixture(current: current, target: target, perturbed: perturbed, steps: steps)
    }()

    override class var defaultMetrics: [XCTMetric] {
        [XCTClockMetric(), XCTCPUMetric(), XCTMemoryMetric()]
    }

    func testSampledJacobianReleasePerformance() throws {
        try XCTSkipUnless(
            ProcessInfo.processInfo.environment["XDREMUX_RUN_PERFORMANCE_MEASUREMENTS"] == "1",
            "release performance measurements are opt-in"
        )
        let fixture = Self.fixture
        let options = XCTMeasureOptions()
        options.iterationCount = 3
        var result: [Double] = []
        measure(metrics: Self.defaultMetrics, options: options) {
            result = try! ConstrainedPolynomialStyleDataProducer.solveSampledUpdateForTesting(
                currentRGB: fixture.current,
                targetRGB: fixture.target,
                perturbedRGB: fixture.perturbed,
                steps: fixture.steps
            )
        }
        XCTAssertEqual(result.count, 12)
        XCTAssertTrue(result.allSatisfy(\.isFinite))
    }

    func testLegacyFullDerivativeReferencePerformance() throws {
        try XCTSkipUnless(
            ProcessInfo.processInfo.environment["XDREMUX_RUN_PERFORMANCE_MEASUREMENTS"] == "1",
            "release performance measurements are opt-in"
        )
        let fixture = Self.fixture
        let options = XCTMeasureOptions()
        options.iterationCount = 3
        var result: [Double] = []
        measure(metrics: Self.defaultMetrics, options: options) {
            result = try! Self.legacyFullDerivativeSolve(
                current: fixture.current,
                target: fixture.target,
                perturbed: fixture.perturbed,
                steps: fixture.steps
            )
        }
        XCTAssertEqual(result.count, 12)
        XCTAssertTrue(result.allSatisfy(\.isFinite))
    }

    private static func legacyFullDerivativeSolve(
        current: [Float],
        target: [Float],
        perturbed: [[Float]],
        steps: [Double]
    ) throws -> [Double] {
        let count = 12
        let derivatives = zip(perturbed, steps).map { raster, step in
            zip(raster, current).map { ($0 - $1) / Float(step) }
        }
        var normal = Array(repeating: Array(repeating: 0.0, count: count), count: count)
        var gradient = Array(repeating: 0.0, count: count)
        let stride = max(1, current.count / (50_000 * 3))
        var sampleCount = 0
        for pixel in Swift.stride(from: 0, to: current.count / 3, by: stride) {
            for channel in 0..<3 {
                let sample = pixel * 3 + channel
                let residual = Double(target[sample] - current[sample])
                let huberWeight = min(1.0, 12.0 / max(12.0, abs(residual)))
                sampleCount += 1
                for row in 0..<count {
                    let rowValue = Double(derivatives[row][sample])
                    gradient[row] += huberWeight * rowValue * residual
                    for column in row..<count {
                        normal[row][column] += huberWeight
                            * rowValue * Double(derivatives[column][sample])
                    }
                }
            }
        }
        if sampleCount > 0 {
            let normalization = 1.0 / Double(sampleCount)
            for row in 0..<count {
                gradient[row] *= normalization
                for column in row..<count {
                    normal[row][column] *= normalization
                }
            }
        }
        for row in 0..<count {
            for column in 0..<row { normal[row][column] = normal[column][row] }
        }
        let trace = (0..<count).reduce(0.0) { $0 + normal[$1][$1] }
        let ridge = max(trace / Double(count) * 1e-6, 1e-9)
        for index in 0..<count { normal[index][index] += ridge }
        var solution = try solveLinearSystem(normal, gradient)
        let epsilon = 1.0 / 32.0
        for index in solution.indices {
            solution[index] = min(epsilon, max(-epsilon, solution[index]))
        }
        return solution
    }

    private static func solveLinearSystem(
        _ matrix: [[Double]],
        _ vector: [Double]
    ) throws -> [Double] {
        let count = vector.count
        var augmented = zip(matrix, vector).map { $0 + [$1] }
        for pivot in 0..<count {
            let best = (pivot..<count).max {
                abs(augmented[$0][pivot]) < abs(augmented[$1][pivot])
            }!
            guard abs(augmented[best][pivot]) > 1e-12 else {
                throw NSError(domain: "PerformanceMeasurementTests", code: 1)
            }
            if best != pivot { augmented.swapAt(best, pivot) }
            let divisor = augmented[pivot][pivot]
            for column in pivot...count { augmented[pivot][column] /= divisor }
            for row in 0..<count where row != pivot {
                let factor = augmented[row][pivot]
                for column in pivot...count {
                    augmented[row][column] -= factor * augmented[pivot][column]
                }
            }
        }
        return augmented.map { $0[count] }
    }
}
