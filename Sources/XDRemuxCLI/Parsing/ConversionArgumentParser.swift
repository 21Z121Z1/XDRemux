import Foundation
import XDRemuxCore

struct CommonConversionArguments {
    var conversion = ConversionOptions()
    var output = OutputOptions()
    private var selectedVerbosity: OutputVerbosity?
    private var applePortraitEnabled = false
    private var applePhotographicStylesEnabled = false
    private var oppoCompatibilityWasExplicit = false
    private var oppoCameraTailWasExplicit = false
    private var discardPortraitData = false

    mutating func consume(
        _ option: String,
        cursor: inout ConversionArgumentParser.ArgumentCursor,
        mode: CLIProductMode
    ) throws -> Bool {
        switch option {
        case "--apple-photographic-styles":
            applePhotographicStylesEnabled = true
        case "--apple-portrait":
            applePortraitEnabled = true
        case "--oppo-compatible":
            conversion.oppoCompatibility = .auto
            oppoCompatibilityWasExplicit = true
        case "--discard-portrait-data":
            discardPortraitData = true
        case "--overwrite":
            conversion.overwrite = true
        case "--quiet":
            try setVerbosity(.quiet, option: option)
        case "--verbose":
            try setVerbosity(.verbose, option: option)
        case "--debug":
            try setVerbosity(.debug, option: option)
        case "--format":
            let value = try cursor.nextValue(for: option)
            guard let format = OutputFormat(rawValue: value) else {
                throw CLIError.invalidValue(option: option, value: value)
            }
            output.format = format
        case "--language":
            let value = try cursor.nextValue(for: option)
            guard let language = OutputLanguage(rawValue: value) else {
                throw CLIError.invalidValue(option: option, value: value)
            }
            output.language = language
        case "--apple-styles" where mode.allowsInternalOptions:
            applePhotographicStylesEnabled = true
        case "--family" where mode.allowsInternalOptions:
            let value = try cursor.nextValue(for: option)
            guard let parsed = Family(rawValue: value) else {
                throw CLIError.invalidValue(option: option, value: value)
            }
            conversion.family = parsed
        case "--input-processing" where mode.allowsInternalOptions:
            let value = try cursor.nextValue(for: option)
            guard let parsed = InputProcessingBranch(rawValue: value) else {
                throw CLIError.invalidValue(option: option, value: value)
            }
            conversion.inputProcessingBranch = parsed
        case "--diagnostics-dir" where mode.allowsInternalOptions,
             "--debug-dir" where mode.allowsInternalOptions:
            conversion.diagnosticsDirectoryURL = URL(
                fileURLWithPath: try cursor.nextValue(for: option),
                isDirectory: true
            )
        case "--oppo-camera-tail" where mode.allowsInternalOptions:
            let value = try cursor.nextValue(for: option)
            guard let parsed = OppoCameraTail(rawValue: value) else {
                throw CLIError.invalidValue(option: option, value: value)
            }
            conversion.oppoCameraTail = parsed
            oppoCameraTailWasExplicit = true
        case "--tmap-format" where mode.allowsInternalOptions:
            let value = try cursor.nextValue(for: option)
            guard let parsed = TmapFormat(rawValue: value) else {
                throw CLIError.invalidValue(option: option, value: value)
            }
            conversion.tmapFormat = parsed
        case "--oppo-compat" where mode.allowsInternalOptions:
            conversion.oppoCompatibility = cursor.consumeOptionalOppoCompatibility() ?? .on
            oppoCompatibilityWasExplicit = true
        case "--no-oppo-compat" where mode.allowsInternalOptions:
            conversion.oppoCompatibility = .off
            oppoCompatibilityWasExplicit = true
        default:
            return false
        }
        return true
    }

