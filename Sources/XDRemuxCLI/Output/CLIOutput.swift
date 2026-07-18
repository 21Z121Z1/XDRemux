import Darwin
import Foundation
import XDRemuxCore

final class OutputWriter {
    private let body: (String) -> Void

    init(_ body: @escaping (String) -> Void) {
        self.body = body
    }

    func write(_ text: String) {
        body(text)
    }

    static func file(_ handle: FileHandle) -> OutputWriter {
        OutputWriter { text in
            handle.write(Data(text.utf8))
        }
    }
}

enum ConversionDisplayMode: String, Sendable {
    case standard
    case oppo
    case styles
    case portrait
    case combined

    init(options: ConversionOptions) {
        if options.appleFeatures.photographicStyles && options.appleFeatures.portrait {
            self = .combined
        } else if options.appleFeatures.photographicStyles {
            self = .styles
        } else if options.appleFeatures.portrait {
            self = .portrait
        } else if options.oppoCompatibility.wantsOppoCompat {
            self = .oppo
        } else {
            self = .standard
        }
    }

    var messageKey: MessageKey {
        switch self {
        case .standard: return .modeStandard
        case .oppo: return .modeOppo
        case .styles: return .modeStyles
        case .portrait: return .modePortrait
        case .combined: return .modeCombined
        }
    }
}

enum BatchFileOutcome {
    case converted
    case skipped
    case failed(ConversionFailure)
}

struct BatchProgressSnapshot: Equatable {
    var total: Int
    var completed = 0
    var converted = 0
    var skipped = 0
    var failed = 0
    var active = 0
    var current: String?
}

final class CLIReporter: @unchecked Sendable {
    private let options: OutputOptions
    private let localizer: Localizer
    private let stdout: OutputWriter
    private let stderr: OutputWriter
    private let interactive: Bool
    private let lock = NSRecursiveLock()
    private var structuredEvents: [[String: Any]] = []
    private var phases: [ConversionPhase] = []
    private var currentPhase: ConversionPhase?
    private var batch: BatchProgressSnapshot?
    private var progressStride = 1
    private var lastLineProgress = 0
    private var cursorHidden = false
    private var dynamicLineCount = 0
    private var finished = false
    private var reportedFailureKeys = Set<String>()
    private var reportedWarningKeys = Set<String>()

    init(
        options: OutputOptions,
        localizer: Localizer,
        isTTY: Bool? = nil,
        environment: [String: String] = ProcessInfo.processInfo.environment,
        stdout: OutputWriter = .file(.standardOutput),
        stderr: OutputWriter = .file(.standardError)
    ) {
        self.options = options
        self.localizer = localizer
        self.stdout = stdout
        self.stderr = stderr
        let terminal = isTTY ?? (isatty(STDERR_FILENO) == 1)
        interactive = terminal && environment["CI"] == nil && options.format == .text
    }

    deinit {
        finish()
    }

    static func plannedPhases(for options: ConversionOptions) -> [ConversionPhase] {
        var result: [ConversionPhase] = [
            .readingSource,
            .extractingGainMap,
            .reconstructingHDR,
        ]
        if options.appleFeatures.portrait {
            result.append(.generatingPortraitResources)
        }
        if options.appleFeatures.photographicStyles {
            result.append(.generatingPhotographicStyles)
        }
        result.append(contentsOf: [.writingContainer, .verifyingOutput])
        return result
    }

    func writeHelp(_ text: String) {
        lock.withLock {
            if options.format == .text {
                stdout.write(text + "\n")
            } else {
                emitStructured([
                    "event": "help",
                    "text": text,
                ])
            }
        }
    }

    func beginSingle(input: URL, output: URL, mode: ConversionDisplayMode, phases: [ConversionPhase]) {
        lock.withLock {
            self.phases = phases
            if options.verbosity != .quiet {
                emitStructured([
                    "event": "conversion_started",
                    "input": input.path,
                    "output": output.path,
                    "mode": mode.rawValue,
                ])
            }
            guard options.format == .text, options.verbosity != .quiet else { return }
            stderr.write("XDRemux\n\n")
            stderr.write("\(localizer.text(.labelInput))   \(input.lastPathComponent)\n")
            stderr.write("\(localizer.text(.labelOutput))  \(output.path)\n")
            stderr.write("\(localizer.text(.labelMode))    \(localizer.text(mode.messageKey))\n\n")
            beginDynamicIfNeeded()
        }
    }

