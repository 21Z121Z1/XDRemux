import Foundation
import XCTest
@testable import XDRemuxCore

final class HEIFMotionPhotoParserTests: XCTestCase {
    func testHEIFMotionPhotoExtractsOnlyMPVDPayloadWithTrailingVendorBox() throws {
        let videoPayload = fakeMP4Payload()
        let trailingSEFD = bmffBox(type: "sefd", payload: Data(repeating: 0x5a, count: 24))
        let xmp = xmpPacket(
            primaryPadding: 8,
            motionLength: Int64(videoPayload.count + trailingSEFD.count),
            timestampUs: 1_540_401
        )
        let prefix = bmffBox(type: "ftyp", payload: Data("heic\0\0\0\0heicmif1".utf8))
            + bmffBox(type: "free", payload: Data(xmp.utf8))
        let mpvdOffset = Int64(prefix.count)
        let mpvd = bmffBox(type: "mpvd", payload: videoPayload)
        let payloadStart = mpvdOffset + 8
        let payloadEnd = payloadStart + Int64(videoPayload.count)
        let data = prefix + mpvd + trailingSEFD
        let url = try writeTemporary(data, extension: "heic")
        defer { try? FileManager.default.removeItem(at: url) }

        let asset = try XCTUnwrap(AndroidMotionPhotoParser.parse(url: url))
        XCTAssertEqual(asset.sourceKind, .androidHeifMotionPhotoV1)
        XCTAssertEqual(asset.presentationTimestampUs, 1_540_401)
        XCTAssertEqual(asset.stillResourceRange.lowerBound, 0)
        XCTAssertEqual(asset.stillResourceRange.upperBound, mpvdOffset)
        XCTAssertEqual(asset.videoResourceRange.lowerBound, payloadStart)
        XCTAssertEqual(asset.videoResourceRange.upperBound, payloadEnd)
        XCTAssertLessThan(asset.videoResourceRange.upperBound, Int64(data.count))
    }

    func testHEIFMotionPhotoRejectsNonEightBytePrimaryPadding() throws {
        let videoPayload = fakeMP4Payload()
        let xmp = xmpPacket(
            primaryPadding: 0,
            motionLength: Int64(videoPayload.count),
            timestampUs: 1_000_000
        )
        let prefix = bmffBox(type: "ftyp", payload: Data("heic\0\0\0\0heicmif1".utf8))
            + bmffBox(type: "free", payload: Data(xmp.utf8))
        let data = prefix + bmffBox(type: "mpvd", payload: videoPayload)
        let url = try writeTemporary(data, extension: "heic")
        defer { try? FileManager.default.removeItem(at: url) }

        XCTAssertThrowsError(try AndroidMotionPhotoParser.parse(url: url)) { error in
            XCTAssertEqual(error as? MotionPhotoParsingError, .invalidItemLength)
        }
    }

    func testHEIFMotionPhotoRejectsDirectoryStartThatDoesNotMatchMPVDPayload() throws {
        let videoPayload = fakeMP4Payload()
        let xmp = xmpPacket(
            primaryPadding: 8,
            motionLength: Int64(videoPayload.count - 1),
            timestampUs: 1_000_000
        )
        let prefix = bmffBox(type: "ftyp", payload: Data("heic\0\0\0\0heicmif1".utf8))
            + bmffBox(type: "free", payload: Data(xmp.utf8))
        let data = prefix + bmffBox(type: "mpvd", payload: videoPayload)
        let url = try writeTemporary(data, extension: "heic")
        defer { try? FileManager.default.removeItem(at: url) }

        XCTAssertThrowsError(try AndroidMotionPhotoParser.parse(url: url)) { error in
            XCTAssertEqual(error as? MotionPhotoParsingError, .invalidByteRange)
        }
    }

    private func xmpPacket(
        primaryPadding: Int64,
        motionLength: Int64,
        timestampUs: Int64
    ) -> String {
        """
        <x:xmpmeta xmlns:x="adobe:ns:meta/">
          <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
            <rdf:Description xmlns:GCamera="http://ns.google.com/photos/1.0/camera/"
                             xmlns:GContainer="http://ns.google.com/photos/1.0/container/"
                             xmlns:GContainerItem="http://ns.google.com/photos/1.0/container/item/"
                             GCamera:MotionPhoto="1"
                             GCamera:MotionPhotoVersion="1"
                             GCamera:MotionPhotoPresentationTimestampUs="\(timestampUs)">
              <GContainer:Directory><rdf:Seq>
                <rdf:li rdf:parseType="Resource"><GContainer:Item GContainerItem:Mime="image/heic" GContainerItem:Semantic="Primary" GContainerItem:Length="0" GContainerItem:Padding="\(primaryPadding)"/></rdf:li>
                <rdf:li rdf:parseType="Resource"><GContainer:Item GContainerItem:Mime="image/heic" GContainerItem:Semantic="GainMap" GContainerItem:Length="0" GContainerItem:Padding="0"/></rdf:li>
                <rdf:li rdf:parseType="Resource"><GContainer:Item GContainerItem:Mime="video/mp4" GContainerItem:Semantic="MotionPhoto" GContainerItem:Length="\(motionLength)" GContainerItem:Padding="0"/></rdf:li>
              </rdf:Seq></GContainer:Directory>
            </rdf:Description>
          </rdf:RDF>
        </x:xmpmeta>
        """
    }

    private func fakeMP4Payload() -> Data {
        bmffBox(type: "ftyp", payload: Data("isom\0\0\0\0isommp42".utf8))
            + bmffBox(type: "mdat", payload: Data(repeating: 0x11, count: 16))
    }

    private func bmffBox(type: String, payload: Data) -> Data {
        precondition(type.utf8.count == 4)
        let size = UInt32(8 + payload.count)
        var data = Data([
            UInt8((size >> 24) & 0xff),
            UInt8((size >> 16) & 0xff),
            UInt8((size >> 8) & 0xff),
            UInt8(size & 0xff),
        ])
        data.append(Data(type.utf8))
        data.append(payload)
        return data
    }

    private func writeTemporary(_ data: Data, extension ext: String) throws -> URL {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-heif-motion-\(UUID().uuidString).\(ext)")
        try data.write(to: url, options: .atomic)
        return url
    }
}
