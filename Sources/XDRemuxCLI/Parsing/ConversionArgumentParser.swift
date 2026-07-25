import Foundation
import XDRemuxCore

struct CommonConversionArguments {
    var family = Family.auto
    var debugDirectoryPath: String?
    var oppoCompatibility: OppoCompatibility = .off
    var inputProcessingBranch = InputProcessingBranch.hybrid
    var applePortraitEnabled = false
    var applePhotographicStylesEnabled = false
    var appleStylesRawDNGPath: String?
    var appleStyleDataProducer = AppleStyleDataProducerMode.unspecified
    var appleStyleDataProducerWasExplicit = false
    var oppoCompatibilityWasExplicit = false
    var oppoCameraTail = OppoCameraTail.preserveWithoutPrivateHDR
    var oppoCameraTailWasExplicit = false
    var discardPortraitData = false
    var tmapFormat = TmapFormat.imageIO

    mutating func consume(
        _ option: String,
        cursor: inout ConversionArgumentParser.ArgumentCursor
    ) throws -> Bool {
        switch option {
        case "--apple-photographic-styles", "--apple-styles":
            applePhotographicStylesEnabled = true
        case "--apple-style-data-producer":
            let value = try cursor.nextValue(for: option)
            guard let parsed = AppleStyleDataProducerMode(rawValue: value),
                  parsed != .unspecified else {
                throw CLIError.invalidValue(option: option, value: value)
            }
            appleStyleDataProducer = parsed
            appleStyleDataProducerWasExplicit = true
        case "--apple-portrait":
            applePortraitEnabled = true
        case "--apple-styles-raw-dng":
            appleStylesRawDNGPath = try cursor.nextValue(for: option)
        case "--family":
            let value = try cursor.nextValue(for: option)
            guard let parsed = Family(rawValue: value) else {
                throw CLIError.invalidValue(option: option, value: value)
            }
            family = parsed
        case "--input-processing":
            let value = try cursor.nextValue(for: option)
            guard let parsed = InputProcessingBranch(rawValue: value) else {
                throw CLIError.invalidValue(option: option, value: value)
            }
            inputProcessingBranch = parsed
        case "--debug-dir":
            debugDirectoryPath = try cursor.nextValue(for: option)
        case "--oppo-camera-tail":
            let value = try cursor.nextValue(for: option)
            guard let parsed = OppoCameraTail(rawValue: value) else {
                throw CLIError.invalidValue(option: option, value: value)
            }
            oppoCameraTail = parsed
            oppoCameraTailWasExplicit = true
        case "--tmap-format":
            let value = try cursor.nextValue(for: option)
            guard let parsed = TmapFormat(rawValue: value) else {
                throw CLIError.invalidValue(option: option, value: value)
            }
            tmapFormat = parsed
        case "--oppo-compat":
            oppoCompatibility = cursor.consumeOptionalOppoCompatibility() ?? .on
            oppoCompatibilityWasExplicit = true
        case "--no-oppo-compat":
            oppoCompatibility = .off
            oppoCompatibilityWasExplicit = true
        case "--oppo-compatible":
            oppoCompatibility = .auto
            oppoCompatibilityWasExplicit = true
        case "--discard-portrait-data":
            discardPortraitData = true
        default:
            return false
        }
        return true
    }

    mutating func resolve() throws -> AppleFeatureOptions {
        let appleFeatures = AppleFeatureOptions(
            photographicStyles: applePhotographicStylesEnabled,
            portrait: applePortraitEnabled
        )
        if appleStyleDataProducerWasExplicit, !applePhotographicStylesEnabled {
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
        if applePhotographicStylesEnabled, !appleStyleDataProducerWasExplicit {
            appleStyleDataProducer = .constrainedSolver
        }
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
        if appleFeatures.isEnabled {
            oppoCompatibility = .off
            if applePortraitEnabled {
                oppoCameraTail = .preserveWithoutPortraitOrPrivateHDR
            } else if !oppoCameraTailWasExplicit {
                oppoCameraTail = .preserveWithoutPrivateHDR
            }
        } else if discardPortraitData, !oppoCameraTailWasExplicit {
            oppoCameraTail = oppoCompatibility.wantsOppoCompat
                ? .preserveWithoutPortrait
                : .preserveWithoutPortraitOrPrivateHDR
        } else if !oppoCameraTailWasExplicit, oppoCompatibility.wantsOppoCompat {
            oppoCameraTail = .preserve
        }
        return appleFeatures
    }
}

enum ConversionArgumentParser {
    struct ArgumentCursor {
        let arguments: [String]
        var index = 0

        mutating func nextOption() -> String? {
            guard index < arguments.count else { return nil }
            defer { index += 1 }
            return arguments[index]
        }

        mutating func nextValue(for option: String) throws -> String {
            guard index < arguments.count else {
                throw CLIError.missingArgument(option)
            }
            defer { index += 1 }
            return arguments[index]
        }

        mutating func consumeOptionalOppoCompatibility() -> OppoCompatibility? {
            guard index < arguments.count,
                  let parsed = OppoCompatibility(rawValue: arguments[index]) else {
                return nil
            }
            index += 1
            return parsed
        }
    }

