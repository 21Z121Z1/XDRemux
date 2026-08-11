import Foundation
import XCTest
@testable import XDRemuxCore

final class MotionPhotoSecurityTests: XCTestCase {
    func testRejectsDTDAndEntityDeclarations() throws {
        let video = fakeMP4()
        let xmp = """
        <x:xmpmeta xmlns:x="adobe:ns:meta/">
          <!DOCTYPE rdf:RDF [<!ENTITY injected "MotionPhoto">]>
          <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
            <rdf:Description xmlns:Camera="http://ns.google.com/photos/1.0/camera/"
                             xmlns:Container="http://ns.google.com/photos/1.0/container/"
                             xmlns:Item="http://ns.google.com/photos/1.0/container/item/"
                             Camera:MotionPhoto="1" Camera:MotionPhotoVersion="1">
              <Container:Directory><rdf:Seq>
                <rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="image/jpeg" Item:Semantic="Primary" Item:Length="0" Item:Padding="0"/></rdf:li>
                <rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="video/mp4" Item:Semantic="&injected;" Item:Length="\(video.count)" Item:Padding="0"/></rdf:li>
              </rdf:Seq></Container:Directory>
            </rdf:Description>
          </rdf:RDF>
        </x:xmpmeta>
        """
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-motion-dtd-\(UUID().uuidString).jpg")
        let jpeg = Data([0xff, 0xd8]) + Data(xmp.utf8) + Data([0xff, 0xd9])
        try (jpeg + video).write(to: url, options: .atomic)
        defer { try? FileManager.default.removeItem(at: url) }

        XCTAssertThrowsError(try AndroidMotionPhotoParser.parse(url: url)) { error in
            XCTAssertEqual(error as? MotionPhotoParsingError, .malformedXMP)
        }
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
