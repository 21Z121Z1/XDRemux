import ArgumentParser
import Foundation
import XDRemuxCore

private struct ResolvedCommonConversionArguments {
    let family: Family
    let debugDirectoryPath: String?
    let oppoCompatibility: OppoCompatibility
    let inputProcessingBranch: InputProcessingBranch
    let appleFeatures: AppleFeatureOptions
    let appleStylesRawDNGPath: String?
    let appleStyleDataProducer: AppleStyleDataProducerMode
    let oppoCameraTail: OppoCameraTail
    let tmapFormat: TmapFormat
}

private struct CommonConversionArguments: ParsableArguments {
    @Flag(
        name: [.customLong("apple-photographic-styles"), .customLong("apple-styles")],
        help: "Generate Apple Photographic Styles metadata."
    )
    var applePhotographicStylesEnabled = false

    @Option(
        name: .customLong("apple-style-data-producer"),
        help: "Style-data producer: constrained-solver, learn-node, or identity-fallback."
    )
    var appleStyleDataProducerRaw: String?

    @Flag(name: .customLong("apple-portrait"), help: "Generate Apple Portrait metadata.")
    var applePortraitEnabled = false

    @Option(name: .customLong("apple-styles-raw-dng"), help: "Matching RAW DNG for Styles analysis.")
    var appleStylesRawDNGPath: String?

    @Option(name: .customLong("family"), help: "Source ProXDR family: auto, x6, or x7.")
    var familyRaw = Family.auto.rawValue

    @Option(
        name: .customLong("input-processing"),
        help: "Input processing branch: system, system-decoded, hybrid, or passthrough."
    )
    var inputProcessingRaw = InputProcessingBranch.hybrid.rawValue

    @Option(name: .customLong("debug-dir"), help: "Directory for retained diagnostic artifacts.")
    var debugDirectoryPath: String?

    @Option(name: .customLong("oppo-camera-tail"), help: "OPPO private-tail preservation policy.")
    var oppoCameraTailRaw: String?

    @Option(name: .customLong("tmap-format"), help: "Tone-map metadata format: imageio or strict.")
    var tmapFormatRaw = TmapFormat.imageIO.rawValue

    @Option(
        name: .customLong("oppo-compat"),
        defaultAsFlag: OppoCompatibility.on.rawValue,
        help: "Fine-grained OPPO compatibility mode. A bare flag means on."
    )
    var oppoCompatibilityRaw: String?

    @Flag(name: .customLong("no-oppo-compat"), help: "Disable OPPO-compatible output.")
    var noOppoCompatibility = false

    @Flag(name: .customLong("oppo-compatible"), help: "Enable automatic OPPO Gallery compatibility.")
    var oppoCompatible = false

    @Flag(name: .customLong("discard-portrait-data"), help: "Discard OPPO portrait/depth editing data.")
    var discardPortraitData = false

