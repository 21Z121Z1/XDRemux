import Foundation
import XCTest
@testable import XDRemuxCore

final class HEIFRustConformanceTests: XCTestCase {
    private struct Tile {
        let payload: Data
        let width: Int
        let height: Int
    }

    private struct CorpusCase {
        let name: String
        let gainWidth: Int
        let gainHeight: Int
        let tileWidth: Int
        let tileHeight: Int
        let channelCount: Int
        let rotation: UInt8
        let tiles: [Tile]
    }

    private struct SourceFixture {
        let data: Data
        let primaryPayload: Data
    }

    private func hex(_ data: Data) -> String {
        data.map { String(format: "%02x", $0) }.joined()
    }

    private func validHVCC(channelCount: Int) -> Data {
        var result = Data(repeating: 0, count: 19)
        result[0] = 1
        result[1] = 4
        result[16] = channelCount == 1 ? 0 : 3
        return result
    }

    private func nclxColorBox() -> Data {
        makeBox("colr", payload: Data([
            0x6e, 0x63, 0x6c, 0x78,
            0x00, 0x02, 0x00, 0x02, 0x00, 0x02, 0x80,
        ]))
    }

    private func makeIPMA(_ entries: [(Int, [(Int, Bool)])]) throws -> Data {
        var payload = Data([0, 0, 0, 0])
        appendUInt32BE(entries.count, to: &payload)
        for (itemID, associations) in entries {
            payload.append(try makeIPMAEntry(itemID, associations, flags: 0, version: 0))
        }
        return makeBox("ipma", payload: payload)
    }

    private func makeILoc(
        _ entries: [(itemID: Int, method: Int, offset: Int, length: Int)]
    ) -> Data {
        var payload = Data([1, 0, 0, 0, 0x44, 0x00])
        appendUInt16BE(entries.count, to: &payload)
        for entry in entries {
            appendUInt16BE(entry.itemID, to: &payload)
            appendUInt16BE(entry.method, to: &payload)
            appendUInt16BE(0, to: &payload)
            appendUInt16BE(1, to: &payload)
            appendUInt32BE(entry.offset, to: &payload)
            appendUInt32BE(entry.length, to: &payload)
        }
        return makeBox("iloc", payload: payload)
    }

    private func makeIRef() -> Data {
        var payload = Data([0, 0, 0, 0])
        payload.append(makeIrefBox(type: "cdsc", from: 3, to: [1], version: 0))
        payload.append(makeIrefBox(type: "dimg", from: 5, to: [1, 4], version: 0))
        payload.append(makeIrefBox(type: "auxl", from: 4, to: [1, 5], version: 0))
        payload.append(makeIrefBox(type: "cdsc", from: 6, to: [1, 5], version: 0))
        return makeBox("iref", payload: payload)
    }

