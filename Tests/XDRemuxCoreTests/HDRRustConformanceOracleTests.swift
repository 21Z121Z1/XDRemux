import Foundation
import XCTest
@testable import XDRemuxCore

final class HDRRustConformanceOracleTests: XCTestCase {
    private struct EDRCase {
        let name: String
        let mode: ExtractionMode
        let values: [Double]
    }

    private func fixtureURL() -> URL {
        URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .appendingPathComponent("fixtures/hdr_edr_cases.tsv")
    }

    private func loadCases() throws -> [EDRCase] {
        let text = try String(contentsOf: fixtureURL(), encoding: .utf8)
        return try text.split(whereSeparator: \.isNewline).enumerated().compactMap { index, rawLine in
            let line = rawLine.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !line.isEmpty, !line.hasPrefix("#") else { return nil }
            let fields = line.split(separator: "\t", omittingEmptySubsequences: false)
            guard fields.count == 3 else {
                throw XDRemuxError.invalidLHDR("HDR EDR fixture line \(index + 1) must have 3 tab-separated fields")
            }
            let mode: ExtractionMode
            switch fields[1] {
            case "lhdr": mode = .lhdr
            case "uhdr": mode = .uhdr
            default:
                throw XDRemuxError.invalidLHDR("HDR EDR fixture line \(index + 1) has unknown mode \(fields[1])")
            }
            let values = try fields[2].split(separator: ",").map { word -> Double in
                guard let bits = UInt32(word, radix: 16) else {
                    throw XDRemuxError.invalidLHDR("HDR EDR fixture line \(index + 1) has invalid float32 bits \(word)")
                }
                return Double(Float(bitPattern: bits))
            }
            return EDRCase(name: String(fields[0]), mode: mode, values: values)
        }
    }

    private func caseNamed(_ name: String) throws -> EDRCase {
        try XCTUnwrap(loadCases().first { $0.name == name }, "missing HDR EDR case \(name)")
    }

    private func bits(_ value: Double) -> String {
        String(format: "%016llx", value.bitPattern)
    }

    private func bitsList(_ values: [Double]) -> String {
        values.map(bits).joined(separator: ",")
    }

    private func hex(_ data: Data) -> String {
        data.map { String(format: "%02x", $0) }.joined()
    }

    private func resolvedLine(name: String, resolved: ResolvedScale) -> String {
        [
            "resolve", name,
            bits(resolved.edrScale),
            bits(resolved.ratioMin),
            bits(resolved.ratioMax),
            bits(resolved.gamma),
            bits(resolved.epsilonSdr),
            bits(resolved.epsilonHdr),
            bits(resolved.displayRatioSdr),
            bits(resolved.displayRatioHdr),
            bits(resolved.scale),
            bits(resolved.gainMapMin),
            bits(resolved.gainMapMax),
            bits(resolved.baseHeadroom),
            bits(resolved.alternateHeadroom),
            resolved.source,
            String(resolved.channelCount),
            bitsList(resolved.perChannelGainMapMin),
            bitsList(resolved.perChannelGainMapMax),
            bitsList(resolved.perChannelGamma),
            bitsList(resolved.perChannelBaseOffset),
            bitsList(resolved.perChannelAlternateOffset),
        ].joined(separator: "\t")
    }

    private func vectorText() throws -> String {
        var lines: [String] = []
        for item in try loadCases() {
            let resolved = try EDRScaleResolver.resolve(metaFloats: item.values, mode: item.mode)
            lines.append(resolvedLine(name: item.name, resolved: resolved))
            if item.mode == .lhdr, item.values[0] < 3.0 {
                let knee = EDRScaleResolver.getKneePointResult(resolved.edrScale)
                lines.append("knee\t\(item.name)\t\(bits(knee.value))\t\(knee.source)")
            }
        }
        return lines.joined(separator: "\n") + "\n"
    }

    private func allByteMask() -> GainMapRaster {
        GainMapRaster(
            width: 256,
            height: 1,
            bytesPerRow: 256,
            channelCount: 1,
            data: Data((0...255).map(UInt8.init))
        )
    }

    private func paddedMask() -> GainMapRaster {
        let width = 257
        let height = 2
        let bytesPerRow = 300
        var data = Data(repeating: 0xA5, count: bytesPerRow * height)
        for x in 0..<width {
            data[x] = UInt8((x * 17 + 3) & 0xFF)
            data[bytesPerRow + x] = UInt8(255 - ((x * 11) & 0xFF))
        }
        return GainMapRaster(
            width: width,
            height: height,
            bytesPerRow: bytesPerRow,
            channelCount: 1,
            data: data
        )
    }

