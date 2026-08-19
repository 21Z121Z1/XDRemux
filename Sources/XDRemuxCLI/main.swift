import ArgumentParser
import Foundation
import XDRemuxAppleFeatures

let arguments = Array(CommandLine.arguments.dropFirst())
let parserControlFlags: Set<String> = [
    "--help",
    "-h",
    "--help-hidden",
    "--generate-completion-script",
    "--experimental-dump-help",
]
let isParserControlRequest = arguments.first == "help"
    || arguments.contains(where: parserControlFlags.contains)

do {
    if isParserControlRequest {
        // Help/completion requests belong entirely to ArgumentParser. In
        // particular, do not let the Motion Photo batch pre-parser consume a
        // command such as `batch --help` as if it were a conversion request.
        XDRemuxRootCommand.main(arguments)
    } else if try MotionPhotoCLIIntegration.handleIfNeeded(arguments) {
        // The Motion Photo integration fully handled this command.
    } else {
        XDRemuxRootCommand.main(arguments)
        try MotionPhotoCLIIntegration.finishPendingBatchIfNeeded()
    }
} catch {
    MotionPhotoCLIIntegration.printFailure(error)
    exit(1)
}
