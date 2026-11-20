import Foundation
import Observation
import AppKit

enum PhotoCategorizationAppState: Equatable {
    case idle
    case scanning
    case ready
    case copying
    case cancelling
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
    private(set) var inputURLs: [URL] = []
    var outputDirectory: URL? {
        didSet {
            if outputDirectory != oldValue, state == .ready {
                preview.removeAll()
                items.removeAll()
                completedCount = 0
                state = .idle
            }
        }
    }
    var items: [PhotoCategorizationItem] = []
    var state: PhotoCategorizationAppState = .idle
    var completedCount = 0

    var canScan: Bool { !inputURLs.isEmpty && !isBusy }
    var canCopy: Bool { state == .ready && !preview.isEmpty && items.contains { $0.disposition == .dryRun } }
    var isBusy: Bool { operation != nil }
    var canCancel: Bool { state == .scanning || state == .copying }
    var categorizedCount: Int { items.count { $0.classification.mode != nil } }
    var unclassifiedCount: Int { items.count { $0.classification.mode == nil } }
    var rootCount: Int { unclassifiedCount } // compatibility for older UI/tests
    var duplicateCount: Int { items.count { $0.disposition == .duplicate } }
    var failedCount: Int { items.count { $0.disposition == .failed } }
    var modeSummary: String {
        let counts = Dictionary(grouping: items.compactMap(\.classification.mode), by: { $0 })
            .mapValues(\.count)
        return OppoCaptureMode.allCases.compactMap { mode in
            counts[mode].map { "\(mode.folderName) \($0)" }
        }.joined(separator: " · ")
    }

    private var operation: CategorizationCancellationToken?
    private var preview: [PlannedRoot] = []

    func addInputs(_ urls: [URL]) {
        guard !isBusy else { return }
        var existing = Set(inputURLs.map { $0.standardizedFileURL.path })
        for url in urls where existing.insert(url.standardizedFileURL.path).inserted {
            inputURLs.append(url)
        }
        preview.removeAll()
        items.removeAll()
        state = .idle
    }

    func clear() {
        guard !isBusy else { return }
        inputURLs.removeAll()
        outputDirectory = nil
        preview.removeAll()
        items.removeAll()
        completedCount = 0
        state = .idle
    }

    func scan() {
        guard canScan else { return }
        let token = CategorizationCancellationToken()
        operation = token
        state = .scanning
        preview.removeAll()
        completedCount = 0
        let inputs = inputURLs
        let destination = outputDirectory
        Task { [weak self] in
            do {
                let snapshot = try await Task.detached(priority: .userInitiated) {
                    try Self.planCategorization(
                        roots: Self.categorizationRoots(inputs: inputs, outputDirectory: destination),
                        token: token
                    )
                }.value
                guard let self, operation === token else { return }
                operation = nil
                if token.isCancelled {
                    state = .cancelled
                } else if outputDirectory != destination {
                    items.removeAll()
                    state = .idle
                } else {
                    preview = snapshot.roots
                    items = snapshot.items
                    state = .ready
                }
            } catch {
                guard let self, operation === token else { return }
                operation = nil
                state = token.isCancelled ? .cancelled : .failed(String(describing: error))
            }
        }
    }

    func copyPlannedFiles() {
        guard canCopy else { return }
        let token = CategorizationCancellationToken()
        operation = token
        state = .copying
        completedCount = 0
        let plan = preview
        Task { [weak self] in
            let result = await Task.detached(priority: .userInitiated) {
                Self.copy(roots: plan, token: token)
            }.value
            guard let self, operation === token else { return }
            operation = nil
            let completedByID = Dictionary(uniqueKeysWithValues: result.items.map { ($0.id, $0) })
            items = items.map { completedByID[$0.id] ?? $0 }
            completedCount = result.items.count
            if token.isCancelled {
                state = .cancelled
            } else if let error = result.error {
                state = .failed(error)
            } else if items.contains(where: { $0.disposition == .failed }) {
                state = .failed("部分文件复制失败")
            } else {
                state = .completed
            }
        }
    }

    private struct PlannedRoot: Sendable {
        let root: CategorizationRoot
        let planID: String
    }

    nonisolated private static func planCategorization(
        roots: [CategorizationRoot],
        token: CategorizationCancellationToken
    ) throws -> (roots: [PlannedRoot], items: [PhotoCategorizationItem]) {
        var plans: [PlannedRoot] = []
        var items: [PhotoCategorizationItem] = []
        var seen = Set<String>()
        for root in roots {
            if token.isCancelled { break }
            let result = try RustCLIClient.categorize(
                inputs: root.inputs, outputDirectory: root.outputDirectory, dryRun: true
            )
            guard let planID = result.planID else {
                throw RustCLIClient.ClientError.unsupportedConfiguration("分类预览缺少计划标识。")
            }
            plans.append(PlannedRoot(root: root, planID: planID))
            items.append(contentsOf: result.items.filter { seen.insert($0.id).inserted })
        }
        return (plans, items)
    }

    nonisolated private static func copy(
        roots: [PlannedRoot],
        token: CategorizationCancellationToken
    ) -> (items: [PhotoCategorizationItem], error: String?) {
        var items: [PhotoCategorizationItem] = []
        var seen = Set<String>()
        for plan in roots {
            if token.isCancelled { break }
            do {
                let result = try RustCLIClient.categorize(
                    inputs: plan.root.inputs, outputDirectory: plan.root.outputDirectory,
                    dryRun: false, expectedPlan: plan.planID
                )
                items.append(contentsOf: result.items.filter { seen.insert($0.id).inserted })
            } catch {
                return (items, String(describing: error))
            }
        }
        return (items, nil)
    }

    private struct CategorizationRoot: Sendable {
        let inputs: [URL]
        let outputDirectory: URL
    }

    nonisolated private static func categorizationRoots(
        inputs: [URL],
        outputDirectory: URL?
    ) -> [CategorizationRoot] {
        if let outputDirectory {
            return [CategorizationRoot(inputs: inputs, outputDirectory: outputDirectory)]
        }
        var grouped: [String: [URL]] = [:]
        for input in inputs {
            let isDirectory = (try? input.resourceValues(forKeys: [.isDirectoryKey]).isDirectory) == true
            let root = isDirectory ? input : input.deletingLastPathComponent()
            grouped[root.standardizedFileURL.path, default: []].append(input)
        }
        return grouped.keys.sorted().compactMap { key in
            guard let groupedInputs = grouped[key] else { return nil }
            return CategorizationRoot(
                inputs: groupedInputs,
                outputDirectory: URL(fileURLWithPath: key)
            )
        }
    }

    func cancel() {
        guard canCancel, let operation else { return }
        state = .cancelling
        operation.cancel()
    }

    func revealResults() {
        let urls = items
            .filter { $0.disposition == .copied || $0.disposition == .duplicate }
            .map(\.destinationURL)
        guard !urls.isEmpty else { return }
        NSWorkspace.shared.activateFileViewerSelecting(urls)
    }
}
