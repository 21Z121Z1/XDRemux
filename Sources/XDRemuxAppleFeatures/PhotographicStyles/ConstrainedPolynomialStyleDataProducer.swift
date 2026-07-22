import CoreGraphics
import Dispatch
import Foundation
import ImageIO
import XDRemuxCore

package struct ConstrainedPolynomialStyleDataProducer {
    private static let solverAdmission = DispatchSemaphore(value: 1)
    private static let directParameterIndices = [0, 1, 2, 3, 7, 11]
    private static let solverRefinementParameterNames = [
        "constantToR", "constantToG", "constantToB",
        "RToR", "RToG", "RToB",
        "GToR", "GToG", "GToB",
        "BToR", "BToG", "BToB",
    ]
    private static let solverRefinementParameterIndices = Array(0..<12)
    private static let basisNames = [
        "constant", "R", "G", "B", "R2", "RG", "RB", "G2", "GB", "B2",
    ]
    private static let outputNames = ["R", "G", "B"]
    private static let epsilon = 1.0 / 32.0
    private static let linearBound = 1.0 / 8.0
    private static let quadraticBound = 1.0 / 16.0
    private static let iterationCount = 2
    private static let lineSearchScales = [1.0, 0.5, 0.25, 0.125]
    private static let analysisMaximumDimension = 1024
    private static let nativeIncrementEnvelopeRMSE8: [String: Double] = [
        "tone": 9.6,
        "color": 8.8,
        "combined": 10.2,
        "intensity": 8.0,
    ]
    private static let nativeIncrementSegmentEnvelopeRMSE8: [String: Double] = [
        "tone": 10.4,
        "color": 7.8,
        "combined": 13.0,
        "intensity": 5.6,
    ]
    private static let nativeIncrementCurvatureEnvelopeRMSE8: [String: Double] = [
        "tone": 18.8,
        "color": 7.0,
        "combined": 23.6,
        "intensity": 3.6,
    ]

    private struct Raster {
        let width: Int
        let height: Int
        let rgb: [Float]
    }

    private struct Metrics {
        let rmse8: Double
        let mae8: Double
        let maximumAbsolute8: Double

        var dictionary: [String: Any] {
            [
                "rmse8": rmse8,
                "mae8": mae8,
                "maximumAbsolute8": maximumAbsolute8,
            ]
        }
    }

    private struct StyleSetting {
        let tone: Double
        let color: Double
        let intensity: Double
        let cast: String
    }

    private struct ResponsePair {
        let name: String
        let minus: StyleSetting
        let midpoint: StyleSetting
        let plus: StyleSetting
    }

    private struct RenderRequest {
        let heicURL: URL
        let outputDirectory: URL
        let label: String
        let enabled: Bool
        let tone: Double
        let color: Double
        let intensity: Double
        let cast: String

        init(
            heicURL: URL,
            outputDirectory: URL,
            label: String,
            enabled: Bool,
            tone: Double = 0,
            color: Double = 0,
            intensity: Double = 1,
            cast: String = "Standard"
        ) {
            self.heicURL = heicURL
            self.outputDirectory = outputDirectory
            self.label = label
            self.enabled = enabled
            self.tone = tone
            self.color = color
            self.intensity = intensity
            self.cast = cast
        }

        var pngURL: URL { outputDirectory.appendingPathComponent("\(label).png") }
        var manifestURL: URL { outputDirectory.appendingPathComponent("\(label).json") }

        var dictionary: [String: Any] {
            [
                "photo": heicURL.path,
                "output": pngURL.path,
                "manifest": manifestURL.path,
                "tone": tone,
                "color": color,
                "intensity": intensity,
                "enabled": enabled,
                "maximumDimension": analysisMaximumDimension,
                "cast": cast,
            ]
        }
    }

    package func makeStyleData(
        preliminaryHEICURL: URL,
        identityStylePropertyList: Data,
        outputDirectory: URL
    ) throws -> AppleStyleDataResult {
        let admissionStartedAt = CFAbsoluteTimeGetCurrent()
        Self.solverAdmission.wait()
        let admissionWaitSeconds = CFAbsoluteTimeGetCurrent() - admissionStartedAt
        defer { Self.solverAdmission.signal() }
        let solverStartedAt = CFAbsoluteTimeGetCurrent()
        let fileManager = FileManager.default
        try fileManager.createDirectory(
            at: outputDirectory,
            withIntermediateDirectories: true
        )
        let executable = try AppleNativeToolchain.styleScenePayloadExecutable()
        let identityCoefficients = Array(
            repeating: 0.0,
            count: AppleStyleDataLayout.blockValueCount
        )
        let initializationDirectory = outputDirectory.appendingPathComponent("initialization")
        let identityHEICURL = try Self.materialize(
            baseHEICURL: preliminaryHEICURL,
            identityStylePropertyList: identityStylePropertyList,
            coefficientDeltas: identityCoefficients,
            outputDirectory: initializationDirectory,
            label: "identity"
        )
        let initialRasters = try Self.render(
            executable: executable,
            requests: [
                RenderRequest(
                    heicURL: preliminaryHEICURL,
                    outputDirectory: outputDirectory.appendingPathComponent("target"),
                    label: "disabled",
                    enabled: false
                ),
                RenderRequest(
                    heicURL: identityHEICURL,
                    outputDirectory: initializationDirectory,
                    label: "identity",
                    enabled: true
                ),
            ]
        )
        guard initialRasters.count == 2 else {
            throw CLIError.invalidContainer("constrained key-1 renderer returned an incomplete initial batch")
        }
        let target = initialRasters[0]
        let identityRender = (heicURL: identityHEICURL, raster: initialRasters[1])
        let identityMetrics = Self.metrics(identityRender.raster, target)
        let analyticCoefficients = try Self.fitGlobalPolynomial(
            sourceRGB8: identityRender.raster.rgb,
            targetRGB8: target.rgb
        )
        var coefficients = identityCoefficients
        var bestCoefficients = identityCoefficients
        var bestMetrics = identityMetrics
        var iterationRows: [[String: Any]] = []
        var initializationCandidates: [[String: Any]] = []

        var initializationWork: [(
            scale: Double,
            coefficients: [Double],
            request: RenderRequest
        )] = []
        for scale in Self.lineSearchScales {
            let candidate = analyticCoefficients.map { $0 * scale }
            let label = String(format: "global-quadratic-s%03d", Int(scale * 100))
            let heicURL = try Self.materialize(
                baseHEICURL: preliminaryHEICURL,
                identityStylePropertyList: identityStylePropertyList,
                coefficientDeltas: candidate,
                outputDirectory: initializationDirectory,
                label: label
            )
            initializationWork.append((
                scale: scale,
                coefficients: candidate,
                request: RenderRequest(
                    heicURL: heicURL,
                    outputDirectory: initializationDirectory,
                    label: label,
                    enabled: true
                )
            ))
        }
        let initializationRasters = try Self.render(
            executable: executable,
            requests: initializationWork.map(\.request)
        )
        for (work, raster) in zip(initializationWork, initializationRasters) {
            let candidateMetrics = Self.metrics(raster, target)
            initializationCandidates.append([
                "scale": work.scale,
                "metrics": candidateMetrics.dictionary,
            ])
            if candidateMetrics.rmse8 < bestMetrics.rmse8 {
                bestMetrics = candidateMetrics
                bestCoefficients = work.coefficients
                coefficients = work.coefficients
            }
        }
        let initializationCompletedAt = CFAbsoluteTimeGetCurrent()

        iterationRows.append([
            "stage": "analytic-global-quadratic-initialization",
            "identityMetrics": identityMetrics.dictionary,
            "lineSearchCandidates": initializationCandidates,
            "selectedMetrics": bestMetrics.dictionary,
            "accepted": bestMetrics.rmse8 < identityMetrics.rmse8,
            "coefficientDeltas": Self.coefficientDictionary(analyticCoefficients),
        ])

        for iteration in 0..<Self.iterationCount {
            let iterationDirectory = outputDirectory.appendingPathComponent(
                String(format: "iteration-%02d", iteration),
                isDirectory: true
            )
            let current = try Self.materializeAndRender(
                executable: executable,
                baseHEICURL: preliminaryHEICURL,
                identityStylePropertyList: identityStylePropertyList,
                coefficientDeltas: coefficients,
                outputDirectory: iterationDirectory,
                label: "current"
            )
            let beforeMetrics = Self.metrics(current.raster, target)
            if beforeMetrics.rmse8 < bestMetrics.rmse8 {
                bestMetrics = beforeMetrics
                bestCoefficients = coefficients
            }

            var derivativeWork: [(
                refinementIndex: Int,
                coefficientIndex: Int,
                step: Double,
                request: RenderRequest
            )] = []
            let jacobianDirectory = iterationDirectory.appendingPathComponent(
                "jacobian",
                isDirectory: true
            )
            for (refinementIndex, coefficientIndex) in Self.solverRefinementParameterIndices.enumerated() {
                var perturbed = coefficients
                let bound = Self.bound(forCoefficientIndex: coefficientIndex)
                var step = min(bound, coefficients[coefficientIndex] + Self.epsilon)
                    - coefficients[coefficientIndex]
                if step <= 0 {
                    step = max(-bound, coefficients[coefficientIndex] - Self.epsilon)
                        - coefficients[coefficientIndex]
                }
                perturbed[coefficientIndex] += step
                let label = Self.solverRefinementParameterNames[refinementIndex]
                let heicURL = try Self.materialize(
                    baseHEICURL: preliminaryHEICURL,
                    identityStylePropertyList: identityStylePropertyList,
                    coefficientDeltas: perturbed,
                    outputDirectory: jacobianDirectory,
                    label: label
                )
                derivativeWork.append((
                    refinementIndex: refinementIndex,
                    coefficientIndex: coefficientIndex,
                    step: step,
                    request: RenderRequest(
                        heicURL: heicURL,
                        outputDirectory: jacobianDirectory,
                        label: label,
                        enabled: true
                    )
                ))
            }
            let derivativeRasters = try Self.render(
                executable: executable,
                requests: derivativeWork.map(\.request)
            )
            var derivatives: [[Float]] = []
            var derivativeRows: [[String: Any]] = []
            for (work, rendered) in zip(derivativeWork, derivativeRasters) {
                guard rendered.rgb.count == current.raster.rgb.count else {
                    throw CLIError.invalidContainer(
                        "constrained key-1 renderer returned inconsistent raster dimensions"
                    )
                }
                let derivative = zip(rendered.rgb, current.raster.rgb).map {
                    ($0 - $1) / Float(work.step)
                }
                derivatives.append(derivative)
                let derivativeRMS = sqrt(
                    derivative.reduce(0.0) { $0 + Double($1 * $1) }
                        / Double(max(1, derivative.count))
                )
                derivativeRows.append([
                    "parameter": Self.solverRefinementParameterNames[work.refinementIndex],
                    "coefficientIndex": work.coefficientIndex,
                    "step": work.step,
                    "derivativeRMS8": derivativeRMS,
                    "metricsAgainstDisabled": Self.metrics(rendered, target).dictionary,
                ])
            }

            let update = try Self.solveUpdate(
                current: current.raster,
                target: target,
                derivatives: derivatives
            )
            var proposed = coefficients
            var proposedMetrics = beforeMetrics
            var selectedScale = 0.0
            var lineSearchRows: [[String: Any]] = []
            var lineSearchWork: [(
                scale: Double,
                coefficients: [Double],
                request: RenderRequest
            )] = []
            for scale in Self.lineSearchScales {
                var candidate = coefficients
                for (refinementIndex, coefficientIndex) in Self.solverRefinementParameterIndices.enumerated() {
                    let bound = Self.bound(forCoefficientIndex: coefficientIndex)
                    candidate[coefficientIndex] = min(
                        bound,
                        max(
                            -bound,
                            coefficients[coefficientIndex] + update[refinementIndex] * scale
                        )
                    )
                }
                let label = String(format: "proposed-s%03d", Int(scale * 100))
                let heicURL = try Self.materialize(
                    baseHEICURL: preliminaryHEICURL,
                    identityStylePropertyList: identityStylePropertyList,
                    coefficientDeltas: candidate,
                    outputDirectory: iterationDirectory,
                    label: label
                )
                lineSearchWork.append((
                    scale: scale,
                    coefficients: candidate,
                    request: RenderRequest(
                        heicURL: heicURL,
                        outputDirectory: iterationDirectory,
                        label: label,
                        enabled: true
                    )
                ))
            }
            let lineSearchRasters = try Self.render(
                executable: executable,
                requests: lineSearchWork.map(\.request)
            )
            for (work, raster) in zip(lineSearchWork, lineSearchRasters) {
                let candidateMetrics = Self.metrics(raster, target)
                lineSearchRows.append([
                    "scale": work.scale,
                    "metrics": candidateMetrics.dictionary,
                ])
                if candidateMetrics.rmse8 < proposedMetrics.rmse8 {
                    proposed = work.coefficients
                    proposedMetrics = candidateMetrics
                    selectedScale = work.scale
                }
            }
            let accepted = selectedScale > 0
            iterationRows.append([
                "iteration": iteration,
                "coefficientDeltasBefore": Self.coefficientDictionary(coefficients),
                "metricsBefore": beforeMetrics.dictionary,
                "jacobian": derivativeRows,
                "refinementUpdate": Self.refinementDictionary(update),
                "lineSearchCandidates": lineSearchRows,
                "selectedScale": selectedScale,
                "coefficientDeltasProposed": Self.coefficientDictionary(proposed),
                "metricsProposed": proposedMetrics.dictionary,
                "accepted": accepted,
            ])
            guard accepted else { break }
            coefficients = proposed
            if proposedMetrics.rmse8 < bestMetrics.rmse8 {
                bestMetrics = proposedMetrics
                bestCoefficients = proposed
            }
        }
        let refinementCompletedAt = CFAbsoluteTimeGetCurrent()

        guard bestCoefficients.contains(where: { abs($0) > 0 }),
              bestMetrics.rmse8 < identityMetrics.rmse8 * 0.98 else {
            let result: [String: Any] = [
                "schema": "xdremux-constrained-polynomial-key1-v4",
                "status": "rejected_no_improvement",
                "identityMetrics": identityMetrics.dictionary,
                "bestMetrics": bestMetrics.dictionary,
                "iterations": iterationRows,
            ]
            try Self.writeJSON(
                result,
                to: outputDirectory.appendingPathComponent("solver-result.json")
            )
            throw CLIError.invalidContainer(
                "constrained key-1 producer did not improve the complete-Neutrino neutral reconstruction; no identity fallback was applied"
            )
        }

        let styleData = try Self.styleData(coefficientDeltas: bestCoefficients)
        let finalLayout = try AppleStyleDataLayout.validate(styleData)
        let styleDataURL = outputDirectory.appendingPathComponent(
            "constrained-polynomial-style-data.f16.bin"
        )
        try styleData.write(to: styleDataURL, options: .atomic)
        let improvement = 1 - bestMetrics.rmse8 / identityMetrics.rmse8
        let responseEnvelope = try Self.validateResponseEnvelope(
            executable: executable,
            preliminaryHEICURL: preliminaryHEICURL,
            identityStylePropertyList: identityStylePropertyList,
            styleData: styleData,
            outputDirectory: outputDirectory.appendingPathComponent(
                "response-envelope",
                isDirectory: true
            )
        )
        let responseCompletedAt = CFAbsoluteTimeGetCurrent()
        print(String(
            format: "styles solver admissionWait=%.3fs initialization=%.3fs refinement=%.3fs response=%.3fs total=%.3fs",
            admissionWaitSeconds,
            initializationCompletedAt - solverStartedAt,
            refinementCompletedAt - initializationCompletedAt,
            responseCompletedAt - refinementCompletedAt,
            responseCompletedAt - solverStartedAt
        ))
        guard responseEnvelope["passed"] as? Bool == true else {
            let rejectedResult: [String: Any] = [
                "schema": "xdremux-constrained-polynomial-key1-v4",
                "status": "rejected_native_increment_envelope",
                "identityMetrics": identityMetrics.dictionary,
                "bestMetrics": bestMetrics.dictionary,
                "rmseImprovementFraction": improvement,
                "responseEnvelope": responseEnvelope,
                "iterations": iterationRows,
            ]
            try Self.writeJSON(
                rejectedResult,
                to: outputDirectory.appendingPathComponent("solver-result.json")
            )
            throw CLIError.invalidContainer(
                "constrained key-1 producer exceeded the native key-increment response envelope; no identity fallback was applied"
            )
        }
        let solverResult: [String: Any] = [
            "schema": "xdremux-constrained-polynomial-key1-v4",
            "status": "accepted",
            "evidence": "complete Neutrino composition",
            "parameterization": "global 10-term encoded-RGB quadratic polynomial repeated over 12x9x8, followed by twelve-direction complete-Neutrino linear-matrix refinement",
            "analysisMaximumDimension": Self.analysisMaximumDimension,
            "epsilon": Self.epsilon,
            "linearBound": Self.linearBound,
            "quadraticBound": Self.quadraticBound,
            "coefficientDeltas": Self.coefficientDictionary(bestCoefficients),
            "identityMetrics": identityMetrics.dictionary,
            "bestMetrics": bestMetrics.dictionary,
            "rmseImprovementFraction": improvement,
            "styleDataSHA256": sha256Hex(styleData),
            "responseEnvelope": responseEnvelope,
            "iterations": iterationRows,
        ]
        try Self.writeJSON(
            solverResult,
            to: outputDirectory.appendingPathComponent("solver-result.json")
        )
        let sourceData = try Data(
            contentsOf: preliminaryHEICURL,
            options: [.mappedIfSafe]
        )
        let targetPNG = outputDirectory
            .appendingPathComponent("target")
            .appendingPathComponent("disabled.png")
        return AppleStyleDataResult(
            styleData: styleData,
            styleDataSHA256: sha256Hex(styleData),
            polynomialCount: AppleStyleDataLayout.polynomialCount,
            blockValueCount: AppleStyleDataLayout.blockValueCount,
            tileCount: AppleStyleDataLayout.tileCount,
            producer: "constrainedSolver",
            producerVersion: "full-consumer-global-quadratic-v5",
            sourceSHA256: sha256Hex(sourceData),
            targetSHA256: sha256Hex(try Data(contentsOf: targetPNG)),
            sourceDomain: "complete Neutrino graph conditioned by this photo's Base, Gain Map, Linear Thumbnail, Style Delta, and semantic resources",
            targetDomain: "same photo rendered by complete Neutrino with SemanticStyle disabled",
            learnBufferKind: nil,
            solverKind: "global-rgb-quadratic-irls-consumer-linear-matrix-jacobian-native-response-shape-gate-v5",
            evidence: .completeNeutrinoConstrainedSolver,
            sceneMatched: true,
            identityFallback: false,
            fallbackKind: nil,
            reconstructionMetrics: [
                "status": "neutral_scene_matched",
                "identity": identityMetrics.dictionary,
                "selected": bestMetrics.dictionary,
                "rmseImprovementFraction": improvement,
                "coefficientDeltas": Self.coefficientDictionary(bestCoefficients),
                "styleDataLayout": finalLayout,
                "responseEnvelope": responseEnvelope,
            ],
            warnings: [
                "Global quadratic constrained solver selected; 12x9 local residual fitting and device HDR appearance remain separate acceptance gates."
            ]
        )
    }

    private static func materializeAndRender(
        executable: URL,
        baseHEICURL: URL,
        identityStylePropertyList: Data,
        coefficientDeltas: [Double],
        outputDirectory: URL,
        label: String
    ) throws -> (heicURL: URL, raster: Raster) {
        let heicURL = try materialize(
            baseHEICURL: baseHEICURL,
            identityStylePropertyList: identityStylePropertyList,
            coefficientDeltas: coefficientDeltas,
            outputDirectory: outputDirectory,
            label: label
        )
        let raster = try render(
            executable: executable,
            heicURL: heicURL,
            outputDirectory: outputDirectory,
            label: label,
            enabled: true
        )
        return (heicURL, raster)
    }

    private static func materialize(
        baseHEICURL: URL,
        identityStylePropertyList: Data,
        coefficientDeltas: [Double],
        outputDirectory: URL,
        label: String
    ) throws -> URL {
        try FileManager.default.createDirectory(
            at: outputDirectory,
            withIntermediateDirectories: true
        )
        let key = try styleData(coefficientDeltas: coefficientDeltas)
        let keyURL = outputDirectory.appendingPathComponent("\(label).f16.bin")
        try key.write(to: keyURL, options: .atomic)
        let heicURL: URL
        if key == (try AppleStyleDataLayout.completeIdentity()) {
            heicURL = baseHEICURL
        } else {
            heicURL = outputDirectory.appendingPathComponent("\(label).heic")
            try injectStyleData(
                key,
                into: baseHEICURL,
                identityStylePropertyList: identityStylePropertyList,
                outputURL: heicURL
            )
        }
        return heicURL
    }

    private static func render(
        executable: URL,
        heicURL: URL,
        outputDirectory: URL,
        label: String,
        enabled: Bool,
        tone: Double = 0,
        color: Double = 0,
        intensity: Double = 1,
        cast: String = "Standard"
    ) throws -> Raster {
        let rasters = try render(
            executable: executable,
            requests: [
                RenderRequest(
                    heicURL: heicURL,
                    outputDirectory: outputDirectory,
                    label: label,
                    enabled: enabled,
                    tone: tone,
                    color: color,
                    intensity: intensity,
                    cast: cast
                ),
            ]
        )
        guard let raster = rasters.first else {
            throw CLIError.invalidContainer("complete-Neutrino renderer returned an empty batch")
        }
        return raster
    }

    private static func render(
        executable: URL,
        requests: [RenderRequest]
    ) throws -> [Raster] {
        guard !requests.isEmpty else { return [] }
        for request in requests {
            try FileManager.default.createDirectory(
                at: request.outputDirectory,
                withIntermediateDirectories: true
            )
        }
        let planURL = FileManager.default.temporaryDirectory.appendingPathComponent(
            "xdremux-neutrino-style-render-batch-\(UUID().uuidString).json"
        )
        defer { try? FileManager.default.removeItem(at: planURL) }
        let plan: [String: Any] = [
            "schema": "xdremux-neutrino-style-render-batch-v1",
            "requests": requests.map(\.dictionary),
        ]
        let planData = try JSONSerialization.data(
            withJSONObject: plan,
            options: [.sortedKeys, .withoutEscapingSlashes]
        )
        try planData.write(to: planURL, options: .atomic)
        let process = try AppleNativeToolchain.run(
            executable,
            arguments: ["--render-style-batch", planURL.path],
            timeout: 180
        )
        let result = try? JSONSerialization.jsonObject(with: process.stdout) as? [String: Any]
        guard !process.timedOut,
              process.status == 0,
              result?["passed"] as? Bool == true else {
            let stderr = String(data: process.stderr, encoding: .utf8) ?? ""
            let stdout = String(data: process.stdout, encoding: .utf8) ?? ""
            throw CLIError.invalidContainer(
                "complete-Neutrino key-1 calibration render batch failed: "
                    + (process.timedOut ? "renderer exceeded 180 seconds; " : "")
                    + [stderr, stdout]
                        .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
                        .filter { !$0.isEmpty }
                        .joined(separator: " ")
            )
        }
        return try requests.map { try decodeRGB8($0.pngURL) }
    }

    private static func decodeRGB8(_ url: URL) throws -> Raster {
        guard let source = CGImageSourceCreateWithURL(url as CFURL, nil),
              let image = CGImageSourceCreateImageAtIndex(source, 0, nil) else {
            throw CLIError.invalidContainer("cannot decode constrained key-1 render")
        }
        let width = image.width
        let height = image.height
        var rgba = Data(count: width * height * 4)
        let colorSpace = CGColorSpace(name: CGColorSpace.displayP3)
            ?? CGColorSpaceCreateDeviceRGB()
        let created = rgba.withUnsafeMutableBytes { raw -> Bool in
            guard let address = raw.baseAddress,
                  let context = CGContext(
                    data: address,
                    width: width,
                    height: height,
                    bitsPerComponent: 8,
                    bytesPerRow: width * 4,
                    space: colorSpace,
                    bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
                  ) else { return false }
            context.draw(image, in: CGRect(x: 0, y: 0, width: width, height: height))
            return true
        }
        guard created else {
            throw CLIError.invalidContainer("cannot rasterize constrained key-1 render")
        }
        var rgb = [Float]()
        rgb.reserveCapacity(width * height * 3)
        for offset in stride(from: 0, to: rgba.count, by: 4) {
            rgb.append(Float(rgba[offset]))
            rgb.append(Float(rgba[offset + 1]))
            rgb.append(Float(rgba[offset + 2]))
        }
        return Raster(width: width, height: height, rgb: rgb)
    }

    private static func validateResponseEnvelope(
        executable: URL,
        preliminaryHEICURL: URL,
        identityStylePropertyList: Data,
        styleData: Data,
        outputDirectory: URL
    ) throws -> [String: Any] {
        try FileManager.default.createDirectory(
            at: outputDirectory,
            withIntermediateDirectories: true
        )
        let selectedHEICURL = outputDirectory.appendingPathComponent("selected-key.heic")
        try injectStyleData(
            styleData,
            into: preliminaryHEICURL,
            identityStylePropertyList: identityStylePropertyList,
            outputURL: selectedHEICURL
        )
        let standardMinus = StyleSetting(tone: -1, color: 0, intensity: 1, cast: "Standard")
        let standardPlus = StyleSetting(tone: 1, color: 0, intensity: 1, cast: "Standard")
        let pairs = [
            ResponsePair(
                name: "tone",
                minus: standardMinus,
                midpoint: StyleSetting(tone: 0, color: 0, intensity: 1, cast: "Standard"),
                plus: standardPlus
            ),
            ResponsePair(
                name: "color",
                minus: StyleSetting(tone: 0, color: -1, intensity: 1, cast: "Standard"),
                midpoint: StyleSetting(tone: 0, color: 0, intensity: 1, cast: "Standard"),
                plus: StyleSetting(tone: 0, color: 1, intensity: 1, cast: "Standard")
            ),
            ResponsePair(
                name: "combined",
                minus: StyleSetting(tone: -1, color: -1, intensity: 1, cast: "Standard"),
                midpoint: StyleSetting(tone: 0, color: 0, intensity: 1, cast: "Standard"),
                plus: StyleSetting(tone: 1, color: 1, intensity: 1, cast: "Standard")
            ),
            ResponsePair(
                name: "intensity",
                minus: StyleSetting(tone: 0, color: 0, intensity: 0, cast: "Cool"),
                midpoint: StyleSetting(tone: 0, color: 0, intensity: 0.5, cast: "Cool"),
                plus: StyleSetting(tone: 0, color: 0, intensity: 1, cast: "Cool")
            ),
        ]
        var rows: [[String: Any]] = []
        var failures: [String] = []
        var renderCache: [String: Raster] = [:]
        var renderWork: [(cacheKey: String, request: RenderRequest)] = []

        func cacheKey(owner: String, setting: StyleSetting) -> String {
            [
                owner,
                String(setting.tone),
                String(setting.color),
                String(setting.intensity),
                setting.cast,
            ].joined(separator: "|")
        }

        func register(
            _ heicURL: URL,
            owner: String,
            pairName: String,
            side: String,
            setting: StyleSetting
        ) {
            let key = cacheKey(owner: owner, setting: setting)
            guard !renderWork.contains(where: { $0.cacheKey == key }) else { return }
            renderWork.append((
                cacheKey: key,
                request: RenderRequest(
                    heicURL: heicURL,
                    outputDirectory: outputDirectory.appendingPathComponent(owner),
                    label: "\(pairName)-\(side)",
                    enabled: true,
                    tone: setting.tone,
                    color: setting.color,
                    intensity: setting.intensity,
                    cast: setting.cast
                )
            ))
        }

        for pair in pairs {
            for (side, setting) in [
                ("minus", pair.minus),
                ("midpoint", pair.midpoint),
                ("plus", pair.plus),
            ] {
                register(
                    selectedHEICURL,
                    owner: "candidate",
                    pairName: pair.name,
                    side: side,
                    setting: setting
                )
                register(
                    preliminaryHEICURL,
                    owner: "identity",
                    pairName: pair.name,
                    side: side,
                    setting: setting
                )
            }
        }
        let responseRasters = try render(
            executable: executable,
            requests: renderWork.map(\.request)
        )
        for (work, raster) in zip(renderWork, responseRasters) {
            renderCache[work.cacheKey] = raster
        }

        func rendered(
            _ heicURL: URL,
            owner: String,
            pairName: String,
            side: String,
            setting: StyleSetting
        ) throws -> Raster {
            let key = cacheKey(owner: owner, setting: setting)
            guard let cached = renderCache[key] else {
                throw CLIError.invalidContainer(
                    "missing batched native response render \(owner)/\(pairName)-\(side) for \(heicURL.lastPathComponent)"
                )
            }
            return cached
        }

        for pair in pairs {
            let candidateMinus = try rendered(
                selectedHEICURL,
                owner: "candidate",
                pairName: pair.name,
                side: "minus",
                setting: pair.minus
            )
            let candidateMidpoint = try rendered(
                selectedHEICURL,
                owner: "candidate",
                pairName: pair.name,
                side: "midpoint",
                setting: pair.midpoint
            )
            let candidatePlus = try rendered(
                selectedHEICURL,
                owner: "candidate",
                pairName: pair.name,
                side: "plus",
                setting: pair.plus
            )
            let identityMinus = try rendered(
                preliminaryHEICURL,
                owner: "identity",
                pairName: pair.name,
                side: "minus",
                setting: pair.minus
            )
            let identityMidpoint = try rendered(
                preliminaryHEICURL,
                owner: "identity",
                pairName: pair.name,
                side: "midpoint",
                setting: pair.midpoint
            )
            let identityPlus = try rendered(
                preliminaryHEICURL,
                owner: "identity",
                pairName: pair.name,
                side: "plus",
                setting: pair.plus
            )
            let endpointIncrement = try incrementalResponseMetrics(
                candidateMinus: candidateMinus,
                candidatePlus: candidatePlus,
                identityMinus: identityMinus,
                identityPlus: identityPlus
            )
            let lowerSegmentIncrement = try incrementalResponseMetrics(
                candidateMinus: candidateMinus,
                candidatePlus: candidateMidpoint,
                identityMinus: identityMinus,
                identityPlus: identityMidpoint
            )
            let upperSegmentIncrement = try incrementalResponseMetrics(
                candidateMinus: candidateMidpoint,
                candidatePlus: candidatePlus,
                identityMinus: identityMidpoint,
                identityPlus: identityPlus
            )
            let curvatureIncrement = try incrementalCurvatureMetrics(
                candidateMinus: candidateMinus,
                candidateMidpoint: candidateMidpoint,
                candidatePlus: candidatePlus,
                identityMinus: identityMinus,
                identityMidpoint: identityMidpoint,
                identityPlus: identityPlus
            )
            let endpointLimit = nativeIncrementEnvelopeRMSE8[pair.name]!
            let segmentLimit = nativeIncrementSegmentEnvelopeRMSE8[pair.name]!
            let curvatureLimit = nativeIncrementCurvatureEnvelopeRMSE8[pair.name]!
            if endpointIncrement.rmse8 > endpointLimit {
                failures.append(
                    String(
                        format: "%@ key increment RMSE %.4f exceeds native envelope %.4f",
                        pair.name,
                        endpointIncrement.rmse8,
                        endpointLimit
                    )
                )
            }
            for (side, metrics) in [
                ("lower", lowerSegmentIncrement),
                ("upper", upperSegmentIncrement),
            ] where metrics.rmse8 > segmentLimit {
                failures.append(
                    String(
                        format: "%@ %@-segment key increment RMSE %.4f exceeds native envelope %.4f",
                        pair.name,
                        side,
                        metrics.rmse8,
                        segmentLimit
                    )
                )
            }
            if curvatureIncrement.rmse8 > curvatureLimit {
                failures.append(
                    String(
                        format: "%@ key-increment curvature RMSE %.4f exceeds native envelope %.4f",
                        pair.name,
                        curvatureIncrement.rmse8,
                        curvatureLimit
                    )
                )
            }
            let minusHealth = rasterHealth(candidateMinus)
            let midpointHealth = rasterHealth(candidateMidpoint)
            let plusHealth = rasterHealth(candidatePlus)
            let meanLumaDelta8 = (plusHealth["meanLuma8"] as? Double ?? 0)
                - (minusHealth["meanLuma8"] as? Double ?? 0)
            let meanChromaDelta8 = (plusHealth["meanChroma8"] as? Double ?? 0)
                - (minusHealth["meanChroma8"] as? Double ?? 0)
            let directionPassed = responseDirectionPassed(
                name: pair.name,
                meanLumaDelta8: meanLumaDelta8,
                meanChromaDelta8: meanChromaDelta8
            )
            if !directionPassed {
                failures.append(
                    String(
                        format: "%@ has wrong response direction (luma delta %.4f, chroma delta %.4f)",
                        pair.name,
                        meanLumaDelta8,
                        meanChromaDelta8
                    )
                )
            }
            for (side, health) in [
                ("minus", minusHealth),
                ("midpoint", midpointHealth),
                ("plus", plusHealth),
            ] {
                let black = health["blackFraction"] as? Double ?? 1
                let white = health["whiteFraction"] as? Double ?? 1
                let luma = health["meanLuma8"] as? Double ?? 0
                if black > 0.95 || white > 0.95 || luma < 1 || luma > 254 {
                    failures.append("\(pair.name) \(side) endpoint is catastrophically clipped")
                }
            }
            rows.append([
                "name": pair.name,
                "minus": [
                    "tone": pair.minus.tone,
                    "color": pair.minus.color,
                    "intensity": pair.minus.intensity,
                    "cast": pair.minus.cast,
                ],
                "midpoint": [
                    "tone": pair.midpoint.tone,
                    "color": pair.midpoint.color,
                    "intensity": pair.midpoint.intensity,
                    "cast": pair.midpoint.cast,
                ],
                "plus": [
                    "tone": pair.plus.tone,
                    "color": pair.plus.color,
                    "intensity": pair.plus.intensity,
                    "cast": pair.plus.cast,
                ],
                "candidateMinusHealth": minusHealth,
                "candidateMidpointHealth": midpointHealth,
                "candidatePlusHealth": plusHealth,
                "direction": [
                    "meanLumaDelta8": meanLumaDelta8,
                    "meanChromaDelta8": meanChromaDelta8,
                    "criterion": directionCriterion(for: pair.name),
                    "passed": directionPassed,
                ],
                "endpointKeyIncrement": endpointIncrement.dictionary,
                "lowerSegmentKeyIncrement": lowerSegmentIncrement.dictionary,
                "upperSegmentKeyIncrement": upperSegmentIncrement.dictionary,
                "keyIncrementCurvature": curvatureIncrement.dictionary,
                "nativeEndpointEnvelopeRMSE8": endpointLimit,
                "nativeSegmentEnvelopeRMSE8": segmentLimit,
                "nativeCurvatureEnvelopeRMSE8": curvatureLimit,
                "passed": endpointIncrement.rmse8 <= endpointLimit
                    && lowerSegmentIncrement.rmse8 <= segmentLimit
                    && upperSegmentIncrement.rmse8 <= segmentLimit
                    && curvatureIncrement.rmse8 <= curvatureLimit
                    && directionPassed,
            ])
        }
        let result: [String: Any] = [
            "schema": "xdremux-key1-native-increment-response-gate-v2",
            "evidence": "complete Neutrino composition",
            "reference": "two Apple native key-only identity injection controls with a conservative guard band",
            "comparison": "candidate-minus-identity key increment across endpoints, each half segment, and the midpoint second difference",
            "passed": failures.isEmpty,
            "failures": failures,
            "pairs": rows,
        ]
        try writeJSON(
            result,
            to: outputDirectory.appendingPathComponent("response-envelope.json")
        )
        return result
    }

    private static func incrementalResponseMetrics(
        candidateMinus: Raster,
        candidatePlus: Raster,
        identityMinus: Raster,
        identityPlus: Raster
    ) throws -> Metrics {
        let count = candidateMinus.rgb.count
        guard count > 0,
              candidatePlus.rgb.count == count,
              identityMinus.rgb.count == count,
              identityPlus.rgb.count == count else {
            throw CLIError.invalidContainer(
                "native response envelope renders have inconsistent dimensions"
            )
        }
        var squared = 0.0
        var absolute = 0.0
        var maximum = 0.0
        for index in 0..<count {
            let candidateResponse = candidatePlus.rgb[index] - candidateMinus.rgb[index]
            let identityResponse = identityPlus.rgb[index] - identityMinus.rgb[index]
            let difference = Double(candidateResponse - identityResponse)
            squared += difference * difference
            absolute += abs(difference)
            maximum = max(maximum, abs(difference))
        }
        return Metrics(
            rmse8: sqrt(squared / Double(count)),
            mae8: absolute / Double(count),
            maximumAbsolute8: maximum
        )
    }

    private static func incrementalCurvatureMetrics(
        candidateMinus: Raster,
        candidateMidpoint: Raster,
        candidatePlus: Raster,
        identityMinus: Raster,
        identityMidpoint: Raster,
        identityPlus: Raster
    ) throws -> Metrics {
        let count = candidateMinus.rgb.count
        guard count > 0,
              candidateMidpoint.rgb.count == count,
              candidatePlus.rgb.count == count,
              identityMinus.rgb.count == count,
              identityMidpoint.rgb.count == count,
              identityPlus.rgb.count == count else {
            throw CLIError.invalidContainer(
                "native response curvature renders have inconsistent dimensions"
            )
        }
        var squared = 0.0
        var absolute = 0.0
        var maximum = 0.0
        for index in 0..<count {
            let candidateCurvature = candidatePlus.rgb[index]
                - 2 * candidateMidpoint.rgb[index]
                + candidateMinus.rgb[index]
            let identityCurvature = identityPlus.rgb[index]
                - 2 * identityMidpoint.rgb[index]
                + identityMinus.rgb[index]
            let difference = Double(candidateCurvature - identityCurvature)
            squared += difference * difference
            absolute += abs(difference)
            maximum = max(maximum, abs(difference))
        }
        return Metrics(
            rmse8: sqrt(squared / Double(count)),
            mae8: absolute / Double(count),
            maximumAbsolute8: maximum
        )
    }

    package static func incrementalResponseRMSE8(
        candidateMinus: [Float],
        candidatePlus: [Float],
        identityMinus: [Float],
        identityPlus: [Float]
    ) throws -> Double {
        let count = candidateMinus.count
        guard count > 0,
              candidatePlus.count == count,
              identityMinus.count == count,
              identityPlus.count == count,
              candidateMinus.allSatisfy(\.isFinite),
              candidatePlus.allSatisfy(\.isFinite),
              identityMinus.allSatisfy(\.isFinite),
              identityPlus.allSatisfy(\.isFinite) else {
            throw CLIError.invalidContainer("invalid native response envelope vectors")
        }
        var squared = 0.0
        for index in 0..<count {
            let candidateResponse = candidatePlus[index] - candidateMinus[index]
            let identityResponse = identityPlus[index] - identityMinus[index]
            let difference = Double(candidateResponse - identityResponse)
            squared += difference * difference
        }
        return sqrt(squared / Double(count))
    }

    package static func incrementalCurvatureRMSE8(
        candidateMinus: [Float],
        candidateMidpoint: [Float],
        candidatePlus: [Float],
        identityMinus: [Float],
        identityMidpoint: [Float],
        identityPlus: [Float]
    ) throws -> Double {
        let count = candidateMinus.count
        let vectors = [
            candidateMinus,
            candidateMidpoint,
            candidatePlus,
            identityMinus,
            identityMidpoint,
            identityPlus,
        ]
        guard count > 0,
              vectors.allSatisfy({ $0.count == count && $0.allSatisfy(\.isFinite) }) else {
            throw CLIError.invalidContainer("invalid native response curvature vectors")
        }
        var squared = 0.0
        for index in 0..<count {
            let candidateCurvature = candidatePlus[index]
                - 2 * candidateMidpoint[index]
                + candidateMinus[index]
            let identityCurvature = identityPlus[index]
                - 2 * identityMidpoint[index]
                + identityMinus[index]
            let difference = Double(candidateCurvature - identityCurvature)
            squared += difference * difference
        }
        return sqrt(squared / Double(count))
    }

    package static func responseDirectionPassed(
        name: String,
        meanLumaDelta8: Double,
        meanChromaDelta8: Double
    ) -> Bool {
        guard meanLumaDelta8.isFinite, meanChromaDelta8.isFinite else { return false }
        switch name {
        case "tone":
            return meanLumaDelta8 > 0
        case "color":
            return meanChromaDelta8 > 0
        case "combined":
            return meanLumaDelta8 > 0 && meanChromaDelta8 > 0
        case "intensity":
            return true
        default:
            return false
        }
    }

    private static func directionCriterion(for name: String) -> String {
        switch name {
        case "tone":
            return "plus endpoint mean luma must exceed minus endpoint mean luma"
        case "color":
            return "plus endpoint mean chroma must exceed minus endpoint mean chroma"
        case "combined":
            return "plus endpoint mean luma and mean chroma must exceed minus endpoint"
        case "intensity":
            return "no scalar direction criterion; native increment shape only"
        default:
            return "unknown response family"
        }
    }

    private static func rasterHealth(_ raster: Raster) -> [String: Any] {
        var black = 0
        var white = 0
        var luma = 0.0
        var chroma = 0.0
        let pixelCount = raster.rgb.count / 3
        for pixel in 0..<pixelCount {
            let offset = pixel * 3
            let red = Double(raster.rgb[offset])
            let green = Double(raster.rgb[offset + 1])
            let blue = Double(raster.rgb[offset + 2])
            if red <= 1, green <= 1, blue <= 1 { black += 1 }
            if red >= 254, green >= 254, blue >= 254 { white += 1 }
            luma += red * 0.22897456 + green * 0.69173852 + blue * 0.07928691
            chroma += max(red, max(green, blue)) - min(red, min(green, blue))
        }
        let denominator = Double(max(1, pixelCount))
        return [
            "meanLuma8": luma / denominator,
            "meanChroma8": chroma / denominator,
            "blackFraction": Double(black) / denominator,
            "whiteFraction": Double(white) / denominator,
        ]
    }

    package static func styleData(parameters: [Double]) throws -> Data {
        guard parameters.count == directParameterIndices.count else {
            throw CLIError.invalidContainer("invalid constrained key-1 parameter vector")
        }
        var coefficientDeltas = Array(
            repeating: 0.0,
            count: AppleStyleDataLayout.blockValueCount
        )
        for (parameterIndex, coefficientIndex) in directParameterIndices.enumerated() {
            coefficientDeltas[coefficientIndex] = parameters[parameterIndex]
        }
        return try styleData(coefficientDeltas: coefficientDeltas)
    }

    package static func styleData(coefficientDeltas: [Double]) throws -> Data {
        guard coefficientDeltas.count == AppleStyleDataLayout.blockValueCount else {
            throw CLIError.invalidContainer("invalid constrained key-1 coefficient vector")
        }
        for index in coefficientDeltas.indices {
            guard coefficientDeltas[index].isFinite,
                  abs(coefficientDeltas[index]) <= bound(forCoefficientIndex: index) + 1e-9 else {
                throw CLIError.invalidContainer("invalid constrained key-1 coefficient vector")
            }
        }
        var block = [Float](repeating: 0, count: AppleStyleDataLayout.blockValueCount)
        for index in AppleStyleDataLayout.identityIndices { block[index] = 1 }
        for index in coefficientDeltas.indices {
            block[index] += Float(coefficientDeltas[index])
        }
        var result = Data()
        result.reserveCapacity(AppleStyleDataLayout.byteCount)
        for _ in 0..<AppleStyleDataLayout.tileCount {
            for value in block {
                var bits = Float16(value).bitPattern.littleEndian
                withUnsafeBytes(of: &bits) { result.append(contentsOf: $0) }
            }
        }
        _ = try AppleStyleDataLayout.validate(result)
        return result
    }

    package static func fitGlobalPolynomial(
        sourceRGB8: [Float],
        targetRGB8: [Float]
    ) throws -> [Double] {
        guard sourceRGB8.count == targetRGB8.count,
              sourceRGB8.count >= 3,
              sourceRGB8.count.isMultiple(of: 3),
              sourceRGB8.allSatisfy(\.isFinite),
              targetRGB8.allSatisfy(\.isFinite) else {
            throw CLIError.invalidContainer(
                "invalid raster pair for constrained global polynomial fit"
            )
        }
        let termCount = AppleStyleDataLayout.polynomialCount
        var coefficients = Array(
            repeating: 0.0,
            count: AppleStyleDataLayout.blockValueCount
        )
        let pixelCount = sourceRGB8.count / 3
        let pixelStride = max(1, pixelCount / 100_000)
        let sampledPixelCount = (pixelCount + pixelStride - 1) / pixelStride
        var basisValues = Array(
            repeating: 0.0,
            count: sampledPixelCount * termCount
        )
        sourceRGB8.withUnsafeBufferPointer { source in
            basisValues.withUnsafeMutableBufferPointer { basis in
                for sampledPixel in 0..<sampledPixelCount {
                    let sourceOffset = sampledPixel * pixelStride * 3
                    let basisOffset = sampledPixel * termCount
                    let red = Double(source[sourceOffset]) / 255.0
                    let green = Double(source[sourceOffset + 1]) / 255.0
                    let blue = Double(source[sourceOffset + 2]) / 255.0
                    basis[basisOffset] = 1
                    basis[basisOffset + 1] = red
                    basis[basisOffset + 2] = green
                    basis[basisOffset + 3] = blue
                    basis[basisOffset + 4] = red * red
                    basis[basisOffset + 5] = red * green
                    basis[basisOffset + 6] = red * blue
                    basis[basisOffset + 7] = green * green
                    basis[basisOffset + 8] = green * blue
                    basis[basisOffset + 9] = blue * blue
                }
            }
        }

        for _ in 0..<3 {
            for output in 0..<3 {
                var normal = Array(repeating: 0.0, count: termCount * termCount)
                var rightHandSide = Array(repeating: 0.0, count: termCount)
                sourceRGB8.withUnsafeBufferPointer { source in
                    targetRGB8.withUnsafeBufferPointer { target in
                        basisValues.withUnsafeBufferPointer { basis in
                            coefficients.withUnsafeBufferPointer { coefficientBuffer in
                                normal.withUnsafeMutableBufferPointer { normalBuffer in
                                    rightHandSide.withUnsafeMutableBufferPointer { rhs in
                                        for sampledPixel in 0..<sampledPixelCount {
                                            let sourceOffset = sampledPixel * pixelStride * 3
                                            let basisOffset = sampledPixel * termCount
                                            let observed = Double(
                                                target[sourceOffset + output]
                                                    - source[sourceOffset + output]
                                            ) / 255.0
                                            var predicted = 0.0
                                            for term in 0..<termCount {
                                                predicted += basis[basisOffset + term]
                                                    * coefficientBuffer[term * 3 + output]
                                            }
                                            let residual = observed - predicted
                                            let huberThreshold = 4.0 / 255.0
                                            var weight = min(
                                                1.0,
                                                huberThreshold / max(huberThreshold, abs(residual))
                                            )
                                            if source[sourceOffset + output] <= 2
                                                || source[sourceOffset + output] >= 253 {
                                                weight *= 0.25
                                            }
                                            for row in 0..<termCount {
                                                let rowValue = basis[basisOffset + row]
                                                rhs[row] += weight * rowValue * observed
                                                let normalRowOffset = row * termCount
                                                for column in row..<termCount {
                                                    normalBuffer[normalRowOffset + column] += weight
                                                        * rowValue * basis[basisOffset + column]
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                for row in 0..<termCount {
                    for column in 0..<row {
                        normal[row * termCount + column] = normal[column * termCount + row]
                    }
                }
                let trace = (0..<termCount).reduce(0.0) {
                    $0 + normal[$1 * termCount + $1]
                }
                let ridge = max(trace / Double(termCount) * 1e-5, 1e-9)
                for term in 0..<termCount {
                    normal[term * termCount + term] += ridge * (term >= 4 ? 10 : 1)
                }
                let normalMatrix = (0..<termCount).map { row in
                    Array(normal[(row * termCount)..<((row + 1) * termCount)])
                }
                let solution = try solveLinearSystem(normalMatrix, rightHandSide)
                for term in 0..<termCount {
                    let coefficientIndex = term * 3 + output
                    let bound = bound(forCoefficientIndex: coefficientIndex)
                    coefficients[coefficientIndex] = min(
                        bound,
                        max(-bound, solution[term])
                    )
                }
            }
        }
        return coefficients
    }

    private static func bound(forCoefficientIndex index: Int) -> Double {
        index / 3 >= 4 ? quadraticBound : linearBound
    }

    private static func injectStyleData(
        _ styleData: Data,
        into heicURL: URL,
        identityStylePropertyList: Data,
        outputURL: URL
    ) throws {
        let identity = try AppleStyleDataLayout.completeIdentity()
        guard uniqueRange(of: identity, in: identityStylePropertyList) != nil else {
            throw CLIError.invalidContainer(
                "identity key 1 does not occur exactly once in the preliminary style plist"
            )
        }
        var replacementPropertyList = identityStylePropertyList
        replacementPropertyList.replaceSubrange(
            uniqueRange(of: identity, in: identityStylePropertyList)!,
            with: styleData
        )
        let source = try Data(contentsOf: heicURL, options: [.mappedIfSafe])
        guard let range = uniqueRange(of: identityStylePropertyList, in: source) else {
            throw CLIError.invalidContainer(
                "preliminary style plist does not occur exactly once in the HEIC"
            )
        }
        var output = source
        output.replaceSubrange(range, with: replacementPropertyList)
        guard output.count == source.count else {
            throw CLIError.invalidContainer("key-1 injection changed HEIC byte length")
        }
        try output.write(to: outputURL, options: .atomic)
    }

    private static func uniqueRange(of needle: Data, in haystack: Data) -> Range<Data.Index>? {
        guard let first = haystack.range(of: needle) else { return nil }
        let remainder = first.upperBound..<haystack.endIndex
        guard haystack.range(of: needle, in: remainder) == nil else { return nil }
        return first
    }

    private static func metrics(_ left: Raster, _ right: Raster) -> Metrics {
        precondition(left.width == right.width && left.height == right.height)
        var squared = 0.0
        var absolute = 0.0
        var maximum = 0.0
        for index in left.rgb.indices {
            let difference = Double(left.rgb[index] - right.rgb[index])
            squared += difference * difference
            absolute += abs(difference)
            maximum = max(maximum, abs(difference))
        }
        let count = Double(max(1, left.rgb.count))
        return Metrics(
            rmse8: sqrt(squared / count),
            mae8: absolute / count,
            maximumAbsolute8: maximum
        )
    }

    private static func solveUpdate(
        current: Raster,
        target: Raster,
        derivatives: [[Float]]
    ) throws -> [Double] {
        let count = solverRefinementParameterNames.count
        guard derivatives.count == count,
              derivatives.allSatisfy({ $0.count == current.rgb.count }),
              target.rgb.count == current.rgb.count else {
            throw CLIError.invalidContainer("invalid constrained key-1 Jacobian")
        }
        var normal = Array(repeating: Array(repeating: 0.0, count: count), count: count)
        var gradient = Array(repeating: 0.0, count: count)
        let stride = max(1, current.rgb.count / (50_000 * 3))
        for pixel in Swift.stride(from: 0, to: current.rgb.count / 3, by: stride) {
            for channel in 0..<3 {
                let sample = pixel * 3 + channel
                let residual = Double(target.rgb[sample] - current.rgb[sample])
                let huberWeight = min(1.0, 12.0 / max(12.0, abs(residual)))
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
        for row in 0..<count {
            for column in 0..<row { normal[row][column] = normal[column][row] }
        }
        let trace = (0..<count).reduce(0.0) { $0 + normal[$1][$1] }
        let ridge = max(trace / Double(count) * 1e-6, 1e-9)
        for index in 0..<count { normal[index][index] += ridge }
        var solution = try solveLinearSystem(normal, gradient)
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
                throw CLIError.invalidContainer("constrained key-1 Jacobian is singular")
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

    private static func refinementDictionary(_ values: [Double]) -> [String: Double] {
        Dictionary(uniqueKeysWithValues: zip(solverRefinementParameterNames, values))
    }

    private static func coefficientDictionary(_ values: [Double]) -> [String: Double] {
        var result: [String: Double] = [:]
        for term in 0..<basisNames.count {
            for output in 0..<outputNames.count {
                result["\(basisNames[term])->\(outputNames[output])"] = values[term * 3 + output]
            }
        }
        return result
    }

    private static func writeJSON(_ value: [String: Any], to url: URL) throws {
        let data = try JSONSerialization.data(
            withJSONObject: value,
            options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        )
        try data.write(to: url, options: .atomic)
    }
}
