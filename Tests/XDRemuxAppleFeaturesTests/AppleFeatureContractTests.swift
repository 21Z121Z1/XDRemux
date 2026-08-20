import Foundation
import XCTest
import XDRemuxCore
@testable import XDRemuxAppleFeatures

final class AppleFeatureContractTests: XCTestCase {
    func testStyleDataIdentityLayoutIsFiniteAndByteStable() throws {
        let identity = try AppleStyleDataLayout.completeIdentity()
        let report = try AppleStyleDataLayout.validate(identity)

        XCTAssertEqual(identity.count, 51_840)
        XCTAssertEqual(sha256Hex(identity), AppleStyleDataLayout.identitySHA256)
        XCTAssertEqual(report["valueCount"] as? Int, 25_920)
        XCTAssertEqual(report["completeIdentity"] as? Bool, true)
        XCTAssertEqual(report["identityResidualRMSE"] as? Double, 0)
    }

    func testStyleDataLayoutRejectsMalformedAndNonfiniteBuffers() throws {
        XCTAssertThrowsError(try AppleStyleDataLayout.validate(Data(count: 51_838)))

        var nonfinite = try AppleStyleDataLayout.completeIdentity()
        nonfinite[0] = 0x00
        nonfinite[1] = 0x7c
        XCTAssertThrowsError(try AppleStyleDataLayout.validate(nonfinite))
    }

    func testStylePolynomialBasisUsesVerifiedTenTermOrder() throws {
        let basis = try AppleStyleDataLayout.basis(red: 2, green: 3, blue: 5)
        XCTAssertEqual(basis, [1, 2, 3, 5, 4, 6, 10, 9, 15, 25])
        XCTAssertThrowsError(
            try AppleStyleDataLayout.basis(red: .nan, green: 0, blue: 0)
        )
    }

    func testPhotoDerivedLightMapClampsOnlyAtNativeSerializationBoundary() throws {
        let source = Array(repeating: Float(-1), count: 16 * 32)
            + Array(repeating: Float(2), count: 16 * 32)
        let map = try ApplePhotographicStylesPipeline.lightMap(
            source,
            width: 32,
            height: 32,
            valueScale: 1,
            outputMinimum: 0.040740966796875,
            outputMaximum: 0.75830078125,
            storageOrientation: 1
        )
        let values = stride(from: 0, to: map.count, by: 2).map { offset -> Float in
            let bits = UInt16(map[offset]) | UInt16(map[offset + 1]) << 8
            return Float(Float16(bitPattern: bits))
        }

        XCTAssertEqual(map.count, 2_048)
        XCTAssertEqual(values.min(), Float(Float16(0.040740966796875)))
        XCTAssertEqual(values.max(), Float(Float16(0.75830078125)))
        XCTAssertThrowsError(try ApplePhotographicStylesPipeline.lightMap(
            [0], width: 2, height: 2, valueScale: 1, storageOrientation: 1
        ))
    }

    func testNeutralStyleDeltaProtocolResourceIsByteStable() throws {
        let hashes = try ApplePhotographicStylesPipeline
            .neutralStyleDeltaProtocolResourceHashes()
        XCTAssertEqual(
            hashes["annexB"],
            "d02017d9f516dbe7ef156bb92000311180cd4a1ff0aab1b3753bc2cc71ca8846"
        )
        XCTAssertEqual(
            hashes["itemPayload"],
            "14b04fcde02476f24f83a893d245b4d06728954e8ad004f416b6e3a956eba216"
        )
        XCTAssertEqual(
            hashes["hvcC"],
            "35ecc004d07192f4e9c8a44c0a9edb598599b7a6d0c59b8165a5fb433f5746a5"
        )
    }

    func testAppleNativeHelperTimeoutTerminatesTheChild() throws {
        let result = try AppleNativeToolchain.run(
            URL(fileURLWithPath: "/bin/sleep"),
            arguments: ["2"],
            timeout: 0.05
        )
        XCTAssertTrue(result.timedOut)
        XCTAssertNotEqual(result.status, 0)
    }

