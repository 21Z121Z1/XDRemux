#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def update(path: str, transform) -> None:
    file = ROOT / path
    text = file.read_text(encoding="utf-8")
    updated = transform(text)
    if updated == text:
        print(f"phase3: {path} already up to date")
    else:
        file.write_text(updated, encoding="utf-8")


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if new in text:
        return text
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected one replacement, found {count}")
    return text.replace(old, new, 1)


def update_style(text: str) -> str:
    old = '''            let pixelCount = current.count / 3
            let parameterBase = parameter * sampleCount
            var sampled = 0
            var nextSamplePixel = 0
            var squared = 0.0

            // One streaming pass computes the full-raster diagnostic RMS while retaining only the
            // derivative samples used by solveUpdate. No temporary derivative raster is created.
            for pixel in 0..<pixelCount {
                let store = pixel == nextSamplePixel
                let base = pixel * 3
                for channel in 0..<3 {
                    let derivative = (rendered[base + channel] - current[base + channel]) / floatStep
                    squared += Double(derivative * derivative)
                    if store {
                        values[parameterBase + sampled] = derivative
                        sampled += 1
                    }
                }
                if store {
                    nextSamplePixel += pixelStride
                }
            }
            guard sampled == sampleCount else {
                throw CLIError.invalidContainer("constrained key-1 sampled Jacobian count mismatch")
            }
            return sqrt(squared / Double(max(1, current.count)))
'''
    new = '''            let pixelCount = current.count / 3
            let parameterBase = parameter * sampleCount
            var sampled = 0
            var squared = 0.0

            // The normal-equation solver consumes exactly this sample grid. Do not spend a second
            // full-raster pass computing diagnostics for derivatives the solver will discard.
            for pixel in Swift.stride(from: 0, to: pixelCount, by: pixelStride) {
                let base = pixel * 3
                for channel in 0..<3 {
                    let derivative = (rendered[base + channel] - current[base + channel]) / floatStep
                    values[parameterBase + sampled] = derivative
                    squared += Double(derivative * derivative)
                    sampled += 1
                }
            }
            guard sampled == sampleCount else {
                throw CLIError.invalidContainer("constrained key-1 sampled Jacobian count mismatch")
            }
            return sqrt(squared / Double(max(1, sampleCount)))
'''
    text = replace_once(text, old, new, "sampled Jacobian work bound")

    old = '''                    "derivativeRMS8": derivativeRMS,
                    "metricsAgainstDisabled": Self.metrics(rendered, target).dictionary,
'''
    new = '''                    "derivativeRMS8": derivativeRMS,
                    "derivativeRMS8Sampling": "solver-sample-grid",
                    "derivativeRMS8SampleCount": jacobian.sampleCount,
                    "metricsAgainstDisabled": Self.metrics(rendered, target).dictionary,
'''
    text = replace_once(text, old, new, "derivative diagnostic provenance")

    # Candidate files are disposable scratch inputs. Closing the FileHandle establishes visibility
    # to the renderer; forcing every candidate through fsync would turn the clone optimization into
    # synchronous storage latency without adding an acceptance guarantee.
    old = '''                try handle.write(contentsOf: styleData)
                try handle.synchronize()
                try handle.close()
'''
    new = '''                try handle.write(contentsOf: styleData)
                try handle.close()
'''
    text = replace_once(text, old, new, "candidate fsync removal")
    return text


def update_encoder(text: str) -> str:
    old = '''        try raster.data.write(to: rawURL, options: .atomic)
'''
    new = '''        // UUID-scoped scratch has no durability contract; a direct write avoids an otherwise
        // redundant safe-save/rename before the helper maps the bytes read-only.
        try raster.data.write(to: rawURL)
'''
    return replace_once(text, old, new, "raw raster scratch write")


