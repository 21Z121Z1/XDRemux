import Foundation
import XDRemuxCore

enum ContainerConformanceOracleError: Error, CustomStringConvertible {
    case invalid(String)

    var description: String {
        switch self {
        case .invalid(let message): return message
        }
    }
}

func hex(_ data: Data) -> String {
    data.map { String(format: "%02x", $0) }.joined()
}

func bits(_ value: Double) -> String {
    String(format: "%016llx", value.bitPattern)
}

func resetDirectory(_ url: URL) throws {
    let manager = FileManager.default
    if manager.fileExists(atPath: url.path) {
        try manager.removeItem(at: url)
    }
    try manager.createDirectory(at: url, withIntermediateDirectories: true)
}

func writeSnapshot(inputURL: URL, outputURL: URL) throws {
    let data = try Data(contentsOf: inputURL, options: [.mappedIfSafe])
    let snapshot = try ContainerConformanceSupport.snapshot(from: data)
    try resetDirectory(outputURL)

    try snapshot.metaBytes.write(to: outputURL.appendingPathComponent("meta.bin"), options: .atomic)
    try snapshot.maskJPEGData.write(to: outputURL.appendingPathComponent("mask.bin"), options: .atomic)

    var lines: [String] = []
    lines.append("mode\t\(snapshot.mode)")
    lines.append("data-base\t\(snapshot.dataBase)")
    lines.append("manifest\t\(snapshot.extensionStart)\t\(snapshot.jsonStart)\t\(snapshot.jsonEnd)")
    if let local = snapshot.localHDRInfo {
        lines.append(
            "local-hdr\t\(bits(local.version))\t\(bits(local.length))\t\(bits(local.metaSize))\t\(bits(local.offset))"
        )
    } else {
        lines.append("local-hdr\tnone")
    }
    lines.append("meta-floats\t\(snapshot.metaFloats.map(bits).joined(separator: ","))")

    for entry in snapshot.entries {
        guard let nameData = entry.name.data(using: .utf8) else {
            throw ContainerConformanceOracleError.invalid("manifest entry name is not UTF-8")
        }
        lines.append(
            "entry\t\(entry.jsonOrder)\t\(hex(nameData))\t\(entry.offset)\t\(entry.length)\t\(entry.start)\t\(entry.end)"
        )
    }

    for (index, pair) in snapshot.portraitBlocks.sorted(by: { $0.key < $1.key }).enumerated() {
        guard let nameData = pair.key.data(using: .utf8) else {
            throw ContainerConformanceOracleError.invalid("block name is not UTF-8")
        }
        let filename = String(format: "block-%04d.bin", index)
        try pair.value.write(to: outputURL.appendingPathComponent(filename), options: .atomic)
        lines.append("block\t\(index)\t\(hex(nameData))\t\(pair.value.count)")
    }

    let summary = lines.joined(separator: "\n") + "\n"
    try Data(summary.utf8).write(to: outputURL.appendingPathComponent("summary.tsv"), options: .atomic)
}

do {
    let arguments = Array(CommandLine.arguments.dropFirst())
    guard arguments.count == 2 else {
        throw ContainerConformanceOracleError.invalid(
            "usage: ContainerConformanceOracle <input-file> <output-directory>"
        )
    }
    try writeSnapshot(
        inputURL: URL(fileURLWithPath: arguments[0]),
        outputURL: URL(fileURLWithPath: arguments[1])
    )
} catch {
    FileHandle.standardError.write(Data("\(error)\n".utf8))
    exit(1)
}
