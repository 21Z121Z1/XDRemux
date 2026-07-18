import Foundation
import XDRemuxCore

public enum CLIProductMode: Sendable {
    case production
    case developer

    var executableName: String {
        self == .production ? "xdremux" : "xdremux-dev"
    }

    var allowsInternalOptions: Bool { self == .developer }
}

enum OutputVerbosity: String, Sendable {
    case quiet
    case normal
    case verbose
    case debug
}

enum OutputFormat: String, Sendable {
    case text
    case json
    case jsonl
}

enum OutputLanguage: String, Sendable {
    case automatic = "auto"
    case simplifiedChinese = "zh-Hans"
    case english = "en"
}

struct OutputOptions: Sendable {
    var verbosity: OutputVerbosity = .normal
    var format: OutputFormat = .text
    var language: OutputLanguage?

    static func bootstrap(from arguments: [String]) -> OutputOptions {
        var result = OutputOptions()
        var index = 0
        while index < arguments.count {
            let argument = arguments[index]
            switch argument {
            case "--quiet": result.verbosity = .quiet
            case "--verbose": result.verbosity = .verbose
            case "--debug": result.verbosity = .debug
            case "--format" where index + 1 < arguments.count:
                result.format = OutputFormat(rawValue: arguments[index + 1]) ?? .text
                index += 1
            case "--language" where index + 1 < arguments.count:
                result.language = OutputLanguage(rawValue: arguments[index + 1])
                index += 1
            default: break
            }
            index += 1
        }
        return result
    }
}

struct ConversionOptions: Sendable {
    var family = Family.auto
    var diagnosticsDirectoryURL: URL?
    var oppoCompatibility: OppoCompatibility = .off
    var inputProcessingBranch = InputProcessingBranch.hybrid
    var appleFeatures = AppleFeatureOptions.disabled
    var oppoCameraTail = OppoCameraTail.preserveWithoutPrivateHDR
    var tmapFormat = TmapFormat.imageIO
    var overwrite = false

    func configuration(eventHandler: ConversionEventHandler?) -> ConversionConfiguration {
        ConversionConfiguration(
            family: family,
            oppoCompatibility: oppoCompatibility,
            inputProcessingBranch: inputProcessingBranch,
            oppoCameraTail: oppoCameraTail,
            tmapFormat: tmapFormat,
            debugDirectory: diagnosticsDirectoryURL,
            skipExisting: !overwrite,
            applePhotographicStyles: appleFeatures.photographicStyles,
            applePortrait: appleFeatures.portrait,
            eventHandler: eventHandler
        )
    }
}

struct ConvertCommand {
    let inputURL: URL
    let outputURL: URL
    let outputWasExplicit: Bool
    let conversion: ConversionOptions
    let output: OutputOptions
}

struct BatchCommand {
    let inputDirURL: URL
    let outputDirURL: URL
    let glob: String
    let jobs: Int
    let conversion: ConversionOptions
    let output: OutputOptions

    func conversionConfiguration(eventHandler: ConversionEventHandler?) -> ConversionConfiguration {
        var configuration = conversion.configuration(eventHandler: eventHandler)
        configuration.outputDirectory = outputDirURL
        configuration.maxConcurrentJobs = jobs
        return configuration
    }
}