def update_view_model(text: str) -> str:
    anchor = '''enum OutputPreparationDisposition: String, Sendable, Equatable {
    case ready
    case skippedExistingValidOutput
    /// An unusable output is already at the destination. It is replaced only
    /// once a fresh conversion has succeeded, never deleted up front.
    case replacesExistingInvalidOutput
}

'''
    counts = anchor + '''struct ConversionQueueStatusCounts: Equatable {
    var pending = 0
    var running = 0
    var converted = 0
    var skipped = 0
    var failed = 0
    var cancelled = 0

    init(queue: [ConversionQueueItem]) {
        for item in queue {
            switch item.status {
            case .pending: pending += 1
            case .running: running += 1
            case .converted: converted += 1
            case .skippedExisting: skipped += 1
            case .failed: failed += 1
            case .cancelled: cancelled += 1
            }
        }
    }

    var processed: Int { converted + skipped + failed + cancelled }
}

'''
    if "struct ConversionQueueStatusCounts" not in text:
        if anchor not in text:
            raise RuntimeError("queue counts insertion anchor missing")
        text = text.replace(anchor, counts, 1)

    old = '''    var totalFiles: Int { queue.count }

    var processedCount: Int {
        queue.filter { $0.status.isTerminal }.count
    }

    var pendingCount: Int {
        queue.filter { $0.status == .pending }.count
    }

    var runningCount: Int {
        queue.filter { $0.status == .running }.count
    }

    var convertedCount: Int {
        queue.filter { $0.status == .converted }.count
    }

    var skippedCount: Int {
        queue.filter { $0.status == .skippedExisting }.count
    }

    var failedCount: Int {
        queue.filter { $0.status == .failed }.count
    }

    var cancelledCount: Int {
        queue.filter { $0.status == .cancelled }.count
    }

    var progressFraction: Double {
        guard totalFiles > 0 else { return 0 }
        return Double(processedCount) / Double(totalFiles)
    }
'''
    new = '''    var totalFiles: Int { queue.count }

    var queueStatusCounts: ConversionQueueStatusCounts {
        ConversionQueueStatusCounts(queue: queue)
    }

    var processedCount: Int { queueStatusCounts.processed }
    var pendingCount: Int { queueStatusCounts.pending }
    var runningCount: Int { queueStatusCounts.running }
    var convertedCount: Int { queueStatusCounts.converted }
    var skippedCount: Int { queueStatusCounts.skipped }
    var failedCount: Int { queueStatusCounts.failed }
    var cancelledCount: Int { queueStatusCounts.cancelled }

    var progressFraction: Double {
        guard totalFiles > 0 else { return 0 }
        return Double(queueStatusCounts.processed) / Double(totalFiles)
    }
'''
    text = replace_once(text, old, new, "queue status aggregate")

    old = '''    var visibleErrors: [String] {
        queue
            .reversed()
            .filter { $0.status == .failed }
            .prefix(5)
            .map { "\\($0.inputURL.lastPathComponent): \\($0.errorMessage ?? "未知错误")" }
    }
'''
    new = '''    var visibleErrors: [String] {
        var result: [String] = []
        result.reserveCapacity(5)
        for item in queue.reversed() where item.status == .failed {
            result.append("\\(item.inputURL.lastPathComponent): \\(item.errorMessage ?? "未知错误")")
            if result.count == 5 { break }
        }
        return result
    }
'''
    text = replace_once(text, old, new, "bounded visible errors")

    old = '''        let runConfig = config
        let fileSizes = runnableItems.map { Self.fileSize($0.inputURL) }
'''
    new = '''        // Queue structure is immutable while processing (`canEditQueue == false`), so build the
        // UUID projection once instead of rescanning the whole value array for every result.
        var queueIndexByID: [UUID: Int] = [:]
        queueIndexByID.reserveCapacity(queue.count)
        for index in queue.indices {
            queueIndexByID[queue[index].id] = index
        }

        let runConfig = config
        let fileSizes = runnableItems.map { Self.fileSize($0.inputURL) }
'''
    text = replace_once(text, old, new, "queue index projection")

    old = '''            @MainActor
            func schedule(_ item: WorkItem) {
                if let index = queue.firstIndex(where: { $0.id == item.id }) {
                    queue[index].status = .running
                    queue[index].errorMessage = nil
                    queue[index].startedAt = Date()
                    queue[index].finishedAt = nil
                }
'''
    new = '''            @MainActor
            func schedule(_ item: WorkItem) {
                if let index = queueIndexByID[item.id],
                   index < queue.count,
                   queue[index].id == item.id {
                    queue[index].status = .running
                    queue[index].errorMessage = nil
                    queue[index].startedAt = Date()
                    queue[index].finishedAt = nil
                }
'''
    text = replace_once(text, old, new, "schedule stable index")

    old = '''                active -= 1
                apply(result)
'''
    new = '''                active -= 1
                apply(result, index: queueIndexByID[result.id])
'''
    text = replace_once(text, old, new, "result stable index")

    old = '''    private func apply(_ result: QueueWorkResult) {
        guard let index = queue.firstIndex(where: { $0.id == result.id }) else { return }
        queue[index].status = result.status
'''
    new = '''    private func apply(_ result: QueueWorkResult, index: Int?) {
        guard let index,
              index < queue.count,
              queue[index].id == result.id else { return }
        queue[index].status = result.status
'''
    text = replace_once(text, old, new, "apply stable index")
    return text


