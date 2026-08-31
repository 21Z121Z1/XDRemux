import Foundation
import XDRemuxCore

enum FormatConformanceOracleError: Error, CustomStringConvertible {
    case invalid(String)

    var description: String {
        switch self {
        case .invalid(let message):
            return message
        }
    }
}

func fourCCHex(_ type: String) throws -> String {
    guard let bytes = type.data(using: .isoLatin1), bytes.count == 4 else {
        throw FormatConformanceOracleError.invalid("cannot encode FourCC \(type.debugDescription) as four Latin-1 bytes")
    }
    return bytes.map { String(format: "%02x", $0) }.joined()
}

func requiredBox(_ boxes: [ISOBMFFBox], type: String, context: String) throws -> ISOBMFFBox {
    guard let box = boxes.first(where: { $0.type == type }) else {
        throw FormatConformanceOracleError.invalid("\(context): required box \(type) is missing")
    }
    return box
}

func canonicalSummary(for url: URL) throws -> String {
    let data = try Data(contentsOf: url, options: [.mappedIfSafe])
    let top = isobmffBoxes(in: data, start: 0, end: data.count)
    guard !top.isEmpty else {
        throw FormatConformanceOracleError.invalid("top-level HEIF contains no boxes")
    }

    var lines: [String] = ["file\t\(data.count)"]
    for box in top {
        lines.append(
            "box\t\(try fourCCHex(box.type))\t\(box.boxStart)\t\(box.dataStart)\t\(box.dataEnd)\t\(box.size)"
        )
    }
    let parsedEnd = top.last.map { $0.boxStart + $0.size } ?? 0
    lines.append("trailer\t\(parsedEnd)\t\(data.count - parsedEnd)")

    let meta = try requiredBox(top, type: "meta", context: "top-level HEIF")
    let metaChildren = isobmffBoxes(in: data, start: meta.dataStart + 4, end: meta.dataEnd)
    let iloc = try requiredBox(metaChildren, type: "iloc", context: "meta")
    let iinf = try requiredBox(metaChildren, type: "iinf", context: "meta")
    let pitm = try requiredBox(metaChildren, type: "pitm", context: "meta")
    let iprp = try requiredBox(metaChildren, type: "iprp", context: "meta")
    let iref = metaChildren.first(where: { $0.type == "iref" })
    let iprpChildren = isobmffBoxes(in: data, start: iprp.dataStart, end: iprp.dataEnd)
    let ipmaBox = try requiredBox(iprpChildren, type: "ipma", context: "iprp")

    lines.append("primary\t\(try parseISOBMFFPITM(data, pitm))")

    let ilocEntries = try parseISOBMFFILoc(data, iloc).sorted { $0.itemID < $1.itemID }
    for entry in ilocEntries {
        let extents = entry.extents
            .map { "\($0.offset):\($0.length)" }
            .joined(separator: ",")
        lines.append(
            "iloc\t\(entry.itemID)\t\(entry.constructionMethod)\t\(entry.dataReferenceIndex)\t\(extents)"
        )
    }

    let itemInfos = parseISOBMFFItemInfos(data, iinf).items.sorted { $0.itemID < $1.itemID }
    for item in itemInfos {
        lines.append("iinf\t\(item.itemID)\t\(try fourCCHex(item.type))\t\(item.flags)")
    }

    let ipma = try parseISOBMFFIPMA(data, ipmaBox)
    for entry in ipma.entries.sorted(by: { $0.itemID < $1.itemID }) {
        let associations = entry.associations.map(String.init).joined(separator: ",")
        lines.append("ipma\t\(entry.itemID)\t\(associations)")
    }

    let refs = parseISOBMFFIRefs(data, iref)
    lines.append("iref-version\t\(refs.version)")
    let sortedRefs = try refs.refs.sorted { left, right in
        let leftType = try fourCCHex(left.type)
        let rightType = try fourCCHex(right.type)
        if leftType != rightType { return leftType < rightType }
        if left.from != right.from { return left.from < right.from }
        if left.to == right.to { return false }
        return left.to.lexicographicallyPrecedes(right.to)
    }
    for ref in sortedRefs {
        let targets = ref.to.map(String.init).joined(separator: ",")
        lines.append("iref\t\(try fourCCHex(ref.type))\t\(ref.from)\t\(targets)")
    }

    let properties = try parseISOBMFFIPCOProps(data, iprp)
    for index in properties.types.keys.sorted() {
        guard let type = properties.types[index] else { continue }
        if let size = properties.sizes[index] {
            lines.append("property\t\(index)\t\(try fourCCHex(type))\t\(size.0)\t\(size.1)")
        } else {
            lines.append("property\t\(index)\t\(try fourCCHex(type))")
        }
    }

    return lines.joined(separator: "\n") + "\n"
}

do {
    let arguments = Array(CommandLine.arguments.dropFirst())
    guard arguments.count == 1 else {
        throw FormatConformanceOracleError.invalid("usage: FormatConformanceOracle <heif-file>")
    }
    let summary = try canonicalSummary(for: URL(fileURLWithPath: arguments[0]))
    FileHandle.standardOutput.write(Data(summary.utf8))
} catch {
    FileHandle.standardError.write(Data("\(error)\n".utf8))
    exit(1)
}
