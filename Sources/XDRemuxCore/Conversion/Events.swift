import Foundation

public struct MessageKey: RawRepresentable, Hashable, Codable, Sendable {
    public let rawValue: String

    public init(rawValue: String) {
        self.rawValue = rawValue
    }
}

public extension MessageKey {
    static let errorSourceNotFound = MessageKey(rawValue: "error.source_not_found")
    static let errorSourceNotSupported = MessageKey(rawValue: "error.source_not_supported")
    static let errorSourceGainMapMissing = MessageKey(rawValue: "error.source_gain_map_missing")
    static let errorSourceGainMapCorrupt = MessageKey(rawValue: "error.source_gain_map_corrupt")
    static let errorPortraitDataUnavailable = MessageKey(rawValue: "error.portrait_data_unavailable")
    static let errorAppleRuntimeUnavailable = MessageKey(rawValue: "error.apple_runtime_unavailable")
    static let errorOutputNotWritable = MessageKey(rawValue: "error.output_not_writable")
    static let errorOutputVerificationFailed = MessageKey(rawValue: "error.output_verification_failed")
    static let errorInternalContainer = MessageKey(rawValue: "error.internal_container_error")
    static let errorInvalidArguments = MessageKey(rawValue: "error.invalid_arguments")
    static let errorBatchIncomplete = MessageKey(rawValue: "error.batch_incomplete")

    static let recoveryCheckSource = MessageKey(rawValue: "recovery.check_source")
    static let recoveryUseSupportedSource = MessageKey(rawValue: "recovery.use_supported_source")
    static let recoveryUsePortraitSource = MessageKey(rawValue: "recovery.use_portrait_source")
    static let recoveryCheckAppleRuntime = MessageKey(rawValue: "recovery.check_apple_runtime")
    static let recoveryCheckOutput = MessageKey(rawValue: "recovery.check_output")
    static let recoveryRetry = MessageKey(rawValue: "recovery.retry")

    static let warningPortraitFlagRecovered = MessageKey(rawValue: "warning.portrait_flag_recovered")
    static let warningPortraitUnavailable = MessageKey(rawValue: "warning.portrait_unavailable")
    static let warningPrivateBridgeFallback = MessageKey(rawValue: "warning.private_bridge_fallback")
}

public enum ConversionPhase: String, CaseIterable, Codable, Sendable {
    case readingSource = "reading_source"
    case extractingGainMap = "extracting_gain_map"
    case reconstructingHDR = "reconstructing_hdr"
    case generatingPhotographicStyles = "generating_photographic_styles"
    case generatingPortraitResources = "generating_portrait_resources"
    case writingContainer = "writing_container"
    case verifyingOutput = "verifying_output"
}

public enum FailureCode: String, Codable, Sendable {
    case sourceNotFound = "source_not_found"
    case sourceNotSupported = "source_not_supported"
    case sourceGainMapMissing = "source_gain_map_missing"
    case sourceGainMapCorrupt = "source_gain_map_corrupt"
    case portraitDataUnavailable = "portrait_data_unavailable"
    case appleRuntimeUnavailable = "apple_runtime_unavailable"
    case outputNotWritable = "output_not_writable"
    case outputVerificationFailed = "output_verification_failed"
    case internalContainerError = "internal_container_error"
    case invalidArguments = "invalid_arguments"
    case batchIncomplete = "batch_incomplete"
}

public enum WarningCode: String, Codable, Sendable {
    case portraitFlagRecovered = "portrait_flag_recovered"
    case portraitUnavailable = "portrait_unavailable"
    case privateBridgeFallback = "private_bridge_fallback"
}

public struct ConversionWarning: Codable, Sendable, Equatable {
    public let code: WarningCode
    public let messageKey: MessageKey
    public let diagnostics: String

    public init(code: WarningCode, messageKey: MessageKey, diagnostics: String) {
        self.code = code
        self.messageKey = messageKey
        self.diagnostics = diagnostics
    }
}

public struct ConversionFailure: Error, @unchecked Sendable {
    public let code: FailureCode
    public let userSummaryKey: MessageKey
    public let recoverySuggestionKey: MessageKey?
    public let diagnostics: String
    public let underlyingError: (any Error)?