    static func parseConvert(_ rawArguments: [String]) throws -> ConvertCommand {
        var cursor = ArgumentCursor(arguments: rawArguments)
        var common = CommonConversionArguments()
        var inputPath: String?
        var outputPath: String?

        while let option = cursor.nextOption() {
            if try common.consume(option, cursor: &cursor) { continue }
            switch option {
            case "--input":
                inputPath = try cursor.nextValue(for: option)
            case "--output":
                outputPath = try cursor.nextValue(for: option)
            default:
                throw CLIError.unknownOption(option)
            }
        }

        guard let inputPath else { throw CLIError.missingArgument("--input") }
        let appleFeatures = try common.resolve()
        return ConvertCommand(
            inputURL: URL(fileURLWithPath: inputPath),
            outputURL: URL(fileURLWithPath: outputPath ?? inputPath),
            family: common.family,
            debugRootURL: common.debugDirectoryPath.map { URL(fileURLWithPath: $0) },
            oppoCompatibility: common.oppoCompatibility,
            inputProcessingBranch: common.inputProcessingBranch,
            appleFeatures: appleFeatures,
            appleStylesRawDNGURL: common.appleStylesRawDNGPath.map { URL(fileURLWithPath: $0) },
            appleStyleDataProducer: common.appleStyleDataProducer,
            oppoCameraTail: common.oppoCameraTail,
            tmapFormat: common.tmapFormat
        )
    }

    static func parseBatch(_ rawArguments: [String]) throws -> BatchCommand {
        var cursor = ArgumentCursor(arguments: rawArguments)
        var common = CommonConversionArguments()
        var inputDirectoryPath: String?
        var outputDirectoryPath: String?
        var glob = "*.heic"
        var jobs = min(ProcessInfo.processInfo.activeProcessorCount, 4)
        var checkpointPath: String?
        var resume = true
        var skipExisting = true
        var categorizeOutput = false

        while let option = cursor.nextOption() {
            if try common.consume(option, cursor: &cursor) { continue }
            switch option {
            case "--input-dir":
                inputDirectoryPath = try cursor.nextValue(for: option)
            case "--output-dir":
                outputDirectoryPath = try cursor.nextValue(for: option)
            case "--glob":
                glob = try cursor.nextValue(for: option)
            case "--jobs":
                let value = try cursor.nextValue(for: option)
                guard let parsed = Int(value), parsed > 0 else {
                    throw CLIError.invalidValue(option: option, value: value)
                }
                jobs = parsed
            case "--checkpoint":
                checkpointPath = try cursor.nextValue(for: option)
            case "--resume":
                resume = true
            case "--no-resume":
                resume = false
            case "--skip-existing":
                skipExisting = true
            case "--no-skip-existing":
                skipExisting = false
            case "--categorize":
                categorizeOutput = true
            default:
                throw CLIError.unknownOption(option)
            }
        }

        guard let inputDirectoryPath else {
            throw CLIError.missingArgument("--input-dir")
        }
        let appleFeatures = try common.resolve()
        return BatchCommand(
            inputDirURL: URL(fileURLWithPath: inputDirectoryPath),
            outputDirURL: URL(fileURLWithPath: outputDirectoryPath ?? inputDirectoryPath),
            family: common.family,
            glob: glob,
            debugRootURL: common.debugDirectoryPath.map { URL(fileURLWithPath: $0) },
            oppoCompatibility: common.oppoCompatibility,
            inputProcessingBranch: common.inputProcessingBranch,
            appleFeatures: appleFeatures,
            appleStylesRawDNGURL: common.appleStylesRawDNGPath.map { URL(fileURLWithPath: $0) },
            appleStyleDataProducer: common.appleStyleDataProducer,
            oppoCameraTail: common.oppoCameraTail,
            tmapFormat: common.tmapFormat,
            jobs: jobs,
            checkpointURL: checkpointPath.map { URL(fileURLWithPath: $0) },
            resume: resume,
            skipExisting: skipExisting,
            categorizeOutput: categorizeOutput
        )
    }

    static func parseCategorize(_ rawArguments: [String]) throws -> CategorizeCommand {
        var cursor = ArgumentCursor(arguments: rawArguments)
        var inputPaths: [String] = []
        var outputDirectoryPath: String?
        var jobs = min(ProcessInfo.processInfo.activeProcessorCount, 4)
        var dryRun = false

        while let option = cursor.nextOption() {
            switch option {
            case "--input":
                inputPaths.append(try cursor.nextValue(for: option))
            case "--output-dir":
                outputDirectoryPath = try cursor.nextValue(for: option)
            case "--jobs":
                let value = try cursor.nextValue(for: option)
                guard let parsed = Int(value), parsed > 0 else {
                    throw CLIError.invalidValue(option: option, value: value)
                }
                jobs = parsed
            case "--dry-run":
                dryRun = true
            default:
                throw CLIError.unknownOption(option)
            }
        }

        guard !inputPaths.isEmpty else { throw CLIError.missingArgument("--input") }
        guard let outputDirectoryPath else { throw CLIError.missingArgument("--output-dir") }
        return CategorizeCommand(
            inputURLs: inputPaths.map { URL(fileURLWithPath: $0) },
            outputDirURL: URL(fileURLWithPath: outputDirectoryPath),
            jobs: jobs,
            dryRun: dryRun
        )
    }
}
