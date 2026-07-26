
import Foundation
import CoreGraphics
import CoreVideo
import Darwin
import ImageIO
import UniformTypeIdentifiers
import CryptoKit

package let cgImageDestinationEncodeGainMapSubsampleFactorCompat =
    "kCGImageDestinationEncodeGainMapSubsampleFactor" as CFString

public enum XDRemuxError: Error, CustomStringConvertible {
    case usage(String)
    case invalidCommand(String)
    case missingArgument(String)
    case unknownOption(String)
    case invalidValue(option: String, value: String)
    case inputNotFound(URL)
    case noFilesMatched(URL, String)
    case unableToRead(URL)
    case unableToReadCheckpoint(URL)
    case unableToWriteCheckpoint(URL)
    case invalidCheckpoint(URL, String)
    case checkpointConfigMismatch(URL, expected: String, actual: String)
    case batchFailed(failures: Int, checkpoint: URL)
    case categorizationFailed(failures: Int)
    case unableToCreateDirectory(URL)
    case outputParentIsNotDirectory(URL)
    case outputPathCollision(output: URL, firstInput: URL, secondInput: URL)
    /// The file carries no OPPO/OnePlus/realme Local HDR payload. `detail`
    /// holds the internal reason for bug reports; the message leads with the
    /// two explanations that actually apply — wrong photo, or already done.
    case notAProXDRPhoto(URL, detail: String)
    /// The file already carries an ISO 21496-1 gain map, so there is nothing
    /// left to remux. Distinct from `notAProXDRPhoto` because the right
    /// response is "you are finished", not "you picked the wrong file".
    case alreadyConverted(URL)
    case qtiMarkerNotFound
    case manifestNotFound
    case invalidLHDR(String)
    case unableToDecodeMask(URL)
    case unableToLoadBaseImage(URL)
    case unableToCreateDestination(URL)
    case unableToFinalizeDestination(URL)
    case unableToCreateMetadata
    case unableToWriteDebugAsset(URL)
    case outputVerificationFailed(URL)
    case gainMapPixelFormatMismatch(URL, expected: UInt32, actual: UInt32?)
    case invalidContainer(String)
    /// The input simply is not an OPPO portrait: the private tail lacks the
    /// depth bundle Apple Portrait needs. Distinct from `invalidContainer` so a
    /// combined Styles+Portrait run can degrade to styles-only for this case
    /// alone and still surface every genuine portrait failure.
    case portraitPrerequisitesMissing(String)
    case appleFeatureRuntimeUnavailable(String)
    case appleFeatureConversionFailed(status: Int32, log: String)

    /// One short line naming what went wrong, without the explanatory paragraph
    /// `description` adds and without repeating a path the caller already
    /// printed. List output uses this so one failure stays one readable line.
    public var headline: String {
        switch self {
        case .notAProXDRPhoto:
            return "not a ProXDR photo"
        case .alreadyConverted:
            return "already converted"
        default:
            return description.split(whereSeparator: \.isNewline).first.map(String.init)
                ?? description
        }
    }

    public var description: String {
        switch self {
        case .usage(let message):
            return message
        case .invalidCommand(let command):
            return "invalid command: \(command)"
        case .missingArgument(let name):
            return "missing required argument: \(name)"
        case .unknownOption(let option):
            return "unknown option: \(option)"
        case .invalidValue(let option, let value):
            return "invalid value for \(option): \(value)"
        case .inputNotFound(let url):
            return "input not found: \(url.path)"
        case .noFilesMatched(let url, let glob):
            return "no files matched \(glob) under \(url.path) (the search includes subfolders; check --glob)"
        case .unableToRead(let url):
            return "unable to read file: \(url.path)"
        case .unableToReadCheckpoint(let url):
            return "unable to read checkpoint: \(url.path)"
        case .unableToWriteCheckpoint(let url):
            return "unable to write checkpoint: \(url.path)"
        case .invalidCheckpoint(let url, let message):
            return "invalid checkpoint \(url.path): \(message)"
        case .checkpointConfigMismatch(let url, let expected, let actual):
            return "checkpoint config mismatch in \(url.path): expected \(expected), got \(actual) (use --no-resume or a different --checkpoint)"
        case .batchFailed(let failures, let checkpoint):
            return "\(failures) file(s) failed to convert; run the same command again to retry "
                + "only those files (checkpoint: \(checkpoint.path))"
        case .categorizationFailed(let failures):
            return "\(failures) file(s) could not be categorized; they were left where they were"
        case .unableToCreateDirectory(let url):
            return "unable to create directory: \(url.path)"
        case .outputParentIsNotDirectory(let url):
            return "output parent is not a directory: \(url.path)"
        case .outputPathCollision(let output, let firstInput, let secondInput):
            return "output path collision \(output.path) (two inputs map to the same output): \(firstInput.path) and \(secondInput.path)"
        case .notAProXDRPhoto(let url, let detail):
            return "not a ProXDR photo: \(url.path)\n"
                + "  XDRemux converts OPPO, OnePlus, and realme photos shot with ProXDR on. "
                + "This file carries no Local HDR data, so there is nothing to convert.\n"
                + "  detail: \(detail)"
        case .alreadyConverted(let url):
            return "already converted: \(url.path)\n"
                + "  This file already carries an ISO 21496-1 gain map. "
                + "Converting it again would not change anything."
        case .qtiMarkerNotFound:
            return "no OPPO Local HDR payload found (no QTI extension marker)"
        case .manifestNotFound:
            return "no OPPO Local HDR payload found (the embedded data index is missing)"
        case .invalidLHDR(let message):
            return "the photo's ProXDR HDR data is damaged or unreadable: \(message)"
        case .unableToDecodeMask(let url):
            return "cannot decode the ProXDR gain-map image inside: \(url.path)"
        case .unableToLoadBaseImage(let url):
            return "cannot decode the photo's SDR base image: \(url.path)"
        case .unableToCreateDestination(let url):
            return "unable to create HEIC destination: \(url.path)"
        case .unableToFinalizeDestination(let url):
            return "failed to finalize HEIC destination: \(url.path)"
        case .unableToCreateMetadata:
            return "unable to create HDR tone-map metadata"
        case .unableToWriteDebugAsset(let url):
            return "unable to write debug artifact: \(url.path)"
        case .outputVerificationFailed(let url):
            return "the converted file has no ISO gain map, so it was rejected: \(url.path)"
        case .gainMapPixelFormatMismatch(let url, let expected, let actual):
            return "gain map pixel format mismatch in \(url.path): expected \(fourCCString(expected)), got \(fourCCString(actual))"
        case .invalidContainer(let message):
            return "invalid HEIC container: \(message)"
        case .portraitPrerequisitesMissing(let message):
            return "not an OPPO portrait photo: \(message)"
        case .appleFeatureRuntimeUnavailable(let message):
            return "Apple feature runtime unavailable: \(message)"
        case .appleFeatureConversionFailed(let status, let log):
            return "Apple feature conversion failed (exit \(status)): \(log)"
        }
    }
}