    private func buildSource(rotation: UInt8) throws -> SourceFixture {
        let primaryPayload = Data([0x50, 0x52, 0x49, 0x4d, 0x41, 0x52, 0x59, 0x01, 0x02])
        let exifPayload = Data([0x45, 0x58, 0x49, 0x46, 0x11, 0x22, 0x33])
        let jpegPayload = Data([0xff, 0xd8, 0x11, 0x22, 0x33, 0xff, 0xd9])
        let idatPrefix = Data([0xde, 0xad, 0xbe, 0xef])
        let tmapPayload = Data((0..<62).map { UInt8(($0 * 17 + 3) & 0xff) })
        let xmpPayload = Data("old-generated-hdrgm-xmp".utf8)

        let primaryIspe = makeIspeBox(width: 8, height: 6)
        let primaryIrot = makeIrotBox(rotation)
        let primaryPixi = makePixiBox(bits: [8, 8, 8])
        let primaryColor = nclxColorBox()
        let primaryHVCC = makeBox("hvcC", payload: validHVCC(channelCount: 3))
        let tmapColor = nclxColorBox()
        let tmapPixi = makePixiBox(bits: [10, 10, 10])

        var ipcoPayload = Data()
        for property in [
            primaryIspe, primaryIrot, primaryPixi, primaryColor,
            primaryHVCC, tmapColor, tmapPixi,
        ] {
            ipcoPayload.append(property)
        }
        let ipco = makeBox("ipco", payload: ipcoPayload)
        let ipma = try makeIPMA([
            (1, [(1, true), (2, true), (3, true), (4, true), (5, true)]),
            (5, [(6, true), (7, true)]),
        ])
        let iprp = makeBox("iprp", payload: ipco + ipma)

        let iinf = makeIinfBox(version: 0, rawInfes: [
            makeInfeBox(itemID: 1, type: "grid"),
            makeInfeBox(itemID: 3, type: "Exif"),
            makeInfeBox(itemID: 4, type: "jpeg", flags: 1),
            makeInfeBox(itemID: 5, type: "tmap"),
            makeMimeInfeBox(itemID: 6),
        ])
        let pitm = makePitmBox(version: 0, primaryID: 1)
        let iref = makeIRef()
        let idat = makeBox("idat", payload: idatPrefix + tmapPayload + xmpPayload)
        let grpl = makeGrplAltrBox(groupID: 50, tmapID: 5, primaryID: 1)
        let ftyp = makeBox(
            "ftyp",
            payload: Data("heic".utf8) + Data([0, 0, 0, 0]) + Data("heicmif1".utf8)
        )
        let between = makeBox("free", payload: Data([0xca, 0xfe, 0xba, 0xbe]))

        func meta(iloc: Data) -> Data {
            var payload = Data([0, 0, 0, 0])
            for part in [pitm, iinf, iloc, iprp, iref, idat, grpl] {
                payload.append(part)
            }
            return makeBox("meta", payload: payload)
        }

        let tmapOffset = idatPrefix.count
        let xmpOffset = tmapOffset + tmapPayload.count
        let placeholder = makeILoc([
            (1, 0, 0, primaryPayload.count),
            (3, 0, 0, exifPayload.count),
            (4, 0, 0, jpegPayload.count),
            (5, 1, tmapOffset, tmapPayload.count),
            (6, 1, xmpOffset, xmpPayload.count),
        ])
        let preliminaryMeta = meta(iloc: placeholder)
        let mdatDataStart = ftyp.count + preliminaryMeta.count + between.count + 8
        let primaryOffset = mdatDataStart
        let exifOffset = primaryOffset + primaryPayload.count
        let jpegOffset = exifOffset + exifPayload.count
        let finalILoc = makeILoc([
            (1, 0, primaryOffset, primaryPayload.count),
            (3, 0, exifOffset, exifPayload.count),
            (4, 0, jpegOffset, jpegPayload.count),
            (5, 1, tmapOffset, tmapPayload.count),
            (6, 1, xmpOffset, xmpPayload.count),
        ])
        let finalMeta = meta(iloc: finalILoc)
        XCTAssertEqual(finalMeta.count, preliminaryMeta.count)

        let mdat = makeBox("mdat", payload: primaryPayload + exifPayload + jpegPayload)
        return SourceFixture(
            data: ftyp + finalMeta + between + mdat,
            primaryPayload: primaryPayload
        )
    }

    private func runRust(
        sourceURL: URL,
        outputURL: URL,
        corpusCase: CorpusCase,
        hvcc: Data
    ) throws {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
        var arguments = [
            "cargo", "run", "--quiet", "--locked", "-p", "xdremux-heif",
            "--example", "heif_conformance", "--",
            sourceURL.path,
            outputURL.path,
            String(corpusCase.gainWidth),
            String(corpusCase.gainHeight),
            String(corpusCase.tileWidth),
            String(corpusCase.tileHeight),
            String(corpusCase.channelCount),
            hex(hvcc),
        ]
        arguments.append(contentsOf: corpusCase.tiles.map {
            "\(hex($0.payload)):\($0.width):\($0.height)"
        })
        process.arguments = arguments
        process.currentDirectoryURL = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = pipe
        try process.run()
        process.waitUntilExit()
        let log = String(
            data: pipe.fileHandleForReading.readDataToEndOfFile(),
            encoding: .utf8
        ) ?? ""
        XCTAssertEqual(process.terminationStatus, 0, "Rust writer failed:\n\(log)")
    }