def update_content_view(text: str) -> str:
    old = '''    private var sidebar: some View {
        VStack(spacing: 0) {
            QueueSidebarHeader(
                stateTitle: stateTitle,
                stateDetail: stateDetail,
                progressFraction: viewModel.progressFraction,
                processedCount: viewModel.processedCount,
                totalFiles: viewModel.totalFiles
            )
'''
    new = '''    private var sidebar: some View {
        let counts = viewModel.queueStatusCounts
        return VStack(spacing: 0) {
            QueueSidebarHeader(
                stateTitle: stateTitle(counts: counts),
                stateDetail: stateDetail(counts: counts),
                progressFraction: viewModel.totalFiles > 0
                    ? Double(counts.processed) / Double(viewModel.totalFiles)
                    : 0,
                processedCount: counts.processed,
                totalFiles: viewModel.totalFiles
            )
'''
    text = replace_once(text, old, new, "sidebar one-pass counts")

    old = '''    private var workbench: some View {
        VStack(spacing: 0) {
            ProgressSurface(
                convertedCount: viewModel.convertedCount,
                skippedCount: viewModel.skippedCount,
                failedCount: viewModel.failedCount,
                cancelledCount: viewModel.cancelledCount,
                pendingCount: viewModel.pendingCount,
                runningCount: viewModel.runningCount,
                totalFiles: viewModel.totalFiles,
'''
    new = '''    private var workbench: some View {
        let counts = viewModel.queueStatusCounts
        return VStack(spacing: 0) {
            ProgressSurface(
                convertedCount: counts.converted,
                skippedCount: counts.skipped,
                failedCount: counts.failed,
                cancelledCount: counts.cancelled,
                pendingCount: counts.pending,
                runningCount: counts.running,
                totalFiles: viewModel.totalFiles,
'''
    text = replace_once(text, old, new, "workbench one-pass counts")

    old = '''                failedCount: viewModel.failedCount,
                completedCount: viewModel.convertedCount + viewModel.skippedCount,
'''
    new = '''                failedCount: counts.failed,
                completedCount: counts.converted + counts.skipped,
'''
    text = replace_once(text, old, new, "footer one-pass counts")

    old = '''    private var stateTitle: String {
        switch viewModel.state {
'''
    new = '''    private func stateTitle(counts: ConversionQueueStatusCounts) -> String {
        switch viewModel.state {
'''
    text = replace_once(text, old, new, "state title count input")
    old = '''        case .completed:
            return viewModel.failedCount == 0 ? AppStrings.conversionFinished : AppStrings.conversionFinishedWithFailures
'''
    new = '''        case .completed:
            return counts.failed == 0 ? AppStrings.conversionFinished : AppStrings.conversionFinishedWithFailures
'''
    text = replace_once(text, old, new, "state title failed count")

    old = '''    private var stateDetail: String {
        if !viewModel.currentFileName.isEmpty {
'''
    new = '''    private func stateDetail(counts: ConversionQueueStatusCounts) -> String {
        if !viewModel.currentFileName.isEmpty {
'''
    text = replace_once(text, old, new, "state detail count input")
    old = '''        return "\\(AppStrings.pending) \\(viewModel.pendingCount)，\\(AppStrings.running) \\(viewModel.runningCount)"
'''
    new = '''        return "\\(AppStrings.pending) \\(counts.pending)，\\(AppStrings.running) \\(counts.running)"
'''
    text = replace_once(text, old, new, "state detail counts")
    return text


