import Foundation
import XCTest
@testable import XDRemuxCore

final class MotionPhotoParserTests: XCTestCase {
    func testParsesAndroidMotionPhotoV1Directory() throws {
        let video = fakeMP4()
        let xmp = motionPhotoXMP(videoLength: video.count, timestampUs: 1_417_000)
        let still = Data([0xFF, 0xD8]) + Data(xmp.utf8) + Data([0xFF, 0xD9])
        let url = try writeTemporary(still + video)
        defer { try? FileManager.default.removeItem(at: url) }

        let asset = try XCTUnwrap(AndroidMotionPhotoParser.parse(url: url))
        XCTAssertEqual(asset.sourceKind, .androidMotionPhotoV1)
        XCTAssertEqual(asset.presentationTimestampUs, 1_417_000)
        XCTAssertEqual(asset.presentationSource, .androidXMP)
        XCTAssertEqual(asset.stillResourceRange.lowerBound, 0)
        XCTAssertEqual(asset.stillResourceRange.upperBound, Int64(still.count))
        XCTAssertEqual(asset.videoResourceRange.lowerBound, Int64(still.count))
        XCTAssertEqual(asset.videoResourceRange.upperBound, Int64(still.count + video.count))
        XCTAssertEqual(asset.items.map(\.semantic), ["Primary", "MotionPhoto"])
    }

    func testParsesLegacyMicroVideoOffsetIntoNormalizedModel() throws {
        let video = fakeMP4()
        let xmp = legacyMicroVideoXMP(videoLength: video.count, timestampUs: 900_000)
        let still = Data([0xFF, 0xD8]) + Data(xmp.utf8) + Data([0xFF, 0xD9])
        let url = try writeTemporary(still + video)
        defer { try? FileManager.default.removeItem(at: url) }

        let asset = try XCTUnwrap(AndroidMotionPhotoParser.parse(url: url))
        XCTAssertEqual(asset.sourceKind, .legacyMicroVideoV1b)
        XCTAssertEqual(asset.presentationTimestampUs, 900_000)
        XCTAssertEqual(asset.presentationSource, .legacyMicroVideoXMP)
        XCTAssertEqual(asset.videoResourceRange.length, Int64(video.count))
    }

    func testKeepsPositiveLengthUltraHDRGainMapInsideStaticResource() throws {
        let video = fakeMP4()
        let gainMap = Data(repeating: 0xAB, count: 64)
        let xmp = motionPhotoXMP(
            videoLength: video.count,
            timestampUs: nil,
            gainMapLength: gainMap.count
        )
        let primaryJPEG = Data([0xFF, 0xD8]) + Data(xmp.utf8) + Data([0xFF, 0xD9])
        let stillContainer = primaryJPEG + gainMap
        let url = try writeTemporary(stillContainer + video)
        defer { try? FileManager.default.removeItem(at: url) }

        let asset = try XCTUnwrap(AndroidMotionPhotoParser.parse(url: url))
        XCTAssertEqual(asset.items.map(\.semantic), ["Primary", "GainMap", "MotionPhoto"])
        XCTAssertEqual(asset.items[1].length, Int64(gainMap.count))
        XCTAssertEqual(asset.stillResourceRange.upperBound, Int64(stillContainer.count))
        XCTAssertEqual(asset.videoResourceRange.lowerBound, Int64(stillContainer.count))
        XCTAssertNil(asset.presentationTimestampUs)
    }

    func testRejectsMissingVersionForMotionPhotoV1() throws {
        let video = fakeMP4()
        let xmp = motionPhotoXMP(videoLength: video.count, timestampUs: nil, version: nil)
        let url = try writeTemporary(Data([0xFF, 0xD8]) + Data(xmp.utf8) + Data([0xFF, 0xD9]) + video)
        defer { try? FileManager.default.removeItem(at: url) }

        XCTAssertThrowsError(try AndroidMotionPhotoParser.parse(url: url)) { error in
            XCTAssertEqual(error as? MotionPhotoParsingError, .unsupportedVersion(nil))
        }
    }

    func testRejectsStaleMotionPhotoXMPWithoutVideoPayload() throws {
        let xmp = motionPhotoXMP(videoLength: 32, timestampUs: nil)
        let url = try writeTemporary(Data([0xFF, 0xD8]) + Data(xmp.utf8) + Data([0xFF, 0xD9]) + Data(repeating: 0, count: 32))
        defer { try? FileManager.default.removeItem(at: url) }

        XCTAssertThrowsError(try AndroidMotionPhotoParser.parse(url: url)) { error in
            XCTAssertEqual(error as? MotionPhotoParsingError, .invalidVideoPayload)
        }
    }

