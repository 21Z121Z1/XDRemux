import Foundation
import XCTest
@testable import XDRemuxCore

final class MotionPhotoOppoSentinelRustConformanceTests: XCTestCase {
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

    private func fakeMP4() -> Data {
        var ftypPayload = Data("isom".utf8)
        ftypPayload.append(contentsOf: [0, 0, 0, 0])
        return makeBox("ftyp", payload: ftypPayload)
            + makeBox("mdat", payload: Data(repeating: 0x89, count: 128))
    }

    func testMinusOneXMPPresentationFallsBackToLpexCoverFrame() throws {
        let video = fakeMP4()
        let xmp = """
        <x:xmpmeta xmlns:x="adobe:ns:meta/">
          <rdf:RDF><rdf:Description xmlns:OpCamera="http://ns.oppo.com/photos/1.0/camera/"
                                    OpCamera:VideoLength="\(video.count)"
                                    GCamera:MotionPhotoPresentationTimestampUs="-1"/></rdf:RDF>
        </x:xmpmeta>
        """
        let lpex = "lpexLivePhotoExtension {\"version\":0,\"coverFramePts\":777777}"
        let data = Data([0xff, 0xd8]) + Data(xmp.utf8) + Data(lpex.utf8)
            + Data([0xff, 0xd9]) + video
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-oppo-sentinel-\(UUID().uuidString).jpg")
        try data.write(to: url, options: .atomic)
        defer { try? FileManager.default.removeItem(at: url) }

        let swift = try XCTUnwrap(OppoMotionPhotoParser.parse(url: url))
        XCTAssertEqual(swift.presentationTimestampUs, 777_777)
        XCTAssertEqual(swift.presentationSource, .oppoCoverFrame)

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
        let error = String(
            data: stderr.fileHandleForReading.readDataToEndOfFile(),
            encoding: .utf8
        ) ?? ""
        XCTAssertEqual(process.terminationStatus, 0, "Rust OPPO sentinel oracle failed: \(error)")
        let output = stdout.fileHandleForReading.readDataToEndOfFile()
        let object = try XCTUnwrap(
            try JSONSerialization.jsonObject(with: output) as? [String: Any]
        )
        XCTAssertEqual((object["presentationTimestampUs"] as? NSNumber)?.int64Value, 777_777)
        XCTAssertEqual(object["presentationSource"] as? String, "oppoCoverFrame")
    }
}
