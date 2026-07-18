import Foundation
import XCTest
import XDRemuxCore
@testable import XDRemuxCLI

final class ConversionArgumentParserTests: XCTestCase {
    func testSingleFileDefaultsPreserveProductConfiguration() throws {
        let command = try ConversionArgumentParser.parseConvert([
            "--input", "/tmp/input.heic",
        ])

        XCTAssertEqual(command.inputURL.path, "/tmp/input.heic")
        XCTAssertEqual(command.outputURL, command.inputURL)
        XCTAssertFalse(command.outputWasExplicit)
        XCTAssertEqual(command.conversion.family, .auto)
        XCTAssertEqual(command.conversion.inputProcessingBranch, .hybrid)
        XCTAssertEqual(command.conversion.oppoCompatibility, .off)
        XCTAssertEqual(command.conversion.oppoCameraTail, .preserveWithoutPrivateHDR)
        XCTAssertEqual(command.conversion.tmapFormat, .imageIO)
        XCTAssertEqual(command.conversion.appleFeatures, .disabled)
        XCTAssertFalse(command.conversion.overwrite)
    }

    func testBatchDefaultsNoLongerExposeCheckpointState() throws {
        let command = try ConversionArgumentParser.parseBatch([
            "--input-dir", "/tmp/input",
        ])

        XCTAssertEqual(command.outputDirURL, command.inputDirURL)
        XCTAssertEqual(command.glob, "*.heic")
        XCTAssertEqual(command.jobs, min(ProcessInfo.processInfo.activeProcessorCount, 4))
        XCTAssertFalse(command.conversion.overwrite)
        XCTAssertThrowsError(try ConversionArgumentParser.parseBatch([
            "--input-dir", "/tmp/input", "--checkpoint", "/tmp/checkpoint.jsonl",
        ]))
        XCTAssertThrowsError(try ConversionArgumentParser.parseBatch([
            "--input-dir", "/tmp/input", "--resume",
        ]))
    }

    func testStandardAndOppoModesKeepTheirTailPolicies() throws {
        let standard = try ConversionArgumentParser.parseConvert([
            "--input", "/tmp/input.heic",
        ])
        let oppo = try ConversionArgumentParser.parseConvert([
            "--input", "/tmp/input.heic",
            "--oppo-compatible",
        ])

        XCTAssertEqual(standard.conversion.oppoCompatibility, .off)
        XCTAssertEqual(standard.conversion.oppoCameraTail, .preserveWithoutPrivateHDR)
        XCTAssertEqual(oppo.conversion.oppoCompatibility, .auto)
        XCTAssertEqual(oppo.conversion.oppoCameraTail, .preserve)
    }

    func testAppleModesAreIndependentAndComposable() throws {
        let styles = try parseApple(["--apple-photographic-styles"])
        let portrait = try parseApple(["--apple-portrait"])
        let combined = try parseApple(["--apple-photographic-styles", "--apple-portrait"])

        XCTAssertEqual(
            styles.conversion.appleFeatures,
            AppleFeatureOptions(photographicStyles: true)
        )
        XCTAssertEqual(styles.conversion.oppoCameraTail, .preserveWithoutPrivateHDR)
        XCTAssertEqual(
            portrait.conversion.appleFeatures,
            AppleFeatureOptions(portrait: true)
        )
        XCTAssertEqual(
            portrait.conversion.oppoCameraTail,
            .preserveWithoutPortraitOrPrivateHDR
        )
        XCTAssertEqual(
            combined.conversion.appleFeatures,
            AppleFeatureOptions(photographicStyles: true, portrait: true)
        )
        XCTAssertEqual(
            combined.conversion.oppoCameraTail,
            .preserveWithoutPortraitOrPrivateHDR
        )
    }

