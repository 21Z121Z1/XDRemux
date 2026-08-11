import Foundation
import XCTest
@testable import XDRemuxCore

final class MotionPhotoNamespaceTests: XCTestCase {
    func testParsesGContainerMotionPhotoV1() throws {
        let video = fakeMP4()
        let xmp = """
        <x:xmpmeta xmlns:x="adobe:ns:meta/">
          <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
            <rdf:Description xmlns:GCamera="http://ns.google.com/photos/1.0/camera/"
                             xmlns:GContainer="http://ns.google.com/photos/1.0/container/"
                             xmlns:GContainerItem="http://ns.google.com/photos/1.0/container/item/"
                             GCamera:MotionPhoto="1"
                             GCamera:MotionPhotoVersion="1"
                             GCamera:MotionPhotoPresentationTimestampUs="500000">
              <GContainer:Directory><rdf:Seq>
                <rdf:li rdf:parseType="Resource"><GContainer:Item GContainerItem:Mime="image/jpeg" GContainerItem:Semantic="Primary" GContainerItem:Length="0" GContainerItem:Padding="0"/></rdf:li>
                <rdf:li rdf:parseType="Resource"><GContainer:Item GContainerItem:Mime="video/mp4" GContainerItem:Semantic="MotionPhoto" GContainerItem:Length="\(video.count)" GContainerItem:Padding="0"/></rdf:li>
              </rdf:Seq></GContainer:Directory>
            </rdf:Description>
          </rdf:RDF>
        </x:xmpmeta>
        """
        let still = Data([0xFF, 0xD8]) + Data(xmp.utf8) + Data([0xFF, 0xD9])
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-gcontainer-\(UUID().uuidString).jpg")
        try (still + video).write(to: url, options: .atomic)
        defer { try? FileManager.default.removeItem(at: url) }

        let asset = try XCTUnwrap(AndroidMotionPhotoParser.parse(url: url))
        XCTAssertEqual(asset.sourceKind, .androidMotionPhotoV1)
        XCTAssertEqual(asset.presentationTimestampUs, 500_000)
        XCTAssertEqual(asset.videoResourceRange.lowerBound, Int64(still.count))
    }

    private func fakeMP4() -> Data {
        var data = Data([0, 0, 0, 16])
        data.append(Data("ftypisom".utf8))
        data.append(contentsOf: [0, 0, 2, 0])
        data.append(contentsOf: [0, 0, 0, 8])
        data.append(Data("mdat".utf8))
        return data
    }
}
