import Foundation
import XDRemuxCore
import XDRemuxAppleFeatures

typealias Family = XDRemuxCore.Family
typealias InputProcessingBranch = XDRemuxCore.InputProcessingBranch
typealias TmapFormat = XDRemuxCore.TmapFormat
typealias OppoCompatibility = XDRemuxCore.OppoCompatibility
typealias OppoCameraTail = XDRemuxCore.OppoCameraTail
typealias ConversionConfig = XDRemuxCore.ConversionConfiguration
typealias XDRemuxError = XDRemuxCore.XDRemuxError

enum AppConversionEngine {
    static func requestForTesting(
        inputURL: URL,
        outputURL: URL,
        config: ConversionConfig
    ) -> ConversionRequest {
        ConversionRequest(
            input: InputSource(url: inputURL),
            output: OutputDestination(url: outputURL),
            configuration: config
        )
    }

    static func convert(
        inputURL: URL,
        outputURL: URL,
        config: ConversionConfig
    ) throws {
        let request = requestForTesting(
            inputURL: inputURL,
            outputURL: outputURL,
            config: config
        )
        if config.appleFeaturesEnabled {
            _ = try AppleFeatureConversionEngine.convert(request)
        } else {
            _ = try ConversionEngine.convert(request)
        }
    }

    static func isValidOutput(_ outputURL: URL, config: ConversionConfig) -> Bool {
        if config.appleFeaturesEnabled {
            return AppleFeatureConversionEngine.isValidOutput(
                outputURL,
                options: config.appleFeatureOptions
            )
        }
        return ConversionEngine.isValidOutput(outputURL, config: config)
    }
}
