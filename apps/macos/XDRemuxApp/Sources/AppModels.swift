import Foundation

// These are presentation-side models. Product policy and file semantics are
// owned by the Rust CLI; the app keeps only the values needed to render a
// queue, bind settings, and decode the CLI's stable JSON receipts.

struct ConversionConfig: Sendable, Equatable {
    var outputDirectory: URL?
    var fileNameSuffix = "_iso"
    var skipExisting = true
    var maxConcurrentJobs = max(1, min(ProcessInfo.processInfo.activeProcessorCount, 4))
    var categorizeOutputByCaptureMode = false
    var applePhotographicStyles = false
    var applePortrait = false

    var appleFeaturesEnabled: Bool {
        applePhotographicStyles || applePortrait
    }

    var oppoGalleryCompatibilityEnabled = false {
        didSet {
            if oppoGalleryCompatibilityEnabled {
                applePhotographicStyles = false
                applePortrait = false
            }
        }
    }
}

enum PhotoAssetType: String, CaseIterable, Identifiable, Sendable, Equatable {
    case staticPhoto = "static-photo"
    case livePhoto = "live-photo"

    var id: String { rawValue }

    var folderName: String {
        switch self {
        case .staticPhoto: return "静态照片"
        case .livePhoto: return "实况照片"
        }
    }
}

enum OppoCaptureMode: String, CaseIterable, Identifiable, Sendable, Equatable {
    case normal
    case master
    case ricohGR = "ricoh-gr"
    case professional
    case portrait
    case night
    case panorama
    case timeLapse = "time-lapse"
    case ultraHighResolution = "ultra-high-resolution"
    case idPhoto = "id-photo"
    case sticker
    case enhancedText = "enhanced-text"
    case groupPhoto = "group-photo"
    case doubleExposure = "double-exposure"
    case beauty

    var id: String { rawValue }

    var folderName: String {
        switch self {
        case .normal: return "普通拍照"
        case .master: return "大师模式"
        case .ricohGR: return "RICOH GR"
        case .professional: return "专业模式"
        case .portrait: return "人像"
        case .night: return "夜景"
        case .panorama: return "全景"
        case .timeLapse: return "延时摄影"
        case .ultraHighResolution: return "超清"
        case .idPhoto: return "证件照"
        case .sticker: return "贴纸"
        case .enhancedText: return "超级文本"
        case .groupPhoto: return "合影"
        case .doubleExposure: return "双重曝光"
        case .beauty: return "美颜"
        }
    }
}

enum OppoPhotoClassificationStatus: String, Sendable, Equatable {
    case categorized
    case missingUserComment = "missing-user-comment"
    case malformedUserComment = "malformed-user-comment"
    case unknownFlags = "unknown-flags"
    case unreadableImage = "unreadable-image"
}

struct PhotoClassification: Sendable, Equatable {
    let assetType: PhotoAssetType
    let mode: OppoCaptureMode?
    let status: OppoPhotoClassificationStatus
}

enum PhotoFolderProjection {
    static let unclassifiedFolderName = "未分类"
}

enum PhotoCategorizationDisposition: String, Sendable, Equatable {
    case copy
    case duplicate
    case copied
    case failed
    case dryRun = "dry-run"
}

struct PhotoCategorizationItem: Identifiable, Sendable, Equatable {
    let sourceURL: URL
    let destinationURL: URL
    let classification: PhotoClassification
    let disposition: PhotoCategorizationDisposition
    let errorDescription: String?

    var id: String { sourceURL.standardizedFileURL.path }
}

enum AppImageError: Error, CustomStringConvertible {
    case unableToLoad(URL)
    case unableToWrite(URL)

    var description: String {
        switch self {
        case .unableToLoad(let url): return "无法读取预览图像: \(url.path)"
        case .unableToWrite(let url): return "无法生成预览图像: \(url.path)"
        }
    }
}
