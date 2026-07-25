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
func expect(_ condition: @autoclosure () -> Bool, _ message: String) throws {
    guard condition() else {
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
        try await testConversionCategorizationMapsModeAndRootDirectories()
        try await testCategorizationPreviewCopyAndDuplicateRerun()
        try await testCategorizationDefaultsToEachSourceDirectory()
        try await testCategorizationCancellationKeepsCancelledState()
        try await testImportedRowsStartWithEmptyThumbnailState()
        try await testOutputCollisionsAreMarkedBeforeConversion()
        try await testOutputPlanFlagsInvalidExistingOutputAsOverwriteRisk()
        try await testOutputParentFileBlocksConversionBeforeWorkStarts()
        try testPreparingOutputRemovesInvalidExistingFileBeforeConversion()
        try testThumbnailRendererProducesBoundedPNGData()
        try testEffectiveConcurrencyRespectsMemoryAndUserLimit()
        try testClearCompletedAndRetryFailedKeepQueuePredictable()
        try testThumbnailResultOnlyAppliesToExistingMatchingQueueItem()
        try testUserFacingCopyUsesClearConversionTerms()
        try testProductPolicyDefaults()
        try testSimplifiedProductSwitches()
        try testIndependentAppleFeatureConfiguration()
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
    private static func testConversionCategorizationMapsModeAndRootDirectories() async throws {
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
        try expect(byName["portrait.heic"]?.outputURL == output.appendingPathComponent("人像/portrait.heic"), "categorized conversion should use the mode directory")
        try expect(byName["unknown.heic"]?.classificationStatus == .unknownFlags, "unknown flags should remain visible in the queue")
        try expect(byName["unknown.heic"]?.outputURL == output.appendingPathComponent("unknown.heic"), "unclassified conversion should stay at the output root")
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
        try expect(viewModel.categorizedCount == 1 && viewModel.rootCount == 1, "preview counts should separate categorized and root items")
        try expect(viewModel.modeSummary.contains("人像 1"), "preview should summarize counts by capture mode")
        try expect(viewModel.items.contains { $0.destinationURL == output.appendingPathComponent("人像/portrait.heic") }, "preview should show the mode destination")
        try expect(viewModel.items.contains { $0.destinationURL == output.appendingPathComponent("plain.jpg") }, "preview should show the root destination")

        viewModel.copyPlannedFiles()
        try await waitForCategorization(viewModel)
        try expect(viewModel.state == .completed, "copy should complete")
        try expect(FileManager.default.fileExists(atPath: output.appendingPathComponent("人像/portrait.heic").path), "copy should create the categorized file")
        try expect(FileManager.default.fileExists(atPath: output.appendingPathComponent("plain.jpg").path), "copy should create the root file")

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
        try expect(destinations.contains(firstDirectory.appendingPathComponent("人像/a.heic")), "first input should use its own parent as the classification root")
        try expect(destinations.contains(secondDirectory.appendingPathComponent("专业模式/b.heic")), "second input should use its own parent as the classification root")
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
    private static func testPreparingOutputRemovesInvalidExistingFileBeforeConversion() throws {
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

        try expect(disposition == .removedExistingInvalidOutput, "invalid existing output should be removed before conversion")
        try expect(!FileManager.default.fileExists(atPath: output.path), "invalid existing output file should be gone")
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
        try expect(AppStrings.oppoCompatLabel == "[实验性] OPPO 相册 HDR 兼容层", "OPPO setting should be clearly marked experimental")
        try expect(AppStrings.oppoGalleryCompatibilityHelp.contains("非 HDR 厂商尾部"), "default product copy should describe the non-HDR tail policy")
        try expect(AppStrings.inputProcessingSystemHelp.contains("ImageIO"), "system mode help should explain ImageIO ownership")
        try expect(AppStrings.inputProcessingHybridHelp.contains("HEVC Rext"), "hybrid mode help should explain the gain map rewrite")
        try expect(AppStrings.inputProcessingPassthroughHelp.contains("ISOBMFF box"), "passthrough mode help should explain container rebuilding")
        try expect(AppStrings.outputPlanOverwriteExisting.contains("覆盖"), "overwrite plan copy should make replacement explicit")
        try expect(AppStrings.previewUnavailable.contains("预览"), "thumbnail failure copy should name preview behavior")
    }

    @MainActor
    private static func testProductPolicyDefaults() throws {
        let config = ConversionConfig()

        try expect(config.family == .auto, "product input family should be detected automatically")
        try expect(config.inputProcessingBranch == .hybrid, "product output should use metadata-preserving remux")
        try expect(config.tmapFormat == .imageIO, "product output should use the device-validated 142-byte tmap")
        try expect(config.oppoCompatibility == .off, "default product output should select standard high-spec ISO encoding")
        try expect(config.oppoCameraTail == .preserveWithoutPrivateHDR, "default product output should preserve only the non-HDR source tail")
        try expect(!config.oppoGalleryCompatibilityEnabled, "OPPO Gallery compatibility should be opt-in")
        try expect(config.preservesPortraitEditingData, "portrait editing data should default to preserved")
        try expect(!config.applePhotographicStyles, "Apple Photographic Styles should be opt-in")
        try expect(!config.applePortrait, "Apple Portrait should be opt-in")
    }

    @MainActor
    private static func testSimplifiedProductSwitches() throws {
        var config = ConversionConfig()

        config.oppoGalleryCompatibilityEnabled = false
        try expect(config.oppoCompatibility == .off, "disabling compatibility should select Hybrid high-spec encoding")
        try expect(config.oppoCameraTail == .preserveWithoutPrivateHDR, "standard ISO mode should remove private HDR tail entries")

        config.preservesPortraitEditingData = false
        try expect(config.oppoCameraTail == .preserveWithoutPortraitOrPrivateHDR, "portrait filtering must not reintroduce private HDR tail entries")
        try expect(config.oppoCompatibility == .off, "portrait switch must not change Gain Map encoding")

        config.oppoGalleryCompatibilityEnabled = true
        try expect(config.oppoCameraTail == .preserveWithoutPortrait, "OPPO compatibility should restore private HDR tail data without portrait resources")
        config.preservesPortraitEditingData = true
        try expect(config.oppoCompatibility == .auto, "enabling compatibility should restore automatic OPPO routing")
        try expect(config.oppoCameraTail == .preserve, "enabling portrait preservation should restore the byte-preserving tail")

        config.oppoGalleryCompatibilityEnabled = false
        try expect(config.oppoCameraTail == .preserveWithoutPrivateHDR, "leaving OPPO compatibility should restore the non-HDR default tail")
    }

    @MainActor
    private static func testIndependentAppleFeatureConfiguration() throws {
        var config = ConversionConfig()
        config.applePhotographicStyles = true
        config.applePortrait = true

        try expect(config.appleFeaturesEnabled, "either Apple capability should activate the shared writer")
        let input = URL(fileURLWithPath: "/tmp/input.heic")
        let output = URL(fileURLWithPath: "/tmp/output.heic")
        let request = AppConversionEngine.requestForTesting(
            inputURL: input,
            outputURL: output,
            config: config
        )
        try expect(request.configuration.applePhotographicStyles, "App must preserve the Styles capability")
        try expect(request.configuration.applePortrait, "App must preserve the independent Portrait capability")
        try expect(request.input.url == input, "App must create one shared input source")
        try expect(request.output.url == output, "combined mode must produce one final HEIC")

        config.applePortrait = false
        let stylesOnly = AppConversionEngine.requestForTesting(
            inputURL: input,
            outputURL: output,
            config: config
        )
        try expect(stylesOnly.configuration.applePhotographicStyles, "Styles must not depend on Portrait")
        try expect(!stylesOnly.configuration.applePortrait, "Portrait must stay independently disabled")

        config.applePortrait = true
        config.oppoGalleryCompatibilityEnabled = true
        try expect(!config.applePhotographicStyles && !config.applePortrait, "enabling OPPO compatibility must clear incompatible Apple capabilities")
    }
}
