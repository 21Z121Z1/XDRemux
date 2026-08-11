import Foundation
import XCTest
@testable import XDRemuxCLI

final class MixedMotionPhotoBatchRoutingTests: XCTestCase {
    func testMixedHEICAndMotionPhotoDefersMotionOutputUntilAfterLegacyBatch() throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-mixed-motion-batch-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        // Only the filename/extension matters for planning the unchanged HEIC pass.
        try Data("not-a-real-heic".utf8).write(
            to: directory.appendingPathComponent("existing.heic"),
            options: .atomic
        )

        let motionURL = directory.appendingPathComponent("motion.jpg")
        try syntheticMotionPhoto().write(to: motionURL, options: .atomic)
        let motionOutput = directory.appendingPathComponent("motion.heic")

        let handled = try MotionPhotoCLIIntegration.handleIfNeeded([
            "batch",
            "--input-dir", directory.path,
        ])

        // False means XDRemuxCommand.main() must run the existing HEIC pass first. Crucially the
        // Motion Photo HEIC has not been written yet, so that pass cannot enumerate it as an input.
        XCTAssertFalse(handled)
        XCTAssertFalse(FileManager.default.fileExists(atPath: motionOutput.path))

        // Clear the queued plan. The deliberately minimal fake MP4 has no playable video track, so
        // the actual conversion is expected to fail after planning; the routing assertion above is
        // the behavior under test.
        XCTAssertThrowsError(try MotionPhotoCLIIntegration.finishPendingBatchIfNeeded())
        XCTAssertFalse(FileManager.default.fileExists(atPath: motionOutput.path))
    }

    private func syntheticMotionPhoto() -> Data {
        let video = fakeBMFF()
        let xmp = """
        <x:xmpmeta xmlns:x="adobe:ns:meta/">
          <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
            <rdf:Description xmlns:Camera="http://ns.google.com/photos/1.0/camera/"
                             xmlns:Container="http://ns.google.com/photos/1.0/container/"
                             xmlns:Item="http://ns.google.com/photos/1.0/container/item/"
                             Camera:MotionPhoto="1" Camera:MotionPhotoVersion="1">
              <Container:Directory><rdf:Seq>
                <rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="image/jpeg" Item:Semantic="Primary" Item:Length="0" Item:Padding="0"/></rdf:li>
                <rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="video/mp4" Item:Semantic="MotionPhoto" Item:Length="\(video.count)" Item:Padding="0"/></rdf:li>
              </rdf:Seq></Container:Directory>
            </rdf:Description>
          </rdf:RDF>
        </x:xmpmeta>
        """
        return Data([0xff, 0xd8]) + Data(xmp.utf8) + Data([0xff, 0xd9]) + video
    }

    private func fakeBMFF() -> Data {
        var data = Data([0, 0, 0, 16])
        data.append(Data("ftypisom".utf8))
        data.append(contentsOf: [0, 0, 0, 0])
        data.append(contentsOf: [0, 0, 0, 8])
        data.append(Data("mdat".utf8))
        return data
    }
}