    func testProductionRejectsInternalOptionsAndDeveloperAcceptsThem() throws {
        let internalOptions = [
            "--family", "x7",
            "--input-processing", "passthrough",
            "--oppo-compat", "auto",
            "--oppo-camera-tail", "watermark",
            "--tmap-format", "strict",
            "--diagnostics-dir", "/tmp/debug",
        ]
        let internalOptionCases = [
            ["--family", "x7"],
            ["--input-processing", "passthrough"],
            ["--oppo-compat", "auto"],
            ["--oppo-camera-tail", "watermark"],
            ["--tmap-format", "strict"],
            ["--diagnostics-dir", "/tmp/debug"],
        ]
        for arguments in internalOptionCases {
            XCTAssertThrowsError(try ConversionArgumentParser.parseConvert(
                ["--input", "/tmp/input.heic"] + arguments,
                mode: .production
            )) { error in
                XCTAssertEqual(
                    String(describing: error),
                    "unknown option: \(arguments[0])"
                )
            }
        }

        let convert = try ConversionArgumentParser.parseConvert(
            ["--input", "/tmp/input.heic"] + internalOptions,
            mode: .developer
        )
        let batch = try ConversionArgumentParser.parseBatch(
            ["--input-dir", "/tmp/input"] + internalOptions,
            mode: .developer
        )

        XCTAssertEqual(convert.conversion.family, batch.conversion.family)
        XCTAssertEqual(
            convert.conversion.inputProcessingBranch,
            batch.conversion.inputProcessingBranch
        )
        XCTAssertEqual(convert.conversion.oppoCameraTail, batch.conversion.oppoCameraTail)
        XCTAssertEqual(convert.conversion.tmapFormat, batch.conversion.tmapFormat)
        XCTAssertEqual(
            convert.conversion.diagnosticsDirectoryURL,
            batch.conversion.diagnosticsDirectoryURL
        )
    }

    func testAppleAndOppoConflictFailsDuringParsing() {
        XCTAssertThrowsError(
            try ConversionArgumentParser.parseConvert([
                "--input", "/tmp/input.heic",
                "--apple-portrait",
                "--oppo-compatible",
            ])
        ) { error in
            XCTAssertEqual(
                String(describing: error),
                "invalid value for --apple-portrait: cannot be combined with --oppo-compatible"
            )
        }
    }

    func testOutputModesAndLanguageAreParsedOnceForBothCommands() throws {
        let options = ["--verbose", "--format", "jsonl", "--language", "zh-Hans"]
        let convert = try ConversionArgumentParser.parseConvert(
            ["--input", "/tmp/input.heic"] + options
        )
        let batch = try ConversionArgumentParser.parseBatch(
            ["--input-dir", "/tmp/input"] + options
        )

        XCTAssertEqual(convert.output.verbosity, .verbose)
        XCTAssertEqual(convert.output.format, .jsonl)
        XCTAssertEqual(convert.output.language, .simplifiedChinese)
        XCTAssertEqual(convert.output.verbosity, batch.output.verbosity)
        XCTAssertEqual(convert.output.format, batch.output.format)
        XCTAssertEqual(convert.output.language, batch.output.language)
    }

    func testMutuallyExclusiveVerbosityAndInvalidArgumentsKeepErrorTypes() {
        XCTAssertThrowsError(try ConversionArgumentParser.parseConvert([])) { error in
            XCTAssertEqual(String(describing: error), "missing required argument: --input")
        }
        XCTAssertThrowsError(
            try ConversionArgumentParser.parseConvert(["--input", "a.heic", "--wat"])
        ) { error in
            XCTAssertEqual(String(describing: error), "unknown option: --wat")
        }
        XCTAssertThrowsError(
            try ConversionArgumentParser.parseBatch(["--input-dir", "/tmp", "--jobs", "0"])
        ) { error in
            XCTAssertEqual(String(describing: error), "invalid value for --jobs: 0")
        }
        XCTAssertThrowsError(
            try ConversionArgumentParser.parseConvert([
                "--input", "a.heic", "--quiet", "--verbose",
            ])
        )
    }

    private func parseApple(_ options: [String]) throws -> ConvertCommand {
        try ConversionArgumentParser.parseConvert([
            "--input", "/tmp/input.heic",
        ] + options)
    }
}