    public init(
        code: FailureCode,
        userSummaryKey: MessageKey,
        recoverySuggestionKey: MessageKey?,
        diagnostics: String,
        underlyingError: (any Error)? = nil
    ) {
        self.code = code
        self.userSummaryKey = userSummaryKey
        self.recoverySuggestionKey = recoverySuggestionKey
        self.diagnostics = diagnostics
        self.underlyingError = underlyingError
    }

    public static func classify(_ error: any Error) -> ConversionFailure {
        if let failure = error as? ConversionFailure {
            return failure
        }

        let diagnostics = String(describing: error)
        guard let error = error as? XDRemuxError else {
            return ConversionFailure(
                code: .internalContainerError,
                userSummaryKey: .errorInternalContainer,
                recoverySuggestionKey: .recoveryRetry,
                diagnostics: diagnostics,
                underlyingError: error
            )
        }

        let mapping: (FailureCode, MessageKey, MessageKey?)
        switch error {
        case .inputNotFound:
            mapping = (.sourceNotFound, .errorSourceNotFound, .recoveryCheckSource)
        case .qtiMarkerNotFound, .manifestNotFound:
            mapping = (.sourceGainMapMissing, .errorSourceGainMapMissing, .recoveryUseSupportedSource)
        case .invalidLHDR, .unableToDecodeMask:
            mapping = (.sourceGainMapCorrupt, .errorSourceGainMapCorrupt, .recoveryUseSupportedSource)
        case .unableToRead, .unableToLoadBaseImage:
            mapping = (.sourceNotSupported, .errorSourceNotSupported, .recoveryUseSupportedSource)
        case .appleFeatureRuntimeUnavailable, .appleFeatureConversionFailed:
            mapping = (.appleRuntimeUnavailable, .errorAppleRuntimeUnavailable, .recoveryCheckAppleRuntime)
        case .unableToCreateDirectory, .outputParentIsNotDirectory,
             .unableToCreateDestination, .unableToFinalizeDestination,
             .unableToWriteDebugAsset:
            mapping = (.outputNotWritable, .errorOutputNotWritable, .recoveryCheckOutput)
        case .outputVerificationFailed, .gainMapPixelFormatMismatch:
            mapping = (.outputVerificationFailed, .errorOutputVerificationFailed, .recoveryRetry)
        case .usage, .invalidCommand, .missingArgument, .unknownOption, .invalidValue:
            mapping = (.invalidArguments, .errorInvalidArguments, nil)
        case .invalidContainer(let message) where message.localizedCaseInsensitiveContains("portrait")
            || message.contains("rear.depth") || message.contains("src.image"):
            mapping = (.portraitDataUnavailable, .errorPortraitDataUnavailable, .recoveryUsePortraitSource)
        case .noFilesMatched:
            mapping = (.sourceNotFound, .errorSourceNotFound, .recoveryCheckSource)
        default:
            mapping = (.internalContainerError, .errorInternalContainer, .recoveryRetry)
        }
        return ConversionFailure(
            code: mapping.0,
            userSummaryKey: mapping.1,
            recoverySuggestionKey: mapping.2,
            diagnostics: diagnostics,
            underlyingError: error
        )
    }
}

public enum ConversionEvent: @unchecked Sendable {
    case started(input: URL, output: URL)
    case phaseChanged(ConversionPhase)
    case progress(completed: Int, total: Int?)
    case warning(ConversionWarning)
    case completed(ConversionResult)
    case failed(ConversionFailure)
    case diagnostic(String)
}

public typealias ConversionEventHandler = @Sendable (ConversionEvent) -> Void

public final class ConversionCancellation: @unchecked Sendable {
    private let lock = NSLock()
    private var cancelled = false

    public init() {}

    public func cancel() {
        lock.lock()
        cancelled = true
        lock.unlock()
    }

    public var isCancelled: Bool {
        lock.lock()
        defer { lock.unlock() }
        return cancelled
    }

    public func checkCancellation() throws {
        if isCancelled {
            throw CancellationError()
        }
    }
}
