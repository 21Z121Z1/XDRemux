import Foundation
import XDRemuxCore

struct ConvertCommand {
    let inputURL: URL
    let outputURL: URL
    let family: Family
    let debugRootURL: URL?
    let oppoCompatibility: OppoCompatibility
    let inputProcessingBranch: InputProcessingBranch
    let appleFeatures: AppleFeatureOptions
    let appleStylesRawDNGURL: URL?
    let appleStyleDataProducer: AppleStyleDataProducerMode
    let oppoCameraTail: OppoCameraTail
    let tmapFormat: TmapFormat

    var configuration: ConversionConfiguration {
        ConversionConfiguration(
            family: family,
            oppoCompatibility: oppoCompatibility,
            inputProcessingBranch: inputProcessingBranch,
            oppoCameraTail: oppoCameraTail,
            tmapFormat: tmapFormat,
            debugDirectory: debugRootURL,
            applePhotographicStyles: appleFeatures.photographicStyles,
            applePortrait: appleFeatures.portrait,
            appleStylesRawDNGURL: appleStylesRawDNGURL,
            appleStyleDataProducer: appleStyleDataProducer,
            eventHandler: CLIOutput.conversionEventHandler
        )
    }
}

struct BatchCommand {
    let inputDirURL: URL
    let outputDirURL: URL
    let family: Family
    let glob: String
    let debugRootURL: URL?
    let oppoCompatibility: OppoCompatibility
    let inputProcessingBranch: InputProcessingBranch
    let appleFeatures: AppleFeatureOptions
    let appleStylesRawDNGURL: URL?
    let appleStyleDataProducer: AppleStyleDataProducerMode
    let oppoCameraTail: OppoCameraTail
    let tmapFormat: TmapFormat
    let jobs: Int
    let checkpointURL: URL?
    let resume: Bool
    let skipExisting: Bool
    let categorizeOutput: Bool

    var conversionConfiguration: ConversionConfiguration {
        ConversionConfiguration(
            family: family,
            oppoCompatibility: oppoCompatibility,
            inputProcessingBranch: inputProcessingBranch,
            oppoCameraTail: oppoCameraTail,
            tmapFormat: tmapFormat,
            debugDirectory: debugRootURL,
            skipExisting: skipExisting,
            maxConcurrentJobs: jobs,
            categorizeOutputByCaptureMode: categorizeOutput,
            applePhotographicStyles: appleFeatures.photographicStyles,
            applePortrait: appleFeatures.portrait,
            appleStylesRawDNGURL: appleStylesRawDNGURL,
            appleStyleDataProducer: appleStyleDataProducer,
            eventHandler: CLIOutput.conversionEventHandler
        )
    }
}

struct CategorizeCommand {
    let inputURLs: [URL]
    let outputDirURL: URL
    let jobs: Int
    let dryRun: Bool
}
