import Foundation
import Observation
import XDRemuxCore
import AppKit

enum PhotoCategorizationAppState: Equatable {
    case idle
    case scanning
    case ready
    case copying
    case completed
    case cancelled
    case failed(String)
}

private final class CategorizationCancellationToken: @unchecked Sendable {
    private let lock = NSLock()
    private var cancelled = false

    func cancel() {
        lock.lock()
        cancelled = true
        lock.unlock()
    }

    var isCancelled: Bool {
        lock.lock()
        defer { lock.unlock() }
        return cancelled
    }
}

@Observable
@MainActor
final class PhotoCategorizationViewModel {
    var inputURLs: [URL] = []
    var outputDirectory: URL?
    var items: [PhotoCategorizationItem] = []
    var state: PhotoCategorizationAppState = .idle
    var completedCount = 0

    var canScan: Bool { !inputURLs.isEmpty && !isBusy }
    var canCopy: Bool { state == .ready && items.contains { $0.disposition == .copy } }
    var isBusy: Bool { state == .scanning || state == .copying }
    var categorizedCount: Int { items.count { $0.classification.mode != nil } }
    var rootCount: Int { items.count { $0.classification.mode == nil } }
    var duplicateCount: Int { items.count { $0.disposition == .duplicate } }
    var failedCount: Int { items.count { $0.disposition == .failed } }
    var modeSummary: String {
        let counts = Dictionary(grouping: items.compactMap(\.classification.mode), by: { $0 })
            .mapValues(\.count)
        return OppoCaptureMode.allCases.compactMap { mode in
            counts[mode].map { "\(mode.folderName) \($0)" }
        }.joined(separator: " · ")
    }

    private var task: Task<Void, Never>?
    private var cancellationToken: CategorizationCancellationToken?

    func addInputs(_ urls: [URL]) {
        guard !isBusy else { return }
        var existing = Set(inputURLs.map { $0.standardizedFileURL.path })
        for url in urls where existing.insert(url.standardizedFileURL.path).inserted {
            inputURLs.append(url)
        }
        items.removeAll()
        state = .idle
    }

    func clear() {
        guard !isBusy else { return }
        inputURLs.removeAll()
        outputDirectory = nil
        items.removeAll()
        completedCount = 0
        state = .idle
    }

    func scan() {
        guard canScan else { return }
        task?.cancel()
        state = .scanning
        completedCount = 0
        let inputs = inputURLs
        let outputDirectory = outputDirectory
        task = Task { [weak self] in
            do {
                let plan = try await Task.detached(priority: .userInitiated) {
                    try PhotoCategorizationEngine.makePlan(
                        inputs: inputs,
                        outputDirectory: outputDirectory
                    )
                }.value
                guard !Task.isCancelled else { return }
                self?.items = plan.items
                self?.state = .ready
            } catch {
                self?.state = .failed(String(describing: error))
            }
        }
    }

    func copyPlannedFiles() {
        guard canCopy else { return }
        state = .copying
        completedCount = 0
        let token = CategorizationCancellationToken()
        cancellationToken = token
        let planItems = items
        let outputDirectory = outputDirectory
        task = Task { [weak self] in
            let results = await Task.detached(priority: .userInitiated) {
                var completed: [PhotoCategorizationItem] = []
                for item in planItems {
                    if token.isCancelled { break }
                    let plan = PhotoCategorizationPlan(outputDirectory: outputDirectory, items: [item])
                    completed.append(contentsOf: PhotoCategorizationEngine.execute(plan, jobs: 1).items)
                }
                return completed
            }.value
            guard let self else { return }
            let completedByID = Dictionary(uniqueKeysWithValues: results.map { ($0.id, $0) })
            items = items.map { completedByID[$0.id] ?? $0 }
            completedCount = results.count
            if token.isCancelled {
                state = .cancelled
            } else if items.contains(where: { $0.disposition == .failed }) {
                state = .failed("部分文件复制失败")
            } else {
                state = .completed
            }
            cancellationToken = nil
        }
    }

    func cancel() {
        cancellationToken?.cancel()
        task?.cancel()
        if isBusy { state = .cancelled }
    }

    func revealResults() {
        let urls = items
            .filter { $0.disposition == .copied || $0.disposition == .duplicate }
            .map(\.destinationURL)
        guard !urls.isEmpty else { return }
        NSWorkspace.shared.activateFileViewerSelecting(urls)
    }
}
