import Foundation
import XCTest
@testable import XDRemuxCore

/// Every field count inside `iloc`, `iinf`, `ipma`, and `pitm` comes from the
/// file being read. These contracts pin the behaviour on inputs that lie about
/// those counts: the parsers must report `invalidContainer`, never index past
/// the box and trap the process.
final class ISOBMFFHardeningTests: XCTestCase {
    private func box(_ type: String, payload: Data) -> (Data, ISOBMFFBox) {
        let data = makeBox(type, payload: payload)
        return (data, ISOBMFFBox(
            type: type,
            dataStart: 8,
            dataEnd: data.count,
            boxStart: 0,
            size: data.count
        ))
    }

    private func assertInvalidContainer(
        _ expression: @autoclosure () throws -> some Any,
        _ message: String,
        file: StaticString = #filePath,
        line: UInt = #line
    ) {
        XCTAssertThrowsError(try expression(), message, file: file, line: line) { error in
            guard case XDRemuxError.invalidContainer = error else {
                return XCTFail("expected invalidContainer, got \(error)", file: file, line: line)
            }
        }
    }

    func testTruncatedILocIsRejected() {
        // version 0, offset/length/base sizes 4, item_count 1, then nothing.
        var payload = Data([0, 0, 0, 0])
        payload.append(contentsOf: [0x44, 0x40])
        payload.append(contentsOf: [0x00, 0x01])
        let (data, iloc) = box("iloc", payload: payload)

        assertInvalidContainer(
            try parseISOBMFFILoc(data, iloc),
            "an iloc promising one entry but carrying none must be rejected"
        )
    }

    func testILocEntryCountLargerThanTheBoxIsRejected() {
        var payload = Data([0, 0, 0, 0])
        payload.append(contentsOf: [0x44, 0x40])
        payload.append(contentsOf: [0xff, 0xff])
        let (data, iloc) = box("iloc", payload: payload)

        assertInvalidContainer(
            try parseISOBMFFILoc(data, iloc),
            "an iloc claiming 65535 entries must not read past the box"
        )
    }

    func testTruncatedInfeChildIsRejected() {
        // iinf version 0, entry_count 1, followed by an infe that declares
        // version 3 (32-bit item_ID) but ends before the item_ID.
        var payload = Data([0, 0, 0, 0])
        payload.append(contentsOf: [0x00, 0x01])
        payload.append(makeBox("infe", payload: Data([3, 0, 0, 0])))
        let (data, iinf) = box("iinf", payload: payload)

        assertInvalidContainer(
            try parseISOBMFFIInf(data, iinf),
            "an infe that ends before its item_ID must be rejected"
        )
    }

    func testTruncatedIPMAIsRejected() {
        // version 0, flags 0, entry_count 2, no association records follow.
        var payload = Data([0, 0, 0, 0])
        payload.append(contentsOf: [0x00, 0x00, 0x00, 0x02])
        let (data, ipma) = box("ipma", payload: payload)

        assertInvalidContainer(
            try parseISOBMFFIPMA(data, ipma),
            "an ipma promising two entries but carrying none must be rejected"
        )
    }

    func testTruncatedPITMIsRejected() {
        let (data, pitm) = box("pitm", payload: Data([0, 0, 0, 0]))

        assertInvalidContainer(
            try parseISOBMFFPITM(data, pitm),
            "a pitm with no item_ID must be rejected"
        )
    }

    func testZeroLargesizeStopsTheBoxWalkInsteadOfLooping() {
        var data = Data()
        appendUInt32BE(1, to: &data)
        data.append(Data("free".utf8))
        appendUInt32BE(0, to: &data)
        appendUInt32BE(0, to: &data)

        XCTAssertTrue(
            isobmffBoxes(in: data, start: 0, end: data.count).isEmpty,
            "a largesize of zero cannot advance the cursor and must end the walk"
        )
    }

    func testVendorBoxTypesWithHighBytesRoundTrip() {
        // Box types are read as ISO Latin-1; re-encoding as ASCII used to
        // force-unwrap nil and trap on any preserved vendor 4CC >= 0x80.
        let raw = Data([0xa9, 0x54, 0x4f, 0x4f])
        let type = String(data: raw, encoding: .isoLatin1)!

        let emitted = makeBox(type, payload: Data([1, 2, 3]))

        XCTAssertEqual(emitted.subdata(in: 4..<8), raw)
        XCTAssertEqual(emitted.count, 11)
    }
}
