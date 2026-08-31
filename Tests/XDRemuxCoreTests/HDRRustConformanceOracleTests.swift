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
        return try text.split(whereSeparator: \ .isNewline).enumerated().compactMap { index, rawLine in
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

    private func bits(_ value: Double) -> String {
        String(format: "%016llx", value.bitPattern)
    }

    private func bitsList(_ values: [Double]) -> String {
        values.map(bits).joined(separator: ",")
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
}