    func testRejectsMotionPhotoItemThatIsNotLast() throws {
        let video = fakeMP4()
        let xmp = motionPhotoXMP(videoLength: video.count, timestampUs: nil, trailingAuxItem: true)
        let url = try writeTemporary(Data([0xFF, 0xD8]) + Data(xmp.utf8) + Data([0xFF, 0xD9]) + video)
        defer { try? FileManager.default.removeItem(at: url) }

        XCTAssertThrowsError(try AndroidMotionPhotoParser.parse(url: url)) { error in
            XCTAssertEqual(error as? MotionPhotoParsingError, .invalidMotionPhotoItem)
        }
    }

    func testRejectsOversizedDeclaredVideoLengthWithoutOverflow() throws {
        let video = fakeMP4()
        let xmp = motionPhotoXMP(videoLengthLiteral: String(Int64.max), timestampUs: nil)
        let url = try writeTemporary(Data([0xFF, 0xD8]) + Data(xmp.utf8) + Data([0xFF, 0xD9]) + video)
        defer { try? FileManager.default.removeItem(at: url) }

        XCTAssertThrowsError(try AndroidMotionPhotoParser.parse(url: url)) { error in
            XCTAssertTrue(
                error as? MotionPhotoParsingError == .arithmeticOverflow
                    || error as? MotionPhotoParsingError == .invalidByteRange
            )
        }
    }

    private func fakeMP4() -> Data {
        var data = Data()
        data.append(contentsOf: [0x00, 0x00, 0x00, 0x10])
        data.append(Data("ftyp".utf8))
        data.append(Data("isom".utf8))
        data.append(contentsOf: [0x00, 0x00, 0x02, 0x00])
        data.append(contentsOf: [0x00, 0x00, 0x00, 0x08])
        data.append(Data("mdat".utf8))
        return data
    }

    private func writeTemporary(_ data: Data) throws -> URL {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-motion-photo-\(UUID().uuidString).jpg")
        try data.write(to: url, options: .atomic)
        return url
    }

    private func motionPhotoXMP(
        videoLength: Int,
        timestampUs: Int64?,
        version: Int? = 1,
        gainMapLength: Int? = nil,
        trailingAuxItem: Bool = false
    ) -> String {
        motionPhotoXMP(
            videoLengthLiteral: String(videoLength),
            timestampUs: timestampUs,
            version: version,
            gainMapLength: gainMapLength,
            trailingAuxItem: trailingAuxItem
        )
    }

    private func motionPhotoXMP(
        videoLengthLiteral: String,
        timestampUs: Int64?,
        version: Int? = 1,
        gainMapLength: Int? = nil,
        trailingAuxItem: Bool = false
    ) -> String {
        let versionAttribute = version.map { " Camera:MotionPhotoVersion=\"\($0)\"" } ?? ""
        let timestampAttribute = timestampUs.map { " Camera:MotionPhotoPresentationTimestampUs=\"\($0)\"" } ?? ""
        let gainMap = gainMapLength.map {
            "<rdf:li rdf:parseType=\"Resource\"><Container:Item Item:Mime=\"image/jpeg\" Item:Semantic=\"GainMap\" Item:Length=\"\($0)\" Item:Padding=\"0\"/></rdf:li>"
        } ?? ""
        let trailing = trailingAuxItem
            ? "<rdf:li rdf:parseType=\"Resource\"><Container:Item Item:Mime=\"application/octet-stream\" Item:Semantic=\"Auxiliary\" Item:Length=\"0\" Item:Padding=\"0\"/></rdf:li>"
            : ""
        return """
        <x:xmpmeta xmlns:x="adobe:ns:meta/">
          <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
            <rdf:Description xmlns:Camera="http://ns.google.com/photos/1.0/camera/" xmlns:Container="http://ns.google.com/photos/1.0/container/" xmlns:Item="http://ns.google.com/photos/1.0/container/item/" Camera:MotionPhoto="1"\(versionAttribute)\(timestampAttribute)>
              <Container:Directory><rdf:Seq>
                <rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="image/jpeg" Item:Semantic="Primary" Item:Length="0" Item:Padding="0"/></rdf:li>
                \(gainMap)
                <rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="video/mp4" Item:Semantic="MotionPhoto" Item:Length="\(videoLengthLiteral)" Item:Padding="0"/></rdf:li>
                \(trailing)
              </rdf:Seq></Container:Directory>
            </rdf:Description>
          </rdf:RDF>
        </x:xmpmeta>
        """
    }

    private func legacyMicroVideoXMP(videoLength: Int, timestampUs: Int64) -> String {
        """
        <x:xmpmeta xmlns:x="adobe:ns:meta/">
          <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
            <rdf:Description xmlns:GCamera="http://ns.google.com/photos/1.0/camera/" GCamera:MicroVideo="1" GCamera:MicroVideoOffset="\(videoLength)" GCamera:MicroVideoPresentationTimestampUs="\(timestampUs)"/>
          </rdf:RDF>
        </x:xmpmeta>
        """
    }
}
