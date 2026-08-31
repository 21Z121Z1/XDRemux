import Foundation
import XCTest
@testable import XDRemuxCore

final class MotionPhotoAndroidRustConformanceTests: XCTestCase {
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

    private func fakeMP4(brand: String = "isom", payloadByte: UInt8 = 0x44) -> Data {
        var payload = Data(brand.utf8)
        payload.append(contentsOf: [0, 0, 2, 0])
        var data = makeBox("ftyp", payload: payload)
        data.append(makeBox("mdat", payload: Data(repeating: payloadByte, count: 4)))
        return data
    }

    private func temporaryFile(_ data: Data, suffix: String = "jpg") throws -> URL {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-android-rust-\(UUID().uuidString).\(suffix)")
        try data.write(to: url, options: .atomic)
        return url
    }

    private func runRust(_ url: URL) throws -> [String: Any] {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
        process.arguments = [
            "cargo", "run", "--quiet", "--locked", "-p", "xdremux-motion-photo",
            "--example", "motion_photo_conformance", "--", "android", url.path,
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
        XCTAssertEqual(process.terminationStatus, 0, "Rust Android Motion Photo oracle failed: \(error)")
        let object = try JSONSerialization.jsonObject(with: output)
        return try XCTUnwrap(object as? [String: Any])
    }

    private func standardXMP(
        videoLength: Int64,
        timestamp: Int64? = nil,
        primaryMime: String = "image/jpeg",
        primaryPadding: Int64 = 0,
        extraItems: String = "",
        includeVersion: Bool = true,
        trailingItems: String = ""
    ) -> String {
        let timestampAttribute = timestamp.map {
            " Camera:MotionPhotoPresentationTimestampUs=\"\($0)\""
        } ?? ""
        let versionAttribute = includeVersion ? " Camera:MotionPhotoVersion=\"1\"" : ""
        return """
        <x:xmpmeta xmlns:x="adobe:ns:meta/">
          <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
            <rdf:Description xmlns:Camera="http://ns.google.com/photos/1.0/camera/"
                             xmlns:Container="http://ns.google.com/photos/1.0/container/"
                             xmlns:Item="http://ns.google.com/photos/1.0/container/item/"
                             Camera:MotionPhoto="1"\(versionAttribute)\(timestampAttribute)>
              <Container:Directory><rdf:Seq>
                <rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="\(primaryMime)" Item:Semantic="Primary" Item:Length="0" Item:Padding="\(primaryPadding)"/></rdf:li>
                \(extraItems)
                <rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="video/mp4" Item:Semantic="MotionPhoto" Item:Length="\(videoLength)" Item:Padding="0"/></rdf:li>
                \(trailingItems)
              </rdf:Seq></Container:Directory>
            </rdf:Description>
          </rdf:RDF>
        </x:xmpmeta>
        """
    }

    private func jpegMotionPhoto(xmp: String, secondary: Data = Data(), video: Data) -> Data {
        Data([0xff, 0xd8]) + Data(xmp.utf8) + Data([0xff, 0xd9]) + secondary + video
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

    private func assertRustAssetMatchesSwift(_ url: URL, file: StaticString = #filePath, line: UInt = #line) throws {
        let swift = try XCTUnwrap(AndroidMotionPhotoParser.parse(url: url), file: file, line: line)
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
        XCTAssertEqual((still["lower"] as? NSNumber)?.int64Value, swift.stillResourceRange.lowerBound, file: file, line: line)
        XCTAssertEqual((still["upper"] as? NSNumber)?.int64Value, swift.stillResourceRange.upperBound, file: file, line: line)
        XCTAssertEqual((video["lower"] as? NSNumber)?.int64Value, swift.videoResourceRange.lowerBound, file: file, line: line)
        XCTAssertEqual((video["upper"] as? NSNumber)?.int64Value, swift.videoResourceRange.upperBound, file: file, line: line)

        let rustItems = try XCTUnwrap(rust["items"] as? [[String: Any]], file: file, line: line)
        XCTAssertEqual(rustItems.count, swift.items.count, file: file, line: line)
        for (rustItem, swiftItem) in zip(rustItems, swift.items) {
            XCTAssertEqual(rustItem["mime"] as? String, swiftItem.mime, file: file, line: line)
            XCTAssertEqual(rustItem["semantic"] as? String, swiftItem.semantic, file: file, line: line)
            XCTAssertEqual((rustItem["length"] as? NSNumber)?.int64Value, swiftItem.length, file: file, line: line)
            XCTAssertEqual((rustItem["padding"] as? NSNumber)?.int64Value, swiftItem.padding, file: file, line: line)
        }
    }

    private func assertRustErrorMatchesSwift(_ url: URL, file: StaticString = #filePath, line: UInt = #line) throws {
        let rust = try runRust(url)
        XCTAssertEqual(rust["status"] as? String, "error", file: file, line: line)
        do {
            _ = try AndroidMotionPhotoParser.parse(url: url)
            XCTFail("Swift parser unexpectedly accepted malformed corpus", file: file, line: line)
        } catch let error as MotionPhotoParsingError {
            XCTAssertEqual(rust["code"] as? String, errorCode(error), file: file, line: line)
        }
    }

    func testSwiftAndRustAndroidParserContractsMatch() throws {
        let video = fakeMP4()

        let normal = jpegMotionPhoto(
            xmp: standardXMP(videoLength: Int64(video.count), timestamp: 1_417_000),
            video: video
        )
        let normalURL = try temporaryFile(normal)
        defer { try? FileManager.default.removeItem(at: normalURL) }
        try assertRustAssetMatchesSwift(normalURL)

        let gainMap = Data(repeating: 0xab, count: 64)
        let gainItem = """
        <rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="image/jpeg" Item:Semantic="GainMap" Item:Length="\(gainMap.count)" Item:Padding="0"/></rdf:li>
        """
        let withGain = jpegMotionPhoto(
            xmp: standardXMP(videoLength: Int64(video.count), extraItems: gainItem),
            secondary: gainMap,
            video: video
        )
        let gainURL = try temporaryFile(withGain)
        defer { try? FileManager.default.removeItem(at: gainURL) }
        try assertRustAssetMatchesSwift(gainURL)

        let legacyXMP = """
        <x:xmpmeta xmlns:x="adobe:ns:meta/">
          <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
            <rdf:Description xmlns:GCamera="http://ns.google.com/photos/1.0/camera/"
                             GCamera:MicroVideo="1"
                             GCamera:MicroVideoOffset="\(video.count)"
                             GCamera:MicroVideoPresentationTimestampUs="900000"/>
          </rdf:RDF>
        </x:xmpmeta>
        """
        let legacy = jpegMotionPhoto(xmp: legacyXMP, video: video)
        let legacyURL = try temporaryFile(legacy)
        defer { try? FileManager.default.removeItem(at: legacyURL) }
        try assertRustAssetMatchesSwift(legacyURL)

        let gcontainerXMP = """
        <x:xmpmeta xmlns:x="adobe:ns:meta/">
          <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
            <rdf:Description xmlns:GCamera="http://ns.google.com/photos/1.0/camera/"
                             xmlns:GContainer="http://ns.google.com/photos/1.0/container/"
                             xmlns:GContainerItem="http://ns.google.com/photos/1.0/container/item/"
                             GCamera:MotionPhoto="1" GCamera:MotionPhotoVersion="1"
                             GCamera:MotionPhotoPresentationTimestampUs="500000">
              <GContainer:Directory><rdf:Seq>
                <rdf:li rdf:parseType="Resource"><GContainer:Item GContainerItem:Mime="image/jpeg" GContainerItem:Semantic="Primary" GContainerItem:Length="0" GContainerItem:Padding="0"/></rdf:li>
                <rdf:li rdf:parseType="Resource"><GContainer:Item GContainerItem:Mime="video/mp4" GContainerItem:Semantic="MotionPhoto" GContainerItem:Length="\(video.count)" GContainerItem:Padding="0"/></rdf:li>
              </rdf:Seq></GContainer:Directory>
            </rdf:Description>
          </rdf:RDF>
        </x:xmpmeta>
        """
        let gcontainer = jpegMotionPhoto(xmp: gcontainerXMP, video: video)
        let gcontainerURL = try temporaryFile(gcontainer)
        defer { try? FileManager.default.removeItem(at: gcontainerURL) }
        try assertRustAssetMatchesSwift(gcontainerURL)

        let innerVideo = fakeMP4(payloadByte: 0x55)
        let sefd = makeBox("sefd", payload: Data([1, 2, 3, 4]))
        let motionLength = innerVideo.count + sefd.count
        let heifXMP = standardXMP(
            videoLength: Int64(motionLength),
            timestamp: 333_000,
            primaryMime: "image/heic",
            primaryPadding: 8
        )
        let heifFTYP = makeBox("ftyp", payload: Data("heic".utf8) + Data([0, 0, 0, 0]))
        let xmpBox = makeBox("free", payload: Data(heifXMP.utf8))
        let heif = heifFTYP + xmpBox + makeBox("mpvd", payload: innerVideo) + sefd
        let heifURL = try temporaryFile(heif, suffix: "heic")
        defer { try? FileManager.default.removeItem(at: heifURL) }
        try assertRustAssetMatchesSwift(heifURL)

        let noXMP = Data(repeating: 0x7f, count: 32)
        let noXMPURL = try temporaryFile(noXMP)
        defer { try? FileManager.default.removeItem(at: noXMPURL) }
        XCTAssertNil(try AndroidMotionPhotoParser.parse(url: noXMPURL))
        XCTAssertEqual(try runRust(noXMPURL)["status"] as? String, "none")

        let dtdXMP = """
        <x:xmpmeta xmlns:x="adobe:ns:meta/">
          <!DOCTYPE rdf:RDF [<!ENTITY injected "MotionPhoto">]>
          <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
            <rdf:Description xmlns:Camera="http://ns.google.com/photos/1.0/camera/" Camera:MotionPhoto="1"/>
          </rdf:RDF>
        </x:xmpmeta>
        """
        let dtdURL = try temporaryFile(Data(dtdXMP.utf8) + video)
        defer { try? FileManager.default.removeItem(at: dtdURL) }
        try assertRustErrorMatchesSwift(dtdURL)

        let missingVersion = jpegMotionPhoto(
            xmp: standardXMP(videoLength: Int64(video.count), includeVersion: false),
            video: video
        )
        let missingVersionURL = try temporaryFile(missingVersion)
        defer { try? FileManager.default.removeItem(at: missingVersionURL) }
        try assertRustErrorMatchesSwift(missingVersionURL)

        let stale = jpegMotionPhoto(xmp: standardXMP(videoLength: 32), video: Data(repeating: 0, count: 32))
        let staleURL = try temporaryFile(stale)
        defer { try? FileManager.default.removeItem(at: staleURL) }
        try assertRustErrorMatchesSwift(staleURL)

        let trailingItem = """
        <rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="application/octet-stream" Item:Semantic="Auxiliary" Item:Length="0" Item:Padding="0"/></rdf:li>
        """
        let notLast = jpegMotionPhoto(
            xmp: standardXMP(videoLength: Int64(video.count), trailingItems: trailingItem),
            video: video
        )
        let notLastURL = try temporaryFile(notLast)
        defer { try? FileManager.default.removeItem(at: notLastURL) }
        try assertRustErrorMatchesSwift(notLastURL)

        let hugeLength = jpegMotionPhoto(
            xmp: standardXMP(videoLength: Int64.max),
            video: video
        )
        let hugeLengthURL = try temporaryFile(hugeLength)
        defer { try? FileManager.default.removeItem(at: hugeLengthURL) }
        try assertRustErrorMatchesSwift(hugeLengthURL)

        let badHEIFXMP = standardXMP(
            videoLength: Int64(motionLength),
            primaryMime: "image/heic",
            primaryPadding: 0
        )
        let badHEIF = heifFTYP + makeBox("free", payload: Data(badHEIFXMP.utf8))
            + makeBox("mpvd", payload: innerVideo) + sefd
        let badHEIFURL = try temporaryFile(badHEIF, suffix: "heic")
        defer { try? FileManager.default.removeItem(at: badHEIFURL) }
        try assertRustErrorMatchesSwift(badHEIFURL)

        var oversizedXMP = Data("<x:xmpmeta".utf8)
        oversizedXMP.append(Data(repeating: 0x41, count: 4 * 1024 * 1024))
        oversizedXMP.append(Data("</x:xmpmeta>".utf8))
        let oversizedURL = try temporaryFile(oversizedXMP)
        defer { try? FileManager.default.removeItem(at: oversizedURL) }
        try assertRustErrorMatchesSwift(oversizedURL)
    }
}
