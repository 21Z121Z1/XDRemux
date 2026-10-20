import Foundation

enum TestFailure: Error, CustomStringConvertible {
    case assertion(String)

    var description: String {
        switch self {
        case .assertion(let message):
            return message
        }
    }
}

@MainActor
func expect(_ condition: @autoclosure () throws -> Bool, _ message: String) throws {
    guard try condition() else {
        throw TestFailure.assertion(message)
    }
}

@MainActor
func makeTempDirectory(_ name: String) throws -> URL {
    let root = FileManager.default.temporaryDirectory
        .appendingPathComponent("xdremuxapp-model-tests-\(UUID().uuidString)", isDirectory: true)
        .appendingPathComponent(name, isDirectory: true)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    return root
}

@MainActor
func writeFile(_ url: URL, bytes: Int = 16) throws {
    try Data(repeating: 0x41, count: bytes).write(to: url)
}

@main
struct XDRemuxAppModelTests {
    @MainActor
    static func main() async throws {
        try await testImportDiscoversHEICFilesAndDeduplicates()
        try await testConversionCategorizationUsesAssetTypeAndModeDirectories()
        try await testCategorizationPreviewCopyAndDuplicateRerun()
        try await testCategorizationDefaultsToEachSourceDirectory()
        try await testCategorizationCancellationKeepsCancelledState()
        try await testImportedRowsStartWithEmptyThumbnailState()
        try await testOutputCollisionsAreMarkedBeforeConversion()
        try await testOutputPlanFlagsInvalidExistingOutputAsOverwriteRisk()
        try await testOutputParentFileBlocksConversionBeforeWorkStarts()
        try testPreparingOutputKeepsInvalidExistingFileUntilConversionSucceeds()
        try testThumbnailRendererProducesBoundedPNGData()
        try testEffectiveConcurrencyRespectsMemoryAndUserLimit()
        try testQueueStatusCountsMatchAllTerminalStates()
        try testClearCompletedAndRetryFailedKeepQueuePredictable()
        try testThumbnailResultOnlyAppliesToExistingMatchingQueueItem()
        try testUserFacingCopyUsesClearConversionTerms()
        try testProductPolicyDefaults()
        try testSimplifiedProductSwitches()
        try testIndependentAppleFeatureConfiguration()
        try await testCLIClientDrainsBothPipesWhileRunning()
        try await testSkipRequiresValidationForTheRequestedFeature()
        try testOutputCannotOverwriteQueuedInputsOrAliases()
        try await testCategorizedConversionCreatesDirectories()
        try await testExistingOutputReplacementPreservesFailureAndSuccess()
        try await testExistingMotionPhotoOutputIsNeverReplacedAsOneFile()
        print("XDRemuxAppModelTests passed")
    }

    @MainActor
    private static func waitForCategorization(
        _ viewModel: PhotoCategorizationViewModel,
        timeout: Duration = .seconds(5)
    ) async throws {
        let clock = ContinuousClock()
        let deadline = clock.now.advanced(by: timeout)
        while viewModel.isBusy, clock.now < deadline {
            try await Task.sleep(for: .milliseconds(10))
        }
        try expect(!viewModel.isBusy, "categorization operation timed out")
    }

