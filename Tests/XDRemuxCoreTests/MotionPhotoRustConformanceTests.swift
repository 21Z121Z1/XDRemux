import Foundation
import XCTest
@testable import XDRemuxCore

final class MotionPhotoRustConformanceTests: XCTestCase {
    private func makeBox(_ type: String, payload: Data) -> Data {
        precondition(type.utf8.count == 4)
        let size = UInt32(payload.count + 8)
        var output = Data([
            UInt8((size >> 24) & 0xff),
            UInt8((size >> 16) & 0xff),
            UInt8((size >> 8) & 0xff),
            UInt8(size & 0xff),
        ])
        output.append(Data(type.utf8))
        output.append(payload)
        return output
    }

    private func fakeMP4(brand: String, payloadByte: UInt8) -> Data {
        var payload = Data(brand.utf8)
        payload.append(contentsOf: [0, 0, 0, 0])
        var data = makeBox("ftyp", payload: payload)
        data.append(makeBox("mdat", payload: Data(repeating: payloadByte, count: 4)))
        return data
    }

    private func temporaryFile(_ data: Data, suffix: String) throws -> URL {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-motion-rust-\(UUID().uuidString).\(suffix)")
        try data.write(to: url, options: .atomic)
        return url
    }

    private func runRust(_ arguments: [String]) throws -> [String: Any] {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
        process.arguments = [
            "cargo", "run", "--quiet", "--locked", "-p", "xdremux-motion-photo",
            "--example", "motion_photo_conformance", "--",
        ] + arguments
        process.currentDirectoryURL = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
        let stdout = Pipe()
        let stderr = Pipe()
        process.standardOutput = stdout
        process.standardError = stderr
        try process.run()
        process.waitUntilExit()
        let output = stdout.fileHandleForReading.readDataToEndOfFile()
        let error = String(
            data: stderr.fileHandleForReading.readDataToEndOfFile(),
            encoding: .utf8
        ) ?? ""
        XCTAssertEqual(process.terminationStatus, 0, "Rust Motion Photo oracle failed: \(error)")
        let object = try JSONSerialization.jsonObject(with: output)
        return try XCTUnwrap(object as? [String: Any])
    }

    private func range(_ object: Any?) throws -> MotionPhotoByteRange {
        let dictionary = try XCTUnwrap(object as? [String: Any])
        let lower = try XCTUnwrap(dictionary["lower"] as? NSNumber).int64Value
        let upper = try XCTUnwrap(dictionary["upper"] as? NSNumber).int64Value
        return try MotionPhotoByteRange(lowerBound: lower, upperBound: upper)
    }

