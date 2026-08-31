import Foundation
import XDRemuxCore

enum MetadataConformanceOracleError: Error, CustomStringConvertible {
    case invalid(String)

    var description: String {
        switch self {
        case .invalid(let message):
            return message
        }
    }
}

func hex(_ data: Data) -> String {
    data.map { String(format: "%02x", $0) }.joined()
}

let compatibilityModes: [(String, OppoCompatibility)] = [
    ("off", .off),
    ("auto", .auto),
    ("on", .on),
    ("tail", .tail),
    ("iso", .iso),
    ("iso-no-local", .isoNoLocal),
    ("iso-graph", .isoGraph),
]

func strictTmap(_ payload: Data) throws -> Data {
    guard payload.count == 62 || payload.count == 142 else {
        throw MetadataConformanceOracleError.invalid(
            "expected a 62- or 142-byte ImageIO tmap payload, got \(payload.count)"
        )
    }
    return payload.prefix(6) + Data([0, 0, 0]) + payload.dropFirst(6)
}

func appendUInt16LE(_ value: UInt16, to data: inout Data) {
    data.append(UInt8(value & 0xff))
    data.append(UInt8((value >> 8) & 0xff))
}

func appendUInt32LELocal(_ value: UInt32, to data: inout Data) {
    data.append(UInt8(value & 0xff))
    data.append(UInt8((value >> 8) & 0xff))
    data.append(UInt8((value >> 16) & 0xff))
    data.append(UInt8((value >> 24) & 0xff))
}

func syntheticExif(_ comment: String) -> Data {
    var userComment = Data("ASCII\0\0\0".utf8)
    userComment.append(Data(comment.utf8))

    var tiff = Data("II".utf8)
    appendUInt16LE(42, to: &tiff)
    appendUInt32LELocal(8, to: &tiff)

    appendUInt16LE(1, to: &tiff)
    appendUInt16LE(0x8769, to: &tiff)
    appendUInt16LE(4, to: &tiff)
    appendUInt32LELocal(1, to: &tiff)
    appendUInt32LELocal(26, to: &tiff)
    appendUInt32LELocal(0, to: &tiff)

    appendUInt16LE(1, to: &tiff)
    appendUInt16LE(0x9286, to: &tiff)
    appendUInt16LE(7, to: &tiff)
    appendUInt32LELocal(UInt32(userComment.count), to: &tiff)
    appendUInt32LELocal(44, to: &tiff)
    appendUInt32LELocal(0, to: &tiff)
    tiff.append(userComment)

    var exif = Data([0, 0, 0, 0])
    exif.append(tiff)
    return exif
}

func emitExtent(_ name: String, _ value: (offset: Int, length: Int)?, into lines: inout [String]) {
    if let value {
        lines.append("extent\t\(name)\t\(value.offset)\t\(value.length)")
    } else {
        lines.append("extent\t\(name)\tnil")
    }
}