    func testConstrainedStyleDataRepeatsOneQuantizedBlockAcrossAllPlanesAndNodes() throws {
        let parameters = [0.01, -0.02, 0.03, 0.04, -0.05, 0.06]
        let styleData = try ConstrainedPolynomialStyleDataProducer.styleData(
            parameters: parameters
        )
        let blockByteCount = AppleStyleDataLayout.blockValueCount * 2
        let firstBlock = styleData.prefix(blockByteCount)

        XCTAssertEqual(styleData.count, AppleStyleDataLayout.byteCount)
        for blockIndex in 0..<AppleStyleDataLayout.tileCount {
            let lower = blockIndex * blockByteCount
            XCTAssertEqual(
                styleData.subdata(in: lower..<(lower + blockByteCount)),
                Data(firstBlock)
            )
        }
        XCTAssertNotEqual(sha256Hex(styleData), AppleStyleDataLayout.identitySHA256)
        XCTAssertEqual(try AppleStyleDataLayout.validate(styleData)["finite"] as? Bool, true)
    }

    func testModelSeededResidualPreservesIdentityParameterization() throws {
        let identity = try AppleStyleDataLayout.completeIdentity()
        let residual = (0..<AppleStyleDataLayout.blockValueCount).map { index in
            index < 12 ? Double(index - 6) / 1024.0 : 0
        }
        let seeded = try ConstrainedPolynomialStyleDataProducer.styleData(
            base: identity,
            coefficientDeltas: residual
        )
        let legacy = try ConstrainedPolynomialStyleDataProducer.styleData(
            coefficientDeltas: residual
        )
        XCTAssertEqual(seeded, legacy)
        XCTAssertEqual(seeded.count, AppleStyleDataLayout.byteCount)
    }

    func testConstrainedStyleDataRejectsMalformedOrNonfiniteParameters() {
        XCTAssertThrowsError(
            try ConstrainedPolynomialStyleDataProducer.styleData(parameters: [0])
        )
        XCTAssertThrowsError(
            try ConstrainedPolynomialStyleDataProducer.styleData(
                parameters: [.nan, 0, 0, 0, 0, 0]
            )
        )
        XCTAssertThrowsError(
            try ConstrainedPolynomialStyleDataProducer.styleData(
                parameters: [1, 0, 0, 0, 0, 0]
            )
        )
    }

    func testDifferentConstrainedInputsProduceDifferentKeyOneHashes() throws {
        let first = try ConstrainedPolynomialStyleDataProducer.styleData(
            parameters: [0.01, 0, 0, 0, 0, 0]
        )
        let second = try ConstrainedPolynomialStyleDataProducer.styleData(
            parameters: [0, 0.01, 0, 0, 0, 0]
        )
        XCTAssertNotEqual(sha256Hex(first), sha256Hex(second))
    }

    func testConstrainedGlobalPolynomialRecoversT0ThroughT5Terms() throws {
        let values: [Float] = [0.10, 0.25, 0.40, 0.55, 0.70, 0.85]
        var source: [Float] = []
        for red in values {
            for green in values {
                for blue in values {
                    source.append(contentsOf: [red * 255, green * 255, blue * 255])
                }
            }
        }
        let epsilon: Float = 1.0 / 32.0
        let teachers: [(name: String, coefficientIndex: Int?, target: ([Float]) -> [Float])] = [
            ("T0", nil, { $0 }),
            ("T1 R->R", 3, { input in
                Self.polynomialTeacher(input, output: 0, epsilon: epsilon) { red, _, _ in red }
            }),
            ("T2 G->R", 6, { input in
                Self.polynomialTeacher(input, output: 0, epsilon: epsilon) { _, green, _ in green }
            }),
            ("T3 constant->R", 0, { input in
                Self.polynomialTeacher(input, output: 0, epsilon: epsilon) { _, _, _ in 1 }
            }),
            ("T4 R2->R", 12, { input in
                Self.polynomialTeacher(input, output: 0, epsilon: epsilon) { red, _, _ in red * red }
            }),
            ("T5 RG->B", 17, { input in
                Self.polynomialTeacher(input, output: 2, epsilon: epsilon) { red, green, _ in red * green }
            }),
        ]

        for teacher in teachers {
            let coefficients = try ConstrainedPolynomialStyleDataProducer
                .fitGlobalPolynomial(
                    sourceRGB8: source,
                    targetRGB8: teacher.target(source)
                )
            if let expected = teacher.coefficientIndex {
                XCTAssertEqual(
                    coefficients[expected],
                    Double(epsilon),
                    accuracy: 0.003,
                    teacher.name
                )
                let sameOutputLeakage = coefficients.indices
                    .filter { $0 % 3 == expected % 3 && $0 != expected }
                    .map { abs(coefficients[$0]) }
                    .max() ?? 0
                XCTAssertLessThan(sameOutputLeakage, 0.003, teacher.name)
            } else {
                XCTAssertLessThan(coefficients.map(abs).max() ?? 0, 1e-9, teacher.name)
            }
        }
    }