    func testSwiftAndRustPureMotionPhotoContractsMatch() throws {
        let lpexJSON = """
        {"version":1,"matrixCount":2,"coverFramePts":1433000,
         "photoCropMatrix":[1,0,0,0,1,0,0,0,1],
         "photoEisMatrix":[1,0,0,0,1,0,0,0,1],
         "matrices":{"frame-A":[1,0,0,0,1,0,0,0,1]},
         "videoSize":[1728,1296],"originPhotoSize":[4096,3072],
         "photoEisCropFactor":[1.11,1.12],"eisCropFactor":[0.90,0.91],
         "photoCropFactor":0.9}
        """
        let lpexPayload = Data("prefix lpexLivePhotoExtension \(lpexJSON) suffix".utf8)
        let lpexURL = try temporaryFile(lpexPayload, suffix: "bin")
        defer { try? FileManager.default.removeItem(at: lpexURL) }
        let swiftLpex = try XCTUnwrap(OppoLpexParser.parseFirstObject(in: lpexPayload))
        let rustLpex = try runRust(["lpex", lpexURL.path])
        XCTAssertEqual((rustLpex["version"] as? NSNumber)?.intValue, swiftLpex.version)
        XCTAssertEqual((rustLpex["matrixCount"] as? NSNumber)?.intValue, swiftLpex.matrixCount)
        XCTAssertEqual((rustLpex["coverFramePtsUs"] as? NSNumber)?.int64Value, swiftLpex.coverFramePtsUs)
        XCTAssertEqual((rustLpex["videoWidth"] as? NSNumber)?.intValue, swiftLpex.videoWidth)
        XCTAssertEqual((rustLpex["videoHeight"] as? NSNumber)?.intValue, swiftLpex.videoHeight)
        XCTAssertEqual((rustLpex["originPhotoWidth"] as? NSNumber)?.intValue, swiftLpex.originPhotoWidth)
        XCTAssertEqual((rustLpex["originPhotoHeight"] as? NSNumber)?.intValue, swiftLpex.originPhotoHeight)
        XCTAssertEqual(
            (rustLpex["photoCropFactor"] as? NSNumber)?.doubleValue,
            swiftLpex.photoCropFactor
        )
        XCTAssertEqual(
            (rustLpex["photoEisCropFactor"] as? [NSNumber])?.map(\.doubleValue),
            swiftLpex.photoEisCropFactor
        )
        XCTAssertEqual(
            (rustLpex["eisCropFactor"] as? [NSNumber])?.map(\.doubleValue),
            swiftLpex.eisCropFactor
        )
        let rustMatrices = try XCTUnwrap(rustLpex["matrices"] as? [String: Any])
        XCTAssertNotNil(rustMatrices["frame-A"])
        XCTAssertNotNil(swiftLpex.matrices["frame-A"])

        var scannerData = Data("noise ftyp noise".utf8)
        let realFTYP = Int64(scannerData.count)
        scannerData.append(fakeMP4(brand: "isom", payloadByte: 0x44))
        let scannerURL = try temporaryFile(scannerData, suffix: "mp4")
        defer { try? FileManager.default.removeItem(at: scannerURL) }
        let scannerRange = try MotionPhotoByteRange(
            lowerBound: 0,
            upperBound: Int64(scannerData.count)
        )
        let swiftOffsets = try ISOBaseMediaStreamScanner.ftypBoxOffsets(
            in: scannerURL,
            range: scannerRange,
            bufferSize: 64
        )
        let rustScan = try runRust([
            "scan", scannerURL.path, "0", String(scannerData.count),
        ])
        let rustOffsets = try XCTUnwrap(rustScan["offsets"] as? [NSNumber]).map(\.int64Value)
        XCTAssertEqual(swiftOffsets, [realFTYP])
        XCTAssertEqual(rustOffsets, swiftOffsets)

        let innerVideo = fakeMP4(brand: "isom", payloadByte: 0x55)
        let heifFTYP = makeBox(
            "ftyp",
            payload: Data("heic".utf8) + Data([0, 0, 0, 0])
        )
        let mpvd = makeBox("mpvd", payload: innerVideo)
        let sefd = makeBox("sefd", payload: Data([1, 2, 3, 4]))
        let heifData = heifFTYP + mpvd + sefd
        let heifURL = try temporaryFile(heifData, suffix: "heic")
        defer { try? FileManager.default.removeItem(at: heifURL) }
        let payloadStart = Int64(heifFTYP.count + 8)
        let motionLength = Int64(heifData.count) - payloadStart
        let items = [
            MotionPhotoItem(mime: "image/heic", semantic: "Primary", length: 0, padding: 8),
            MotionPhotoItem(
                mime: "video/mp4",
                semantic: "MotionPhoto",
                length: motionLength,
                padding: 0
            ),
        ]
        let swiftHEIF = try ISOBMFFMotionPhotoRangeResolver.resolve(
            url: heifURL,
            items: items,
            fileSize: Int64(heifData.count)
        )
        let rustHEIF = try runRust(["heif", heifURL.path, String(motionLength)])
        XCTAssertEqual(try range(rustHEIF["still"]), swiftHEIF.still)
        XCTAssertEqual(try range(rustHEIF["video"]), swiftHEIF.video)

        let topologyLPEX = "lpexLivePhotoExtension {\"version\":1,\"coverFramePts\":1666666}"
        let still = Data([0xff, 0xd8]) + Data(topologyLPEX.utf8) + Data([0xff, 0xd9])
        let stream1 = fakeMP4(brand: "isom", payloadByte: 0x11)
        let stream2 = fakeMP4(brand: "mp42", payloadByte: 0x22)
        let stream1Start = Int64(still.count)
        let stream2Start = stream1Start + Int64(stream1.count)
        let topologyData = still + stream1 + stream2
        let topologyURL = try temporaryFile(topologyData, suffix: "jpg")
        defer { try? FileManager.default.removeItem(at: topologyURL) }
        let fileSize = Int64(topologyData.count)
        let base = MotionPhotoAsset(
            sourceURL: topologyURL,
            sourceKind: .androidMotionPhotoV1,
            items: [
                MotionPhotoItem(mime: "image/jpeg", semantic: "Primary", length: 0, padding: 0),
                MotionPhotoItem(
                    mime: "video/mp4",
                    semantic: "MotionPhoto",
                    length: fileSize - stream2Start,
                    padding: 0
                ),
            ],
            stillResourceRange: try MotionPhotoByteRange(lowerBound: 0, upperBound: stream2Start),
            videoResourceRange: try MotionPhotoByteRange(
                lowerBound: stream2Start,
                upperBound: fileSize
            ),
            presentationTimestampUs: nil,
            presentationSource: nil
        )
        let swiftEnriched = try OppoMotionPhotoParser.enrichIfPresent(base)
        let swiftLayout = try MotionPhotoVideoStreamLayoutResolver.resolve(for: swiftEnriched)
        let rustTopology = try runRust([
            "topology", topologyURL.path,
            String(stream2Start), String(stream2Start), String(fileSize), "1",
        ])
        XCTAssertEqual(try range(rustTopology["still"]), swiftEnriched.stillResourceRange)
        XCTAssertEqual(try range(rustTopology["video"]), swiftEnriched.videoResourceRange)
        XCTAssertEqual(
            (rustTopology["streamCount"] as? NSNumber)?.intValue,
            swiftEnriched.vendorMetadata?.streamCount
        )
        let rustPrimary = try XCTUnwrap(rustTopology["primary"] as? [String: Any])
        XCTAssertEqual(try range(rustPrimary["range"]), swiftLayout.primary.range)
        XCTAssertEqual(rustPrimary["role"] as? String, swiftLayout.primary.role.rawValue)
        let rustAuxiliary = try XCTUnwrap(rustTopology["auxiliaryGeometry"] as? [[String: Any]])
        XCTAssertEqual(rustAuxiliary.count, swiftLayout.auxiliaryGeometry.count)
        XCTAssertEqual(try range(rustAuxiliary[0]["range"]), swiftLayout.auxiliaryGeometry[0].range)
        XCTAssertEqual(
            rustAuxiliary[0]["role"] as? String,
            swiftLayout.auxiliaryGeometry[0].role.rawValue
        )
        XCTAssertEqual(swiftLayout.primary.range.lowerBound, stream1Start)
        XCTAssertEqual(swiftLayout.primary.range.upperBound, stream2Start)
    }
}