func metadataVectors() throws -> String {
    var lines: [String] = []
    let routingSources: [(String, Int)] = [
        ("clear", localHDRFlag | 0x1234),
        ("all", oppoUltraHDRFlag | isoUltraHDRFlag | localHDRFlag | 0x1234),
    ]
    for (sourceName, source) in routingSources {
        for (modeName, mode) in compatibilityModes {
            lines.append("routing\t\(sourceName)\t\(modeName)\t\(targetOppoTagFlags(source, compatibility: mode))")
        }
    }

    let commentData = Data("ASCIIOplus_00000001".utf8)
    for (modeName, mode) in compatibilityModes {
        let adjusted = adjustedOppoUserComment(in: commentData, compatibility: mode) ?? "nil"
        lines.append("comment\t\(modeName)\t\(adjusted)")
    }

    let canonicalRatio = 4.926108360290527
    let canonical = [
        1.0, 1.0, 1.0, 1.0,
        canonicalRatio, canonicalRatio, canonicalRatio,
        1.0, 1.0, 1.0,
        0.0, 0.0, 0.0,
        0.0, 0.0, 0.0,
        1.0, canonicalRatio, canonicalRatio, 0.0,
    ]
    let distinct = [
        1.25, 1.5, 1.75, 1.0,
        4.0, 5.0, 6.0,
        0.8, 1.1, 1.2,
        0.01, 0.02, 0.03,
        0.04, 0.05, 0.06,
        1.5, 6.5, 2.0, 0.0,
    ]
    for (name, info) in [("canonical", canonical), ("distinct", distinct)] {
        let apple = makeAppleTmapPayload(infoFloats: info)
        let native = makeImageIONativeTmapPayload(infoFloats: info)
        lines.append("metadata\t\(name)\tapple-tmap\t\(hex(apple))")
        lines.append("metadata\t\(name)\tnative-tmap\t\(hex(native))")
        lines.append("metadata\t\(name)\tstrict-apple-tmap\t\(hex(try strictTmap(apple)))")
        lines.append("metadata\t\(name)\tstrict-native-tmap\t\(hex(try strictTmap(native)))")
        lines.append("metadata\t\(name)\thdrgm-xmp\t\(hex(makeHdrgmXMP(infoFloats: info)))")
    }

    let exif = syntheticExif("Oplus_00000001")
    var mdat = Data(repeating: 0x55, count: 13)
    mdat.append(exif)
    mdat.append(Data(repeating: 0x77, count: 11))
    let entry = ISOBMFFILocEntry(
        itemID: 7,
        constructionMethod: 0,
        dataReferenceIndex: 0,
        extents: [(offset: 1013, length: exif.count)]
    )
    guard let patchedComment = adjustedOppoUserComment(in: exif, compatibility: .on) else {
        throw MetadataConformanceOracleError.invalid("synthetic OPPO comment did not require activation")
    }
    guard let patch = applyOppoUserCommentPatch(
        &mdat,
        mdatDataStart: 1000,
        exifEntry: entry,
        patchedUserComment: patchedComment
    ) else {
        throw MetadataConformanceOracleError.invalid("synthetic OPPO UserComment patch failed")
    }
    lines.append(
        "patch\t\(patch.sourceRange.lowerBound)\t\(patch.sourceRange.upperBound)\t\(patch.delta)\t\(hex(mdat))"
    )
    emitExtent(
        "before",
        adjustedExtentForOppoUserCommentPatch((offset: 900, length: 20), patch: patch),
        into: &lines
    )
    emitExtent(
        "contains",
        adjustedExtentForOppoUserCommentPatch((offset: 1000, length: 200), patch: patch),
        into: &lines
    )
    emitExtent(
        "after",
        adjustedExtentForOppoUserCommentPatch((offset: 1200, length: 20), patch: patch),
        into: &lines
    )
    emitExtent(
        "partial",
        adjustedExtentForOppoUserCommentPatch(
            (offset: patch.sourceRange.lowerBound + 1, length: 20),
            patch: patch
        ),
        into: &lines
    )

    return lines.joined(separator: "\n") + "\n"
}

func metadataFixtureSummary(_ url: URL) throws -> String {
    let data = try Data(contentsOf: url, options: [.mappedIfSafe])
    var lines: [String] = []
    for (modeName, mode) in compatibilityModes {
        let adjusted = adjustedOppoUserComment(in: data, compatibility: mode) ?? "nil"
        lines.append("fixture\t\(modeName)\t\(adjusted)")
    }
    return lines.joined(separator: "\n") + "\n"
}

do {
    let arguments = Array(CommandLine.arguments.dropFirst())
    let output: String
    if arguments == ["--vectors"] {
        output = try metadataVectors()
    } else if arguments.count == 2, arguments[0] == "--fixture" {
        output = try metadataFixtureSummary(URL(fileURLWithPath: arguments[1]))
    } else {
        throw MetadataConformanceOracleError.invalid(
            "usage: MetadataConformanceOracle --vectors|--fixture <heif-file>"
        )
    }
    FileHandle.standardOutput.write(Data(output.utf8))
} catch {
    FileHandle.standardError.write(Data("\(error)\n".utf8))
    exit(1)
}
