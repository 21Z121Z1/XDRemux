import Foundation

package struct ContainerConformanceManifestEntry {
    package let name: String
    package let offset: Int
    package let length: Int
    package let jsonOrder: Int
    package let start: Int
    package let end: Int
}

package struct ContainerConformanceLocalHDRInfo {
    package let version: Double
    package let length: Double
    package let metaSize: Double
    package let offset: Double
}

package struct ContainerConformanceSnapshot {
    package let mode: String
    package let metaBytes: Data
    package let metaFloats: [Double]
    package let localHDRInfo: ContainerConformanceLocalHDRInfo?
    package let maskJPEGData: Data
    package let extensionStart: Int
    package let jsonStart: Int
    package let jsonEnd: Int
    package let entries: [ContainerConformanceManifestEntry]
    package let dataBase: Int
    package let portraitBlocks: [String: Data]
}

package enum ContainerConformanceSupport {
    package static func snapshot(from data: Data) throws -> ContainerConformanceSnapshot {
        let extracted = try LHDRExtractor.extract(from: data)
        let blocks = try LHDRExtractor.portraitBlocks(from: data)
        let localInfo = extracted.localHDRInfo.map {
            ContainerConformanceLocalHDRInfo(
                version: $0.version,
                length: $0.length,
                metaSize: $0.metaSize,
                offset: $0.offset
            )
        }
        return ContainerConformanceSnapshot(
            mode: extracted.mode.rawValue,
            metaBytes: extracted.metaBytes,
            metaFloats: extracted.metaFloats,
            localHDRInfo: localInfo,
            maskJPEGData: extracted.maskJPEGData,
            extensionStart: extracted.manifestInfo.extensionStart,
            jsonStart: extracted.manifestInfo.jsonStart,
            jsonEnd: extracted.manifestInfo.jsonEnd,
            entries: extracted.manifestInfo.entries.map {
                ContainerConformanceManifestEntry(
                    name: $0.name,
                    offset: $0.offset,
                    length: $0.length,
                    jsonOrder: $0.jsonOrder,
                    start: $0.start,
                    end: $0.end
                )
            },
            dataBase: extracted.dataBase,
            portraitBlocks: blocks
        )
    }
}