    func testConfiguredConstrainedSolverFixtureImprovesSceneMatch() throws {
        let environment = ProcessInfo.processInfo.environment
        guard let fixturePath = environment["XDREMUX_STYLE_SOLVER_FIXTURE"],
              let propertyListPath = environment["XDREMUX_STYLE_SOLVER_IDENTITY_PLIST"],
              let outputPath = environment["XDREMUX_STYLE_SOLVER_OUTPUT"] else {
            throw XCTSkip("configured private constrained-solver fixture is unavailable")
        }
        let fixtureURL = URL(fileURLWithPath: fixturePath)
        let propertyListURL = URL(fileURLWithPath: propertyListPath)
        let outputURL = URL(fileURLWithPath: outputPath, isDirectory: true)
        try? FileManager.default.removeItem(at: outputURL)

        let startedAt = Date()
        let result = try ConstrainedPolynomialStyleDataProducer().makeStyleData(
            preliminaryHEICURL: fixtureURL,
            identityStylePropertyList: try Data(contentsOf: propertyListURL),
            outputDirectory: outputURL
        )
        let identity = try XCTUnwrap(result.reconstructionMetrics["identity"] as? [String: Any])
        let selected = try XCTUnwrap(result.reconstructionMetrics["selected"] as? [String: Any])
        let identityRMSE = try XCTUnwrap(identity["rmse8"] as? Double)
        let selectedRMSE = try XCTUnwrap(selected["rmse8"] as? Double)

        XCTAssertTrue(result.sceneMatched)
        XCTAssertTrue(result.key1IncrementEligible)
        XCTAssertFalse(result.productionEligible)
        XCTAssertLessThan(selectedRMSE, identityRMSE)
        if let maximumText = environment["XDREMUX_STYLE_SOLVER_MAX_RMSE8"],
           let maximum = Double(maximumText) {
            XCTAssertLessThanOrEqual(selectedRMSE, maximum)
        }
        if let expectedSHA256 = environment["XDREMUX_STYLE_SOLVER_EXPECTED_SHA256"] {
            XCTAssertEqual(result.styleDataSHA256, expectedSHA256)
        }
        let report: [String: Any] = [
            "fixture": fixtureURL.path,
            "styleDataSHA256": result.styleDataSHA256,
            "identityRMSE8": identityRMSE,
            "selectedRMSE8": selectedRMSE,
            "rmseImprovementFraction": 1 - selectedRMSE / identityRMSE,
            "elapsedSeconds": Date().timeIntervalSince(startedAt),
        ]
        let reportData = try JSONSerialization.data(
            withJSONObject: report,
            options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        )
        print(String(decoding: reportData, as: UTF8.self))
    }