    @MainActor
    private static func withCLI(
        _ script: String,
        body: @MainActor (URL) async throws -> Void
    ) async throws {
        let root = try makeTempDirectory("cli-client")
        let executable = root.appendingPathComponent("xdremux-test-cli")
        try Data(("#!/bin/sh\nset -eu\n" + script + "\n").utf8).write(to: executable)
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: executable.path)
        let previous = ProcessInfo.processInfo.environment["XDREMUX_CLI"]
        setenv("XDREMUX_CLI", executable.path, 1)
        defer {
            if let previous { setenv("XDREMUX_CLI", previous, 1) }
            else { unsetenv("XDREMUX_CLI") }
            try? FileManager.default.removeItem(at: root.deletingLastPathComponent())
        }
        try await body(root)
    }

    private static let conversionCLI = #"""
    [ "$1" = convert ] || exit 64
    shift
    input=''
    output=''
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --input) input="$2"; shift 2 ;;
            --output) output="$2"; shift 2 ;;
            *) shift ;;
        esac
    done
    [ -d "$(dirname "$output")" ] || exit 65
    printf 'new-output' > "$output"
    case "$input" in
        *motion*) printf 'new-movie' > "${output%.*}.mov" ;;
    esac
    """#

    @MainActor
    private static func runConversion(_ model: XDRemuxViewModel) async throws {
        model.startConversion()
        let clock = ContinuousClock()
        let deadline = clock.now.advanced(by: .seconds(10))
        while clock.now < deadline,
              model.isBusy || model.queue.contains(where: { !$0.status.isTerminal }) {
            try await Task.sleep(for: .milliseconds(10))
        }
        try expect(!model.isBusy && model.queue.allSatisfy { $0.status.isTerminal }, "conversion operation timed out")
    }

    @MainActor
    private static func testCLIClientDrainsBothPipesWhileRunning() async throws {
        try await withCLI(#"""
        helper_pid=$$
        (
            sleep 5 &
            timer=$!
            trap 'kill "$timer" 2>/dev/null || true; wait "$timer" 2>/dev/null || true; exit' TERM
            wait "$timer"
            kill -TERM "$helper_pid" 2>/dev/null || true
        ) </dev/null >/dev/null 2>&1 &
        watchdog=$!
        trap 'kill "$watchdog" 2>/dev/null || true; wait "$watchdog" 2>/dev/null || true' EXIT
        error_chunk=e
        output_chunk=' '
        count=0
        while [ "$count" -lt 10 ]; do
            error_chunk="$error_chunk$error_chunk"
            output_chunk="$output_chunk$output_chunk"
            count=$((count + 1))
        done
        count=0
        while [ "$count" -lt 256 ]; do
            printf '%s' "$error_chunk" >&2
            count=$((count + 1))
        done
        count=0
        while [ "$count" -lt 256 ]; do
            printf '%s' "$output_chunk"
            count=$((count + 1))
        done
        printf '{"items": []}\n'
        """#) { root in
            let input = root.appendingPathComponent("input.heic")
            try writeFile(input)
            let result = try RustCLIClient.classify(inputs: [input])
            try expect(result.isEmpty, "CLI output larger than either pipe must be drained and decoded without deadlock")
        }
    }

    @MainActor
    private static func testSkipRequiresValidationForTheRequestedFeature() async throws {
        try await withCLI(#"""
        printf '%s\n' "$*" >> "$0.log"
        """#) { root in
            let output = root.appendingPathComponent("output.heic")
            try writeFile(output)
            try expect(RustCLIClient.isValidOutput(output), "standard output should use Rust validation")
            var config = ConversionConfig()
            config.applePhotographicStyles = true
            try expect(RustCLIClient.isValidOutput(output, config: config), "Styles should use its supported feature validation")
            config.applePhotographicStyles = false
            config.applePortrait = true
            try expect(!RustCLIClient.isValidOutput(output, config: config), "generic HDR validation cannot justify skipping Portrait")
            config.oppoGalleryCompatibilityEnabled = true
            try expect(!RustCLIClient.isValidOutput(output, config: config), "generic HDR validation cannot justify skipping OPPO compatibility")
            let calls = try String(contentsOf: root.appendingPathComponent("xdremux-test-cli.log"), encoding: .utf8)
                .split(separator: "\n")
            try expect(calls.count == 2 && calls[1].hasSuffix("--apple-styles"), "only supported validation requests should reach the CLI")
        }
    }

    @MainActor
    private static func testOutputCannotOverwriteQueuedInputsOrAliases() throws {
        let root = try makeTempDirectory("protect-inputs")
        defer { try? FileManager.default.removeItem(at: root.deletingLastPathComponent()) }
        let input = root.appendingPathComponent("a.heic")
        let otherInput = root.appendingPathComponent("a_iso.heic")
        try writeFile(input)
        try writeFile(otherInput, bytes: 32)
        let model = XDRemuxViewModel()
        model.config.skipExisting = false
        model.queue = [
            ConversionQueueItem(inputURL: input, outputURL: otherInput),
            ConversionQueueItem(inputURL: otherInput, outputURL: root.appendingPathComponent("a_iso_iso.heic")),
        ]
        _ = model.markOutputCollisionsForTesting()
        try expect(model.queue[0].status == .failed && model.queue[0].outputPlanStatus == .overwritesInput, "suffix naming must not replace another imported original")

        for symbolic in [true, false] {
            let output = root.appendingPathComponent(symbolic ? "symlink-output" : "hardlink-output", isDirectory: true)
            try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
            let alias = output.appendingPathComponent("a.heic")
            if symbolic {
                try FileManager.default.createSymbolicLink(at: alias, withDestinationURL: otherInput)
            } else {
                try FileManager.default.linkItem(at: otherInput, to: alias)
            }
            let aliases = XDRemuxViewModel()
            aliases.config.skipExisting = false
            aliases.config.outputDirectory = output
            aliases.queue = [
                ConversionQueueItem(inputURL: input, outputURL: alias),
                ConversionQueueItem(inputURL: otherInput, outputURL: output.appendingPathComponent("a_iso.heic")),
            ]
            _ = aliases.markOutputCollisionsForTesting()
            try expect(aliases.queue[0].outputPlanStatus == .overwritesInput, "output aliases must protect the original file identity")
        }
        let inPlace = XDRemuxViewModel()
        inPlace.config.outputDirectory = root
        inPlace.config.skipExisting = false
        inPlace.queue = [ConversionQueueItem(inputURL: input, outputURL: input)]
        _ = inPlace.markOutputCollisionsForTesting()
        try expect(!inPlace.canStart && inPlace.queue[0].outputPlanStatus == .overwritesInput, "app output directories must preserve imported originals")
        try expect((try Data(contentsOf: otherInput)) == Data(repeating: 0x41, count: 32), "planning cannot alter original media")
    }

    @MainActor
    private static func testCategorizedConversionCreatesDirectories() async throws {
        try await withCLI(conversionCLI) { root in
            let input = root.appendingPathComponent("input.heic")
            try writeFile(input)
            let model = XDRemuxViewModel()
            model.config.skipExisting = false
            model.config.outputDirectory = root.appendingPathComponent("new-output")
            model.config.categorizeOutputByCaptureMode = true
            model.queue = [ConversionQueueItem(inputURL: input, outputURL: input, captureMode: .portrait)]
            try await runConversion(model)
            let expected = root.appendingPathComponent("new-output/静态照片/人像/input.heic")
            try expect(model.queue[0].status == .converted, "first categorized conversion must create its directories")
            try expect((try Data(contentsOf: expected)) == Data("new-output".utf8), "CLI should write the planned categorized output")
        }
    }

    @MainActor
    private static func testExistingOutputReplacementPreservesFailureAndSuccess() async throws {
        for fail in [false, true] {
            try await withCLI(conversionCLI + (fail ? "\nexit 7" : "")) { root in
                let input = root.appendingPathComponent("input.heic")
                let outputDirectory = root.appendingPathComponent("output")
                try FileManager.default.createDirectory(at: outputDirectory, withIntermediateDirectories: true)
                let output = outputDirectory.appendingPathComponent("input.heic")
                try Data("source".utf8).write(to: input)
                try Data("old-output".utf8).write(to: output)
                let model = XDRemuxViewModel()
                model.config.skipExisting = false
                model.config.outputDirectory = outputDirectory
                model.queue = [ConversionQueueItem(inputURL: input, outputURL: output)]
                try await runConversion(model)
                try expect(model.queue[0].status == (fail ? .failed : .converted), "replacement should reflect the CLI outcome")
                try expect((try Data(contentsOf: output)) == Data((fail ? "old-output" : "new-output").utf8), "failed conversion must retain the prior output; success should replace it")
                try expect((try Data(contentsOf: input)) == Data("source".utf8), "overwrite intent cannot change the source")
                let files = try FileManager.default.contentsOfDirectory(atPath: outputDirectory.path)
                try expect(files == ["input.heic"], "replacement staging files should always be cleaned")
            }
        }
    }

    @MainActor
    private static func testExistingMotionPhotoOutputIsNeverReplacedAsOneFile() async throws {
        try await withCLI(conversionCLI) { root in
            // Exercise both the Rust classification and the actual two-file
            // result when classification was unavailable during import.
            for assetType in [PhotoAssetType.livePhoto, .staticPhoto] {
                let input = root.appendingPathComponent("motion.heic")
                try Data("source".utf8).write(to: input)
                let outputDirectory = root.appendingPathComponent(assetType.rawValue)
                try FileManager.default.createDirectory(at: outputDirectory, withIntermediateDirectories: true)
                let output = outputDirectory.appendingPathComponent("motion.heic")
                let movie = output.deletingPathExtension().appendingPathExtension("mov")
                try Data("old-still".utf8).write(to: output)
                try Data("old-movie".utf8).write(to: movie)
                let model = XDRemuxViewModel()
                model.config.skipExisting = false
                model.config.outputDirectory = outputDirectory
                model.queue = [ConversionQueueItem(inputURL: input, outputURL: output, assetType: assetType)]
                try await runConversion(model)
                try expect(model.queue[0].status == .failed, "existing Motion Photo output needs pair publication owned by Rust")
                try expect((try Data(contentsOf: output)) == Data("old-still".utf8), "failed pair replacement must retain the still")
                try expect((try Data(contentsOf: movie)) == Data("old-movie".utf8), "failed pair replacement must retain the movie")
                let files = try FileManager.default.contentsOfDirectory(atPath: outputDirectory.path).sorted()
                try expect(files == ["motion.heic", "motion.mov"], "unexpected pair staging must clean both resources")
            }
        }
    }

    @MainActor
    private static func testConversionCategorizationUsesAssetTypeAndModeDirectories() async throws {
        let root = try makeTempDirectory("conversion-categorize")
        defer { try? FileManager.default.removeItem(at: root.deletingLastPathComponent()) }
        let output = root.appendingPathComponent("output", isDirectory: true)
        let portrait = root.appendingPathComponent("portrait.heic")
        let unknown = root.appendingPathComponent("unknown.heic")
        try Data("header-oplus_18-tail".utf8).write(to: portrait)
        try Data("header-oplus_17179869184-tail".utf8).write(to: unknown)

        let viewModel = XDRemuxViewModel()
        viewModel.config.outputDirectory = output
        viewModel.config.categorizeOutputByCaptureMode = true
        _ = await viewModel.importFiles(from: [portrait, unknown])

        let byName = Dictionary(uniqueKeysWithValues: viewModel.queue.map { ($0.inputURL.lastPathComponent, $0) })
        try expect(byName["portrait.heic"]?.captureMode == .portrait, "conversion queue should expose the parsed capture mode")
        try expect(byName["portrait.heic"]?.outputURL == output.appendingPathComponent("静态照片/人像/portrait.heic"), "categorized conversion should use the mode directory")
        try expect(byName["unknown.heic"]?.classificationStatus == .unknownFlags, "unknown flags should remain visible in the queue")
        try expect(byName["unknown.heic"]?.outputURL == output.appendingPathComponent("静态照片/未分类/unknown.heic"), "unclassified conversion should use the static-photo / unclassified projection")
    }

    @MainActor
    private static func testCategorizationPreviewCopyAndDuplicateRerun() async throws {
        let root = try makeTempDirectory("standalone-categorize")
        defer { try? FileManager.default.removeItem(at: root.deletingLastPathComponent()) }
        let input = root.appendingPathComponent("input", isDirectory: true)
        let output = input.appendingPathComponent("output", isDirectory: true)
        try FileManager.default.createDirectory(at: input, withIntermediateDirectories: true)
        try Data("header-oplus_18-tail".utf8).write(to: input.appendingPathComponent("portrait.heic"))
        try Data("no user comment".utf8).write(to: input.appendingPathComponent("plain.jpg"))

        let viewModel = PhotoCategorizationViewModel()
        viewModel.addInputs([input])
        viewModel.outputDirectory = output
        viewModel.scan()
        try await waitForCategorization(viewModel)

        try expect(viewModel.state == .ready, "scan should produce a ready preview")
        try expect(viewModel.items.count == 2, "preview should include both supported files")
        try expect(viewModel.categorizedCount == 1 && viewModel.rootCount == 1, "preview counts should separate categorized and unclassified items")
        try expect(viewModel.modeSummary.contains("人像 1"), "preview should summarize counts by capture mode")
        try expect(viewModel.items.contains { $0.destinationURL == output.appendingPathComponent("静态照片/人像/portrait.heic") }, "preview should show the mode destination")
        try expect(viewModel.items.contains { $0.destinationURL == output.appendingPathComponent("静态照片/未分类/plain.jpg") }, "preview should show the unclassified destination")

        viewModel.copyPlannedFiles()
        try await waitForCategorization(viewModel)
        try expect(viewModel.state == .completed, "copy should complete")
        try expect(FileManager.default.fileExists(atPath: output.appendingPathComponent("静态照片/人像/portrait.heic").path), "copy should create the categorized file")
        try expect(FileManager.default.fileExists(atPath: output.appendingPathComponent("静态照片/未分类/plain.jpg").path), "copy should create the unclassified file")

        viewModel.scan()
        try await waitForCategorization(viewModel)
        try expect(viewModel.duplicateCount == 2, "a repeated scan should plan both files as duplicates")
    }

    @MainActor
    private static func testCategorizationCancellationKeepsCancelledState() async throws {
        let root = try makeTempDirectory("categorize-cancel")
        defer { try? FileManager.default.removeItem(at: root.deletingLastPathComponent()) }
        let input = root.appendingPathComponent("source.heic")
        try Data("oplus_18".utf8).write(to: input)

        let viewModel = PhotoCategorizationViewModel()
        viewModel.addInputs([input])
        viewModel.outputDirectory = root.appendingPathComponent("output", isDirectory: true)
        viewModel.scan()
        viewModel.cancel()
        try await Task.sleep(for: .milliseconds(50))
        try expect(viewModel.state == .cancelled, "cancel should leave scanning in the cancelled state")
    }

    @MainActor
    private static func testCategorizationDefaultsToEachSourceDirectory() async throws {
        let root = try makeTempDirectory("categorize-source-roots")
        defer { try? FileManager.default.removeItem(at: root.deletingLastPathComponent()) }
        let firstDirectory = root.appendingPathComponent("first", isDirectory: true)
        let secondDirectory = root.appendingPathComponent("second", isDirectory: true)
        try FileManager.default.createDirectory(at: firstDirectory, withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: secondDirectory, withIntermediateDirectories: true)
        let first = firstDirectory.appendingPathComponent("a.heic")
        let second = secondDirectory.appendingPathComponent("b.heic")
        try Data("oplus_18".utf8).write(to: first)
        try Data("oplus_256".utf8).write(to: second)

        let viewModel = PhotoCategorizationViewModel()
        viewModel.addInputs([first, second])
        viewModel.scan()
        try await waitForCategorization(viewModel)

        let destinations = Set(viewModel.items.map(\.destinationURL))
        try expect(destinations.contains(firstDirectory.appendingPathComponent("静态照片/人像/a.heic")), "first input should use its own parent as the classification root")
        try expect(destinations.contains(secondDirectory.appendingPathComponent("静态照片/专业模式/b.heic")), "second input should use its own parent as the classification root")
    }

    @MainActor
    private static func testImportDiscoversHEICFilesAndDeduplicates() async throws {
        let root = try makeTempDirectory("import")
        defer { try? FileManager.default.removeItem(at: root.deletingLastPathComponent()) }

        let nested = root.appendingPathComponent("nested", isDirectory: true)
        try FileManager.default.createDirectory(at: nested, withIntermediateDirectories: true)
        try writeFile(root.appendingPathComponent("a.heic"))
        try writeFile(nested.appendingPathComponent("b.HEIC"))
        try writeFile(root.appendingPathComponent("ignore.jpg"))

        let viewModel = XDRemuxViewModel()
        let added = await viewModel.importFiles(from: [root])

        try expect(added == 2, "expected two HEIC files to be imported")
        try expect(viewModel.queue.count == 2, "queue should contain imported HEIC files")
        try expect(viewModel.queue.allSatisfy { $0.status == .pending }, "new queue rows should be pending")

        let addedAgain = await viewModel.importFiles(from: [root])
        try expect(addedAgain == 0, "duplicate import should add no new rows")
        try expect(viewModel.queue.count == 2, "duplicate import should not duplicate queue rows")
    }

    @MainActor
    private static func testImportedRowsStartWithEmptyThumbnailState() async throws {
        let root = try makeTempDirectory("thumbnail-empty")
        defer { try? FileManager.default.removeItem(at: root.deletingLastPathComponent()) }
        let input = root.appendingPathComponent("source.heic")
        try writeFile(input)

        let viewModel = XDRemuxViewModel()
        _ = await viewModel.importFiles(from: [input])

        guard let item = viewModel.queue.first else {
            throw TestFailure.assertion("expected imported item")
        }
        try expect(viewModel.thumbnailStatus(for: item.id) == .empty, "imported rows should start with empty thumbnail state")
    }

    @MainActor
    private static func testOutputCollisionsAreMarkedBeforeConversion() async throws {
        let root = try makeTempDirectory("collision-input")
        let out = try makeTempDirectory("collision-output")
        defer {
            try? FileManager.default.removeItem(at: root.deletingLastPathComponent())
            try? FileManager.default.removeItem(at: out.deletingLastPathComponent())
        }

        let left = root.appendingPathComponent("left", isDirectory: true)
        let right = root.appendingPathComponent("right", isDirectory: true)
        try FileManager.default.createDirectory(at: left, withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: right, withIntermediateDirectories: true)
        try writeFile(left.appendingPathComponent("same.heic"))
        try writeFile(right.appendingPathComponent("same.heic"))

        let viewModel = XDRemuxViewModel()
        viewModel.config.outputDirectory = out
        _ = await viewModel.importFiles(from: [root])

        let collisionCount = viewModel.markOutputCollisionsForTesting()
        try expect(collisionCount == 2, "both colliding queue rows should be marked")
        try expect(viewModel.queue.allSatisfy { $0.status == .failed }, "colliding rows should fail before conversion")
        try expect(viewModel.queue.allSatisfy { $0.outputPlanStatus == .duplicateOutput }, "colliding rows should show duplicate output plan state")
        try expect(viewModel.queue.allSatisfy { ($0.errorMessage ?? "").contains("输出路径冲突") }, "collisions should explain the output conflict")
    }

    @MainActor
    private static func testOutputPlanFlagsInvalidExistingOutputAsOverwriteRisk() async throws {
        let root = try makeTempDirectory("overwrite-risk-input")
        let out = try makeTempDirectory("overwrite-risk-output")
        defer {
            try? FileManager.default.removeItem(at: root.deletingLastPathComponent())
            try? FileManager.default.removeItem(at: out.deletingLastPathComponent())
        }

        let input = root.appendingPathComponent("a.heic")
        let output = out.appendingPathComponent("a.heic")
        try writeFile(input)
        try writeFile(output, bytes: 32)

        let viewModel = XDRemuxViewModel()
        viewModel.config.outputDirectory = out
        _ = await viewModel.importFiles(from: [input])
        viewModel.refreshOutputURLsForPendingItems()

        try expect(viewModel.queue.first?.outputURL == output, "output directory should map to the expected target path")
        try expect(viewModel.queue.first?.outputPlanStatus == .willOverwriteExisting, "invalid existing output should be visible as overwrite risk")
        try expect(viewModel.canStart, "overwrite risk should not block conversion because conversion can replace the target")
    }

    @MainActor
    private static func testOutputParentFileBlocksConversionBeforeWorkStarts() async throws {
        let root = try makeTempDirectory("parent-file-input")
        defer { try? FileManager.default.removeItem(at: root.deletingLastPathComponent()) }

        let input = root.appendingPathComponent("a.heic")
        let outputParentFile = root.appendingPathComponent("not-a-directory")
        try writeFile(input)
        try writeFile(outputParentFile)

        let viewModel = XDRemuxViewModel()
        viewModel.config.outputDirectory = outputParentFile
        _ = await viewModel.importFiles(from: [input])

        let blockedCount = viewModel.markOutputCollisionsForTesting()
        try expect(blockedCount == 1, "parent file should block the row before conversion")
        try expect(viewModel.queue.first?.status == .failed, "blocked parent file should mark the row failed")
        try expect(viewModel.queue.first?.outputPlanStatus == .outputParentIsFile, "blocked parent file should be reflected in output plan state")
        try expect(!viewModel.canStart, "blocked rows should not keep start enabled")
    }

    @MainActor
    private static func testPreparingOutputKeepsInvalidExistingFileUntilConversionSucceeds() throws {
        let root = try makeTempDirectory("prepare-output")
        defer { try? FileManager.default.removeItem(at: root.deletingLastPathComponent()) }

        let input = root.appendingPathComponent("input.heic")
        let output = root.appendingPathComponent("output.heic")
        try writeFile(input)
        try writeFile(output, bytes: 32)

        let disposition = try XDRemuxViewModel.prepareOutputForConversionForTesting(
            inputURL: input,
            outputURL: output,
            skipExisting: true
        )

        try expect(
            disposition == .replacesExistingInvalidOutput,
            "invalid existing output should be scheduled for replacement"
        )
        try expect(
            FileManager.default.fileExists(atPath: output.path),
            "invalid existing output must survive preparation so a failed conversion cannot destroy it"
        )
    }

    @MainActor
    private static func testThumbnailRendererProducesBoundedPNGData() throws {
        let icon = URL(fileURLWithPath: "apps/macos/XDRemuxApp/Assets.xcassets/AppIcon.appiconset/icon_32x32.png")
        let data = try XDRemuxViewModel.makeThumbnailPNGDataForTesting(for: icon, maxPixelSize: 24)

        try expect(data.count > 8, "thumbnail renderer should produce image data")
        try expect(data.prefix(8) == Data([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]), "thumbnail renderer should produce PNG output")
    }

    @MainActor
    private static func testEffectiveConcurrencyRespectsMemoryAndUserLimit() throws {
        let viewModel = XDRemuxViewModel()
        viewModel.config.maxConcurrentJobs = 4

        let small = viewModel.effectiveConcurrencyForTesting(
            fileSizes: [10 * 1024 * 1024, 12 * 1024 * 1024],
            physicalMemory: 64 * 1024 * 1024 * 1024,
            processorCount: 8
        )
        try expect(small == 4, "small files should keep the configured concurrency")

        let cpuCapped = viewModel.effectiveConcurrencyForTesting(
            fileSizes: [10 * 1024 * 1024],
            physicalMemory: 64 * 1024 * 1024 * 1024,
            processorCount: 2
        )
        try expect(cpuCapped == 2, "concurrency should respect the available processor count")

        let large = viewModel.effectiveConcurrencyForTesting(
            fileSizes: [20 * 1024 * 1024 * 1024],
            physicalMemory: 16 * 1024 * 1024 * 1024,
            processorCount: 8
        )
        try expect(large == 1, "oversized inputs should drop concurrency to one")
    }

    @MainActor
    private static func testQueueStatusCountsMatchAllTerminalStates() throws {
        let root = FileManager.default.temporaryDirectory
        let statuses: [ConversionQueueStatus] = [
            .pending, .running, .converted, .converted,
            .skippedExisting, .failed, .cancelled,
        ]
        let viewModel = XDRemuxViewModel()
        viewModel.queue = statuses.enumerated().map { index, status in
            ConversionQueueItem(
                inputURL: root.appendingPathComponent("input-\(index).heic"),
                outputURL: root.appendingPathComponent("output-\(index).heic"),
                status: status
            )
        }
        let counts = viewModel.queueStatusCounts
        try expect(counts.pending == 1, "queue summary should count pending rows once")
        try expect(counts.running == 1, "queue summary should count running rows once")
        try expect(counts.converted == 2, "queue summary should count converted rows once")
        try expect(counts.skipped == 1, "queue summary should count skipped rows once")
        try expect(counts.failed == 1, "queue summary should count failed rows once")
        try expect(counts.cancelled == 1, "queue summary should count cancelled rows once")
        try expect(counts.processed == 5, "queue summary should keep pending/running out of processed")
    }

    @MainActor
    private static func testClearCompletedAndRetryFailedKeepQueuePredictable() throws {
        let root = URL(fileURLWithPath: "/tmp")
        let viewModel = XDRemuxViewModel()
        viewModel.queue = [
            ConversionQueueItem(inputURL: root.appendingPathComponent("ok.heic"), outputURL: root.appendingPathComponent("ok_iso.heic"), status: .converted),
            ConversionQueueItem(inputURL: root.appendingPathComponent("skip.heic"), outputURL: root.appendingPathComponent("skip_iso.heic"), status: .skippedExisting),
            ConversionQueueItem(inputURL: root.appendingPathComponent("bad.heic"), outputURL: root.appendingPathComponent("bad_iso.heic"), status: .failed, errorMessage: "boom"),
            ConversionQueueItem(inputURL: root.appendingPathComponent("todo.heic"), outputURL: root.appendingPathComponent("todo_iso.heic"), status: .pending)
        ]

        viewModel.clearCompleted()
        try expect(viewModel.queue.map(\.status) == [.failed, .pending], "clearCompleted should remove only completed and skipped rows")

        viewModel.retryFailed()
        try expect(viewModel.queue.map(\.status) == [.pending, .pending], "retryFailed should reset failed rows to pending")
        try expect(viewModel.queue.allSatisfy { $0.errorMessage == nil }, "retryFailed should clear old errors")
    }

    @MainActor
    private static func testThumbnailResultOnlyAppliesToExistingMatchingQueueItem() throws {
        let root = URL(fileURLWithPath: "/tmp")
        let originalURL = root.appendingPathComponent("original.heic")
        let changedURL = root.appendingPathComponent("changed.heic")
        let staleID = UUID()
        let currentID = UUID()
        let viewModel = XDRemuxViewModel()
        viewModel.queue = [
            ConversionQueueItem(id: currentID, inputURL: changedURL, outputURL: root.appendingPathComponent("changed_iso.heic"))
        ]

        let staleApplied = viewModel.applyThumbnailFailureForTesting(
            id: staleID,
            inputURL: originalURL,
            message: "stale"
        )
        try expect(!staleApplied, "thumbnail result for missing row should be ignored")

        let changedApplied = viewModel.applyThumbnailFailureForTesting(
            id: currentID,
            inputURL: originalURL,
            message: "wrong-url"
        )
        try expect(!changedApplied, "thumbnail result for stale input URL should be ignored")

        let currentApplied = viewModel.applyThumbnailFailureForTesting(
            id: currentID,
            inputURL: changedURL,
            message: "decode failed"
        )
        try expect(currentApplied, "thumbnail result for current row should be applied")
        try expect(viewModel.thumbnailStatus(for: currentID) == .failed("decode failed"), "current thumbnail failure should be stored")
    }

    @MainActor
    private static func testUserFacingCopyUsesClearConversionTerms() throws {
        try expect(AppStrings.addHEIC == "添加 HEIC", "add action should name the input type")
        try expect(AppStrings.startConversion == "开始转换", "primary action should describe conversion")
        try expect(AppStrings.emptyQueueTitle == "拖入或添加 ProXDR HEIC 文件", "empty state should use the ProXDR input term")
        try expect(AppStrings.statConverted == "成功", "converted statistic should read as success")
        try expect(AppStrings.statSkipped == "已跳过", "skipped statistic should be explicit")
        try expect(AppStrings.statCancelled == "已取消", "cancelled statistic should be explicit")
        try expect(AppStrings.statTotal == "总计", "total statistic should use the requested term")
        try expect(AppStrings.oppoGalleryCompatibilityHelp.contains("Rust"), "OPPO setting should name the canonical product owner")
        try expect(AppStrings.outputPlanOverwriteExisting.contains("覆盖"), "overwrite plan copy should make replacement explicit")
        try expect(AppStrings.previewUnavailable.contains("预览"), "thumbnail failure copy should name preview behavior")
    }

    @MainActor
    private static func testProductPolicyDefaults() throws {
        let config = ConversionConfig()

        try expect(config.fileNameSuffix == "_iso", "default output naming should remain stable")
        try expect(config.skipExisting, "default conversion should skip an already-valid output")
        try expect(config.maxConcurrentJobs >= 1, "default concurrency should always be positive")
        try expect(!config.oppoGalleryCompatibilityEnabled, "OPPO Gallery compatibility should be opt-in")
        try expect(!config.applePhotographicStyles, "Apple Photographic Styles should be opt-in")
        try expect(!config.applePortrait, "Apple Portrait should be opt-in")
    }

    @MainActor
    private static func testSimplifiedProductSwitches() throws {
        var config = ConversionConfig()

        config.oppoGalleryCompatibilityEnabled = false
        try expect(!config.oppoGalleryCompatibilityEnabled, "standard ISO mode should remain the default")

        config.oppoGalleryCompatibilityEnabled = true
        try expect(config.oppoGalleryCompatibilityEnabled, "OPPO compatibility should be an explicit product intent")
        try expect(!config.applePhotographicStyles && !config.applePortrait, "OPPO compatibility should clear incompatible Apple intents")

        config.oppoGalleryCompatibilityEnabled = false
        try expect(!config.oppoGalleryCompatibilityEnabled, "disabling OPPO compatibility should restore standard HDR")
    }

    @MainActor
    private static func testIndependentAppleFeatureConfiguration() throws {
        var config = ConversionConfig()
        config.applePhotographicStyles = true

        try expect(config.appleFeaturesEnabled, "either Apple capability should activate the shared writer")
        let stylesArguments = try RustCLIClient.productArgumentsForTesting(config: config)
        try expect(
            stylesArguments == ["--apple-styles"],
            "Styles intent must map to the canonical Rust CLI"
        )

        config.applePhotographicStyles = false
        config.applePortrait = true
        let portraitArguments = try RustCLIClient.productArgumentsForTesting(config: config)
        try expect(
            portraitArguments == ["--apple-portrait"],
            "Portrait intent must map to the canonical Rust CLI"
        )

        config.applePhotographicStyles = true
        try expect(
            (try? RustCLIClient.productArgumentsForTesting(config: config)) == nil,
            "the app must not silently collapse two incompatible Rust product intents"
        )

        config.applePhotographicStyles = false
        config.applePortrait = true
        config.oppoGalleryCompatibilityEnabled = true
        try expect(!config.applePhotographicStyles && !config.applePortrait, "enabling OPPO compatibility must clear incompatible Apple capabilities")
    }
}
