import Foundation
import XCTest
import XDRemuxCore
@testable import XDRemuxCLI

final class ConversionArgumentParserTests: XCTestCase {
    func testSingleFileDefaultsPreserveLegacyConfiguration() throws {
        let command = try ConversionArgumentParser.parseConvert([
            "--input", "/tmp/input.heic",
        ])

        XCTAssertEqual(command.inputURL.path, "/tmp/input.heic")
        XCTAssertEqual(command.outputURL, command.inputURL)
        XCTAssertEqual(command.family, .auto)
        XCTAssertEqual(command.inputProcessingBranch, .hybrid)
        XCTAssertEqual(command.oppoCompatibility, .off)
        XCTAssertEqual(command.oppoCameraTail, .preserveWithoutPrivateHDR)
        XCTAssertEqual(command.tmapFormat, .imageIO)
        XCTAssertEqual(command.appleFeatures, .disabled)
        XCTAssertEqual(command.appleStyleDataProducer, .unspecified)
    }

    func testBatchDefaultsPreserveCheckpointContract() throws {
        let command = try ConversionArgumentParser.parseBatch([
            "--input-dir", "/tmp/input",
        ])

        XCTAssertEqual(command.outputDirURL, command.inputDirURL)
        XCTAssertEqual(command.glob, "*.heic")
        XCTAssertEqual(command.jobs, min(ProcessInfo.processInfo.activeProcessorCount, 4))
        XCTAssertNil(command.checkpointURL)
        XCTAssertTrue(command.resume)
        XCTAssertTrue(command.skipExisting)
        XCTAssertFalse(command.categorizeOutput)
    }