    func testConfiguredModelSeededSolverFixtureImprovesSceneMatch() throws {
        let environment = ProcessInfo.processInfo.environment
        guard let fixturePath = environment["XDREMUX_STYLE_SOLVER_FIXTURE"],
              let propertyListPath = environment["XDREMUX_STYLE_SOLVER_IDENTITY_PLIST"],
              let seedPath = environment["XDREMUX_STYLE_SOLVER_MODEL_SEED"],
              let outputPath = environment["XDREMUX_STYLE_SOLVER_OUTPUT"] else {
            throw XCTSkip("configured private model-seeded solver fixture is unavailable")
        }
        let fixtureURL = URL(fileURLWithPath: fixturePath)
        let propertyListURL = URL(fileURLWithPath: propertyListPath)
        let seedURL = URL(fileURLWithPath: seedPath)
        let outputURL = URL(fileURLWithPath: outputPath, isDirectory: true)
        try? FileManager.default.removeItem(at: outputURL)
        let validateResponse = environment["XDREMUX_STYLE_SOLVER_VALIDATE_RESPONSE"] != "off"

        let startedAt = Date()
        let result = try ConstrainedPolynomialStyleDataProducer().makeModelSeededStyleData(
            preliminaryHEICURL: fixtureURL,
            identityStylePropertyList: try Data(contentsOf: propertyListURL),
            seedStyleData: try Data(contentsOf: seedURL),
            outputDirectory: outputURL,
            validateNativeResponse: validateResponse
        )
        let identity = try XCTUnwrap(result.reconstructionMetrics["identity"] as? [String: Any])
        let seed = try XCTUnwrap(result.reconstructionMetrics["seed"] as? [String: Any])
        let selected = try XCTUnwrap(result.reconstructionMetrics["selected"] as? [String: Any])
        let identityRMSE = try XCTUnwrap(identity["rmse8"] as? Double)
        let seedRMSE = try XCTUnwrap(seed["rmse8"] as? Double)
        let selectedRMSE = try XCTUnwrap(selected["rmse8"] as? Double)

        XCTAssertTrue(result.sceneMatched)
        XCTAssertTrue(result.key1IncrementEligible)
        XCTAssertLessThan(selectedRMSE, identityRMSE * 0.98)
        XCTAssertLessThanOrEqual(selectedRMSE, seedRMSE)
        let report: [String: Any] = [
            "fixture": fixtureURL.path,
            "seed": seedURL.path,
            "styleDataSHA256": result.styleDataSHA256,
            "identityRMSE8": identityRMSE,
            "seedRMSE8": seedRMSE,
            "selectedRMSE8": selectedRMSE,
            "rmseImprovementFraction": 1 - selectedRMSE / identityRMSE,
            "nativeResponseValidated": validateResponse,
            "elapsedSeconds": Date().timeIntervalSince(startedAt),
        ]
        let reportData = try JSONSerialization.data(
            withJSONObject: report,
            options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        )
        print(String(decoding: reportData, as: UTF8.self))
    }

    func testConfiguredModelFastPathMeetsBoundedProxyContract() throws {
        let environment = ProcessInfo.processInfo.environment
        guard let fixturePath = environment["XDREMUX_STYLE_SOLVER_FIXTURE"],
              let propertyListPath = environment["XDREMUX_STYLE_SOLVER_IDENTITY_PLIST"],
              let seedPath = environment["XDREMUX_STYLE_SOLVER_MODEL_SEED"],
              let linearThumbnailPath = environment["XDREMUX_STYLE_LINEAR_THUMBNAIL"],
              let outputPath = environment["XDREMUX_STYLE_SOLVER_OUTPUT"] else {
            throw XCTSkip("configured private model fast-path fixture is unavailable")
        }
        let outputURL = URL(fileURLWithPath: outputPath, isDirectory: true)
        try? FileManager.default.removeItem(at: outputURL)
        let result = try ConstrainedPolynomialStyleDataProducer().makeModelFastStyleData(
            preliminaryHEICURL: URL(fileURLWithPath: fixturePath),
            identityStylePropertyList: try Data(
                contentsOf: URL(fileURLWithPath: propertyListPath)
            ),
            modelStyleData: try Data(contentsOf: URL(fileURLWithPath: seedPath)),
            linearThumbnailURL: URL(fileURLWithPath: linearThumbnailPath),
            outputDirectory: outputURL
        )
        let elapsed = try XCTUnwrap(
            result.reconstructionMetrics["elapsedSeconds"] as? Double
        )
        XCTAssertLessThanOrEqual(elapsed, 20)
        XCTAssertEqual(result.sceneMatched, !result.identityFallback)
        XCTAssertFalse(result.productionEligible)
        if let expected = environment["XDREMUX_STYLE_FAST_EXPECT_MODEL"] {
            XCTAssertEqual(result.sceneMatched, expected == "1")
        }
        print(String(data: try Data(
            contentsOf: outputURL.appendingPathComponent("fast-path-result.json")
        ), encoding: .utf8) ?? "")
    }

