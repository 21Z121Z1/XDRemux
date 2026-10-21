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
        try testLivePhotoOutputArgumentsAndMixedQueuePlans()
        try await testPVTChoiceReachesRustWithoutChangingOrdinaryOutput()
        try await testExistingPVTDestinationsAreNeverAdoptedOrReplaced()
        try await testExistingPVTPackagesAreNotScannedOrUsedAsOutputDirectories()
        try await testPVTPreviewUsesSourceImage()
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
    private static func testLivePhotoOutputArgumentsAndMixedQueuePlans() throws {
        var config = ConversionConfig()
        try expect(config.livePhotoOutput == .pair, "Live Photo output should default to the existing file pair")
        try expect(try RustCLIClient.productArgumentsForTesting(config: config) == [], "default CLI arguments must remain empty")
        config.livePhotoOutput = .pvt
        try expect(try RustCLIClient.productArgumentsForTesting(config: config) == ["--live-photo-output", "pvt"], "PVT should forward the typed Rust container option")
        config.applePhotographicStyles = true
        try expect(try RustCLIClient.productArgumentsForTesting(config: config) == ["--live-photo-output", "pvt", "--apple-styles"], "container selection must preserve other explicit product intents")

        let root = try makeTempDirectory("pvt-plans")
        defer { try? FileManager.default.removeItem(at: root.deletingLastPathComponent()) }
        let motion = root.appendingPathComponent("motion.heic")
        let ordinary = root.appendingPathComponent("ordinary.heic")
        try writeFile(motion)
        try writeFile(ordinary)
        let model = XDRemuxViewModel()
        model.config.livePhotoOutput = .pvt
        model.queue = [ConversionQueueItem(inputURL: motion, outputURL: motion, assetType: .livePhoto),
                       ConversionQueueItem(inputURL: ordinary, outputURL: ordinary)]
        model.refreshOutputURLsForPendingItems()
        try expect(model.queue.map(\.outputURL.lastPathComponent) == ["motion_iso.pvt", "ordinary_iso.heic"], "only Rust-classified Live Photos should plan a PVT directory")
        model.config.outputDirectory = root.appendingPathComponent("output")
        model.config.categorizeOutputByCaptureMode = true
        model.refreshOutputURLsForPendingItems()
        try expect(model.queue[0].outputURL == root.appendingPathComponent("output/实况照片/未分类/motion.pvt", isDirectory: true), "PVT should retain the existing asset/mode folder projection")
        try expect(model.queue[1].outputURL == root.appendingPathComponent("output/静态照片/未分类/ordinary.heic"), "mixed ordinary HDR should retain its HEIC path")
        model.config.livePhotoOutput = .pair
        model.refreshOutputURLsForPendingItems()
        try expect(model.queue[0].outputURL.pathExtension == "heic", "switching pending rows back to pair should refresh their output paths")
    }

    @MainActor
    private static func testPVTChoiceReachesRustWithoutChangingOrdinaryOutput() async throws {
        try await withCLI(#"""
        [ "$1" = convert ] || exit 64
        printf '%s\n' "$@" >> "$0.log"
        shift
        output=''
        format=pair
        while [ "$#" -gt 0 ]; do
            case "$1" in
                --output) output="$2"; shift 2 ;;
                --live-photo-output) format="$2"; shift 2 ;;
                *) shift ;;
            esac
        done
        [ "$format" = pvt ] || exit 65
        case "$output" in
            *.pvt) mkdir "$output"; printf 'rust-owned-package' > "$output/fixture-marker" ;;
            *.heic) printf 'ordinary-hdr' > "$output" ;;
            *) exit 66 ;;
        esac
        """#) { root in
            let motion = root.appendingPathComponent("motion source.heic")
            let ordinary = root.appendingPathComponent("ordinary source.heic")
            try writeFile(motion)
            try writeFile(ordinary)
            let model = XDRemuxViewModel()
            model.config.outputDirectory = root.appendingPathComponent("mixed outputs")
            model.config.livePhotoOutput = .pvt
            model.config.maxConcurrentJobs = 1
            model.queue = [ConversionQueueItem(inputURL: motion, outputURL: motion, assetType: .livePhoto),
                           ConversionQueueItem(inputURL: ordinary, outputURL: ordinary)]
            try await runConversion(model)
            try expect(model.queue.allSatisfy { $0.status == .converted }, "the app should launch the same Rust intent for a mixed queue")
            try expect(model.queue[0].outputURL.lastPathComponent == "motion source.pvt", "the output/reveal URL should be the outer package")
            try expect(model.queue[1].outputURL.lastPathComponent == "ordinary source.heic", "the ordinary result must remain HEIC")
            try expect(try Data(contentsOf: model.queue[0].outputURL.appendingPathComponent("fixture-marker")) == Data("rust-owned-package".utf8), "only Rust's process should supply package contents")
            try expect(try Data(contentsOf: ordinary) == Data(repeating: 0x41, count: 16), "ordinary queued originals must survive")
            try expect(try Data(contentsOf: motion) == Data(repeating: 0x41, count: 16), "Motion Photo queued originals must survive")
            let calls = try String(contentsOf: root.appendingPathComponent("xdremux-test-cli.log"), encoding: .utf8)
            try expect(calls.components(separatedBy: "--live-photo-output\npvt").count == 3, "both calls should forward PVT selection without splitting user paths")
        }
    }

    @MainActor
    private static func testExistingPVTDestinationsAreNeverAdoptedOrReplaced() async throws {
        try await withCLI("printf '%s\\n' \"$*\" >> \"$0.log\"") { root in
            let input = root.appendingPathComponent("motion.heic")
            try writeFile(input)
            for skip in [true, false] {
                for kind in ["file", "directory", "symlink", "symlink-directory", "dangling"] {
                    let directory = root.appendingPathComponent("\(skip)-\(kind)")
                    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
                    let output = directory.appendingPathComponent("motion.pvt")
                    let target = directory.appendingPathComponent("target")
                    if kind == "directory" {
                        try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
                        try Data("keep-member".utf8).write(to: output.appendingPathComponent("member.heic"))
                    } else if kind == "symlink-directory" {
                        try FileManager.default.createDirectory(at: target, withIntermediateDirectories: true)
                        try Data("keep-target-member".utf8).write(to: target.appendingPathComponent("member.heic"))
                        try FileManager.default.createSymbolicLink(at: output, withDestinationURL: target)
                    } else if kind == "symlink" || kind == "dangling" {
                        if kind == "symlink" { try Data("keep-target".utf8).write(to: target) }
                        try FileManager.default.createSymbolicLink(at: output, withDestinationURL: target)
                    } else {
                        try Data("keep-file".utf8).write(to: output)
                    }
                    let model = XDRemuxViewModel()
                    model.config.outputDirectory = directory
                    model.config.livePhotoOutput = .pvt
                    model.config.skipExisting = skip
                    model.queue = [ConversionQueueItem(inputURL: input, outputURL: output, assetType: .livePhoto)]
                    try expect(!RustCLIClient.isValidOutput(output, config: model.config), "a successful generic validate cannot authorize PVT reuse without durable provenance")
                    try expect((try? XDRemuxViewModel.prepareOutputForConversionForTesting(inputURL: input, outputURL: output, skipExisting: skip)) == nil, "preparation must reject every existing PVT entry")
                    _ = model.markOutputCollisionsForTesting()
                    try expect(model.queue[0].outputPlanStatus == .existingPackage && !model.canStart, "an existing PVT must block both skip and replacement")
                    if kind == "directory" {
                        try expect(try FileManager.default.contentsOfDirectory(atPath: output.path) == ["member.heic"], "an existing package must keep its member set")
                        try expect(try Data(contentsOf: output.appendingPathComponent("member.heic")) == Data("keep-member".utf8), "existing member bytes must survive")
                    } else if kind == "symlink-directory" {
                        try expect(try FileManager.default.destinationOfSymbolicLink(atPath: output.path) == target.path, "directory aliases must survive")
                        try expect(try FileManager.default.contentsOfDirectory(atPath: target.path) == ["member.heic"], "aliased package members must survive")
                        try expect(try Data(contentsOf: target.appendingPathComponent("member.heic")) == Data("keep-target-member".utf8), "aliased package bytes must survive")
                    } else if kind == "symlink" || kind == "dangling" {
                        try expect(try FileManager.default.destinationOfSymbolicLink(atPath: output.path) == target.path, "existing and dangling links must survive")
                        if kind == "symlink" { try expect(try Data(contentsOf: target) == Data("keep-target".utf8), "link targets must survive") }
                    } else {
                        try expect(try Data(contentsOf: output) == Data("keep-file".utf8), "existing file bytes must survive")
                    }
                }
            }
            for skip in [true, false] {
                let directory = root.appendingPathComponent("ordinary-name-alias-\(skip)")
                try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
                let package = directory.appendingPathComponent("existing.pvt", isDirectory: true)
                try FileManager.default.createDirectory(at: package, withIntermediateDirectories: true)
                let member = package.appendingPathComponent("member.heic")
                try Data("keep-aliased-package".utf8).write(to: member)
                let ordinaryInput = root.appendingPathComponent("foo.heic")
                try writeFile(ordinaryInput)
                let output = directory.appendingPathComponent("foo.heic")
                try FileManager.default.createSymbolicLink(at: output, withDestinationURL: package)
                let model = XDRemuxViewModel()
                model.config.outputDirectory = directory
                model.config.livePhotoOutput = .pvt
                model.config.skipExisting = skip
                model.queue = [ConversionQueueItem(inputURL: ordinaryInput, outputURL: output)]
                try expect((try? XDRemuxViewModel.prepareOutputForConversionForTesting(inputURL: ordinaryInput, outputURL: output, skipExisting: skip)) == nil, "an ordinary output name resolving directly to PVT must never enter reuse or replacement")
                _ = model.markOutputCollisionsForTesting()
                try expect(model.queue[0].outputURL.pathExtension == "heic", "ordinary rows should retain HEIC naming even when that name is an unsafe alias")
                try expect(model.queue[0].outputPlanStatus == .existingPackage && !model.canStart, "the resolved current node must block a package alias for both skip settings")
                try expect(try FileManager.default.destinationOfSymbolicLink(atPath: output.path) == package.path, "the ordinary-name output link must survive")
                try expect(try FileManager.default.contentsOfDirectory(atPath: package.path) == ["member.heic"], "ordinary-name aliases cannot change the package member set")
                try expect(try Data(contentsOf: member) == Data("keep-aliased-package".utf8), "ordinary-name aliases cannot change package member bytes")
                try expect(try Data(contentsOf: ordinaryInput) == Data(repeating: 0x41, count: 16), "ordinary-name package aliases cannot change the queued original")
            }
            let aliasDirectory = root.appendingPathComponent("queued-input-alias")
            try FileManager.default.createDirectory(at: aliasDirectory, withIntermediateDirectories: true)
            let otherInput = root.appendingPathComponent("other-original.heic")
            try writeFile(otherInput, bytes: 32)
            let alias = aliasDirectory.appendingPathComponent("motion.pvt")
            try FileManager.default.createSymbolicLink(at: alias, withDestinationURL: otherInput)
            let aliases = XDRemuxViewModel()
            aliases.config.outputDirectory = aliasDirectory
            aliases.config.livePhotoOutput = .pvt
            aliases.queue = [ConversionQueueItem(inputURL: input, outputURL: alias, assetType: .livePhoto),
                             ConversionQueueItem(inputURL: otherInput, outputURL: otherInput)]
            _ = aliases.markOutputCollisionsForTesting()
            try expect(aliases.queue[0].outputPlanStatus == .overwritesInput, "PVT aliases must retain queued-original identity protection")
            try expect(try Data(contentsOf: otherInput) == Data(repeating: 0x41, count: 32), "an output alias cannot modify another queued original")
            try expect(!FileManager.default.fileExists(atPath: root.appendingPathComponent("xdremux-test-cli.log").path), "blocked PVT entries should never reach validate or convert")
            try expect(try Data(contentsOf: input) == Data(repeating: 0x41, count: 16), "planning and preparation must preserve the queued original")
        }
    }

    @MainActor
    private static func testPVTPreviewUsesSourceImage() async throws {
        let root = try makeTempDirectory("pvt-preview")
        defer { try? FileManager.default.removeItem(at: root.deletingLastPathComponent()) }
        let icon = URL(fileURLWithPath: "apps/macos/XDRemuxApp/Assets.xcassets/AppIcon.appiconset/icon_32x32.png")
        let input = root.appendingPathComponent("motion.heic")
        try FileManager.default.copyItem(at: icon, to: input)
        let output = root.appendingPathComponent("motion.pvt", isDirectory: true)
        try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
        let item = ConversionQueueItem(inputURL: input, outputURL: output, status: .converted, assetType: .livePhoto)
        let model = XDRemuxViewModel()
        model.queue = [item]
        model.loadThumbnailIfNeeded(for: item)
        let clock = ContinuousClock()
        let deadline = clock.now.advanced(by: .seconds(5))
        while model.thumbnailStatus(for: item.id) == .loading, clock.now < deadline {
            try await Task.sleep(for: .milliseconds(10))
        }
        guard case .ready = model.thumbnailStatus(for: item.id) else {
            throw TestFailure.assertion("PVT preview must read the source image rather than the opaque output directory")
        }
        try expect(model.queue[0].outputURL == output, "Finder reveal should retain the outer package URL")
        try expect(try FileManager.default.contentsOfDirectory(atPath: output.path).isEmpty, "preview must not populate or modify the package")
    }

    @MainActor
    private static func testExistingPVTPackagesAreNotScannedOrUsedAsOutputDirectories() async throws {
        try await withCLI("printf '{\"items\": []}\\n'") { root in
            let package = root.appendingPathComponent("existing.PVT", isDirectory: true)
            try FileManager.default.createDirectory(at: package, withIntermediateDirectories: true)
            let member = package.appendingPathComponent("member.heic")
            try Data("unchanged-member".utf8).write(to: member)
            let scanner = XDRemuxViewModel()
            _ = await scanner.importFiles(from: [package, member, root])
            try expect(scanner.queue.isEmpty, "directory discovery must treat PVT as an opaque package")

            let input = root.appendingPathComponent("source.heic")
            try writeFile(input)
            let alias = root.appendingPathComponent("package-alias")
            try FileManager.default.createSymbolicLink(at: alias, withDestinationURL: package)
            for format in LivePhotoOutput.allCases {
                for outputDirectory in [package, package.appendingPathComponent("nested"), alias] {
                    let model = XDRemuxViewModel()
                    model.config.livePhotoOutput = format
                    model.config.outputDirectory = outputDirectory
                    model.queue = [ConversionQueueItem(inputURL: input, outputURL: input, assetType: .livePhoto)]
                    _ = model.markOutputCollisionsForTesting()
                    try expect(model.queue[0].outputPlanStatus == .existingPackage && !model.canStart, "both pair and PVT outputs must not add members inside an existing package")
                    try expect((try? XDRemuxViewModel.prepareOutputForConversionForTesting(inputURL: input, outputURL: model.queue[0].outputURL, skipExisting: true)) == nil, "execution preparation must reject package descendants as well as planning")
                }
            }
            try expect(try FileManager.default.contentsOfDirectory(atPath: package.path) == ["member.heic"], "discovery and output planning must preserve the complete package")
            try expect(try Data(contentsOf: member) == Data("unchanged-member".utf8), "discovery and output planning must preserve its original media bytes")
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
        try expect(AppStrings.livePhotoOutputTitle == "实况照片输出", "container selection should name Live Photo output")
        try expect(AppStrings.livePhotoOutputPair == "照片 + 视频文件对", "pair label should name both members")
        try expect(AppStrings.livePhotoOutputPvt == "PVT 文件包", "PVT label should identify a package")
        try expect(AppStrings.livePhotoOutputHelp == "选择实况照片的封装方式。PVT 为目录包，封装不额外重编码；动态墙纸可用性尚未验证。", "container help must preserve the packaging and unverified wallpaper boundary")
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