def update_app_tests(text: str) -> str:
    old = '''        try testEffectiveConcurrencyRespectsMemoryAndUserLimit()
        try testClearCompletedAndRetryFailedKeepQueuePredictable()
'''
    new = '''        try testEffectiveConcurrencyRespectsMemoryAndUserLimit()
        try testQueueStatusCountsMatchAllTerminalStates()
        try testClearCompletedAndRetryFailedKeepQueuePredictable()
'''
    text = replace_once(text, old, new, "app test invocation")

    anchor = '''    @MainActor
    private static func testClearCompletedAndRetryFailedKeepQueuePredictable() throws {
'''
    if "testQueueStatusCountsMatchAllTerminalStates" in text and text.count("testQueueStatusCountsMatchAllTerminalStates") > 1:
        return text
    if anchor not in text:
        raise RuntimeError("app queue test insertion anchor missing")
    test = '''    @MainActor
    private static func testQueueStatusCountsMatchAllTerminalStates() throws {
        let root = FileManager.default.temporaryDirectory
        let statuses: [ConversionQueueStatus] = [
            .pending, .running, .converted, .converted,
            .skippedExisting, .failed, .cancelled,
        ]
        let viewModel = XDRemuxViewModel()
        viewModel.queue = statuses.enumerated().map { index, status in
            ConversionQueueItem(
                inputURL: root.appendingPathComponent("input-\\(index).heic"),
                outputURL: root.appendingPathComponent("output-\\(index).heic"),
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

'''
    return text.replace(anchor, test + anchor, 1)


update(
    "Sources/XDRemuxAppleFeatures/PhotographicStyles/ConstrainedPolynomialStyleDataProducer.swift",
    update_style,
)
update("Sources/XDRemuxCore/HEIF/DirectTiledHEVCGainMapEncoder.swift", update_encoder)
update("apps/macos/XDRemuxApp/Sources/XDRemuxViewModel.swift", update_view_model)
update("apps/macos/XDRemuxApp/Sources/ContentView.swift", update_content_view)
update("apps/macos/XDRemuxApp/Tests/XDRemuxAppModelTests.swift", update_app_tests)

# Extend the static architecture guard to cover the UI/control-plane allocations removed here.
arch = ROOT / "Tests" / "test_performance_design.py"
text = arch.read_text(encoding="utf-8")
if "test_app_queue_status_projection_avoids_filter_arrays" not in text:
    insertion = '''
    def test_app_queue_status_projection_avoids_filter_arrays(self) -> None:
        source = self.source("apps/macos/XDRemuxApp/Sources/XDRemuxViewModel.swift")
        self.assertNotIn("queue.filter { $0.status", source)
        self.assertNotIn("queue.firstIndex(where: { $0.id == item.id })", source)
        self.assertIn("struct ConversionQueueStatusCounts", source)
        self.assertIn("queueIndexByID.reserveCapacity(queue.count)", source)

'''
    marker = '\n\nif __name__ == "__main__":\n'
    if marker not in text:
        raise RuntimeError("architecture test footer missing")
    arch.write_text(text.replace(marker, insertion + marker, 1), encoding="utf-8")

print("performance-by-design phase3 applied")