    func testConfiguredCoreMLReverseKey1MatchesPythonReference() throws {
        let environment = ProcessInfo.processInfo.environment
        guard let modelPath = environment["XDREMUX_REVERSE_KEY1_COREML_MODEL"],
              let styledPath = environment["XDREMUX_REVERSE_KEY1_STYLED"],
              let unstyledPath = environment["XDREMUX_REVERSE_KEY1_UNSTYLED"],
              let referencePath = environment["XDREMUX_REVERSE_KEY1_REFERENCE"] else {
            throw XCTSkip("configured private Core ML reverse-key fixture is unavailable")
        }
        let prediction = try ReverseKey1CoreMLPredictor.predict(
            modelURL: URL(fileURLWithPath: modelPath),
            styledURL: URL(fileURLWithPath: styledPath),
            unstyledURL: URL(fileURLWithPath: unstyledPath)
        )
        let reference = try Data(contentsOf: URL(fileURLWithPath: referencePath))
        XCTAssertEqual(prediction.styleData.count, reference.count)
        let actualValues = prediction.styleData.withUnsafeBytes {
            Array($0.bindMemory(to: UInt16.self))
        }
        let referenceValues = reference.withUnsafeBytes {
            Array($0.bindMemory(to: UInt16.self))
        }
        var maximum = 0.0
        var absolute = 0.0
        for (actual, expected) in zip(actualValues, referenceValues) {
            let error = abs(
                Double(Float(Float16(bitPattern: actual)))
                    - Double(Float(Float16(bitPattern: expected)))
            )
            maximum = max(maximum, error)
            absolute += error
        }
        XCTAssertLessThanOrEqual(maximum, 0.02)
        let report: [String: Any] = [
            "maximumAbsoluteError": maximum,
            "meanAbsoluteError": absolute / Double(actualValues.count),
            "timing": prediction.timing,
        ]
        print(String(data: try JSONSerialization.data(
            withJSONObject: report,
            options: [.prettyPrinted, .sortedKeys]
        ), encoding: .utf8) ?? "")
    }

    func testConfiguredUniversalStyleCoreMLMatchesPythonReference() throws {
        let environment = ProcessInfo.processInfo.environment
        guard let modelPath = environment["XDREMUX_UNIVERSAL_STYLE_COREML_MODEL"],
              let inputPath = environment["XDREMUX_UNIVERSAL_STYLE_INPUT"],
              let referencePath = environment["XDREMUX_UNIVERSAL_STYLE_REFERENCE_DIR"] else {
            throw XCTSkip("configured universal Photographic Style fixture is unavailable")
        }
        let inputURL = URL(fileURLWithPath: inputPath)
        let prediction = try UniversalPhotographicStyleCoreMLPredictor.predict(
            modelURL: URL(fileURLWithPath: modelPath),
            styledURL: inputURL,
            metadataURL: inputURL
        )
        let referenceURL = URL(fileURLWithPath: referencePath)
        let key1Error = try maximumFloat16Error(
            prediction.styleData,
            Data(contentsOf: referenceURL.appendingPathComponent("key1.bin"))
        )
        let cError = try maximumFloat16Error(
            prediction.lightMapCData,
            Data(contentsOf: referenceURL.appendingPathComponent("c.bin"))
        )
        let dError = try maximumFloat16Error(
            prediction.lightMapDData,
            Data(contentsOf: referenceURL.appendingPathComponent("d.bin"))
        )
        XCTAssertLessThanOrEqual(key1Error, 0.02)
        XCTAssertLessThanOrEqual(cError, 0.02)
        XCTAssertLessThanOrEqual(dError, 0.02)
        let expectedGTC = try Data(
            contentsOf: referenceURL.appendingPathComponent("key3-gtc.bin")
        )
        XCTAssertEqual(prediction.gtcData.count, expectedGTC.count)
        let gtcMaximum = zip(prediction.gtcData, expectedGTC).map {
            abs(Int($0.0) - Int($0.1))
        }.max() ?? 0
        XCTAssertLessThanOrEqual(gtcMaximum, 2)
        let referenceReport = try XCTUnwrap(
            try JSONSerialization.jsonObject(
                with: Data(contentsOf: referenceURL.appendingPathComponent("report.json"))
            ) as? [String: Any]
        )
        let referenceScalars = try XCTUnwrap(referenceReport["scalars"] as? [String: Double])
        for (name, expected) in referenceScalars {
            XCTAssertEqual(try XCTUnwrap(prediction.scalars[name]), expected, accuracy: 0.1)
        }
        let expectedUncertainty = try XCTUnwrap(referenceReport["uncertainty"] as? Double)
        XCTAssertEqual(prediction.uncertainty, expectedUncertainty, accuracy: 0.1)
        print(String(data: try JSONSerialization.data(
            withJSONObject: [
                "key1MaximumAbsoluteError": key1Error,
                "gtcMaximumByteError": gtcMaximum,
                "lightMapCMaximumAbsoluteError": cError,
                "lightMapDMaximumAbsoluteError": dError,
                "uncertainty": prediction.uncertainty,
                "timing": prediction.timing,
            ],
            options: [.prettyPrinted, .sortedKeys]
        ), encoding: .utf8) ?? "")
    }