    func testCategorizeCommandAndBatchSwitch() throws {
        let categorize = try ConversionArgumentParser.parseCategorize([
            "--input", "/tmp/a.heic",
            "--input", "/tmp/photos",
            "--output-dir", "/tmp/output",
            "--jobs", "2",
            "--dry-run",
        ])
        XCTAssertEqual(categorize.inputURLs.map(\.path), ["/tmp/a.heic", "/tmp/photos"])
        XCTAssertEqual(categorize.outputDirURL.path, "/tmp/output")
        XCTAssertEqual(categorize.jobs, 2)
        XCTAssertTrue(categorize.dryRun)

        let batch = try ConversionArgumentParser.parseBatch([
            "--input-dir", "/tmp/input",
            "--categorize",
        ])
        XCTAssertTrue(batch.categorizeOutput)
        XCTAssertThrowsError(try ConversionArgumentParser.parseConvert([
            "--input", "/tmp/input.heic",
            "--categorize",
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

        XCTAssertEqual(standard.oppoCompatibility, .off)
        XCTAssertEqual(standard.oppoCameraTail, .preserveWithoutPrivateHDR)
        XCTAssertEqual(oppo.oppoCompatibility, .auto)
        XCTAssertEqual(oppo.oppoCameraTail, .preserve)
    }

    func testAppleModesAreIndependentAndComposable() throws {
        let styles = try parseApple(["--apple-photographic-styles"])
        let portrait = try parseApple(["--apple-portrait"])
        let combined = try parseApple(["--apple-photographic-styles", "--apple-portrait"])

        XCTAssertEqual(styles.appleFeatures, AppleFeatureOptions(photographicStyles: true))
        XCTAssertEqual(styles.appleStyleDataProducer, .constrainedSolver)
        XCTAssertEqual(styles.oppoCameraTail, .preserveWithoutPrivateHDR)
        XCTAssertEqual(portrait.appleFeatures, AppleFeatureOptions(portrait: true))
        XCTAssertEqual(portrait.oppoCameraTail, .preserveWithoutPortraitOrPrivateHDR)
        XCTAssertEqual(
            combined.appleFeatures,
            AppleFeatureOptions(photographicStyles: true, portrait: true)
        )
        XCTAssertEqual(combined.oppoCameraTail, .preserveWithoutPortraitOrPrivateHDR)
    }

    func testPhotographicStylesDefaultToConstrainedSolverAndFallbacksRemainExplicit() throws {
        let defaulted = try parseApple(["--apple-photographic-styles"])
        XCTAssertEqual(defaulted.appleStyleDataProducer, .constrainedSolver)
        XCTAssertEqual(
            defaulted.configuration.appleStyleDataProducer,
            .constrainedSolver
        )

        let batchDefaulted = try ConversionArgumentParser.parseBatch([
            "--input-dir", "/tmp/input",
            "--apple-photographic-styles",
        ])
        XCTAssertEqual(batchDefaulted.appleStyleDataProducer, .constrainedSolver)
        XCTAssertEqual(
            batchDefaulted.conversionConfiguration.appleStyleDataProducer,
            .constrainedSolver
        )

        let diagnostic = try parseApple([
            "--apple-photographic-styles",
            "--apple-style-data-producer", "learn-node",
        ])
        XCTAssertEqual(diagnostic.appleStyleDataProducer, .learnNodeDiagnostic)

        let identity = try parseApple([
            "--apple-photographic-styles",
            "--apple-style-data-producer", "identity-fallback",
        ])
        XCTAssertEqual(identity.appleStyleDataProducer, .identityFallback)

        let batchIdentity = try ConversionArgumentParser.parseBatch([
            "--input-dir", "/tmp/input",
            "--apple-photographic-styles",
            "--apple-style-data-producer", "identity-fallback",
        ])
        XCTAssertEqual(batchIdentity.appleStyleDataProducer, .identityFallback)

        XCTAssertThrowsError(
            try ConversionArgumentParser.parseConvert([
                "--input", "/tmp/input.heic",
                "--apple-style-data-producer", "identity-fallback",
            ])
        )
    }

    func testRawDNGOptionIsOptionalAndOnlyAppliesToPhotographicStyles() throws {
        let command = try ConversionArgumentParser.parseConvert([
            "--input", "/tmp/input.heic",
            "--apple-photographic-styles",
            "--apple-styles-raw-dng", "/tmp/IMG.dng",
        ])
        XCTAssertEqual(command.appleStylesRawDNGURL?.path, "/tmp/IMG.dng")
        XCTAssertEqual(command.configuration.appleStylesRawDNGURL?.path, "/tmp/IMG.dng")

        XCTAssertThrowsError(try ConversionArgumentParser.parseConvert([
            "--input", "/tmp/input.heic",
            "--apple-styles-raw-dng", "/tmp/IMG.dng",
        ])) { error in
            XCTAssertEqual(
                String(describing: error),
                "invalid value for --apple-styles-raw-dng: requires --apple-photographic-styles"
            )
        }
    }

    func testConvertAndBatchShareCommonOptionSemantics() throws {
        let common = [
            "--family", "x7",
            "--input-processing", "passthrough",
            "--oppo-camera-tail", "watermark",
            "--tmap-format", "strict",
            "--debug-dir", "/tmp/debug",
        ]
        let convert = try ConversionArgumentParser.parseConvert(
            ["--input", "/tmp/input.heic"] + common
        )
        let batch = try ConversionArgumentParser.parseBatch(
            ["--input-dir", "/tmp/input"] + common
        )

        XCTAssertEqual(convert.family, batch.family)
        XCTAssertEqual(convert.inputProcessingBranch, batch.inputProcessingBranch)
        XCTAssertEqual(convert.oppoCameraTail, batch.oppoCameraTail)
        XCTAssertEqual(convert.tmapFormat, batch.tmapFormat)
        XCTAssertEqual(convert.debugRootURL, batch.debugRootURL)
    }

    func testAppleAndOppoConflictPreservesLegacyError() {
        XCTAssertThrowsError(
            try ConversionArgumentParser.parseConvert([
                "--input", "/tmp/input.heic",
                "--apple-portrait",
                "--oppo-compatible",
            ])
        ) { error in
            XCTAssertEqual(
                String(describing: error),
                "invalid value for --apple-portrait: cannot be combined with OPPO-compatible output"
            )
        }
    }

    func testMissingUnknownAndInvalidArgumentsKeepErrorTypes() {
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
    }

    private func parseApple(_ options: [String]) throws -> ConvertCommand {
        try ConversionArgumentParser.parseConvert([
            "--input", "/tmp/input.heic",
        ] + options)
    }
}
