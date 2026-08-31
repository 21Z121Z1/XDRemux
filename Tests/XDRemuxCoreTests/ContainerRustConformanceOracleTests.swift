import Foundation
import XCTest
@testable import XDRemuxCore

final class ContainerRustConformanceOracleTests: XCTestCase {
    private func repositoryRoot() -> URL {
        URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
    }

    private func hex(_ data: Data) -> String {
        data.map { String(format: "%02x", $0) }.joined()
    }

    private func bits(_ value: Double) -> String {
        String(format: "%016llx", value.bitPattern)
    }

    private func resetDirectory(_ url: URL, fileManager: FileManager) throws {
        if fileManager.fileExists(atPath: url.path) {
            try fileManager.removeItem(at: url)
        }
        try fileManager.createDirectory(at: url, withIntermediateDirectories: true)
    }

    private func writeSnapshot(
        _ snapshot: ContainerConformanceSnapshot,
        to outputURL: URL,
        fileManager: FileManager
    ) throws {
        try resetDirectory(outputURL, fileManager: fileManager)
        try snapshot.metaBytes.write(
            to: outputURL.appendingPathComponent("meta.bin"),
            options: .atomic
        )
        try snapshot.maskJPEGData.write(
            to: outputURL.appendingPathComponent("mask.bin"),
            options: .atomic
        )

        var lines: [String] = []
        lines.append("mode\t\(snapshot.mode)")
        lines.append("data-base\t\(snapshot.dataBase)")
        lines.append(
            "manifest\t\(snapshot.extensionStart)\t\(snapshot.jsonStart)\t\(snapshot.jsonEnd)"
        )
        if let local = snapshot.localHDRInfo {
            lines.append(
                "local-hdr\t\(bits(local.version))\t\(bits(local.length))\t\(bits(local.metaSize))\t\(bits(local.offset))"
            )
        } else {
            lines.append("local-hdr\tnone")
        }
        lines.append("meta-floats\t\(snapshot.metaFloats.map(bits).joined(separator: ","))")

        for entry in snapshot.entries {
            guard let nameData = entry.name.data(using: .utf8) else {
                XCTFail("manifest entry name is not UTF-8")
                continue
            }
            lines.append(
                "entry\t\(entry.jsonOrder)\t\(hex(nameData))\t\(entry.offset)\t\(entry.length)\t\(entry.start)\t\(entry.end)"
            )
        }

        for (index, pair) in snapshot.portraitBlocks.sorted(by: { $0.key < $1.key }).enumerated() {
            guard let nameData = pair.key.data(using: .utf8) else {
                XCTFail("block name is not UTF-8")
                continue
            }
            let filename = String(format: "block-%04d.bin", index)
            try pair.value.write(
                to: outputURL.appendingPathComponent(filename),
                options: .atomic
            )
            lines.append("block\t\(index)\t\(hex(nameData))\t\(pair.value.count)")
        }

        let summary = lines.joined(separator: "\n") + "\n"
        try Data(summary.utf8).write(
            to: outputURL.appendingPathComponent("summary.tsv"),
            options: .atomic
        )
    }

    private func flattenedError(_ error: Error) -> String {
        String(describing: error)
            .replacingOccurrences(of: "\t", with: " ")
            .replacingOccurrences(of: "\r", with: " ")
            .replacingOccurrences(of: "\n", with: " ")
    }

    func testEmitRepositoryFixtureSnapshots() throws {
        guard let outputPath = ProcessInfo.processInfo.environment["XDREMUX_CONTAINER_ORACLE_ROOT"],
              !outputPath.isEmpty else {
            throw XCTSkip("set XDREMUX_CONTAINER_ORACLE_ROOT to emit Swift container fixture snapshots")
        }

        let fileManager = FileManager.default
        let outputRoot = URL(fileURLWithPath: outputPath, isDirectory: true)
        try resetDirectory(outputRoot, fileManager: fileManager)

        let repositoryRoot = repositoryRoot()
        let fixturesRoot = repositoryRoot.appendingPathComponent("fixtures", isDirectory: true)
        let supportedExtensions = Set(["jpg", "jpeg", "heic", "heif"])
        let candidates = try fileManager
            .contentsOfDirectory(
                at: fixturesRoot,
                includingPropertiesForKeys: nil,
                options: [.skipsHiddenFiles]
            )
            .filter { supportedExtensions.contains($0.pathExtension.lowercased()) }
            .sorted { $0.lastPathComponent < $1.lastPathComponent }

        XCTAssertFalse(candidates.isEmpty, "repository must expose candidate container fixtures")

        var acceptedLines: [String] = []
        var rejectedLines: [String] = []
        var acceptedCount = 0
        var lhdrCount = 0
        var uhdrCount = 0

        for fixtureURL in candidates {
            let relativePath = "fixtures/\(fixtureURL.lastPathComponent)"
            do {
                let data = try Data(contentsOf: fixtureURL, options: [.mappedIfSafe])
                let snapshot = try ContainerConformanceSupport.snapshot(from: data)
                let snapshotName = String(format: "fixture-%04d", acceptedCount)
                let snapshotURL = outputRoot.appendingPathComponent(snapshotName, isDirectory: true)
                try writeSnapshot(snapshot, to: snapshotURL, fileManager: fileManager)
                acceptedLines.append("\(snapshotName)\t\(relativePath)\t\(snapshot.mode)")
                acceptedCount += 1
                switch snapshot.mode {
                case "lhdr": lhdrCount += 1
                case "uhdr": uhdrCount += 1
                default: XCTFail("unexpected extraction mode \(snapshot.mode) for \(relativePath)")
                }
            } catch {
                rejectedLines.append("\(relativePath)\t\(flattenedError(error))")
            }
        }

        let acceptedText = acceptedLines.isEmpty ? "" : acceptedLines.joined(separator: "\n") + "\n"
        let rejectedText = rejectedLines.isEmpty ? "" : rejectedLines.joined(separator: "\n") + "\n"
        try Data(acceptedText.utf8).write(
            to: outputRoot.appendingPathComponent("accepted.tsv"),
            options: .atomic
        )
        try Data(rejectedText.utf8).write(
            to: outputRoot.appendingPathComponent("rejected.tsv"),
            options: .atomic
        )

        print(
            "Swift container fixture oracle: candidates=\(candidates.count) accepted=\(acceptedCount) lhdr=\(lhdrCount) uhdr=\(uhdrCount) rejected=\(rejectedLines.count)"
        )
        if !rejectedLines.isEmpty {
            print("Swift container rejected fixtures:\n\(rejectedLines.joined(separator: "\n"))")
        }

        XCTAssertGreaterThanOrEqual(
            acceptedCount,
            2,
            "container conformance requires at least two repository fixtures accepted by current Swift"
        )
    }
}