    private func maximumFloat16Error(_ actual: Data, _ expected: Data) throws -> Double {
        XCTAssertEqual(actual.count, expected.count)
        XCTAssertEqual(actual.count % 2, 0)
        let actualValues = actual.withUnsafeBytes { Array($0.bindMemory(to: UInt16.self)) }
        let expectedValues = expected.withUnsafeBytes { Array($0.bindMemory(to: UInt16.self)) }
        return zip(actualValues, expectedValues).map {
            abs(
                Double(Float(Float16(bitPattern: $0.0)))
                    - Double(Float(Float16(bitPattern: $0.1)))
            )
        }.max() ?? 0
    }

    func testNativeReverseKeyProxyInversionSelfTest() throws {
        let executable = try AppleNativeToolchain.learnExecutable()
        let process = try AppleNativeToolchain.run(
            executable,
            arguments: ["--self-test-composition"],
            timeout: 10
        )
        XCTAssertFalse(process.timedOut)
        XCTAssertEqual(process.status, 0)
        let result = try XCTUnwrap(
            JSONSerialization.jsonObject(with: process.stdout) as? [String: Any]
        )
        XCTAssertEqual(result["passed"] as? Bool, true)
        XCTAssertEqual(result["inversionPreservesIdentity"] as? Bool, true)
        XCTAssertEqual(result["inversionRoundTrips"] as? Bool, true)
    }

    func testNativeIncrementResponseMetricSubtractsIdentityResponse() throws {
        let candidateMinus: [Float] = [0, 10, 20, 30]
        let candidatePlus: [Float] = [2, 14, 26, 42]
        let identityMinus: [Float] = [0, 8, 18, 30]
        let identityPlus: [Float] = [1, 11, 23, 39]
        let rmse = try ConstrainedPolynomialStyleDataProducer.incrementalResponseRMSE8(
            candidateMinus: candidateMinus,
            candidatePlus: candidatePlus,
            identityMinus: identityMinus,
            identityPlus: identityPlus
        )
        // Candidate response is [2,4,6,12], identity response is [1,3,5,9].
        XCTAssertEqual(rmse, sqrt(3.0), accuracy: 0.000_001)
        XCTAssertThrowsError(
            try ConstrainedPolynomialStyleDataProducer.incrementalResponseRMSE8(
                candidateMinus: [.nan],
                candidatePlus: [0],
                identityMinus: [0],
                identityPlus: [0]
            )
        )
    }

    func testNativeIncrementCurvatureMetricSubtractsIdentityCurvature() throws {
        let rmse = try ConstrainedPolynomialStyleDataProducer.incrementalCurvatureRMSE8(
            candidateMinus: [0, 4, 8],
            candidateMidpoint: [2, 7, 12],
            candidatePlus: [6, 14, 22],
            identityMinus: [0, 3, 7],
            identityMidpoint: [1, 5, 10],
            identityPlus: [3, 9, 17]
        )
        // Candidate curvature is [2,4,6], identity curvature is [1,2,4].
        XCTAssertEqual(rmse, sqrt(3.0), accuracy: 0.000_001)
        XCTAssertThrowsError(
            try ConstrainedPolynomialStyleDataProducer.incrementalCurvatureRMSE8(
                candidateMinus: [0],
                candidateMidpoint: [.infinity],
                candidatePlus: [0],
                identityMinus: [0],
                identityMidpoint: [0],
                identityPlus: [0]
            )
        )
    }