package typealias CLIError = XDRemuxError

package func fourCCString(_ value: UInt32?) -> String {
    guard let value else { return "missing" }
    var bigEndian = value.bigEndian
    let label = withUnsafeBytes(of: &bigEndian) { buffer -> String in
        let bytes = buffer.map { byte -> UInt8 in
            (32...126).contains(byte) ? byte : UInt8(ascii: ".")
        }
        return String(bytes: bytes, encoding: .ascii) ?? "????"
    }
    return "\(value) ('\(label)')"
}

public enum Family: String, CaseIterable, Codable, Sendable, Identifiable, Hashable {
    case auto
    case x6
    case x7

    public var id: String { rawValue }
}

public enum InputProcessingBranch: String, CaseIterable, Codable, Sendable, Identifiable, Hashable {
    case system
    case systemDecoded = "system-decoded"
    case hybrid
    case passthrough

    public var id: String { rawValue }
}

public enum TmapFormat: String, CaseIterable, Codable, Sendable, Identifiable, Hashable {
    case strict = "strict"
    case imageIO = "imageio"

    public var id: String { rawValue }
}

public enum OppoCompatibility: String, CaseIterable, Codable, Sendable, Identifiable, Hashable {
    case auto
    case iso
    case isoNoLocal = "iso-no-local"
    case isoGraph = "iso-graph"
    case on
    case tail
    case off

    public var id: String { rawValue }
    public var wantsOppoCompat: Bool { self != .off }
}

public enum OppoCameraTail: String, CaseIterable, Codable, Sendable, Identifiable, Hashable {
    case off
    case watermark
    case compact
    case preserve
    case preserveWithoutPortrait = "preserve-without-portrait"
    case preserveWithoutPortraitOrPrivateHDR = "preserve-without-portrait-or-private-hdr"
    case preserveWithoutPrivateUHDR = "preserve-without-private-uhdr"
    case preserveWithoutPrivateHDR = "preserve-without-private-hdr"
    case preserveNoUHDR = "preserve-no-uhdr"
    case preserveNoHDR = "preserve-no-hdr"

    public var id: String { rawValue }
}

public struct AppleFeatureOptions: Hashable, Sendable {
    public var photographicStyles: Bool
    public var portrait: Bool

    public init(photographicStyles: Bool = false, portrait: Bool = false) {
        self.photographicStyles = photographicStyles
        self.portrait = portrait
    }

    public static let disabled = AppleFeatureOptions()

    public var isEnabled: Bool { photographicStyles || portrait }

    public var stableDescription: String {
        "styles=\(photographicStyles);portrait=\(portrait)"
    }
}

public enum AppleStyleDataProducerMode: String, CaseIterable, Sendable, Hashable {
    case unspecified
    case constrainedSolver = "constrained-solver"
    case learnNodeDiagnostic = "learn-node"
    case identityFallback = "identity-fallback"

    public var resolvedForPhotographicStyles: Self {
        self == .unspecified ? .constrainedSolver : self
    }
}

public enum ConversionEvent: Equatable, Sendable {
    case diagnostic(String)
}

public typealias ConversionEventHandler = @Sendable (ConversionEvent) -> Void

