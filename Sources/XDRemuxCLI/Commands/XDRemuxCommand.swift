import Darwin
import Foundation
import XDRemuxAppleFeatures
import XDRemuxCore

public enum XDRemuxCommand {
    private static let fileManager = FileManager.default

    public static func main(mode: CLIProductMode) {
        let status = run(
            arguments: Array(CommandLine.arguments.dropFirst()),
            mode: mode
        )
        exit(status)
    }

    @discardableResult
    static func run(
        arguments: [String],
        mode: CLIProductMode,
        environment: [String: String] = ProcessInfo.processInfo.environment,
        preferredLanguages: [String] = Locale.preferredLanguages,
        isTTY: Bool? = nil,
        stdout: OutputWriter = .file(.standardOutput),
        stderr: OutputWriter = .file(.standardError)
    ) -> Int32 {
        let bootstrap = OutputOptions.bootstrap(from: arguments)
        let localizer = Localizer(
            requested: bootstrap.language,
            environment: environment,
            preferredLanguages: preferredLanguages
        )
        let reporter = CLIReporter(
            options: bootstrap,
            localizer: localizer,
            isTTY: isTTY,
            environment: environment,
            stdout: stdout,
            stderr: stderr
        )
        defer { reporter.finish() }

        guard let command = arguments.first else {
            reporter.writeHelp(help(mode: mode, localizer: localizer))
            return 2
        }
        if ["-h", "--help", "help"].contains(command) || arguments.contains("--help") {
            reporter.writeHelp(help(mode: mode, localizer: localizer))
            return 0
        }

        do {
            switch command {
            case "convert":
                let parsed = try ConversionArgumentParser.parseConvert(
                    Array(arguments.dropFirst()),
                    mode: mode
                )
                try runConvert(parsed, reporter: reporter)
                return 0
            case "batch":
                let parsed = try ConversionArgumentParser.parseBatch(
                    Array(arguments.dropFirst()),
                    mode: mode
                )
                return try runBatch(parsed, reporter: reporter) ? 5 : 0
            case "validate-apple" where mode == .developer:
                try runAppleValidation(Array(arguments.dropFirst()), stdout: stdout)
                return 0
            case "validate-portrait" where mode == .developer:
                try runPortraitValidation(Array(arguments.dropFirst()), stdout: stdout)
                return 0
            case "portrait-self-test" where mode == .developer:
                guard arguments.count == 1 else {
                    throw CLIError.invalidValue(option: command, value: "unexpected arguments")
                }
                try writeJSON(
                    AppleFeatureConversionEngine.portraitSelfTestReport(),
                    to: nil,
                    stdout: stdout
                )
                return 0
            default:
                throw CLIError.invalidCommand(command)
            }
        } catch {
            let failure = ConversionFailure.classify(error)
            reporter.reportFailure(failure, input: inputURL(in: arguments))
            return exitCode(for: failure)
        }
    }

    private static func runConvert(_ command: ConvertCommand, reporter: CLIReporter) throws {
        try validateInputType(command.inputURL, appleFeatures: command.conversion.appleFeatures)
        let displayMode = ConversionDisplayMode(options: command.conversion)
        reporter.beginSingle(
            input: command.inputURL,
            output: command.outputURL,
            mode: displayMode,
            phases: CLIReporter.plannedPhases(for: command.conversion)
        )
        if command.outputWasExplicit,
           !command.conversion.overwrite,
           command.outputURL.standardizedFileURL != command.inputURL.standardizedFileURL,
           fileManager.fileExists(atPath: command.outputURL.path) {
            throw ConversionFailure(
                code: .outputNotWritable,
                userSummaryKey: .errorOutputNotWritable,
                recoverySuggestionKey: .recoveryCheckOutput,
                diagnostics: "output already exists: \(command.outputURL.path)"
            )
        }

        reporter.diagnostic(configurationDescription(command.conversion), input: command.inputURL)
        let eventHandler: ConversionEventHandler = { event in
            switch event {
            case .started, .completed, .failed: break
            default:
                reporter.handleSingle(event, input: command.inputURL, output: command.outputURL)
            }
        }
        let item = BatchWorkItem(inputURL: command.inputURL, outputURL: command.outputURL)
        try BatchCoordinator.convertAtomically(item: item) { input, output in
            var configuration = command.conversion.configuration(eventHandler: eventHandler)
            configuration.outputDirectory = output.deletingLastPathComponent()
            try convert(input: input, output: output, configuration: configuration)
        }
        let result = ConversionResult(
            input: InputSource(url: command.inputURL),
            output: OutputDestination(url: command.outputURL)
        )
        reporter.handleSingle(.completed(result), input: command.inputURL, output: command.outputURL)
    }