    mutating func resolve() throws {
        let appleFeatures = AppleFeatureOptions(
            photographicStyles: applePhotographicStylesEnabled,
            portrait: applePortraitEnabled
        )
        if appleFeatures.isEnabled,
           oppoCompatibilityWasExplicit,
           conversion.oppoCompatibility.wantsOppoCompat {
            throw CLIError.invalidValue(
                option: applePhotographicStylesEnabled
                    ? "--apple-photographic-styles"
                    : "--apple-portrait",
                value: "cannot be combined with --oppo-compatible"
            )
        }
        conversion.appleFeatures = appleFeatures
        if appleFeatures.isEnabled {
            conversion.oppoCompatibility = .off
            if applePortraitEnabled {
                conversion.oppoCameraTail = .preserveWithoutPortraitOrPrivateHDR
            } else if !oppoCameraTailWasExplicit {
                conversion.oppoCameraTail = .preserveWithoutPrivateHDR
            }
        } else if discardPortraitData, !oppoCameraTailWasExplicit {
            conversion.oppoCameraTail = conversion.oppoCompatibility.wantsOppoCompat
                ? .preserveWithoutPortrait
                : .preserveWithoutPortraitOrPrivateHDR
        } else if !oppoCameraTailWasExplicit, conversion.oppoCompatibility.wantsOppoCompat {
            conversion.oppoCameraTail = .preserve
        }
    }

    private mutating func setVerbosity(_ verbosity: OutputVerbosity, option: String) throws {
        if let selectedVerbosity, selectedVerbosity != verbosity {
            throw CLIError.invalidValue(
                option: option,
                value: "cannot be combined with --\(selectedVerbosity.rawValue)"
            )
        }
        selectedVerbosity = verbosity
        output.verbosity = verbosity
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

    static func parseConvert(
        _ rawArguments: [String],
        mode: CLIProductMode = .production
    ) throws -> ConvertCommand {
        var cursor = ArgumentCursor(arguments: rawArguments)
        var common = CommonConversionArguments()
        var inputPath: String?
        var outputPath: String?

        while let option = cursor.nextOption() {
            if try common.consume(option, cursor: &cursor, mode: mode) { continue }
            switch option {
            case "--input": inputPath = try cursor.nextValue(for: option)
            case "--output": outputPath = try cursor.nextValue(for: option)
            default: throw CLIError.unknownOption(option)
            }
        }

        guard let inputPath else { throw CLIError.missingArgument("--input") }
        try common.resolve()
        return ConvertCommand(
            inputURL: URL(fileURLWithPath: inputPath),
            outputURL: URL(fileURLWithPath: outputPath ?? inputPath),
            outputWasExplicit: outputPath != nil,
            conversion: common.conversion,
            output: common.output
        )
    }

    static func parseBatch(
        _ rawArguments: [String],
        mode: CLIProductMode = .production
    ) throws -> BatchCommand {
        var cursor = ArgumentCursor(arguments: rawArguments)
        var common = CommonConversionArguments()
        var inputDirectoryPath: String?
        var outputDirectoryPath: String?
        var glob = "*.heic"
        var jobs = min(ProcessInfo.processInfo.activeProcessorCount, 4)

        while let option = cursor.nextOption() {
            if try common.consume(option, cursor: &cursor, mode: mode) { continue }
            switch option {
            case "--input-dir": inputDirectoryPath = try cursor.nextValue(for: option)
            case "--output-dir": outputDirectoryPath = try cursor.nextValue(for: option)
            case "--glob": glob = try cursor.nextValue(for: option)
            case "--jobs":
                let value = try cursor.nextValue(for: option)
                guard let parsed = Int(value), parsed > 0 else {
                    throw CLIError.invalidValue(option: option, value: value)
                }
                jobs = parsed
            default: throw CLIError.unknownOption(option)
            }
        }

        guard let inputDirectoryPath else {
            throw CLIError.missingArgument("--input-dir")
        }
        try common.resolve()
        let inputDirectoryURL = URL(fileURLWithPath: inputDirectoryPath, isDirectory: true)
        return BatchCommand(
            inputDirURL: inputDirectoryURL,
            outputDirURL: URL(
                fileURLWithPath: outputDirectoryPath ?? inputDirectoryPath,
                isDirectory: true
            ),
            glob: glob,
            jobs: jobs,
            conversion: common.conversion,
            output: common.output
        )
    }
}
