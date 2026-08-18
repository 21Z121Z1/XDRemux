import Foundation
import XCTest
@testable import XDRemuxCore

final class PerformanceDesignCoreTests: XCTestCase {
    func testDataBackedTIFFReaderParsesSyntheticLittleEndianDNG() throws {
        var data = Data([0x49, 0x49, 0x2A, 0x00])
        appendUInt32LEForTest(8, to: &data)
        appendUInt16LEForTest(2, to: &data)

        // ImageWidth: LONG, count 1, inline value 4032.
        appendUInt16LEForTest(0x0100, to: &data)
        appendUInt16LEForTest(4, to: &data)
        appendUInt32LEForTest(1, to: &data)
        appendUInt32LEForTest(4032, to: &data)

        // ImageLength: LONG, count 1, inline value 3024.
        appendUInt16LEForTest(0x0101, to: &data)
        appendUInt16LEForTest(4, to: &data)
        appendUInt32LEForTest(1, to: &data)
        appendUInt32LEForTest(3024, to: &data)

        // No next IFD.
        appendUInt32LEForTest(0, to: &data)

        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-perf-tiff-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let url = root.appendingPathComponent("synthetic.dng")
        try data.write(to: url)

        let metadata = try CoreImageRAW.extractDNGMetadata(from: url)
        XCTAssertEqual(metadata.endian, "little")
        XCTAssertEqual(metadata.rawIFDName, "IFD0")
        XCTAssertEqual(metadata.rawWidth, 4032)
        XCTAssertEqual(metadata.rawHeight, 3024)
    }

    func testGainMapReconstructorBorrowedMaskMatchesReferenceFormula() throws {
        let width = 4
        let height = 2
        let bytesPerRow = 6
        let sourceBytes: [UInt8] = [
            0, 64, 128, 255, 0xEE, 0xEE,
            10, 100, 200, 250, 0xDD, 0xDD,
        ]
        let mask = GainMapRaster(
            width: width,
            height: height,
            bytesPerRow: bytesPerRow,
            channelCount: 1,
            data: Data(sourceBytes)
        )
        let scale = ResolvedScale(
            edrScale: 4,
            ratioMin: 1,
            ratioMax: 4,
            gamma: 1,
            epsilonSdr: 0,
            epsilonHdr: 0,
            displayRatioSdr: 1,
            displayRatioHdr: 4,
            scale: 4,
            gainMapMin: 0,
            gainMapMax: 2,
            baseHeadroom: 0,
            alternateHeadroom: 2,
            source: "synthetic",
            channelCount: 1,
            perChannelGainMapMin: [0],
            perChannelGainMapMax: [2],
            perChannelGamma: [1],
            perChannelBaseOffset: [0],
            perChannelAlternateOffset: [0]
        )

        let result = try GainMapReconstructor.reconstruct(
            mask: mask,
            family: .x7,
            scale: scale,
            metaFloats: [3.0]
        ).raster

        XCTAssertEqual(result.width, width)
        XCTAssertEqual(result.height, height)
        XCTAssertEqual(result.channelCount, 1)
        XCTAssertEqual(result.bytesPerRow, 256)

        let expectedRows = [
            sourceBytes[0..<4].map { referenceGainMapByte($0, edrScale: 4) },
            sourceBytes[6..<10].map { referenceGainMapByte($0, edrScale: 4) },
        ]
        result.data.withUnsafeBytes { raw in
            let bytes = raw.bindMemory(to: UInt8.self)
            for y in 0..<height {
                for x in 0..<width {
                    XCTAssertEqual(
                        bytes[y * result.bytesPerRow + x],
                        expectedRows[y][x],
                        "pixel \(x),\(y)"
                    )
                }
            }
        }
    }

    func testDirectTiledEncoderAcceptsRawRowStridedMonochromeRaster() throws {
        let width = 64
        let height = 64
        let bytesPerRow = 80
        var bytes = Data(count: bytesPerRow * height)
        bytes.withUnsafeMutableBytes { raw in
            guard let base = raw.bindMemory(to: UInt8.self).baseAddress else { return }
            for y in 0..<height {
                for x in 0..<width {
                    base[y * bytesPerRow + x] = UInt8((x * 3 + y * 5) & 0xFF)
                }
            }
        }
        let raster = GainMapRaster(
            width: width,
            height: height,
            bytesPerRow: bytesPerRow,
            channelCount: 1,
            data: bytes
        )
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-perf-direct-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        let scratch = root.appendingPathComponent("output.heic")

        let encoded = try DirectTiledHEVCGainMapEncoder.encode(
            raster: raster,
            scratchBaseURL: scratch
        )
        XCTAssertEqual(encoded.width, width)
        XCTAssertEqual(encoded.height, height)
        XCTAssertEqual(encoded.channelCount, 1)
        XCTAssertEqual(encoded.tilePayloads.count, 1)
        XCTAssertFalse(encoded.tilePayloads[0].isEmpty)
        XCTAssertFalse(encoded.hvcC.isEmpty)
    }

    private func referenceGainMapByte(_ byte: UInt8, edrScale: Double) -> UInt8 {
        let maskValue = Double(byte) / 255.0
        let idx0 = min(1000, max(0, Int(maskValue * 1000)))
        let linGray = pow(Double(idx0) / 1000.0, 0.625)
        let idx1 = min(1000, max(0, Int(linGray * 1000)))
        let linear = pow(Double(idx1) / 1000.0, 2.2)
        let gammaFactor = pow(1.0 / edrScale, 1.0 / 2.2)
        let headroomScale = (1.0 - gammaFactor) / gammaFactor
        let idx2 = min(1000, max(0, Int(linear * 1000)))
        let boosted = pow(Double(idx2) / 1000.0 * headroomScale + 1.0, 2.2)
        let idx3 = min(8000, max(0, Int(min(boosted, 8.0) * 1000.0)))
        let x = Double(idx3) / 1000.0
        let clamped = min(max(x, 1.0), edrScale)
        let log2Scale = 255.0 / log2(edrScale)
        let gain = Int(log2Scale * log2(clamped))
        return UInt8(min(255, max(0, gain)))
    }

    private func appendUInt16LEForTest(_ value: UInt16, to data: inout Data) {
        var little = value.littleEndian
        withUnsafeBytes(of: &little) { data.append(contentsOf: $0) }
    }

    private func appendUInt32LEForTest(_ value: UInt32, to data: inout Data) {
        var little = value.littleEndian
        withUnsafeBytes(of: &little) { data.append(contentsOf: $0) }
    }
}
