import Foundation
import XDRemuxCore

extension MessageKey {
    static let helpTitle = MessageKey(rawValue: "help.title")
    static let helpUsage = MessageKey(rawValue: "help.usage")
    static let helpCommands = MessageKey(rawValue: "help.commands")
    static let helpConvert = MessageKey(rawValue: "help.command.convert")
    static let helpBatch = MessageKey(rawValue: "help.command.batch")
    static let helpOptions = MessageKey(rawValue: "help.options")
    static let helpDeveloper = MessageKey(rawValue: "help.developer")
    static let helpOutput = MessageKey(rawValue: "help.output")
    static let helpLanguage = MessageKey(rawValue: "help.language")

    static let labelInput = MessageKey(rawValue: "label.input")
    static let labelOutput = MessageKey(rawValue: "label.output")
    static let labelMode = MessageKey(rawValue: "label.mode")
    static let labelCurrent = MessageKey(rawValue: "label.current")
    static let labelConverted = MessageKey(rawValue: "label.converted")
    static let labelSkipped = MessageKey(rawValue: "label.skipped")
    static let labelFailed = MessageKey(rawValue: "label.failed")
    static let labelActive = MessageKey(rawValue: "label.active")

    static let modeStandard = MessageKey(rawValue: "mode.standard")
    static let modeOppo = MessageKey(rawValue: "mode.oppo")
    static let modeStyles = MessageKey(rawValue: "mode.styles")
    static let modePortrait = MessageKey(rawValue: "mode.portrait")
    static let modeCombined = MessageKey(rawValue: "mode.combined")

    static let phaseReadingSource = MessageKey(rawValue: "phase.reading_source")
    static let phaseExtractingGainMap = MessageKey(rawValue: "phase.extracting_gain_map")
    static let phaseReconstructingHDR = MessageKey(rawValue: "phase.reconstructing_hdr")
    static let phaseGeneratingStyles = MessageKey(rawValue: "phase.generating_photographic_styles")
    static let phaseGeneratingPortrait = MessageKey(rawValue: "phase.generating_portrait_resources")
    static let phaseWritingContainer = MessageKey(rawValue: "phase.writing_container")
    static let phaseVerifyingOutput = MessageKey(rawValue: "phase.verifying_output")

    static let statusSingleCompleted = MessageKey(rawValue: "status.single_completed")
    static let statusSingleSkipped = MessageKey(rawValue: "status.single_skipped")
    static let statusBatchStarted = MessageKey(rawValue: "status.batch_started")
    static let statusBatchProgress = MessageKey(rawValue: "status.batch_progress")
    static let statusBatchCompleted = MessageKey(rawValue: "status.batch_completed")
    static let statusFileCompleted = MessageKey(rawValue: "status.file_completed")
    static let statusFileSkipped = MessageKey(rawValue: "status.file_skipped")
    static let statusFileFailed = MessageKey(rawValue: "status.file_failed")
    static let statusWarningPlain = MessageKey(rawValue: "status.warning_plain")
    static let statusWarning = MessageKey(rawValue: "status.warning")
    static let statusError = MessageKey(rawValue: "status.error")
    static let statusRecovery = MessageKey(rawValue: "status.recovery")
    static let statusFailureReport = MessageKey(rawValue: "status.failure_report")

    static let argumentMissing = MessageKey(rawValue: "argument.missing")
    static let argumentUnknown = MessageKey(rawValue: "argument.unknown")
    static let argumentInvalid = MessageKey(rawValue: "argument.invalid")
    static let argumentInvalidCommand = MessageKey(rawValue: "argument.invalid_command")
    static let argumentIncompatible = MessageKey(rawValue: "argument.incompatible")
}

struct Localizer: Sendable {
    let language: OutputLanguage
    private let bundle: Bundle

    init(
        requested: OutputLanguage?,
        environment: [String: String] = ProcessInfo.processInfo.environment,
        preferredLanguages: [String] = Locale.preferredLanguages
    ) {
        let selected = Self.resolveLanguage(
            requested: requested,
            environmentValue: environment["XDREMUX_LANGUAGE"],
            preferredLanguages: preferredLanguages
        )
        language = selected
        let localization = selected == .simplifiedChinese ? "zh-hans" : "en"
        if let url = Bundle.module.url(forResource: localization, withExtension: "lproj"),
           let localizedBundle = Bundle(url: url) {
            bundle = localizedBundle
        } else {
            bundle = Bundle.module
        }
    }

    func text(_ key: MessageKey, _ arguments: CVarArg...) -> String {
        let format = formatString(for: key)
        guard !arguments.isEmpty else { return format }
        return String(format: format, locale: Locale(identifier: "en_US_POSIX"), arguments: arguments)
    }

    func formatString(for key: MessageKey) -> String {
        bundle.localizedString(forKey: key.rawValue, value: key.rawValue, table: "Localizable")
    }

    static func resolveLanguage(
        requested: OutputLanguage?,
        environmentValue: String?,
        preferredLanguages: [String]
    ) -> OutputLanguage {
        if let requested {
            if requested != .automatic { return requested }
            return match(preferredLanguages) ?? .english
        }
        if let environmentValue,
           let environmentLanguage = OutputLanguage(rawValue: environmentValue) {
            if environmentLanguage != .automatic { return environmentLanguage }
            return match(preferredLanguages) ?? .english
        }
        return match(preferredLanguages) ?? .english
    }

    private static func match(_ preferredLanguages: [String]) -> OutputLanguage? {
        for identifier in preferredLanguages {
            let normalized = identifier.lowercased()
            if normalized == "zh-cn" || normalized == "zh-hans" || normalized.hasPrefix("zh-hans-") {
                return .simplifiedChinese
            }
            if normalized == "en" || normalized.hasPrefix("en-") {
                return .english
            }
        }
        return nil
    }
}

extension ConversionPhase {
    var messageKey: MessageKey {
        switch self {
        case .readingSource: return .phaseReadingSource
        case .extractingGainMap: return .phaseExtractingGainMap
        case .reconstructingHDR: return .phaseReconstructingHDR
        case .generatingPhotographicStyles: return .phaseGeneratingStyles
        case .generatingPortraitResources: return .phaseGeneratingPortrait
        case .writingContainer: return .phaseWritingContainer
        case .verifyingOutput: return .phaseVerifyingOutput
        }
    }
}
