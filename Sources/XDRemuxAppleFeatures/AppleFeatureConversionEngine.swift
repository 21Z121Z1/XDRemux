import Foundation
import XDRemuxCore

public enum AppleFeatureConversionEngine {
    public static func convert(_ request: ConversionRequest) throws -> ConversionResult {
        let configuration = request.configuration
        guard configuration.appleFeaturesEnabled else {
            return try ConversionEngine.convert(request)
        }
        guard !configuration.oppoCompatibility.wantsOppoCompat else {
            throw XDRemuxError.invalidValue(
                option: configuration.applePhotographicStyles
                    ? "--apple-photographic-styles"
                    : "--apple-portrait",
                value: "cannot be combined with OPPO-compatible output"
            )
        }

        if configuration.applePhotographicStyles {
            do {
                try ApplePhotographicStylesPipeline.convert(
                    inputURL: request.input.url,
                    outputURL: request.output.url,
                    configuration: configuration
                )
            } catch {
                // The shared semantic directory is optional for Styles-only runs. On macOS 26
                // Foundation can report ENOENT while tearing down that never-materialized scratch
                // path after the final HEIC has already been written. Cleanup must be idempotent:
                // accept this one narrow error only when the complete output independently passes
                // the same Apple/Neutrino validation used by validate-apple.
                guard isValidatedStylesScratchCleanupRace(
                    error,
                    outputURL: request.output.url,
                    expectsPortrait: configuration.applePortrait
                ) else {
                    throw error
                }
                configuration.eventHandler?(.diagnostic(
                    "ignored missing Photographic Styles shared-semantics scratch path after validated output"
                ))
            }
        } else {
            _ = try PortraitConversionPipeline.convertIfNeeded(
                inputURL: request.input.url,
                outputURL: request.output.url,
                mode: configuration.applePortrait ? .on : .off,
                eventHandler: configuration.eventHandler
            )
        }
        return ConversionResult(input: request.input, output: request.output)
    }

    public static func convert(
        inputURL: URL,
        outputURL: URL,
        configuration: ConversionConfiguration
    ) throws {
        _ = try convert(
            ConversionRequest(
                input: InputSource(url: inputURL),
                output: OutputDestination(url: outputURL),
                configuration: configuration
            )
        )
    }

    public static func isValidOutput(
        _ outputURL: URL,
        options: AppleFeatureOptions
    ) -> Bool {
        if options.photographicStyles {
            return ApplePhotographicStylesPipeline.isValidOutput(
                outputURL,
                expectsPortrait: options.portrait
            )
        }
        return options.portrait
            ? PortraitConversionPipeline.isValidOutput(outputURL)
            : ConversionEngine.isValidISOGainMapOutput(outputURL)
    }

    public static func validationReport(
        for outputURL: URL,
        expectsPortrait: Bool
    ) throws -> [String: Any] {
        try ApplePhotographicStylesPipeline.validateExistingOutput(
            outputURL,
            expectsPortrait: expectsPortrait
        )
    }

    public static func portraitValidationReport(for outputURL: URL) throws -> [String: Any] {
        try PortraitConversionPipeline.validationReport(outputURL)
    }

    public static func portraitSelfTestReport() throws -> [String: Any] {
        try PortraitConversionPipeline.coreSelfTestReport()
    }

    public static func isConvertiblePortraitInput(_ inputURL: URL) -> Bool {
        PortraitConversionPipeline.isConvertibleInput(inputURL)
    }

    public static func hasValidISOGainMap(_ inputURL: URL) -> Bool {
        PortraitConversionPipeline.hasValidISOGainMap(inputURL)
    }

    private static func isValidatedStylesScratchCleanupRace(
        _ error: Error,
        outputURL: URL,
        expectsPortrait: Bool
    ) -> Bool {
        let cocoa = error as NSError
        guard cocoa.domain == NSCocoaErrorDomain,
              cocoa.code == CocoaError.Code.fileNoSuchFile.rawValue,
              let path = cocoa.userInfo[NSFilePathErrorKey] as? String,
              path.contains(".shared-semantics-") else {
            return false
        }
        return ApplePhotographicStylesPipeline.isValidOutput(
            outputURL,
            expectsPortrait: expectsPortrait
        )
    }
}