    func resolve() throws -> ResolvedCommonConversionArguments {
        guard let family = Family(rawValue: familyRaw) else {
            throw CLIError.invalidValue(option: "--family", value: familyRaw)
        }
        guard let inputProcessingBranch = InputProcessingBranch(rawValue: inputProcessingRaw) else {
            throw CLIError.invalidValue(option: "--input-processing", value: inputProcessingRaw)
        }
        guard let tmapFormat = TmapFormat(rawValue: tmapFormatRaw) else {
            throw CLIError.invalidValue(option: "--tmap-format", value: tmapFormatRaw)
        }

        let explicitCompatibilityCount = [
            oppoCompatibilityRaw != nil,
            noOppoCompatibility,
            oppoCompatible,
        ].filter { $0 }.count
        guard explicitCompatibilityCount <= 1 else {
            throw CLIError.invalidValue(
                option: "--oppo-compat",
                value: "OPPO compatibility switches are mutually exclusive"
            )
        }

        let oppoCompatibility: OppoCompatibility
        let oppoCompatibilityWasExplicit: Bool
        if let raw = oppoCompatibilityRaw {
            guard let parsed = OppoCompatibility(rawValue: raw) else {
                throw CLIError.invalidValue(option: "--oppo-compat", value: raw)
            }
            oppoCompatibility = parsed
            oppoCompatibilityWasExplicit = true
        } else if noOppoCompatibility {
            oppoCompatibility = .off
            oppoCompatibilityWasExplicit = true
        } else if oppoCompatible {
            oppoCompatibility = .auto
            oppoCompatibilityWasExplicit = true
        } else {
            oppoCompatibility = .off
            oppoCompatibilityWasExplicit = false
        }

        let producerWasExplicit = appleStyleDataProducerRaw != nil
        var styleDataProducer = AppleStyleDataProducerMode.unspecified
        if let raw = appleStyleDataProducerRaw {
            guard let parsed = AppleStyleDataProducerMode(rawValue: raw), parsed != .unspecified else {
                throw CLIError.invalidValue(option: "--apple-style-data-producer", value: raw)
            }
            styleDataProducer = parsed
        }
        if producerWasExplicit, !applePhotographicStylesEnabled {
            throw CLIError.invalidValue(
                option: "--apple-style-data-producer",
                value: "requires --apple-photographic-styles"
            )
        }
        if appleStylesRawDNGPath != nil, !applePhotographicStylesEnabled {
            throw CLIError.invalidValue(
                option: "--apple-styles-raw-dng",
                value: "requires --apple-photographic-styles"
            )
        }
        if applePhotographicStylesEnabled, !producerWasExplicit {
            styleDataProducer = .constrainedSolver
        }

        let appleFeatures = AppleFeatureOptions(
            photographicStyles: applePhotographicStylesEnabled,
            portrait: applePortraitEnabled
        )
        if appleFeatures.isEnabled,
           oppoCompatibilityWasExplicit,
           oppoCompatibility.wantsOppoCompat {
            throw CLIError.invalidValue(
                option: applePhotographicStylesEnabled
                    ? "--apple-photographic-styles"
                    : "--apple-portrait",
                value: "cannot be combined with OPPO-compatible output"
            )
        }

        let cameraTailWasExplicit = oppoCameraTailRaw != nil
        var cameraTail: OppoCameraTail
        if let raw = oppoCameraTailRaw {
            guard let parsed = OppoCameraTail(rawValue: raw) else {
                throw CLIError.invalidValue(option: "--oppo-camera-tail", value: raw)
            }
            cameraTail = parsed
        } else {
            cameraTail = .preserveWithoutPrivateHDR
        }

        var effectiveCompatibility = oppoCompatibility
        if appleFeatures.isEnabled {
            effectiveCompatibility = .off
            if applePortraitEnabled {
                cameraTail = .preserveWithoutPortraitOrPrivateHDR
            } else if !cameraTailWasExplicit {
                cameraTail = .preserveWithoutPrivateHDR
            }
        } else if discardPortraitData, !cameraTailWasExplicit {
            cameraTail = effectiveCompatibility.wantsOppoCompat
                ? .preserveWithoutPortrait
                : .preserveWithoutPortraitOrPrivateHDR
        } else if !cameraTailWasExplicit, effectiveCompatibility.wantsOppoCompat {
            cameraTail = .preserve
        }

        return ResolvedCommonConversionArguments(
            family: family,
            debugDirectoryPath: debugDirectoryPath,
            oppoCompatibility: effectiveCompatibility,
            inputProcessingBranch: inputProcessingBranch,
            appleFeatures: appleFeatures,
            appleStylesRawDNGPath: appleStylesRawDNGPath,
            appleStyleDataProducer: styleDataProducer,
            oppoCameraTail: cameraTail,
            tmapFormat: tmapFormat
        )
    }
}

private struct ConvertArguments: ParsableArguments {
    @Option(name: .customLong("input"), help: "Input HEIC/HEIF or supported portrait JPEG.")
    var inputPath: String

    @Option(name: .customLong("output"), help: "Output HEIC. Defaults to replacing the input.")
    var outputPath: String?

    @OptionGroup var common: CommonConversionArguments

    func command() throws -> ConvertCommand {
        let resolved = try common.resolve()
        return ConvertCommand(
            inputURL: URL(fileURLWithPath: inputPath),
            outputURL: URL(fileURLWithPath: outputPath ?? inputPath),
            family: resolved.family,
            debugRootURL: resolved.debugDirectoryPath.map { URL(fileURLWithPath: $0) },
            oppoCompatibility: resolved.oppoCompatibility,
            inputProcessingBranch: resolved.inputProcessingBranch,
            appleFeatures: resolved.appleFeatures,
            appleStylesRawDNGURL: resolved.appleStylesRawDNGPath.map { URL(fileURLWithPath: $0) },
            appleStyleDataProducer: resolved.appleStyleDataProducer,
            oppoCameraTail: resolved.oppoCameraTail,
            tmapFormat: resolved.tmapFormat
        )
    }
}

