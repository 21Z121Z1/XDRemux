import Foundation
import XDRemuxCore

enum AppleFeatureEventForwarder {
    static func preparationHandler(
        _ downstream: ConversionEventHandler?
    ) -> ConversionEventHandler? {
        guard let downstream else { return nil }
        return { event in
            if case .phaseChanged(let phase) = event,
               phase == .readingSource
                || phase == .writingContainer
                || phase == .verifyingOutput {
                return
            }
            downstream(event)
        }
    }
}

public enum AppleFeatureConversionEngine {
    public static func convert(_ request: ConversionRequest) throws -> ConversionResult {
        let configuration = request.configuration
        guard configuration.appleFeaturesEnabled else {
            return try ConversionEngine.convert(request)
        }
        let eventHandler = configuration.eventHandler
        eventHandler?(.started(input: request.input.url, output: request.output.url))
        do {
            try configuration.cancellation?.checkCancellation()
            guard !configuration.oppoCompatibility.wantsOppoCompat else {
                throw XDRemuxError.invalidValue(
                    option: configuration.applePhotographicStyles
                        ? "--apple-photographic-styles"
                        : "--apple-portrait",
                    value: "cannot be combined with OPPO-compatible output"
                )
            }

            try AppleNativeToolchain.withCancellation(configuration.cancellation) {
                if configuration.applePhotographicStyles {
                    try ApplePhotographicStylesPipeline.convert(
                        inputURL: request.input.url,
                        outputURL: request.output.url,
                        configuration: configuration
                    )
                } else {
                    _ = try PortraitConversionPipeline.convertIfNeeded(
                        inputURL: request.input.url,
                        outputURL: request.output.url,
                        mode: configuration.applePortrait ? .on : .off,
                        eventHandler: eventHandler
                    )
                }
            }
            try configuration.cancellation?.checkCancellation()
            let result = ConversionResult(input: request.input, output: request.output)
            eventHandler?(.completed(result))
            return result
        } catch is CancellationError {
            throw CancellationError()
        } catch {
            eventHandler?(.failed(ConversionFailure.classify(error)))
            throw error
        }
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

    public static func isValidOutput(
        _ outputURL: URL,
        configuration: ConversionConfiguration
    ) -> Bool {
        if configuration.appleFeaturesEnabled {
            return isValidOutput(outputURL, options: configuration.appleFeatureOptions)
        }
        return ConversionEngine.isValidOutput(outputURL, config: configuration)
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
}
