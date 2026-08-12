import Foundation
import CoreGraphics
import ImageIO
import UniformTypeIdentifiers
import XCTest
@testable import XDRemuxAppleFeatures

final class LivePhotoStillWriterMetadataTests: XCTestCase {
    func testMotionPhotoXMPIsExcludedAndEXIFIsPreserved() throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-livephoto-metadata-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        let cleanJPEG = directory.appendingPathComponent("clean.jpg")
        let sourceJPEG = directory.appendingPathComponent("motion-xmp.jpg")
        let outputHEIC = directory.appendingPathComponent("output.heic")
        let exifDate = "2026:08:11 12:34:56"
        try createJPEG(at: cleanJPEG, exifDate: exifDate)
        try injectMotionPhotoXMP(into: cleanJPEG, outputURL: sourceJPEG)

        let sourceXMP = try XCTUnwrap(xmpPacket(at: sourceJPEG))
        XCTAssertTrue(sourceXMP.contains("MotionPhoto"))

        let identifier = UUID().uuidString
        try AppleLivePhotoStillWriter.write(
            stillInputURL: sourceJPEG,
            outputURL: outputHEIC,
            assetIdentifier: identifier
        )

        let outputXMP = xmpPacket(at: outputHEIC) ?? ""
        XCTAssertFalse(outputXMP.contains("MotionPhoto"))
        XCTAssertFalse(outputXMP.contains("GContainer"))
        XCTAssertEqual(AppleLivePhotoStillWriter.assetIdentifier(in: outputHEIC), identifier)

        guard let source = CGImageSourceCreateWithURL(outputHEIC as CFURL, nil),
              let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any],
              let exif = properties[kCGImagePropertyExifDictionary] as? [CFString: Any] else {
            XCTFail("could not read output EXIF")
            return
        }
        XCTAssertEqual(exif[kCGImagePropertyExifDateTimeOriginal] as? String, exifDate)
    }

    /// Characterization-only guard for the private Apple MakerNote serialization that ImageIO
    /// itself emits when given MakerApple[17]. This keeps the portable writer derived from bytes
    /// produced by the platform oracle rather than from folklore about the MakerNote layout.
    func testCharacterizeNativeMakerNoteBytes() throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-native-makernote-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        let sourceJPEG = directory.appendingPathComponent("source.jpg")
        let outputHEIC = directory.appendingPathComponent("reference.heic")
        try createJPEG(at: sourceJPEG, exifDate: "2026:08:12 09:20:00")
        let identifier = "DF64C2AE-ED3C-4778-BFCA-C15277E521D2"
        try AppleLivePhotoStillWriter.write(
            stillInputURL: sourceJPEG,
            outputURL: outputHEIC,
            assetIdentifier: identifier
        )
        XCTAssertEqual(AppleLivePhotoStillWriter.assetIdentifier(in: outputHEIC), identifier)

        let data = try Data(contentsOf: outputHEIC)
        let signature = Data("Apple iOS".utf8)
        let range = try XCTUnwrap(data.range(of: signature), "ImageIO output contains no Apple iOS MakerNote signature")
        let start = range.lowerBound
        let end = min(data.count, start + 512)
        let bytes = data[start..<end]
        let hex = bytes.map { String(format: "%02x", $0) }.joined()
        print("REFERENCE_APPLE_MAKERNOTE_OFFSET=\(start)")
        print("REFERENCE_APPLE_MAKERNOTE_HEX=\(hex)")
    }

    private func createJPEG(at url: URL, exifDate: String) throws {
        let colorSpace = CGColorSpaceCreateDeviceRGB()
        guard let context = CGContext(
            data: nil,
            width: 48,
            height: 32,
            bitsPerComponent: 8,
            bytesPerRow: 0,
            space: colorSpace,
            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
        ),
        let destination = CGImageDestinationCreateWithURL(
            url as CFURL,
            UTType.jpeg.identifier as CFString,
            1,
            nil
        ) else {
            XCTFail("could not create source JPEG")
            return
        }
        context.setFillColor(CGColor(red: 0.35, green: 0.25, blue: 0.15, alpha: 1))
        context.fill(CGRect(x: 0, y: 0, width: 48, height: 32))
        guard let image = context.makeImage() else {
            XCTFail("could not create source image")
            return
        }
        let properties: [CFString: Any] = [
            kCGImagePropertyExifDictionary: [
                kCGImagePropertyExifDateTimeOriginal: exifDate,
            ] as CFDictionary,
        ]
        CGImageDestinationAddImage(destination, image, properties as CFDictionary)
        XCTAssertTrue(CGImageDestinationFinalize(destination))
    }

    private func injectMotionPhotoXMP(into inputURL: URL, outputURL: URL) throws {
        let original = try Data(contentsOf: inputURL)
        guard original.count >= 2, original[0] == 0xff, original[1] == 0xd8 else {
            XCTFail("generated fixture is not JPEG")
            return
        }
        let xmp = """
        <x:xmpmeta xmlns:x="adobe:ns:meta/">
          <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
            <rdf:Description xmlns:GCamera="http://ns.google.com/photos/1.0/camera/"
                             xmlns:GContainer="http://ns.google.com/photos/1.0/container/"
                             GCamera:MotionPhoto="1" GCamera:MotionPhotoVersion="1"/>
          </rdf:RDF>
        </x:xmpmeta>
        """
        var payload = Data("http://ns.adobe.com/xap/1.0/".utf8)
        payload.append(0)
        payload.append(Data(xmp.utf8))
        let segmentLength = payload.count + 2
        guard segmentLength <= Int(UInt16.max) else {
            XCTFail("test XMP APP1 segment is too large")
            return
        }
        var segment = Data([0xff, 0xe1])
        segment.append(UInt8((segmentLength >> 8) & 0xff))
        segment.append(UInt8(segmentLength & 0xff))
        segment.append(payload)

        var output = Data(original.prefix(2))
        output.append(segment)
        output.append(original.dropFirst(2))
        try output.write(to: outputURL, options: .atomic)
    }

    private func xmpPacket(at url: URL) -> String? {
        guard let source = CGImageSourceCreateWithURL(url as CFURL, nil),
              let metadata = CGImageSourceCopyMetadataAtIndex(source, 0, nil),
              let data = CGImageMetadataCreateXMPData(metadata, nil) else {
            return nil
        }
        return String(data: data as Data, encoding: .utf8)
    }
}
