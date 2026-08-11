import Foundation
import XDRemuxAppleFeatures

let arguments = Array(CommandLine.arguments.dropFirst())
do {
    if try !MotionPhotoCLIIntegration.handleIfNeeded(arguments) {
        XDRemuxCommand.main()
    }
} catch {
    MotionPhotoCLIIntegration.printFailure(error)
    exit(1)
}
