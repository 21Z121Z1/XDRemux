import Foundation
import XCTest
@testable import XDRemuxCore

final class OppoMotionPhotoMetadataTests: XCTestCase {
    func testParsesLpexTransformFields() throws {
        let json = """
        {"version":1,"matrixCount":2,"coverFramePts":1433000,
         "photoCropMatrix":[1,0,0,0,1,0,0,0,1],
         "photoEisMatrix":[1,0,0,0,1,0,0,0,1],
         "matrices":{"1433000":[1,0,0,0,1,0,0,0,1]},
         "videoSize":[1728,1296],"originPhotoSize":[4096,3072],
         "photoCropFactor":0.9,"eisCropFactor":[0.9,0.9]}
        """
        let payload = Data("prefix lpexLivePhotoExtension \(json) suffix".utf8)
        let metadata = try XCTUnwrap(OppoLpexParser.parseFirstObject(in: payload))
        XCTAssertEqual(metadata.version, 1)
        XCTAssertEqual(metadata.matrixCount, 2)
        XCTAssertEqual(metadata.coverFramePtsUs, 1_433_000)
        XCTAssertEqual(metadata.photoCropMatrix?.count, 9)
        XCTAssertEqual(metadata.photoEisMatrix?.count, 9)
        XCTAssertEqual(metadata.videoWidth, 1728)
        XCTAssertEqual(metadata.videoHeight, 1296)
        XCTAssertEqual(metadata.originPhotoWidth, 4096)
        XCTAssertEqual(metadata.originPhotoHeight, 3072)
        XCTAssertEqual(metadata.photoCropFactor, 0.9)
    }

    func testRejectsNonFiniteMatrix() throws {
        let json = """
        {"version":1,"coverFramePts":1000,
         "photoCropMatrix":["nan",0,0,0,1,0,0,0,1]}
        """
        let payload = Data("lpexLivePhotoExtension \(json)".utf8)
        let metadata = try XCTUnwrap(OppoLpexParser.parseFirstObject(in: payload))
        XCTAssertNil(metadata.photoCropMatrix)
    }
}
