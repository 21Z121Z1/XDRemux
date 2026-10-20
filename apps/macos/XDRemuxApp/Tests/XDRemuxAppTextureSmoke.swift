import Foundation

@main
struct XDRemuxAppTextureSmoke {
    static func main() throws {
        let args = Array(CommandLine.arguments.dropFirst())
        precondition(args.count == 3)
        let input = URL(fileURLWithPath: args[0])
        let baseline = URL(fileURLWithPath: args[1])
        let output = URL(fileURLWithPath: args[2])
        var config = ConversionConfig()
        precondition(AppConversionEngine.isValidOutput(baseline, config: config))
        config.applePhotographicStyles = true
        precondition(!AppConversionEngine.isValidOutput(baseline, config: config),
                     "A plain HDR output must not skip the combined Styles conversion")
        try AppConversionEngine.convert(inputURL: input, outputURL: output, config: config)
        precondition(AppConversionEngine.isValidOutput(output, config: config))
        print("app-styles-texture-bridge-pass")
    }
}
