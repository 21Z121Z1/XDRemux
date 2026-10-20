import Foundation

enum AppConversionEngine {
    static func convert(
        inputURL: URL,
        outputURL: URL,
        config: ConversionConfig
    ) throws {
        try RustCLIClient.convert(inputURL: inputURL, outputURL: outputURL, config: config)
    }

    static func isValidOutput(_ outputURL: URL, config: ConversionConfig) -> Bool {
        return RustCLIClient.isValidOutput(outputURL, config: config)
    }
}
