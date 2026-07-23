import Foundation
import XDRemuxCore

enum CLIOutput {
    static let conversionEventHandler: ConversionEventHandler = { event in
        switch event {
        case .diagnostic(let message):
            FileHandle.standardError.write(Data("\(message)\n".utf8))
        }
    }
}
