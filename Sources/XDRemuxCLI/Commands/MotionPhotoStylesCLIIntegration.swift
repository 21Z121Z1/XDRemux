import Foundation
import XDRemuxCore
import XDRemuxAppleFeatures

/// Opt-in bridge for the one Apple-feature combination Motion Photo can already support safely:
/// convert the Android Motion Photo to an Apple Live Photo pair, then add Photographic Styles to
/// the still before the pair is validated and atomically published. Plain Motion Photo conversion
/// remains owned by MotionPhotoCLIIntegration.
enum MotionPhotoStylesCLIIntegration {
    static func handleIfNeeded(_ arguments: [String]) throws -> Bool {
        guard arguments.first == "convert" else { return false }
        let rawArguments = Array(arguments.dropFirst())
        guard let inputPath = optionValue("--input", in: rawArguments) else { return false }
        let inputURL = URL(fileURLWithPath: inputPath).standardizedFileURL
        guard isSupportedMotionPhotoExtension(inputURL),
              AppleLivePhotoConversionEngine.isMotionPhotoInput(inputURL) else {
            return false
        }

        let command = try ConversionArgumentParser.parseConvert(rawArguments)
        guard command.appleFeatures.photographicStyles else { return false }
        guard !command.appleFeatures.portrait else {
            throw CLIError.invalidValue(
                option: "--apple-portrait",
                value: "Motion Photo + Photographic Styles is supported; combined Motion Photo + Portrait still requires a portrait-capable fixture"
            )
        }
        guard !command.oppoCompatibility.wantsOppoCompat else {
            throw CLIError.invalidValue(
                option: "--oppo-compatible",
                value: "Motion Photo conversion produces an Apple Live Photo pair"
            )
        }

        let outputImageURL: URL
        if rawArguments.contains("--output") {
            outputImageURL = command.outputURL.standardizedFileURL
        } else {
            var reserved = Set<String>()
            outputImageURL = MotionPhotoBatchPlanner.reserveOutputImageURL(
                for: inputURL,
                inputRootURL: inputURL.deletingLastPathComponent(),
                outputDirectoryURL: inputURL.deletingLastPathComponent(),
                reservedPaths: &reserved,
                fileExists: { _ in false }
            )
        }
        let outputExtension = outputImageURL.pathExtension.lowercased()
        guard outputExtension == "heic" || outputExtension == "heif" else {
            throw CLIError.invalidValue(
                option: "--output",
                value: "Motion Photo + Photographic Styles still output must use .heic or .heif"
            )
        }

        let result = try AppleLivePhotoConversionEngine.convert(
            inputURL: inputURL,
            outputImageURL: outputImageURL,
            photographicStylesConfiguration: command.configuration
        )
        print(
            "converted Motion Photo + Photographic Styles \(inputURL.lastPathComponent) -> "
                + "\(result.imageURL.path) + \(result.videoURL.path)"
        )
        for diagnostic in result.diagnostics {
            print("  \(diagnostic)")
        }
        return true
    }

    private static func optionValue(_ option: String, in arguments: [String]) -> String? {
        guard let index = arguments.firstIndex(of: option), index + 1 < arguments.count else {
            return nil
        }
        return arguments[index + 1]
    }

    private static func isSupportedMotionPhotoExtension(_ url: URL) -> Bool {
        switch url.pathExtension.lowercased() {
        case "jpg", "jpeg", "heic", "heif": return true
        default: return false
        }
    }
}