    func handleSingle(_ event: ConversionEvent, input: URL, output: URL) {
        lock.withLock {
            switch event {
            case .started:
                break
            case .phaseChanged(let phase):
                showSinglePhase(phase)
            case .progress(let completed, let total):
                emitStructured([
                    "event": "conversion_progress",
                    "completed": completed,
                    "total": total.map { $0 as Any } ?? NSNull(),
                    "input": input.path,
                ])
            case .warning(let warning):
                reportWarning(warning, input: input)
            case .completed:
                completeSingle(input: input, output: output)
            case .failed(let failure):
                reportFailure(failure, input: input)
            case .diagnostic(let message):
                diagnostic(message, input: input)
            }
        }
    }

    func beginBatch(total: Int, jobs: Int, mode: ConversionDisplayMode) {
        lock.withLock {
            batch = BatchProgressSnapshot(total: total)
            progressStride = max(1, Int(ceil(Double(max(total, 1)) / 20.0)))
            if options.verbosity != .quiet {
                emitStructured([
                    "event": "batch_started",
                    "total": total,
                    "jobs": jobs,
                    "mode": mode.rawValue,
                ])
            }
            guard options.format == .text, options.verbosity != .quiet else { return }
            stderr.write(localizer.text(.statusBatchStarted, total, jobs) + "\n")
            beginDynamicIfNeeded()
            redrawBatch()
        }
    }

    func batchFileStarted(_ input: URL) {
        lock.withLock {
            guard var snapshot = batch else { return }
            snapshot.active += 1
            snapshot.current = input.lastPathComponent
            batch = snapshot
            redrawBatch()
        }
    }

    func handleBatchEvent(_ event: ConversionEvent, input: URL) {
        lock.withLock {
            switch event {
            case .warning(let warning): reportWarning(warning, input: input)
            case .diagnostic(let message): diagnostic(message, input: input)
            case .phaseChanged(let phase):
                currentPhase = phase
                if options.verbosity == .debug {
                    diagnostic("phase=\(phase.rawValue)", input: input)
                }
            default: break
            }
        }
    }

    func batchFileFinished(input: URL, output: URL, outcome: BatchFileOutcome) {
        lock.withLock {
            guard var snapshot = batch else { return }
            snapshot.active = max(0, snapshot.active - 1)
            snapshot.completed += 1
            switch outcome {
            case .converted:
                snapshot.converted += 1
                if options.verbosity != .quiet {
                    emitStructured([
                        "event": "conversion_completed",
                        "input": input.path,
                        "output": output.path,
                    ])
                }
                if options.format == .text, options.verbosity == .verbose || options.verbosity == .debug {
                    printAboveProgress(localizer.text(.statusFileCompleted, input.lastPathComponent))
                }
            case .skipped:
                snapshot.skipped += 1
                if options.verbosity != .quiet {
                    emitStructured([
                        "event": "conversion_skipped",
                        "input": input.path,
                        "output": output.path,
                        "reason": "existing_output_valid",
                    ])
                }
                if options.format == .text, options.verbosity == .verbose || options.verbosity == .debug {
                    printAboveProgress(localizer.text(.statusFileSkipped, input.lastPathComponent))
                }
            case .failed(let failure):
                snapshot.failed += 1
                reportFailure(failure, input: input)
            }
            batch = snapshot
            emitBatchProgress(snapshot)
            if !interactive,
               options.format == .text,
               options.verbosity != .quiet,
               (snapshot.completed == snapshot.total || snapshot.completed - lastLineProgress >= progressStride) {
                lastLineProgress = snapshot.completed
                stderr.write(progressText(snapshot) + "\n")
            }
            redrawBatch()
        }
    }

    func completeBatch(failureReportURL: URL?) {
        lock.withLock {
            guard let snapshot = batch else { return }
            clearDynamicLine()
            emitStructured([
                "event": "batch_completed",
                "total": snapshot.total,
                "converted": snapshot.converted,
                "skipped": snapshot.skipped,
                "failed": snapshot.failed,
                "failure_report": failureReportURL.map { $0.path as Any } ?? NSNull(),
            ])
            if options.format == .text {
                stderr.write(localizer.text(
                    .statusBatchCompleted,
                    snapshot.converted,
                    snapshot.skipped,
                    snapshot.failed
                ) + "\n")
                if let failureReportURL {
                    stderr.write(localizer.text(.statusFailureReport, failureReportURL.path) + "\n")
                }
            }
            restoreCursor()
        }
    }