    private static func runBatch(_ command: BatchCommand, reporter: CLIReporter) throws -> Bool {
        try ensureDirectory(command.outputDirURL, fileManager: fileManager)
        let inputRootPath = command.inputDirURL.standardizedFileURL.path
        let outputRootPath = command.outputDirURL.standardizedFileURL.path
        let excludedOutputRoot = outputRootPath.hasPrefix(inputRootPath + "/")
            ? command.outputDirURL
            : nil
        let inputs = try enumerateInputs(
            root: command.inputDirURL,
            glob: command.glob,
            excluding: excludedOutputRoot
        )
        guard !inputs.isEmpty else {
            throw CLIError.noFilesMatched(command.inputDirURL, command.glob)
        }
        let items = inputs.map {
            BatchWorkItem(
                inputURL: $0,
                outputURL: BatchCoordinator.outputURL(
                    input: $0,
                    inputRoot: command.inputDirURL,
                    outputRoot: command.outputDirURL
                )
            )
        }
        try assertNoOutputCollisions(items)
        let displayMode = ConversionDisplayMode(options: command.conversion)
        reporter.beginBatch(total: items.count, jobs: command.jobs, mode: displayMode)
        reporter.diagnostic(configurationDescription(command.conversion), input: nil)

        let result = BatchCoordinator.run(
            items: items,
            jobs: command.jobs,
            overwrite: command.conversion.overwrite,
            diagnosticsAvailable: command.output.verbosity == .debug
                || command.conversion.diagnosticsDirectoryURL != nil,
            reporter: reporter,
            isValid: { item in
                isValidOutput(item.outputURL, input: item.inputURL, command: command)
            },
            convert: { item in
                try BatchCoordinator.convertAtomically(
                    item: item,
                    validateTemporary: { temporary in
                        isValidOutput(temporary, input: item.inputURL, command: command)
                    }
                ) { input, output in
                    try validateInputType(input, appleFeatures: command.conversion.appleFeatures)
                    let eventHandler: ConversionEventHandler = { event in
                        reporter.handleBatchEvent(event, input: item.inputURL)
                    }
                    let configuration = command.conversionConfiguration(eventHandler: eventHandler)
                    try convert(input: input, output: output, configuration: configuration)
                }
            }
        )
        let failureReportURL = try BatchCoordinator.writeFailureReport(
            result.failures,
            outputDirectory: command.outputDirURL
        )
        reporter.completeBatch(failureReportURL: failureReportURL)
        return !result.failures.isEmpty
    }

    private static func convert(
        input: URL,
        output: URL,
        configuration: ConversionConfiguration
    ) throws {
        _ = try AppleFeatureConversionEngine.convert(
            ConversionRequest(
                input: InputSource(url: input),
                output: OutputDestination(url: output),
                configuration: configuration
            )
        )
    }

    private static func isValidOutput(_ output: URL, input: URL, command: BatchCommand) -> Bool {
        guard fileManager.fileExists(atPath: output.path) else { return false }
        let features = command.conversion.appleFeatures
        if features.photographicStyles {
            return AppleFeatureConversionEngine.isValidOutput(
                output,
                options: AppleFeatureOptions(
                    photographicStyles: true,
                    portrait: features.portrait
                        && AppleFeatureConversionEngine.isConvertiblePortraitInput(input)
                )
            )
        }
        if features.portrait {
            return AppleFeatureConversionEngine.isValidOutput(output, options: features)
        }
        return ConversionEngine.isValidOutput(
            output,
            config: command.conversionConfiguration(eventHandler: nil)
        )
    }

