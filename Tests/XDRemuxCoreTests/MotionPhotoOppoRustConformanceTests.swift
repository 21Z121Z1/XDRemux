import Foundation
import XCTest
@testable import XDRemuxCore

final class MotionPhotoOppoRustConformanceTests: XCTestCase {
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

    private func fakeMP4(
        brand: String = "isom",
        payloadSize: Int = 120_000,
        payloadByte: UInt8 = 0x44
    ) -> Data {
        precondition(brand.utf8.count == 4)
        var ftypPayload = Data(brand.utf8)
        ftypPayload.append(contentsOf: [0, 0, 0, 0])
        var data = makeBox("ftyp", payload: ftypPayload)
        data.append(makeBox("mdat", payload: Data(repeating: payloadByte, count: payloadSize)))
        return data
    }

    private func temporaryFile(_ data: Data) throws -> URL {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-oppo-rust-\(UUID().uuidString).jpg")
        try data.write(to: url, options: .atomic)
        return url
    }

    private func runRust(_ url: URL) throws -> [String: Any] {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
        process.arguments = [
            "cargo", "run", "--quiet", "--locked", "-p", "xdremux-motion-photo",
            "--example", "motion_photo_conformance", "--", "oppo", url.path,
        ]
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
        XCTAssertEqual(process.terminationStatus, 0, "Rust OPPO Motion Photo oracle failed: \(error)")
        let object = try JSONSerialization.jsonObject(with: output)
        return try XCTUnwrap(object as? [String: Any])
    }

    private func errorCode(_ error: MotionPhotoParsingError) -> String {
        switch error {
        case .fileTooSmall: return "fileTooSmall"
        case .xmpTooLarge: return "xmpTooLarge"
        case .malformedXMP: return "malformedXMP"
        case .unsupportedVersion: return "unsupportedVersion"
        case .invalidDirectory: return "invalidDirectory"
        case .invalidPrimaryItem: return "invalidPrimaryItem"
        case .invalidMotionPhotoItem: return "invalidMotionPhotoItem"
        case .invalidItemLength: return "invalidItemLength"
        case .arithmeticOverflow: return "arithmeticOverflow"
        case .invalidByteRange: return "invalidByteRange"
        case .invalidVideoPayload: return "invalidVideoPayload"
        case .payloadTooLarge: return "payloadTooLarge"
        }
    }

    private func assertRustAssetMatchesSwift(
        _ url: URL,
        file: StaticString = #filePath,
        line: UInt = #line
    ) throws {
        let swift = try XCTUnwrap(OppoMotionPhotoParser.parse(url: url), file: file, line: line)
        let rust = try runRust(url)
        XCTAssertEqual(rust["status"] as? String, "asset", file: file, line: line)
        XCTAssertEqual(rust["sourceKind"] as? String, swift.sourceKind.rawValue, file: file, line: line)
        XCTAssertEqual(
            (rust["presentationTimestampUs"] as? NSNumber)?.int64Value,
            swift.presentationTimestampUs,
            file: file,
            line: line
        )
        XCTAssertEqual(
            rust["presentationSource"] as? String,
            swift.presentationSource?.rawValue,
            file: file,
            line: line
        )

        let still = try XCTUnwrap(rust["still"] as? [String: Any], file: file, line: line)
        let video = try XCTUnwrap(rust["video"] as? [String: Any], file: file, line: line)
        XCTAssertEqual(
            (still["lower"] as? NSNumber)?.int64Value,
            swift.stillResourceRange.lowerBound,
            file: file,
            line: line
        )
        XCTAssertEqual(
            (still["upper"] as? NSNumber)?.int64Value,
            swift.stillResourceRange.upperBound,
            file: file,
            line: line
        )
        XCTAssertEqual(
            (video["lower"] as? NSNumber)?.int64Value,
            swift.videoResourceRange.lowerBound,
            file: file,
            line: line
        )
        XCTAssertEqual(
            (video["upper"] as? NSNumber)?.int64Value,
            swift.videoResourceRange.upperBound,
            file: file,
            line: line
        )

        let rustItems = try XCTUnwrap(rust["items"] as? [[String: Any]], file: file, line: line)
        XCTAssertEqual(rustItems.count, swift.items.count, file: file, line: line)
        for (rustItem, swiftItem) in zip(rustItems, swift.items) {
            XCTAssertEqual(rustItem["mime"] as? String, swiftItem.mime, file: file, line: line)
            XCTAssertEqual(rustItem["semantic"] as? String, swiftItem.semantic, file: file, line: line)
            XCTAssertEqual(
                (rustItem["length"] as? NSNumber)?.int64Value,
                swiftItem.length,
                file: file,
                line: line
            )
            XCTAssertEqual(
                (rustItem["padding"] as? NSNumber)?.int64Value,
                swiftItem.padding,
                file: file,
                line: line
            )
        }

        if let swiftMetadata = swift.vendorMetadata {
            let rustMetadata = try XCTUnwrap(
                rust["vendorMetadata"] as? [String: Any],
                file: file,
                line: line
            )
            XCTAssertEqual(
                (rustMetadata["coverFramePtsUs"] as? NSNumber)?.int64Value,
                swiftMetadata.coverFramePtsUs,
                file: file,
                line: line
            )
            XCTAssertEqual(
                (rustMetadata["version"] as? NSNumber)?.intValue,
                swiftMetadata.version,
                file: file,
                line: line
            )
            XCTAssertEqual(
                (rustMetadata["matrixCount"] as? NSNumber)?.intValue,
                swiftMetadata.matrixCount,
                file: file,
                line: line
            )
            XCTAssertEqual(
                (rustMetadata["streamCount"] as? NSNumber)?.intValue,
                swiftMetadata.streamCount,
                file: file,
                line: line
            )
            XCTAssertEqual(
                (rustMetadata["videoWidth"] as? NSNumber)?.intValue,
                swiftMetadata.videoWidth,
                file: file,
                line: line
            )
            XCTAssertEqual(
                (rustMetadata["videoHeight"] as? NSNumber)?.intValue,
                swiftMetadata.videoHeight,
                file: file,
                line: line
            )
        } else {
            XCTAssertTrue(rust["vendorMetadata"] is NSNull, file: file, line: line)
        }
    }