    func reportFailure(_ failure: ConversionFailure, input: URL?) {
        lock.withLock {
            let inputPath = input?.path ?? ""
            let dedupeKey = "\(inputPath)|\(failure.code.rawValue)|\(failure.diagnostics)"
            guard reportedFailureKeys.insert(dedupeKey).inserted else { return }
            let summary = localizedFailureSummary(failure)
            var event: [String: Any] = [
                "event": "conversion_failed",
                "error_code": failure.code.rawValue,
                "input": inputPath,
                "message": summary,
            ]
            if options.verbosity == .debug {
                event["diagnostics"] = failure.diagnostics
                event["underlying_error"] = failure.underlyingError
                    .map { String(describing: $0) as Any } ?? NSNull()
            }
            emitStructured(event)
            guard options.format == .text else { return }
            let name = input?.lastPathComponent ?? "XDRemux"
            var message = localizer.text(.statusError, name, summary)
            if options.verbosity == .verbose || options.verbosity == .debug {
                message += " [\(failure.code.rawValue)]"
            }
            printAboveProgress(message)
            if let recovery = failure.recoverySuggestionKey {
                printAboveProgress(localizer.text(.statusRecovery, localizer.text(recovery)))
            }
            if options.verbosity == .debug {
                printAboveProgress("diagnostics: \(failure.diagnostics)")
                if let underlyingError = failure.underlyingError {
                    printAboveProgress("underlying: \(String(reflecting: underlyingError))")
                }
            }
        }
    }

    func diagnostic(_ message: String, input: URL? = nil) {
        lock.withLock {
            guard options.verbosity == .debug else { return }
            emitStructured([
                "event": "diagnostic",
                "input": input.map { $0.path as Any } ?? NSNull(),
                "message": message,
            ])
            if options.format == .text {
                printAboveProgress("debug: \(message)")
            }
        }
    }

    func finish() {
        lock.withLock {
            guard !finished else { return }
            finished = true
            clearDynamicLine()
            restoreCursor()
            if options.format == .json, !structuredEvents.isEmpty {
                let object: [String: Any] = [
                    "schema_version": 1,
                    "events": structuredEvents,
                ]
                writeJSON(object, newline: true)
            }
        }
    }

    private func showSinglePhase(_ phase: ConversionPhase) {
        currentPhase = phase
        guard options.verbosity != .quiet else { return }
        let index = (phases.firstIndex(of: phase) ?? 0) + 1
        let total = max(phases.count, index)
        emitStructured([
            "event": "phase_changed",
            "phase": phase.rawValue,
            "completed": index,
            "total": total,
        ])
        guard options.format == .text, options.verbosity != .quiet else { return }
        let text = "\(localizer.text(phase.messageKey))…  \(index)/\(total)"
        if interactive {
            dynamicWrite("⠹ \(text)")
        } else {
            stderr.write(text + "\n")
        }
    }

    private func completeSingle(input: URL, output: URL) {
        clearDynamicLine()
        emitStructured([
            "event": "conversion_completed",
            "input": input.path,
            "output": output.path,
        ])
        if options.format == .text {
            stderr.write(localizer.text(.statusSingleCompleted, output.path) + "\n")
        }
        restoreCursor()
    }

    private func reportWarning(_ warning: ConversionWarning, input: URL) {
        guard options.verbosity != .quiet else { return }
        if options.verbosity != .debug {
            let key = "\(input.path)|\(warning.code.rawValue)"
            guard reportedWarningKeys.insert(key).inserted else { return }
        }
        let message = localizer.text(warning.messageKey)
        var event: [String: Any] = [
            "event": "conversion_warning",
            "warning_code": warning.code.rawValue,
            "input": input.path,
            "message": message,
        ]
        if options.verbosity == .debug { event["diagnostics"] = warning.diagnostics }
        emitStructured(event)
        guard options.format == .text else { return }
        var line = options.verbosity == .normal
            ? localizer.text(.statusWarningPlain, message)
            : localizer.text(.statusWarning, warning.code.rawValue, message)
        if options.verbosity == .debug { line += "\n  \(warning.diagnostics)" }
        printAboveProgress(line)
    }

    private func localizedFailureSummary(_ failure: ConversionFailure) -> String {
        if failure.code == .invalidArguments, let error = failure.underlyingError as? XDRemuxError {
            switch error {
            case .missingArgument(let option): return localizer.text(.argumentMissing, option)
            case .unknownOption(let option): return localizer.text(.argumentUnknown, option)
            case .invalidValue(let option, let value):
                if value.contains("cannot be combined with") {
                    let other = value.replacingOccurrences(of: "cannot be combined with ", with: "")
                    return localizer.text(.argumentIncompatible, option, other)
                }
                return localizer.text(.argumentInvalid, option, value)
            case .invalidCommand(let command): return localizer.text(.argumentInvalidCommand, command)
            default: break
            }
        }
        return localizer.text(failure.userSummaryKey)
    }