    private func gainMapLine(name: String, raster: GainMapRaster, params: GainMapParams) -> String {
        [
            "gainmap", name,
            String(raster.width),
            String(raster.height),
            String(raster.bytesPerRow),
            String(raster.channelCount),
            params.family.rawValue,
            bits(params.knee),
            bits(params.kneeRange),
            bits(params.headroomScale),
            bits(params.maxBoost),
            bits(params.log2Scale),
            params.kneeSource,
            hex(raster.data),
        ].joined(separator: "\t")
    }

    private func reconstruct(caseName: String, mask: GainMapRaster) throws -> (GainMapRaster, GainMapParams) {
        let item = try caseNamed(caseName)
        let scale = try EDRScaleResolver.resolve(metaFloats: item.values, mode: item.mode)
        return try GainMapReconstructor.reconstruct(
            mask: mask,
            family: .x7,
            scale: scale,
            metaFloats: item.values
        )
    }

    private func gainMapVectorText() throws -> String {
        var lines: [String] = []
        for (outputName, caseName) in [
            ("early-all-bytes", "early-face-mid-highlight"),
            ("modern-all-bytes", "modern-precomputed-f32-source"),
        ] {
            let (raster, params) = try reconstruct(caseName: caseName, mask: allByteMask())
            lines.append(gainMapLine(name: outputName, raster: raster, params: params))
        }
        let (padded, paddedParams) = try reconstruct(
            caseName: "modern-precomputed-f32-source",
            mask: paddedMask()
        )
        lines.append(gainMapLine(name: "padded-stride", raster: padded, params: paddedParams))
        return lines.joined(separator: "\n") + "\n"
    }

    func testEDRSharedVectorsCoverPrecisionAndBranchBoundaries() throws {
        let cases = try loadCases()
        XCTAssertEqual(cases.count, 17)
        let names = Set(cases.map(\.name))
        for required in [
            "early-cfg-sentinel",
            "early-numeric-one-not-cfg",
            "modern-sigmoid-cfg-high",
            "modern-main-low",
            "modern-main-mid",
            "modern-main-high",
            "uhdr-distinct",
        ] {
            XCTAssertTrue(names.contains(required), "missing EDR conformance vector \(required)")
        }

        let sentinel = try XCTUnwrap(cases.first { $0.name == "early-cfg-sentinel" })
        let numericOne = try XCTUnwrap(cases.first { $0.name == "early-numeric-one-not-cfg" })
        let sentinelScale = try EDRScaleResolver.resolve(metaFloats: sentinel.values, mode: sentinel.mode).edrScale
        let numericScale = try EDRScaleResolver.resolve(metaFloats: numericOne.values, mode: numericOne.mode).edrScale
        XCTAssertNotEqual(sentinelScale.bitPattern, numericScale.bitPattern)
    }

    func testEmitEDRVectorsForRustDifferential() throws {
        guard let path = ProcessInfo.processInfo.environment["XDREMUX_HDR_ORACLE_OUTPUT"], !path.isEmpty else {
            throw XCTSkip("set XDREMUX_HDR_ORACLE_OUTPUT to emit Swift EDR vectors")
        }
        try Data(vectorText().utf8).write(to: URL(fileURLWithPath: path), options: .atomic)
    }

    func testGainMapSharedVectorsExhaustByteDomainAndPadding() throws {
        let mask = allByteMask()
        XCTAssertEqual(Set(mask.data), Set(UInt8.min...UInt8.max))

        let (early, _) = try reconstruct(caseName: "early-face-mid-highlight", mask: mask)
        XCTAssertEqual(early.width, 256)
        XCTAssertEqual(early.height, 1)
        XCTAssertEqual(early.bytesPerRow, 256)
        XCTAssertEqual(early.data.count, 256)

        let (modern, _) = try reconstruct(caseName: "modern-precomputed-f32-source", mask: mask)
        XCTAssertEqual(modern.data.count, 256)
        XCTAssertEqual(modern.data[6], 1, "current Swift quantization truncates rather than rounds")

        let (padded, _) = try reconstruct(
            caseName: "modern-precomputed-f32-source",
            mask: paddedMask()
        )
        XCTAssertEqual(padded.width, 257)
        XCTAssertEqual(padded.height, 2)
        XCTAssertEqual(padded.bytesPerRow, 512)
        XCTAssertEqual(padded.data.count, 1024)
        XCTAssertTrue(padded.data[257..<512].allSatisfy { $0 == 0 })
        XCTAssertTrue(padded.data[(512 + 257)..<1024].allSatisfy { $0 == 0 })
    }

    func testEmitGainMapVectorsForRustDifferential() throws {
        guard let path = ProcessInfo.processInfo.environment["XDREMUX_GAINMAP_ORACLE_OUTPUT"], !path.isEmpty else {
            throw XCTSkip("set XDREMUX_GAINMAP_ORACLE_OUTPUT to emit Swift gain map vectors")
        }
        try Data(gainMapVectorText().utf8).write(to: URL(fileURLWithPath: path), options: .atomic)
    }
}
