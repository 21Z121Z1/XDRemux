import Foundation

public struct BatchConfiguration: Sendable {
    public var conversion: ConversionConfiguration
    public var inputDirectory: URL
    public var outputDirectory: URL
    public var glob: String
    public var jobs: Int
    public var overwrite: Bool

    public init(
        conversion: ConversionConfiguration = ConversionConfiguration(),
        inputDirectory: URL,
        outputDirectory: URL,
        glob: String = "*.heic",
        jobs: Int = min(ProcessInfo.processInfo.activeProcessorCount, 4),
        overwrite: Bool = false
    ) {
        self.conversion = conversion
        self.inputDirectory = inputDirectory
        self.outputDirectory = outputDirectory
        self.glob = glob
        self.jobs = jobs
        self.overwrite = overwrite
    }
}

public struct BatchResult: Sendable {
    public let convertedCount: Int
    public let skippedExistingCount: Int
    public let failureCount: Int
    public let outputDirectory: URL
    public let failureReportURL: URL?

    public init(
        convertedCount: Int,
        skippedExistingCount: Int,
        failureCount: Int,
        outputDirectory: URL,
        failureReportURL: URL? = nil
    ) {
        self.convertedCount = convertedCount
        self.skippedExistingCount = skippedExistingCount
        self.failureCount = failureCount
        self.outputDirectory = outputDirectory
        self.failureReportURL = failureReportURL
    }
}