    func testNativeResponseDirectionRejectsReversedToneAndColor() {
        XCTAssertTrue(
            ConstrainedPolynomialStyleDataProducer.responseDirectionPassed(
                name: "tone",
                meanLumaDelta8: 1,
                meanChromaDelta8: -1
            )
        )
        XCTAssertFalse(
            ConstrainedPolynomialStyleDataProducer.responseDirectionPassed(
                name: "tone",
                meanLumaDelta8: -1,
                meanChromaDelta8: 1
            )
        )
        XCTAssertTrue(
            ConstrainedPolynomialStyleDataProducer.responseDirectionPassed(
                name: "color",
                meanLumaDelta8: -1,
                meanChromaDelta8: 1
            )
        )
        XCTAssertFalse(
            ConstrainedPolynomialStyleDataProducer.responseDirectionPassed(
                name: "color",
                meanLumaDelta8: 1,
                meanChromaDelta8: -1
            )
        )
        XCTAssertTrue(
            ConstrainedPolynomialStyleDataProducer.responseDirectionPassed(
                name: "combined",
                meanLumaDelta8: 1,
                meanChromaDelta8: 1
            )
        )
        XCTAssertFalse(
            ConstrainedPolynomialStyleDataProducer.responseDirectionPassed(
                name: "combined",
                meanLumaDelta8: 1,
                meanChromaDelta8: -1
            )
        )
        XCTAssertFalse(
            ConstrainedPolynomialStyleDataProducer.responseDirectionPassed(
                name: "tone",
                meanLumaDelta8: .nan,
                meanChromaDelta8: 1
            )
        )
    }

    private static func polynomialTeacher(
        _ input: [Float],
        output: Int,
        epsilon: Float,
        term: (Float, Float, Float) -> Float
    ) -> [Float] {
        var target = input
        for offset in stride(from: 0, to: input.count, by: 3) {
            let red = input[offset] / 255
            let green = input[offset + 1] / 255
            let blue = input[offset + 2] / 255
            target[offset + output] += epsilon * term(red, green, blue) * 255
        }
        return target
    }

    func testIdentityProducerIsExplicitlyMarkedNotSceneMatched() throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-style-fallback-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: directory) }
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let source = directory.appendingPathComponent("source.bin")
        let target = directory.appendingPathComponent("target.bin")
        try Data("source".utf8).write(to: source)
        try Data("target".utf8).write(to: target)

        let result = try IdentityStyleDataProducer().makeStyleData(
            request: AppleStyleDataRequest(
                sourceURL: source,
                renderedTargetURL: target,
                outputDirectory: directory.appendingPathComponent("output"),
                sourceDomain: "test source",
                targetDomain: "test target"
            )
        )

        XCTAssertEqual(result.styleDataSHA256, AppleStyleDataLayout.identitySHA256)
        XCTAssertTrue(result.identityFallback)
        XCTAssertFalse(result.sceneMatched)
        XCTAssertFalse(result.productionEligible)
        XCTAssertEqual(result.fallbackKind, "complete-identity")
        XCTAssertFalse(result.warnings.isEmpty)
        XCTAssertEqual(result.manifest["sceneMatched"] as? Bool, false)
        XCTAssertEqual(result.manifest["productionEligible"] as? Bool, false)
    }

    func testPortraitCoreSelfTestRemainsByteStable() throws {
        let report = try AppleFeatureConversionEngine.portraitSelfTestReport()

        XCTAssertEqual(report["passed"] as? Bool, true)
        XCTAssertEqual(report["byteStableRoundTrip"] as? Bool, true)
        XCTAssertEqual(report["malformedLengthRejected"] as? Bool, true)
        XCTAssertEqual(report["duplicateRecordRejected"] as? Bool, true)
    }

    func testPortraitSourceGainMapAcceptsRGBAndGrayscaleOnly() {
        XCTAssertTrue(
            PortraitConversionPipeline.isSupportedPortraitSourceGainMapPixelFormat(
                pixelFormatFourCC("444f")
            )
        )
        XCTAssertTrue(
            PortraitConversionPipeline.isSupportedPortraitSourceGainMapPixelFormat(
                pixelFormatFourCC("L008")
            )
        )
        XCTAssertFalse(
            PortraitConversionPipeline.isSupportedPortraitSourceGainMapPixelFormat(
                pixelFormatFourCC("420f")
            )
        )
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