    private static func validateInputType(
        _ inputURL: URL,
        appleFeatures: AppleFeatureOptions
    ) throws {
        let pathExtension = inputURL.pathExtension.lowercased()
        guard pathExtension == "jpg" || pathExtension == "jpeg" else { return }
        guard appleFeatures.portrait else {
            throw CLIError.invalidValue(
                option: "--input",
                value: "JPEG input is supported only with --apple-portrait"
            )
        }
        guard AppleFeatureConversionEngine.hasValidISOGainMap(inputURL),
              AppleFeatureConversionEngine.isConvertiblePortraitInput(inputURL) else {
            throw CLIError.invalidContainer(
                "JPEG Apple Portrait input requires src.image + rear.depth + "
                    + "rear.depth.config and an ImageIO-readable ISO Gain Map"
            )
        }
    }

    private static func runAppleValidation(_ rawArguments: [String], stdout: OutputWriter) throws {
        var inputURL: URL?
        var reportURL: URL?
        var expectsPortrait = false
        var index = 0
        while index < rawArguments.count {
            let option = rawArguments[index]
            index += 1
            switch option {
            case "--input": inputURL = URL(fileURLWithPath: try nextValue(rawArguments, &index, option))
            case "--json": reportURL = URL(fileURLWithPath: try nextValue(rawArguments, &index, option))
            case "--expect-portrait": expectsPortrait = true
            case "--language", "--format": _ = try nextValue(rawArguments, &index, option)
            case "--quiet", "--verbose", "--debug": break
            default: throw CLIError.unknownOption(option)
            }
        }
        guard let inputURL else { throw CLIError.missingArgument("--input") }
        try writeJSON(
            AppleFeatureConversionEngine.validationReport(
                for: inputURL.standardizedFileURL,
                expectsPortrait: expectsPortrait
            ),
            to: reportURL,
            stdout: stdout
        )
    }

    private static func runPortraitValidation(_ rawArguments: [String], stdout: OutputWriter) throws {
        var inputURL: URL?
        var reportURL: URL?
        var index = 0
        while index < rawArguments.count {
            let option = rawArguments[index]
            index += 1
            switch option {
            case "--input": inputURL = URL(fileURLWithPath: try nextValue(rawArguments, &index, option))
            case "--json": reportURL = URL(fileURLWithPath: try nextValue(rawArguments, &index, option))
            case "--language", "--format": _ = try nextValue(rawArguments, &index, option)
            case "--quiet", "--verbose", "--debug": break
            default: throw CLIError.unknownOption(option)
            }
        }
        guard let inputURL else { throw CLIError.missingArgument("--input") }
        try writeJSON(
            AppleFeatureConversionEngine.portraitValidationReport(
                for: inputURL.standardizedFileURL
            ),
            to: reportURL,
            stdout: stdout
        )
    }

    private static func writeJSON(
        _ object: Any,
        to reportURL: URL?,
        stdout: OutputWriter
    ) throws {
        let data = try JSONSerialization.data(
            withJSONObject: object,
            options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        )
        if let reportURL {
            try ensureDirectory(reportURL.deletingLastPathComponent(), fileManager: fileManager)
            try data.write(to: reportURL, options: .atomic)
        }
        stdout.write(String(data: data, encoding: .utf8)! + "\n")
    }

    private static func nextValue(
        _ arguments: [String],
        _ index: inout Int,
        _ option: String
    ) throws -> String {
        guard index < arguments.count else { throw CLIError.missingArgument(option) }
        defer { index += 1 }
        return arguments[index]
    }

