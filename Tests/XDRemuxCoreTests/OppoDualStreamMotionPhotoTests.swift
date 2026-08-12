import Foundation
import XCTest
@testable import XDRemuxCore

final class OppoDualStreamMotionPhotoTests: XCTestCase {
    func testColorOS16VideoLengthMayPointToStream2ButFallbackKeepsBothStreams() throws {
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

        try assertDualStreamAsset(
            url: url,
            stream1Start: stream1Start,
            stream2Start: stream2Start,
            expectedTimestamp: 1_634_640
        )
    }

    func testColorOS16LpexOverridesStandardDirectoryThatDescribesOnlyFinalStream() throws {
        let stream1 = fakeMP4(brand: "isom", payloadByte: 0x33)
        let stream2 = fakeMP4(brand: "mp42", payloadByte: 0x44)
        let xmp = """
        <x:xmpmeta xmlns:x="adobe:ns:meta/">
          <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
            <rdf:Description xmlns:Camera="http://ns.google.com/photos/1.0/camera/"
                             xmlns:Container="http://ns.google.com/photos/1.0/container/"
                             xmlns:Item="http://ns.google.com/photos/1.0/container/item/"
                             Camera:MotionPhoto="1"
                             Camera:MotionPhotoVersion="1"
                             Camera:MotionPhotoPresentationTimestampUs="1634640">
              <Container:Directory><rdf:Seq>
                <rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="image/jpeg" Item:Semantic="Primary" Item:Length="0" Item:Padding="0"/></rdf:li>
                <rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="video/mp4" Item:Semantic="MotionPhoto" Item:Length="\(stream2.count)" Item:Padding="0"/></rdf:li>
              </rdf:Seq></Container:Directory>
            </rdf:Description>
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

        // The generic Android parser necessarily interprets bytes before Stream 2 as part of the
        // static resource because the standard directory only declares the final stream.
        let generic = try XCTUnwrap(AndroidMotionPhotoParser.parse(url: url))
        XCTAssertEqual(generic.videoResourceRange.lowerBound, stream2Start)
        XCTAssertEqual(generic.stillResourceRange.upperBound, stream2Start)

        // The OPPO vendor layer must correct that topology before ImageIO sees the still resource.
        try assertDualStreamAsset(
            url: url,
            stream1Start: stream1Start,
            stream2Start: stream2Start,
            expectedTimestamp: 1_634_640
        )
    }

    func testGenericAndroidLayoutNeverInventsAuxiliaryStream() throws {
        let primaryRange = try MotionPhotoByteRange(lowerBound: 100, upperBound: 200)
        let stillRange = try MotionPhotoByteRange(lowerBound: 0, upperBound: 100)
        let asset = MotionPhotoAsset(
            sourceURL: URL(fileURLWithPath: "/tmp/not-read-for-single-stream-layout.jpg"),
            sourceKind: .androidMotionPhotoV1,
            items: [
                MotionPhotoItem(mime: "image/jpeg", semantic: "Primary", length: 0, padding: 0),
                MotionPhotoItem(mime: "video/mp4", semantic: "MotionPhoto", length: 100, padding: 0),
            ],
            stillResourceRange: stillRange,
            videoResourceRange: primaryRange,
            presentationTimestampUs: nil,
            presentationSource: nil
        )

        let layout = try MotionPhotoVideoStreamLayoutResolver.resolve(for: asset)
        XCTAssertEqual(layout.primary.range, primaryRange)
        XCTAssertEqual(layout.primary.role, .primary)
        XCTAssertTrue(layout.auxiliaryGeometry.isEmpty)
    }

    private func assertDualStreamAsset(
        url: URL,
        stream1Start: Int64,
        stream2Start: Int64,
        expectedTimestamp: Int64
    ) throws {
        let asset = try XCTUnwrap(OppoMotionPhotoParser.parse(url: url))
        XCTAssertEqual(asset.sourceKind, .oppoLivePhoto)
        XCTAssertEqual(asset.presentationTimestampUs, expectedTimestamp)
        XCTAssertEqual(asset.presentationSource, .androidXMP)
        XCTAssertEqual(asset.vendorMetadata?.coverFramePtsUs, 1_666_666)
        XCTAssertEqual(asset.vendorMetadata?.streamCount, 2)
        XCTAssertEqual(asset.stillResourceRange.upperBound, stream1Start)
        XCTAssertEqual(asset.videoResourceRange.lowerBound, stream1Start)
        XCTAssertEqual(
            asset.videoResourceRange.upperBound,
            Int64((try? Data(contentsOf: url).count) ?? 0)
        )

        let primary = try OppoMotionPhotoStreamResolver.primaryVideoRange(for: asset)
        XCTAssertEqual(primary.lowerBound, stream1Start)
        XCTAssertEqual(primary.upperBound, stream2Start)

        let layout = try MotionPhotoVideoStreamLayoutResolver.resolve(for: asset)
        XCTAssertEqual(layout.primary.range, primary)
        XCTAssertEqual(layout.primary.role, .primary)
        XCTAssertEqual(layout.auxiliaryGeometry.count, 1)
        XCTAssertEqual(layout.auxiliaryGeometry[0].role, .auxiliaryGeometry)
        XCTAssertEqual(layout.auxiliaryGeometry[0].range.lowerBound, stream2Start)
        XCTAssertEqual(layout.auxiliaryGeometry[0].range.upperBound, asset.videoResourceRange.upperBound)
        XCTAssertEqual(
            try OppoMotionPhotoStreamResolver.auxiliaryGeometryVideoRanges(for: asset),
            layout.auxiliaryGeometry.map(\.range)
        )
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
