import Foundation
import XCTest
@testable import XDRemuxCore

final class ContainerRustConformanceOracleTests: XCTestCase {
    private struct NamedBlock {
        let name: String
        let data: Data
    }

    private struct CorpusCase {
        let name: String
        let data: Data
        let expectedMode: String
        let expectedDataBaseDelta: Int
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

    private func appendUInt32BE(_ value: UInt32, to data: inout Data) {
        data.append(UInt8((value >> 24) & 0xff))
        data.append(UInt8((value >> 16) & 0xff))
        data.append(UInt8((value >> 8) & 0xff))
        data.append(UInt8(value & 0xff))
    }

    private func appendUInt32LE(_ value: UInt32, to data: inout Data) {
        data.append(UInt8(value & 0xff))
        data.append(UInt8((value >> 8) & 0xff))
        data.append(UInt8((value >> 16) & 0xff))
        data.append(UInt8((value >> 24) & 0xff))
    }

    private func packFloat32LE(_ values: [Float]) -> Data {
        var result = Data()
        result.reserveCapacity(values.count * MemoryLayout<UInt32>.size)
        for value in values {
            var bits = value.bitPattern.littleEndian
            withUnsafeBytes(of: &bits) { result.append(contentsOf: $0) }
        }
        return result
    }

    private func lhdrMetadata() -> Data {
        var values = Array(repeating: Float(0), count: 36)
        values[0] = 3.0
        values[1] = 1.0
        values[2] = 144.0
        values[3] = 0.0
        values[4] = 6.25
        values[5] = -1.0
        values[7] = 1.0
        values[8] = 0.125
        values[9] = 0.75
        values[16] = 1.0
        values[18] = 10.0
        values[19] = 6.0
        values[23] = 0.8
        values[24] = 0.25
        values[29] = 4.0
        values[32] = 2.0
        values[33] = 4.5
        values[34] = 1.0
        return packFloat32LE(values)
    }

    private func validUHDRMetadata() -> Data {
        packFloat32LE([
            1.0, 1.05, 0.95, 1.0,
            4.0, 4.5, 5.0,
            1.0, 0.9, 1.1,
            0.01, 0.02, 0.03,
            0.04, 0.05, 0.06,
            1.0, 4.5, 4.5, 0.0,
        ])
    }

    private func qtiPrefix(marker: String) -> Data {
        let markerData = Data(marker.utf8)
        var result = Data()
        appendUInt32BE(UInt32(4 + markerData.count), to: &result)
        result.append(markerData)
        return result
    }

    private func manifestJSON(
        for blocks: [NamedBlock],
        preludeLength: Int,
        dataAreaLength: Int
    ) -> Data {
        var start = preludeLength
        let objects = blocks.map { block -> String in
            // Current OPPO manifests are consumed in two coordinate systems:
            // blockStart first tries jsonStart - offset, while data-base
            // calibration uses start = offset - length. Model the former exactly
            // here; corpus tail padding is chosen so the calibration anchor also
            // resolves to the actual data base.
            let offset = dataAreaLength - start
            let object = "{\"name\":\"\(block.name)\",\"offset\":\(offset),\"length\":\(block.data.count)}"
            start += block.data.count
            return object
        }
        return Data(("[" + objects.joined(separator: ",") + "]").utf8)
    }

    private func qtiContainer(
        marker: String,
        padding: Int,
        prelude: Data = Data(),
        blocks: [NamedBlock],
        tailPadding: Int
    ) -> Data {
        var result = qtiPrefix(marker: marker)
        result.append(Data(repeating: 0xa5, count: padding))
        result.append(prelude)
        for block in blocks {
            result.append(block.data)
        }
        result.append(Data(repeating: 0xcc, count: tailPadding))
        let dataAreaLength = prelude.count + blocks.reduce(0) { $0 + $1.data.count } + tailPadding
        result.append(
            manifestJSON(
                for: blocks,
                preludeLength: prelude.count,
                dataAreaLength: dataAreaLength
            )
        )
        return result
    }

    private func jxrsContainer(
        prefix: Data,
        prelude: Data = Data(),
        blocks: [NamedBlock],
        tailPadding: Int
    ) -> Data {
        var result = prefix
        result.append(prelude)
        for block in blocks {
            result.append(block.data)
        }
        result.append(Data(repeating: 0xdd, count: tailPadding))
        let dataAreaLength = prelude.count + blocks.reduce(0) { $0 + $1.data.count } + tailPadding
        let json = manifestJSON(
            for: blocks,
            preludeLength: prelude.count,
            dataAreaLength: dataAreaLength
        )
        result.append(json)
        result.append(0)
        result.append(Data("jxrs".utf8))
        appendUInt32LE(UInt32(json.count + 9), to: &result)
        return result
    }