public struct ConversionConfiguration: Sendable {
    public var family: Family
    public var outputDirectory: URL?
    public var oppoCompatibility: OppoCompatibility
    public var inputProcessingBranch: InputProcessingBranch
    public var oppoCameraTail: OppoCameraTail
    public var tmapFormat: TmapFormat
    public var debugDirectory: URL?
    public var fileNameSuffix: String
    public var skipExisting: Bool
    public var maxConcurrentJobs: Int
    public var categorizeOutputByCaptureMode: Bool
    public var applePhotographicStyles: Bool
    public var applePortrait: Bool
    public var appleStylesRawDNGURL: URL?
    public var appleStyleDataProducer: AppleStyleDataProducerMode
    public var eventHandler: ConversionEventHandler?

    public init(
        family: Family = .auto,
        outputDirectory: URL? = nil,
        oppoCompatibility: OppoCompatibility = .off,
        inputProcessingBranch: InputProcessingBranch = .hybrid,
        oppoCameraTail: OppoCameraTail = .preserveWithoutPrivateHDR,
        tmapFormat: TmapFormat = .imageIO,
        debugDirectory: URL? = nil,
        fileNameSuffix: String = "_iso",
        skipExisting: Bool = true,
        maxConcurrentJobs: Int = min(ProcessInfo.processInfo.activeProcessorCount, 4),
        categorizeOutputByCaptureMode: Bool = false,
        applePhotographicStyles: Bool = false,
        applePortrait: Bool = false,
        appleStylesRawDNGURL: URL? = nil,
        appleStyleDataProducer: AppleStyleDataProducerMode = .unspecified,
        eventHandler: ConversionEventHandler? = nil
    ) {
        self.family = family
        self.outputDirectory = outputDirectory
        self.oppoCompatibility = oppoCompatibility
        self.inputProcessingBranch = inputProcessingBranch
        self.oppoCameraTail = oppoCameraTail
        self.tmapFormat = tmapFormat
        self.debugDirectory = debugDirectory
        self.fileNameSuffix = fileNameSuffix
        self.skipExisting = skipExisting
        self.maxConcurrentJobs = maxConcurrentJobs
        self.categorizeOutputByCaptureMode = categorizeOutputByCaptureMode
        self.applePhotographicStyles = applePhotographicStyles
        self.applePortrait = applePortrait
        self.appleStylesRawDNGURL = appleStylesRawDNGURL
        self.appleStyleDataProducer = appleStyleDataProducer
        self.eventHandler = eventHandler
    }

    public var appleFeatureOptions: AppleFeatureOptions {
        get {
            AppleFeatureOptions(
                photographicStyles: applePhotographicStyles,
                portrait: applePortrait
            )
        }
        set {
            applePhotographicStyles = newValue.photographicStyles
            applePortrait = newValue.portrait
        }
    }

    public var appleFeaturesEnabled: Bool {
        applePhotographicStyles || applePortrait
    }

    public var oppoGalleryCompatibilityEnabled: Bool {
        get { oppoCompatibility.wantsOppoCompat }
        set {
            oppoCompatibility = newValue ? .auto : .off
            if newValue {
                applePhotographicStyles = false
                applePortrait = false
            }
            oppoCameraTail = preservesPortraitEditingData
                ? (newValue ? .preserve : .preserveWithoutPrivateHDR)
                : (newValue ? .preserveWithoutPortrait : .preserveWithoutPortraitOrPrivateHDR)
        }
    }

    public var preservesPortraitEditingData: Bool {
        get {
            oppoCameraTail != .preserveWithoutPortrait
                && oppoCameraTail != .preserveWithoutPortraitOrPrivateHDR
        }
        set {
            oppoCameraTail = newValue
                ? (oppoCompatibility.wantsOppoCompat ? .preserve : .preserveWithoutPrivateHDR)
                : (oppoCompatibility.wantsOppoCompat ? .preserveWithoutPortrait : .preserveWithoutPortraitOrPrivateHDR)
        }
    }
}

public typealias ConversionConfig = ConversionConfiguration

public struct InputSource: Hashable, Sendable {
    public let url: URL

    public init(url: URL) {
        self.url = url
    }
}

public struct OutputDestination: Hashable, Sendable {
    public let url: URL

    public init(url: URL) {
        self.url = url
    }
}

public enum OutputTarget: Hashable, Sendable {
    case replaceInput
    case file(URL)

    public func destination(for source: InputSource) -> OutputDestination {
        switch self {
        case .replaceInput:
            return OutputDestination(url: source.url)
        case .file(let url):
            return OutputDestination(url: url)
        }
    }
}

public struct ConversionRequest: Sendable {
    public let input: InputSource
    public let output: OutputDestination
    public var configuration: ConversionConfiguration

    public init(
        input: InputSource,
        output: OutputDestination,
        configuration: ConversionConfiguration = ConversionConfiguration()
    ) {
        self.input = input
        self.output = output
        self.configuration = configuration
    }
}

public struct ConversionResult: Sendable {
    public let input: InputSource
    public let output: OutputDestination

    public init(input: InputSource, output: OutputDestination) {
        self.input = input
        self.output = output
    }
}
