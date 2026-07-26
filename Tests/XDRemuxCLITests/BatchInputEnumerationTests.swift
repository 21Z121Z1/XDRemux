import Foundation
import XCTest
import XDRemuxCore
@testable import XDRemuxCLI

/// `batch` must never re-enumerate its own results. Before these contracts a
/// second `--categorize` run over the same directory picked up the converted
/// files from the first run and failed every one of them.
final class BatchInputEnumerationTests: XCTestCase {
    private var root: URL!

    override func setUpWithError() throws {
        root = URL(fileURLWithPath: NSTemporaryDirectory(), isDirectory: true)
            .appendingPathComponent("xdremux-enumerate-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    }

    override func tearDownWithError() throws {
        try? FileManager.default.removeItem(at: root)
    }

    private func touch(_ relativePath: String) throws -> URL {
        let url = root.appendingPathComponent(relativePath)
        try FileManager.default.createDirectory(
            at: url.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        XCTAssertTrue(FileManager.default.createFile(atPath: url.path, contents: Data([0, 1, 2, 3])))
        return url
    }

    /// Paths are reported relative to the fixture root. The absolute prefix is
    /// unstable — the temporary directory is reached through the /var symlink
    /// and Foundation spells it inconsistently — so anchor on the unique root
    /// directory name instead.
    private func names(_ urls: [URL]) -> [String] {
        let marker = "/\(root.lastPathComponent)/"
        return urls.map { url -> String in
            guard let range = url.path.range(of: marker) else { return url.path }
            return String(url.path[range.upperBound...])
        }.sorted()
    }

    func testCategorizedRunSkipsCaptureModeOutputFolders() throws {
        _ = try touch("a.heic")
        _ = try touch("b.heic")
        _ = try touch("人像/a.heic")
        _ = try touch("普通拍照/b.heic")

        let matched = try XDRemuxCommand.enumerateInputs(
            root: root,
            glob: "*.heic",
            excluding: root,
            categorized: true
        )

        XCTAssertEqual(names(matched), ["a.heic", "b.heic"])
    }

    func testUncategorizedRunStillSeesShootingModeNamedFolders() throws {
        _ = try touch("a.heic")
        _ = try touch("人像/keep.heic")

        let matched = try XDRemuxCommand.enumerateInputs(
            root: root,
            glob: "*.heic",
            excluding: root,
            categorized: false
        )

        XCTAssertEqual(names(matched), ["a.heic", "人像/keep.heic"])
    }

    func testNestedOutputDirectoryIsExcludedFromInputs() throws {
        _ = try touch("a.heic")
        _ = try touch("out/a.heic")
        _ = try touch("out/人像/a.heic")

        let matched = try XDRemuxCommand.enumerateInputs(
            root: root,
            glob: "*.heic",
            excluding: root.appendingPathComponent("out", isDirectory: true),
            categorized: true
        )

        XCTAssertEqual(names(matched), ["a.heic"])
    }

    func testOutputDirectoryOutsideRootExcludesNothing() throws {
        _ = try touch("a.heic")
        _ = try touch("nested/b.heic")

        let matched = try XDRemuxCommand.enumerateInputs(
            root: root,
            glob: "*.heic",
            excluding: URL(fileURLWithPath: NSTemporaryDirectory(), isDirectory: true)
                .appendingPathComponent("xdremux-elsewhere-\(UUID().uuidString)", isDirectory: true),
            categorized: true
        )

        XCTAssertEqual(names(matched), ["a.heic", "nested/b.heic"])
    }
}
