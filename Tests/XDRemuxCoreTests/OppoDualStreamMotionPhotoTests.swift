import Foundation
import XCTest
@testable import XDRemuxCore

final class OppoDualStreamMotionPhotoTests: XCTestCase {
    func testColorOS16VideoLengthMayPointToStream2ButParserKeepsBothStreams() throws {
        let stream1 = fakeMP4(brand: "isom", payloadByte: 0x11)
        let stream2 = fakeMP4(brand: "mp42", payloadByte: 0x22)
        let xmp = """
        <x:xmpmeta xmlns:x="adobe:ns:meta/">
          <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
            <rdf:Description xmlns:OpCamera="http://ns.oppo.com/photos/1.0/camera/"
                             OpCamera:VideoLength="\(stream2.count)"
                             GCamera:MotionPhotoPresentationTimestampUs="1634640"
                             xmlns:GCamera="http://ns.google.com/photos/1.0/camera/"/>
          </rdf:RDF>
        </x:xmpmeta>
        """
        let lpex = """
        lpexLivePhotoExtension {"version":1,"coverFramePts":1666666,"matrixCount":0,"videoSize":[1920,1080]}
        """
        let still = Data([0xff, 0xd8]) + Data(xmp.utf8) + Data(lpex.utf8) + Data([0xff, 0xd9])
        let stream1Start = Int64(still.count)
        let stream2Start = stream1Start + Int64(stream1.count)
        let url = try writeTemporary(still + stream1 + stream2)
        defer { try? FileManager.default.removeItem(at: url) }

        let asset = try XCTUnwrap(OppoMotionPhotoParser.parse(url: url))
        XCTAssertEqual(asset.sourceKind, .oppoLivePhoto)
        XCTAssertEqual(asset.presentationTimestampUs, 1_634_640)
        XCTAssertEqual(asset.presentationSource, .androidXMP)
        XCTAssertEqual(asset.vendorMetadata?.coverFramePtsUs, 1_666_666)
        XCTAssertEqual(asset.vendorMetadata?.streamCount, 2)
        XCTAssertEqual(asset.stillResourceRange.upperBound, stream1Start)
        XCTAssertEqual(asset.videoResourceRange.lowerBound, stream1Start)
        XCTAssertEqual(asset.videoResourceRange.upperBound, Int64(still.count + stream1.count + stream2.count))

        let primary = try OppoMotionPhotoStreamResolver.primaryVideoRange(for: asset)
        XCTAssertEqual(primary.lowerBound, stream1Start)
        XCTAssertEqual(primary.upperBound, stream2Start)
    }

    private func fakeMP4(brand: String, payloadByte: UInt8) -> Data {
        precondition(brand.utf8.count == 4)
        var data = Data([0, 0, 0, 16])
        data.append(Data("ftyp".utf8))
        data.append(Data(brand.utf8))
        data.append(contentsOf: [0, 0, 0, 0])
        data.append(contentsOf: [0, 0, 0, 12])
        data.append(Data("mdat".utf8))
        data.append(contentsOf: [payloadByte, payloadByte, payloadByte, payloadByte])
        return data
    }

    private func writeTemporary(_ data: Data) throws -> URL {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-oppo-dual-\(UUID().uuidString).jpg")
        try data.write(to: url, options: .atomic)
        return url
    }
}