    private func verifySemanticContract(
        output: Data,
        primaryPayload: Data,
        tileCount: Int
    ) throws {
        let top = isobmffBoxes(in: output, start: 0, end: output.count)
        guard let meta = top.first(where: { $0.type == "meta" }) else {
            return XCTFail("output meta missing")
        }
        let children = isobmffBoxes(
            in: output,
            start: meta.dataStart + 4,
            end: meta.dataEnd
        )
        guard let pitm = children.first(where: { $0.type == "pitm" }),
              let iinf = children.first(where: { $0.type == "iinf" }),
              let iloc = children.first(where: { $0.type == "iloc" }),
              let iref = children.first(where: { $0.type == "iref" }),
              let idat = children.first(where: { $0.type == "idat" }) else {
            return XCTFail("output required meta child missing")
        }
        XCTAssertEqual(try parseISOBMFFPITM(output, pitm), 1)
        let items = parseISOBMFFItemInfos(output, iinf).items
        XCTAssertFalse(items.contains(where: { $0.type == "jpeg" }))
        let expectedGridID = 3 + tileCount + 1
        let expectedTmapID = expectedGridID + 1
        XCTAssertTrue(items.contains(where: { $0.itemID == expectedGridID && $0.type == "grid" }))
        XCTAssertTrue(items.contains(where: { $0.itemID == expectedTmapID && $0.type == "tmap" }))

        let refs = parseISOBMFFIRefs(output, iref).refs
        XCTAssertTrue(refs.contains(where: {
            $0.type == "dimg" && $0.from == expectedTmapID && $0.to == [1, expectedGridID]
        }))
        XCTAssertTrue(refs.contains(where: {
            $0.type == "cdsc" && $0.from == 3 && $0.to.contains(expectedTmapID)
        }))

        let locations = try parseISOBMFFILoc(output, iloc)
        guard let primary = locations.first(where: { $0.itemID == 1 }) else {
            return XCTFail("primary iloc missing")
        }
        XCTAssertEqual(try itemPayload(in: output, entry: primary, idat: idat), primaryPayload)
    }

    func testSwiftAndRustDirectHEVCWriterAreByteExact() throws {
        let cases = [
            CorpusCase(
                name: "rgb-edge-odd-rotation",
                gainWidth: 5,
                gainHeight: 3,
                tileWidth: 4,
                tileHeight: 2,
                channelCount: 3,
                rotation: 1,
                tiles: [
                    Tile(payload: Data([0x01, 0x02, 0x03]), width: 4, height: 2),
                    Tile(payload: Data([0x11, 0x12]), width: 1, height: 2),
                    Tile(payload: Data([0x21, 0x22, 0x23, 0x24]), width: 4, height: 1),
                    Tile(payload: Data([0x31]), width: 1, height: 1),
                ]
            ),
            CorpusCase(
                name: "mono-even-rotation",
                gainWidth: 4,
                gainHeight: 4,
                tileWidth: 4,
                tileHeight: 4,
                channelCount: 1,
                rotation: 2,
                tiles: [
                    Tile(payload: Data([0x41, 0x42, 0x43, 0x44]), width: 4, height: 4),
                ]
            ),
        ]

        let fileManager = FileManager.default
        let root = fileManager.temporaryDirectory
            .appendingPathComponent("xdremux-heif-conformance-\(UUID().uuidString)", isDirectory: true)
        try fileManager.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? fileManager.removeItem(at: root) }

        for corpusCase in cases {
            let source = try buildSource(rotation: corpusCase.rotation)
            let sourceURL = root.appendingPathComponent("\(corpusCase.name)-source.heic")
            let swiftURL = root.appendingPathComponent("\(corpusCase.name)-swift.heic")
            let rustURL = root.appendingPathComponent("\(corpusCase.name)-rust.heic")
            try source.data.write(to: sourceURL)
            let hvcc = validHVCC(channelCount: corpusCase.channelCount)

            try replacePrivateJPEGGainMapWithHEVCTiles(
                inputURL: sourceURL,
                outputURL: swiftURL,
                gainMapWidth: corpusCase.gainWidth,
                gainMapHeight: corpusCase.gainHeight,
                tileWidth: corpusCase.tileWidth,
                tileHeight: corpusCase.tileHeight,
                tilePayloads: corpusCase.tiles.map(\.payload),
                tileSizes: corpusCase.tiles.map { ($0.width, $0.height) },
                hvcC: hvcc,
                channelCount: corpusCase.channelCount
            )
            try runRust(
                sourceURL: sourceURL,
                outputURL: rustURL,
                corpusCase: corpusCase,
                hvcc: hvcc
            )

            let swiftOutput = try Data(contentsOf: swiftURL)
            let rustOutput = try Data(contentsOf: rustURL)
            XCTAssertEqual(
                rustOutput,
                swiftOutput,
                "Swift/Rust HEIF byte mismatch for \(corpusCase.name)"
            )
            try verifySemanticContract(
                output: rustOutput,
                primaryPayload: source.primaryPayload,
                tileCount: corpusCase.tiles.count
            )
        }
    }
}
