import Foundation
import XDRemuxAppleFeatures

let arguments = Array(CommandLine.arguments.dropFirst())
do {
    if try MotionPhotoCLIIntegration.handleIfNeeded(arguments) {
        // The Motion Photo integration fully handled this command.
    } else {
        XDRemuxCommand.main()
        try MotionPhotoCLIIntegration.finishPendingBatchIfNeeded()
    }
} catch {
    MotionPhotoCLIIntegration.printFailure(error)
    exit(1)
}
