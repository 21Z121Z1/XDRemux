import AVFoundation
import CryptoKit
import Foundation
import ImageIO
import Photos
import UIKit
import UniformTypeIdentifiers

// Separate cold SDK/module setup from type-checking the real reader. This
// canary never launches an app or accesses any library.
@main
struct SDKCompileProbe {
    static func main() { print("SDK imports compiled; this is not native readback") }
}