private struct BatchArguments: ParsableArguments {
    @Option(name: .customLong("input-dir"), help: "Directory to scan recursively.")
    var inputDirectoryPath: String

    @Option(name: .customLong("output-dir"), help: "Output directory. Defaults to the input directory.")
    var outputDirectoryPath: String?

    @Option(name: .customLong("glob"), help: "Filename pattern to include.")
    var glob = "*.heic"

    @Option(name: .customLong("jobs"), help: "Maximum concurrent conversions.")
    var jobs = min(ProcessInfo.processInfo.activeProcessorCount, 4)

    @Option(name: .customLong("checkpoint"), help: "Checkpoint JSONL path.")
    var checkpointPath: String?

    @Flag(name: .customLong("resume"), inversion: .prefixedNo, help: "Resume successful checkpoint entries.")
    var resume = true

    @Flag(
        name: .customLong("skip-existing"),
        inversion: .prefixedNo,
        help: "Skip outputs that already validate for the current configuration."
    )
    var skipExisting = true

    @Flag(name: .customLong("categorize"), help: "File converted assets by capture-mode classification.")
    var categorizeOutput = false

    @OptionGroup var common: CommonConversionArguments

    func command() throws -> BatchCommand {
        guard jobs > 0 else {
            throw CLIError.invalidValue(option: "--jobs", value: String(jobs))
        }
        let resolved = try common.resolve()
        return BatchCommand(
            inputDirURL: URL(fileURLWithPath: inputDirectoryPath),
            outputDirURL: URL(fileURLWithPath: outputDirectoryPath ?? inputDirectoryPath),
            family: resolved.family,
            glob: glob,
            debugRootURL: resolved.debugDirectoryPath.map { URL(fileURLWithPath: $0) },
            oppoCompatibility: resolved.oppoCompatibility,
            inputProcessingBranch: resolved.inputProcessingBranch,
            appleFeatures: resolved.appleFeatures,
            appleStylesRawDNGURL: resolved.appleStylesRawDNGPath.map { URL(fileURLWithPath: $0) },
            appleStyleDataProducer: resolved.appleStyleDataProducer,
            oppoCameraTail: resolved.oppoCameraTail,
            tmapFormat: resolved.tmapFormat,
            jobs: jobs,
            checkpointURL: checkpointPath.map { URL(fileURLWithPath: $0) },
            resume: resume,
            skipExisting: skipExisting,
            categorizeOutput: categorizeOutput
        )
    }
}

private struct CategorizeArguments: ParsableArguments {
    @Option(name: .customLong("input"), parsing: .unconditionalSingleValue, help: "Input file or directory; repeatable.")
    var inputPaths: [String] = []

    @Option(name: .customLong("output-dir"), help: "Destination root.")
    var outputDirectoryPath: String

    @Option(name: .customLong("jobs"), help: "Maximum concurrent copies.")
    var jobs = min(ProcessInfo.processInfo.activeProcessorCount, 4)

    @Flag(name: .customLong("dry-run"), help: "Plan without copying files.")
    var dryRun = false

    func command() throws -> CategorizeCommand {
        guard !inputPaths.isEmpty else { throw CLIError.missingArgument("--input") }
        guard jobs > 0 else {
            throw CLIError.invalidValue(option: "--jobs", value: String(jobs))
        }
        return CategorizeCommand(
            inputURLs: inputPaths.map { URL(fileURLWithPath: $0) },
            outputDirURL: URL(fileURLWithPath: outputDirectoryPath),
            jobs: jobs,
            dryRun: dryRun
        )
    }
}

enum ConversionArgumentParser {
    static func parseConvert(_ rawArguments: [String]) throws -> ConvertCommand {
        try ConvertArguments.parse(rawArguments).command()
    }

    static func parseBatch(_ rawArguments: [String]) throws -> BatchCommand {
        try BatchArguments.parse(rawArguments).command()
    }

    static func parseCategorize(_ rawArguments: [String]) throws -> CategorizeCommand {
        try CategorizeArguments.parse(rawArguments).command()
    }
}