    static func enumerateInputs(root: URL, glob: String, excluding excludedRoot: URL?) throws -> [URL] {
        guard fileManager.fileExists(atPath: root.path) else {
            throw CLIError.inputNotFound(root)
        }
        let regex = try globToRegex(glob)
        guard let enumerator = fileManager.enumerator(
            at: root,
            includingPropertiesForKeys: [.isRegularFileKey],
            options: [.skipsHiddenFiles]
        ) else {
            throw CLIError.inputNotFound(root)
        }
        let excludedPath = excludedRoot?.standardizedFileURL.path
        var matched: [URL] = []
        for case let fileURL as URL in enumerator {
            let standardizedPath = fileURL.standardizedFileURL.path
            if let excludedPath,
               standardizedPath == excludedPath || standardizedPath.hasPrefix(excludedPath + "/") {
                if standardizedPath == excludedPath {
                    enumerator.skipDescendants()
                }
                continue
            }
            let values = try fileURL.resourceValues(forKeys: [.isRegularFileKey])
            guard values.isRegularFile == true else { continue }
            let relative = fileURL.path.replacingOccurrences(of: root.path + "/", with: "")
            let filename = fileURL.lastPathComponent
            if regex.firstMatch(
                in: relative,
                range: NSRange(relative.startIndex..., in: relative)
            ) != nil || regex.firstMatch(
                in: filename,
                range: NSRange(filename.startIndex..., in: filename)
            ) != nil {
                matched.append(fileURL)
            }
        }
        return matched.sorted { $0.path < $1.path }
    }

    private static func globToRegex(_ glob: String) throws -> NSRegularExpression {
        var pattern = "^"
        for scalar in glob.unicodeScalars {
            switch scalar {
            case "*": pattern += ".*"
            case "?": pattern += "."
            case ".", "(", ")", "[", "]", "{", "}", "+", "^", "$", "|", "\\":
                pattern += "\\\(scalar)"
            default: pattern.append(Character(scalar))
            }
        }
        return try NSRegularExpression(pattern: pattern + "$", options: [.caseInsensitive])
    }

    private static func assertNoOutputCollisions(_ items: [BatchWorkItem]) throws {
        var seen: [String: URL] = [:]
        for item in items {
            let key = item.outputURL.standardizedFileURL.path
            if let prior = seen[key] {
                throw CLIError.outputPathCollision(
                    output: item.outputURL,
                    firstInput: prior,
                    secondInput: item.inputURL
                )
            }
            seen[key] = item.inputURL
        }
    }

    private static func inputURL(in arguments: [String]) -> URL? {
        for (index, argument) in arguments.enumerated() where argument == "--input" {
            guard index + 1 < arguments.count else { return nil }
            return URL(fileURLWithPath: arguments[index + 1])
        }
        return nil
    }

    private static func configurationDescription(_ options: ConversionOptions) -> String {
        [
            "family=\(options.family.rawValue)",
            "input_processing=\(options.inputProcessingBranch.rawValue)",
            "oppo_compatibility=\(options.oppoCompatibility.rawValue)",
            "oppo_camera_tail=\(options.oppoCameraTail.rawValue)",
            "tmap_format=\(options.tmapFormat.rawValue)",
            "apple_features=\(options.appleFeatures.stableDescription)",
            "diagnostics_dir=\(options.diagnosticsDirectoryURL?.path ?? "none")",
        ].joined(separator: " ")
    }

    private static func exitCode(for failure: ConversionFailure) -> Int32 {
        switch failure.code {
        case .invalidArguments: return 2
        case .sourceNotFound, .sourceNotSupported, .sourceGainMapMissing,
             .sourceGainMapCorrupt, .portraitDataUnavailable:
            return 3
        case .outputNotWritable, .outputVerificationFailed, .appleRuntimeUnavailable:
            return 4
        case .batchIncomplete: return 5
        case .internalContainerError: return 1
        }
    }

    private static func help(mode: CLIProductMode, localizer: Localizer) -> String {
        var sections = [
            localizer.text(.helpTitle),
            "",
            "\(localizer.text(.helpUsage)):",
            "  \(mode.executableName) convert --input <file> [--output <file>] [options]",
            "  \(mode.executableName) batch --input-dir <dir> [--output-dir <dir>] [options]",
            "",
            "\(localizer.text(.helpCommands)):",
            localizer.text(.helpConvert),
            localizer.text(.helpBatch),
            "",
            localizer.text(.helpOptions),
            "",
            localizer.text(.helpOutput),
            "",
            localizer.text(.helpLanguage),
        ]
        if mode == .developer {
            sections.append(contentsOf: ["", localizer.text(.helpDeveloper)])
        }
        return sections.joined(separator: "\n")
    }
}