    private func corpusCases() -> [CorpusCase] {
        let meta = lhdrMetadata()
        let maskA = Data([0xff, 0xd8, 0xff, 0xe0, 0x00, 0x02, 0x11, 0x22, 0xff, 0xd9])
        let maskB = Data([0xff, 0xd8, 0xff, 0xe1, 0x00, 0x03, 0x33, 0x44, 0x55, 0xff, 0xd9])
        let portrait = Data([0x50, 0x4f, 0x52, 0x54, 0x52, 0x41, 0x49, 0x54])

        return [
            CorpusCase(
                name: "lhdr-qti-calibrated",
                data: qtiContainer(
                    marker: "QTI Debug",
                    padding: 11,
                    blocks: [
                        NamedBlock(name: "local.hdr.meta.data", data: meta),
                        NamedBlock(name: "local.hdr.linear.mask", data: maskA),
                        NamedBlock(name: "portrait.depth", data: portrait),
                    ],
                    tailPadding: 136
                ),
                expectedMode: "lhdr",
                expectedDataBaseDelta: 11
            ),
            CorpusCase(
                name: "lhdr-qti-float144-fallback",
                data: qtiContainer(
                    marker: "QTI ",
                    padding: 7,
                    prelude: meta,
                    blocks: [
                        NamedBlock(name: "local.hdr.linear.mask", data: maskB),
                    ],
                    tailPadding: 144
                ),
                expectedMode: "lhdr",
                expectedDataBaseDelta: 7
            ),
            CorpusCase(
                name: "uhdr-jxrs-canonical-fallback",
                data: jxrsContainer(
                    prefix: Data([0x4a, 0x58, 0x52, 0x53, 0x10, 0x20]),
                    blocks: [
                        NamedBlock(name: "local.uhdr.gainmap.info", data: Data(count: 80)),
                        NamedBlock(name: "local.uhdr.gainmap.data", data: maskA),
                    ],
                    tailPadding: 80
                ),
                expectedMode: "uhdr",
                expectedDataBaseDelta: 0
            ),
            CorpusCase(
                name: "uhdr-qti-valid-multichannel",
                data: qtiContainer(
                    marker: "QTI Debug",
                    padding: 5,
                    blocks: [
                        NamedBlock(name: "local.uhdr.gainmap.info", data: validUHDRMetadata()),
                        NamedBlock(name: "local.uhdr.gainmap.data", data: maskB),
                    ],
                    tailPadding: 80
                ),
                expectedMode: "uhdr",
                expectedDataBaseDelta: 5
            ),
        ]
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

    func testEmitContainerConformanceCorpus() throws {
        guard let outputPath = ProcessInfo.processInfo.environment["XDREMUX_CONTAINER_ORACLE_ROOT"],
              !outputPath.isEmpty else {
            throw XCTSkip("set XDREMUX_CONTAINER_ORACLE_ROOT to emit Swift container conformance corpus")
        }

        let fileManager = FileManager.default
        let outputRoot = URL(fileURLWithPath: outputPath, isDirectory: true)
        try resetDirectory(outputRoot, fileManager: fileManager)
        let inputsRoot = outputRoot.appendingPathComponent("inputs", isDirectory: true)
        try fileManager.createDirectory(at: inputsRoot, withIntermediateDirectories: true)

        let cases = corpusCases()
        XCTAssertEqual(cases.count, 4)

        var caseLines: [String] = []
        var lhdrCount = 0
        var uhdrCount = 0

        for (index, item) in cases.enumerated() {
            let inputURL = inputsRoot.appendingPathComponent("\(item.name).bin")
            try item.data.write(to: inputURL, options: .atomic)

            let snapshot = try ContainerConformanceSupport.snapshot(from: item.data)
            XCTAssertEqual(snapshot.mode, item.expectedMode, item.name)
            XCTAssertEqual(
                snapshot.dataBase - snapshot.extensionStart,
                item.expectedDataBaseDelta,
                "data-base calibration drifted for \(item.name)"
            )

            if item.name == "lhdr-qti-float144-fallback" {
                XCTAssertEqual(snapshot.metaBytes, lhdrMetadata())
            }
            if item.name == "uhdr-jxrs-canonical-fallback" {
                XCTAssertEqual(snapshot.metaFloats.count, 20)
                XCTAssertEqual(snapshot.metaFloats[0], 1.0)
                XCTAssertEqual(snapshot.metaFloats[4], 4.926)
                XCTAssertFalse(snapshot.metaBytes.allSatisfy { $0 == 0 })
            }
            if item.name == "uhdr-qti-valid-multichannel" {
                XCTAssertEqual(snapshot.metaBytes, validUHDRMetadata())
                XCTAssertNotEqual(snapshot.metaFloats[4], snapshot.metaFloats[5])
                XCTAssertNotEqual(snapshot.metaFloats[5], snapshot.metaFloats[6])
            }

            let snapshotName = String(format: "fixture-%04d", index)
            try writeSnapshot(
                snapshot,
                to: outputRoot.appendingPathComponent(snapshotName, isDirectory: true),
                fileManager: fileManager
            )
            caseLines.append(
                "\(snapshotName)\t\(inputURL.path)\t\(snapshot.mode)\t\(item.name)"
            )

            switch snapshot.mode {
            case "lhdr": lhdrCount += 1
            case "uhdr": uhdrCount += 1
            default: XCTFail("unexpected extraction mode \(snapshot.mode) for \(item.name)")
            }
        }

        XCTAssertEqual(lhdrCount, 2)
        XCTAssertEqual(uhdrCount, 2)
        let caseText = caseLines.joined(separator: "\n") + "\n"
        try Data(caseText.utf8).write(
            to: outputRoot.appendingPathComponent("cases.tsv"),
            options: .atomic
        )
        print("Swift container conformance corpus: cases=\(cases.count) lhdr=\(lhdrCount) uhdr=\(uhdrCount)")
    }
}