    private func emitBatchProgress(_ snapshot: BatchProgressSnapshot) {
        guard options.verbosity != .quiet else { return }
        emitStructured([
            "event": "batch_progress",
            "completed": snapshot.completed,
            "total": snapshot.total,
            "converted": snapshot.converted,
            "skipped": snapshot.skipped,
            "failed": snapshot.failed,
            "active": snapshot.active,
            "current": snapshot.current.map { $0 as Any } ?? NSNull(),
        ])
    }

    private func redrawBatch() {
        guard interactive, options.verbosity != .quiet, let snapshot = batch else { return }
        dynamicWrite(batchProgressText(snapshot))
    }

    private func batchProgressText(_ snapshot: BatchProgressSnapshot) -> String {
        let total = max(snapshot.total, 1)
        let fraction = min(1, Double(snapshot.completed) / Double(total))
        let filled = min(20, Int((fraction * 20).rounded(.down)))
        let bar = String(repeating: "█", count: filled) + String(repeating: "░", count: 20 - filled)
        let percent = Int((fraction * 100).rounded(.down))
        var lines = "[\(bar)] \(snapshot.completed)/\(snapshot.total)  \(percent)%\n"
        lines += "\(localizer.text(.labelConverted)) \(snapshot.converted) · "
        lines += "\(localizer.text(.labelSkipped)) \(snapshot.skipped) · "
        lines += "\(localizer.text(.labelFailed)) \(snapshot.failed) · "
        lines += "\(localizer.text(.labelActive)) \(snapshot.active)"
        if let current = snapshot.current {
            lines += "\n\(localizer.text(.labelCurrent)): \(current)"
        }
        return lines
    }

    private func progressText(_ snapshot: BatchProgressSnapshot) -> String {
        localizer.text(
            .statusBatchProgress,
            snapshot.completed,
            snapshot.total,
            snapshot.converted,
            snapshot.skipped,
            snapshot.failed,
            snapshot.active
        )
    }

    private func beginDynamicIfNeeded() {
        guard interactive, !cursorHidden else { return }
        cursorHidden = true
        stderr.write("\u{001B}[?25l")
        TerminalSignalRestorer.install()
    }

    private func dynamicWrite(_ text: String) {
        guard interactive else { return }
        clearDynamicLine()
        stderr.write("\r\u{001B}[2K" + text.replacingOccurrences(of: "\n", with: "\u{001B}[K\n\u{001B}[2K"))
        dynamicLineCount = text.split(separator: "\n", omittingEmptySubsequences: false).count
    }

    private func printAboveProgress(_ text: String) {
        if interactive {
            clearDynamicLine()
            stderr.write(text + "\n")
            if batch != nil { redrawBatch() }
            else if let currentPhase { showSinglePhase(currentPhase) }
        } else {
            stderr.write(text + "\n")
        }
    }

    private func clearDynamicLine() {
        guard interactive, dynamicLineCount > 0 else { return }
        for index in 0..<dynamicLineCount {
            stderr.write("\r\u{001B}[2K")
            if index + 1 < dynamicLineCount {
                stderr.write("\u{001B}[1A")
            }
        }
        dynamicLineCount = 0
    }

    private func restoreCursor() {
        guard cursorHidden else { return }
        cursorHidden = false
        stderr.write("\u{001B}[?25h")
        TerminalSignalRestorer.uninstall()
    }

    private func emitStructured(_ event: [String: Any]) {
        guard options.format != .text else { return }
        var record = event
        record["schema_version"] = 1
        if options.format == .jsonl {
            writeJSON(record, newline: true)
        } else {
            structuredEvents.append(record)
        }
    }

    private func writeJSON(_ object: Any, newline: Bool) {
        guard JSONSerialization.isValidJSONObject(object),
              let data = try? JSONSerialization.data(
                  withJSONObject: object,
                  options: [.sortedKeys, .withoutEscapingSlashes]
              ),
              var text = String(data: data, encoding: .utf8) else { return }
        if newline { text += "\n" }
        stdout.write(text)
    }
}

private enum TerminalSignalRestorer {
    static func install() {
        signal(SIGINT) { signalNumber in
            let sequence = "\u{001B}[?25h\n"
            sequence.withCString { pointer in
                _ = Darwin.write(STDERR_FILENO, pointer, strlen(pointer))
            }
            signal(signalNumber, SIG_DFL)
            raise(signalNumber)
        }
    }

    static func uninstall() {
        signal(SIGINT, SIG_DFL)
    }
}

private extension NSLocking {
    func withLock<T>(_ body: () throws -> T) rethrows -> T {
        lock()
        defer { unlock() }
        return try body()
    }
}