    private func assertRustNoneMatchesSwift(
        _ url: URL,
        file: StaticString = #filePath,
        line: UInt = #line
    ) throws {
        XCTAssertNil(try OppoMotionPhotoParser.parse(url: url), file: file, line: line)
        let rust = try runRust(url)
        XCTAssertEqual(rust["status"] as? String, "none", file: file, line: line)
    }

    private func assertRustErrorMatchesSwift(
        _ url: URL,
        file: StaticString = #filePath,
        line: UInt = #line
    ) throws {
        let rust = try runRust(url)
        XCTAssertEqual(rust["status"] as? String, "error", file: file, line: line)
        do {
            _ = try OppoMotionPhotoParser.parse(url: url)
            XCTFail("Swift OPPO parser unexpectedly accepted malformed corpus", file: file, line: line)
        } catch let error as MotionPhotoParsingError {
            XCTAssertEqual(rust["code"] as? String, errorCode(error), file: file, line: line)
        }
    }

    private func standardXMP(videoLength: Int, includeVersion: Bool = true) -> String {
        let version = includeVersion ? " Camera:MotionPhotoVersion=\"1\"" : ""
        return """
        <x:xmpmeta xmlns:x="adobe:ns:meta/">
          <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
            <rdf:Description xmlns:Camera="http://ns.google.com/photos/1.0/camera/"
                             xmlns:Container="http://ns.google.com/photos/1.0/container/"
                             xmlns:Item="http://ns.google.com/photos/1.0/container/item/"
                             Camera:MotionPhoto="1"\(version)
                             Camera:MotionPhotoPresentationTimestampUs="1634640">
              <Container:Directory><rdf:Seq>
                <rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="image/jpeg" Item:Semantic="Primary" Item:Length="0" Item:Padding="0"/></rdf:li>
                <rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="video/mp4" Item:Semantic="MotionPhoto" Item:Length="\(videoLength)" Item:Padding="0"/></rdf:li>
              </rdf:Seq></Container:Directory>
            </rdf:Description>
          </rdf:RDF>
        </x:xmpmeta>
        """
    }

