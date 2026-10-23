import Foundation
import Observation

struct HDRLookItem: Decodable, Identifiable, Sendable {
    let input: String
    let output: String
    let status: String
    let error: String?
    var id: String { input }
}

struct HDRLookReceipt: Decodable, Sendable {
    let outputDirectory: String
    let results: [HDRLookItem]
    let failures: Int

    enum CodingKeys: String, CodingKey {
        case outputDirectory = "output_dir"
        case results, failures
    }
}

@Observable
@MainActor
final class HDRLookViewModel {
    var inputURLs: [URL] = []
    var outputDirectory: URL?
    var headroom = 8
    private(set) var receipt: HDRLookReceipt?
    private(set) var isRunning = false
    private(set) var errorMessage: String?

    var canRun: Bool { !inputURLs.isEmpty && !isRunning }

    func addInputs(_ urls: [URL]) {
        guard !isRunning else { return }
        var seen = Set(inputURLs.map { $0.standardizedFileURL.path })
        inputURLs += urls.filter { $0.isFileURL && seen.insert($0.standardizedFileURL.path).inserted }
        receipt = nil
        errorMessage = nil
    }

    func clear() {
        guard !isRunning else { return }
        inputURLs = []
        outputDirectory = nil
        receipt = nil
        errorMessage = nil
    }

    func run() {
        guard canRun else { return }
        let inputs = inputURLs
        let output = outputDirectory
        let headroom = headroom
        isRunning = true
        receipt = nil
        errorMessage = nil
        Task { [weak self] in
            do {
                let receipt = try await Task.detached(priority: .userInitiated) {
                    try RustCLIClient.hdrLook(inputs: inputs, outputDirectory: output, headroom: headroom)
                }.value
                self?.receipt = receipt
            } catch {
                self?.errorMessage = String(describing: error)
            }
            self?.isRunning = false
        }
    }
}
