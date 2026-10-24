import Foundation

enum ConversionSmokeError: Error, CustomStringConvertible {
    case usage

    var description: String {
        switch self {
        case .usage:
            return "Usage: XDRemuxAppConversionSmoke <input.heic> <output> [--apple-styles|--apple-portrait|--oppo-compatible|--hdr-look]"
        }
    }
}

@main
struct XDRemuxAppConversionSmoke {
    static func main() throws {
        let args = Array(CommandLine.arguments.dropFirst())
        guard args.count >= 2 else {
            throw ConversionSmokeError.usage
        }

        let inputURL = URL(fileURLWithPath: args[0])
        let outputURL = URL(fileURLWithPath: args[1])
        if args.count == 3, args[2] == "--hdr-look" {
            let receipt = try RustCLIClient.hdrLook(inputs: [inputURL], outputDirectory: outputURL, headroom: 8)
            guard receipt.failures == 0, !receipt.results.isEmpty else {
                throw RustCLIClient.ClientError.launchFailed(String(describing: receipt))
            }
            for item in receipt.results { print(item.output) }
            return
        }
        var config = ConversionConfig()

        var index = 2
        while index < args.count {
            let option = args[index]
            index += 1
            switch option {
            case "--apple-styles":
                config.applePhotographicStyles = true
            case "--apple-portrait":
                config.applePortrait = true
            case "--oppo-compatible":
                config.oppoGalleryCompatibilityEnabled = true
            default:
                throw ConversionSmokeError.usage
            }
        }

        try AppConversionEngine.convert(inputURL: inputURL, outputURL: outputURL, config: config)
        print(outputURL.path)
    }
}