    func testSwiftAndRustOppoParserContractsMatch() throws {
        let unsignedVideo = fakeMP4(payloadSize: 128)
        let unsigned = Data([0xff, 0xd8, 0xff, 0xd9]) + unsignedVideo
        let unsignedURL = try temporaryFile(unsigned)
        defer { try? FileManager.default.removeItem(at: unsignedURL) }
        try assertRustNoneMatchesSwift(unsignedURL)

        let singleVideo = fakeMP4(payloadByte: 0x22)
        let singleXMP = """
        <x:xmpmeta xmlns:x="adobe:ns:meta/">
          <rdf:RDF><rdf:Description xmlns:OpCamera="http://ns.oppo.com/photos/1.0/camera/"
                                    OpCamera:VideoLength="\(singleVideo.count)"
                                    GCamera:MotionPhotoPresentationTimestampUs="1634640"/></rdf:RDF>
        </x:xmpmeta>
        """
        let single = Data([0xff, 0xd8]) + Data(singleXMP.utf8) + Data([0xff, 0xd9]) + singleVideo
        let singleURL = try temporaryFile(single)
        defer { try? FileManager.default.removeItem(at: singleURL) }
        try assertRustAssetMatchesSwift(singleURL)

        let staleVideo = fakeMP4(payloadByte: 0x33)
        let staleXMP = """
        <x:xmpmeta><rdf:RDF><rdf:Description xmlns:OpCamera="http://ns.oppo.com/photos/1.0/camera/"
                                             OpCamera:VideoLength="100001"/></rdf:RDF></x:xmpmeta>
        """
        let stale = Data([0xff, 0xd8]) + Data(staleXMP.utf8) + Data([0xff, 0xd9]) + staleVideo
        let staleURL = try temporaryFile(stale)
        defer { try? FileManager.default.removeItem(at: staleURL) }
        try assertRustAssetMatchesSwift(staleURL)

        let stream1 = fakeMP4(brand: "isom", payloadSize: 1_024, payloadByte: 0x44)
        let stream2 = fakeMP4(brand: "mp42", payloadSize: 1_024, payloadByte: 0x55)
        let dualXMP = """
        <x:xmpmeta xmlns:x="adobe:ns:meta/">
          <rdf:RDF><rdf:Description xmlns:OpCamera="http://ns.oppo.com/photos/1.0/camera/"
                                    OpCamera:VideoLength="\(stream2.count)"
                                    GCamera:MotionPhotoPresentationTimestampUs="1634640"/></rdf:RDF>
        </x:xmpmeta>
        """
        let lpex = """
        lpexLivePhotoExtension {"version":1,"coverFramePts":1666666,"matrixCount":0,"videoSize":[1920,1080]}
        """
        let dualStill = Data([0xff, 0xd8]) + Data(dualXMP.utf8) + Data(lpex.utf8) + Data([0xff, 0xd9])
        let dual = dualStill + stream1 + stream2
        let dualURL = try temporaryFile(dual)
        defer { try? FileManager.default.removeItem(at: dualURL) }
        try assertRustAssetMatchesSwift(dualURL)

        let standardStream1 = fakeMP4(brand: "isom", payloadSize: 1_024, payloadByte: 0x66)
        let standardStream2 = fakeMP4(brand: "mp42", payloadSize: 1_024, payloadByte: 0x77)
        let standard = standardXMP(videoLength: standardStream2.count)
        let standardStill = Data([0xff, 0xd8]) + Data(standard.utf8) + Data(lpex.utf8) + Data([0xff, 0xd9])
        let standardDual = standardStill + standardStream1 + standardStream2
        let standardDualURL = try temporaryFile(standardDual)
        defer { try? FileManager.default.removeItem(at: standardDualURL) }
        try assertRustAssetMatchesSwift(standardDualURL)

        let coverVideo = fakeMP4(payloadSize: 128, payloadByte: 0x88)
        let coverLpex = "lpexLivePhotoExtension {\"version\":0,\"coverFramePts\":777777}"
        let cover = Data([0xff, 0xd8]) + Data(coverLpex.utf8) + Data([0xff, 0xd9]) + coverVideo
        let coverURL = try temporaryFile(cover)
        defer { try? FileManager.default.removeItem(at: coverURL) }
        try assertRustAssetMatchesSwift(coverURL)

        let recoverVideo = fakeMP4(payloadByte: 0x99)
        let malformedOppoXMP = """
        <x:xmpmeta xmlns:x="adobe:ns:meta/">
          <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
            <rdf:Description xmlns:Camera="http://ns.google.com/photos/1.0/camera/"
                             xmlns:OpCamera="http://ns.oppo.com/photos/1.0/camera/"
                             Camera:MotionPhoto="1"
                             OpCamera:VideoLength="\(recoverVideo.count)"/>
          </rdf:RDF>
        </x:xmpmeta>
        """
        let recover = Data([0xff, 0xd8]) + Data(malformedOppoXMP.utf8) + Data([0xff, 0xd9]) + recoverVideo
        let recoverURL = try temporaryFile(recover)
        defer { try? FileManager.default.removeItem(at: recoverURL) }
        try assertRustAssetMatchesSwift(recoverURL)

        let malformedGenericXMP = """
        <x:xmpmeta xmlns:x="adobe:ns:meta/">
          <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
            <rdf:Description xmlns:Camera="http://ns.google.com/photos/1.0/camera/"
                             xmlns:Container="http://ns.google.com/photos/1.0/container/"
                             xmlns:Item="http://ns.google.com/photos/1.0/container/item/"
                             Camera:MotionPhoto="1">
              <Container:Directory><rdf:Seq>
                <rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="image/jpeg" Item:Semantic="Primary" Item:Length="0" Item:Padding="0"/></rdf:li>
                <rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="video/mp4" Item:Semantic="MotionPhoto" Item:Length="\(recoverVideo.count)" Item:Padding="0"/></rdf:li>
              </rdf:Seq></Container:Directory>
            </rdf:Description>
          </rdf:RDF>
        </x:xmpmeta>
        """
        let malformedGeneric = Data([0xff, 0xd8]) + Data(malformedGenericXMP.utf8)
            + Data([0xff, 0xd9]) + recoverVideo
        let malformedGenericURL = try temporaryFile(malformedGeneric)
        defer { try? FileManager.default.removeItem(at: malformedGenericURL) }
        try assertRustErrorMatchesSwift(malformedGenericURL)
    }
}
