import Foundation
import XDRemuxCore

package struct AppleStyleDataRequest {
    let sourceURL: URL
    let renderedTargetURL: URL
    let outputDirectory: URL
    let sourceDomain: String
    let targetDomain: String
}

package struct AppleStyleDataResult {
    let styleData: Data
    let styleDataSHA256: String
    let polynomialCount: Int
    let blockValueCount: Int
    let tileCount: Int
    let producer: String
    let producerVersion: String
    let sourceSHA256: String
    let targetSHA256: String
    let sourceDomain: String
    let targetDomain: String
    let learnBufferKind: String?
    let solverKind: String?
    let evidence: AppleEvidenceClass
    let sceneMatched: Bool
    let identityFallback: Bool
    let fallbackKind: String?
    let reconstructionMetrics: [String: Any]
    let warnings: [String]

    var key1IncrementEligible: Bool {
        sceneMatched && !identityFallback && fallbackKind == nil
    }

    // Key 1 admission is necessary but cannot establish full-scene or Photos
    // equivalence. Keep the public result fail-closed until those gates pass.
    var productionEligible: Bool { false }

    var manifest: [String: Any] {
        [
            "byteCount": styleData.count,
            "sha256": styleDataSHA256,
            "producer": producer,
            "producerVersion": producerVersion,
            "sourceSHA256": sourceSHA256,
            "targetSHA256": targetSHA256,
            "sourceDomain": sourceDomain,
            "targetDomain": targetDomain,
            "learnBufferKind": learnBufferKind.map { $0 as Any } ?? NSNull(),
            "solverKind": solverKind.map { $0 as Any } ?? NSNull(),
            "evidence": evidence.rawValue,
            "sceneMatched": sceneMatched,
            "key1IncrementEligible": key1IncrementEligible,
            "productionEligible": productionEligible,
            "productionEligibilityBoundary": "key 1 neutral and increment checks are necessary but not sufficient; direct full-scene and real Photos acceptance remain separate",
            "identityFallback": identityFallback,
            "fallbackKind": fallbackKind.map { $0 as Any } ?? NSNull(),
            "polynomialCount": polynomialCount,
            "blockValueCount": blockValueCount,
            "tileCount": tileCount,
            "reconstructionMetrics": reconstructionMetrics,
            "warnings": warnings,
        ]
    }
}

package protocol AppleStyleDataProducing {
    func makeStyleData(request: AppleStyleDataRequest) throws -> AppleStyleDataResult
}

package enum AppleStyleDataLayout {
    static let polynomialCount = 10
    static let channelCount = 3
    static let blockValueCount = polynomialCount * channelCount
    static let tileCount = 12 * 9 * 8
    static let byteCount = blockValueCount * tileCount * 2
    static let identityIndices = Set([3, 7, 11])
    static let identitySHA256 =
        "43e0ae73508cc10684d4be708fa1d19f3b55b8de15cb8e3544ef16300db91dbe"

    static func basis(red: Float, green: Float, blue: Float) throws -> [Float] {
        guard red.isFinite, green.isFinite, blue.isFinite else {
            throw CLIError.invalidContainer(
                "Apple style polynomial basis input contains NaN or Inf"
            )
        }
        return [
            1, red, green, blue,
            red * red, red * green, red * blue,
            green * green, green * blue, blue * blue,
        ]
    }

    static func completeIdentity() throws -> Data {
        var block = Data()
        block.reserveCapacity(blockValueCount * 2)
        for index in 0..<blockValueCount {
            var bits = Float16(identityIndices.contains(index) ? 1 : 0)
                .bitPattern
                .littleEndian
            withUnsafeBytes(of: &bits) { block.append(contentsOf: $0) }
        }
        var result = Data()
        result.reserveCapacity(byteCount)
        for _ in 0..<tileCount {
            result.append(block)
        }
        guard result.count == byteCount, sha256Hex(result) == identitySHA256 else {
            throw CLIError.invalidContainer(
                "generated complete identity key 1 does not match the verified CMImaging coefficient layout"
            )
        }
        return result
    }

    static func validate(_ data: Data) throws -> [String: Any] {
        guard data.count == byteCount else {
            throw CLIError.invalidContainer(
                "Apple style data has \(data.count) bytes; expected \(byteCount)"
            )
        }
        var minimum = Float.infinity
        var maximum = -Float.infinity
        var nonfiniteCount = 0
        var identitySquaredError = 0.0
        var maximumIdentityError = 0.0
        for valueIndex in 0..<(data.count / 2) {
            let offset = valueIndex * 2
            let bits = UInt16(data[offset]) | (UInt16(data[offset + 1]) << 8)
            let value = Float(Float16(bitPattern: bits))
            if !value.isFinite {
                nonfiniteCount += 1
                continue
            }
            minimum = min(minimum, value)
            maximum = max(maximum, value)
            let localIndex = valueIndex % blockValueCount
            let expected: Float = identityIndices.contains(localIndex) ? 1 : 0
            let error = Double(value - expected)
            identitySquaredError += error * error
            maximumIdentityError = max(maximumIdentityError, abs(error))
        }
        guard nonfiniteCount == 0 else {
            throw CLIError.invalidContainer(
                "Apple style data contains \(nonfiniteCount) non-finite Float16 values"
            )
        }
        return [
            "finite": true,
            "valueCount": data.count / 2,
            "minimum": minimum,
            "maximum": maximum,
            "identityResidualRMSE": sqrt(identitySquaredError / Double(data.count / 2)),
            "identityResidualMaximumAbsolute": maximumIdentityError,
            "completeIdentity": sha256Hex(data) == identitySHA256,
        ]
    }
}

// The constrained solver needs a complete identity input for its preliminary
// consumer validation. This baseline is internal and never exposed as a mode
// or emitted as a fallback producer.
package struct SolverIdentityBaselineProducer: AppleStyleDataProducing {
    package func makeStyleData(
        request: AppleStyleDataRequest
    ) throws -> AppleStyleDataResult {
        try FileManager.default.createDirectory(
            at: request.outputDirectory,
            withIntermediateDirectories: true
        )
        let styleData = try AppleStyleDataLayout.completeIdentity()
        try styleData.write(
            to: request.outputDirectory.appendingPathComponent("solver-identity-baseline.f16.bin"),
            options: .atomic
        )
        return AppleStyleDataResult(
            styleData: styleData,
            styleDataSHA256: sha256Hex(styleData),
            polynomialCount: AppleStyleDataLayout.polynomialCount,
            blockValueCount: AppleStyleDataLayout.blockValueCount,
            tileCount: AppleStyleDataLayout.tileCount,
            producer: "solverIdentityBaseline",
            producerVersion: "solver-identity-baseline-v1",
            sourceSHA256: sha256Hex(
                try Data(contentsOf: request.sourceURL, options: [.mappedIfSafe])
            ),
            targetSHA256: sha256Hex(
                try Data(contentsOf: request.renderedTargetURL, options: [.mappedIfSafe])
            ),
            sourceDomain: request.sourceDomain,
            targetDomain: request.targetDomain,
            learnBufferKind: nil,
            solverKind: "constrained-solver-preliminary-baseline",
            evidence: .privateFrameworkIdentity,
            sceneMatched: false,
            identityFallback: false,
            fallbackKind: nil,
            reconstructionMetrics: [
                "status": "internal_solver_baseline",
                "layout": try AppleStyleDataLayout.validate(styleData),
            ],
            warnings: [
                "Internal constrained-solver baseline; not a user-selectable producer."
            ]
        )
    }
}
