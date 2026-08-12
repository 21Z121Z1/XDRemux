import Foundation
import CoreMedia

enum ColorOS16VisionCoverAlignmentError: Error {
    case missingImage
    case missingVideo
    case insufficientCoverEvidence
    case inconsistentTrajectory
}

struct ColorOS16VisionCoverAlignment: Sendable, Equatable {
    let transform: AppleLivePhotoStillTransform
    let pairedFrameCount: Int
    let usableTrajectoryFrameCount: Int
    let coverFrameCount: Int
    let usableCoverFrameCount: Int
    let coverStream1ToStream2: [Double]

    var diagnostic: String {
        let matrix = transform.matrix.map { String(format: "%.6f", $0) }
            .joined(separator: ",")
        let dimensions = transform.referenceDimensions.map { String(format: "%.0f", $0) }
            .joined(separator: "x")
        let coverCounts = "\(usableCoverFrameCount)/\(coverFrameCount) cover frames"
        let trajectoryCounts = "\(usableTrajectoryFrameCount)/\(pairedFrameCount) trajectory frames"
        return [
            "Vision Track5 cover alignment accepted",
            "\(coverCounts) and \(trajectoryCounts) passed geometry gates",
            "matrix=[\(matrix)]",
            "reference=\(dimensions)"
        ].joined(separator: "; ") + "."
    }
}

enum ColorOS16VisionCoverAlignmentAnalyzer {
    static let maximumAnalysisDimension = 1_600
    static let coverWindowSeconds = 0.12
    static let maximumPairDeltaSeconds = 0.06

    struct Frame {
        let index: Int
        let ptsSeconds: Double
        let image: CGImage
    }

    struct VideoFrames {
        let frames: [Frame]
        let durationSeconds: Double
    }

    struct Inputs {
        let primary: VideoFrames
        let auxiliary: VideoFrames
        let still: CGImage
        let coverSeconds: Double
        let referenceDimensions: [Float]
    }

    struct Evidence {
        let pairedFrameCount: Int
        let coverFrameCount: Int
        let stillMatrices: [[Double]]
        let trajectoryMatrices: [[Double]]
        let coverTrajectoryMatrices: [[Double]]
    }

    static func analyze(
        primaryVideoURL: URL,
        auxiliaryVideoURL: URL,
        stillImageURL: URL,
        stillImageTime: CMTime,
        referenceDimensions: [Float]
    ) async throws -> ColorOS16VisionCoverAlignment {
        let inputs = try await loadInputs(
            primaryVideoURL: primaryVideoURL,
            auxiliaryVideoURL: auxiliaryVideoURL,
            stillImageURL: stillImageURL,
            stillImageTime: stillImageTime,
            referenceDimensions: referenceDimensions
        )
        return try makeAlignment(
            evidence: makeEvidence(inputs),
            referenceDimensions: referenceDimensions
        )
    }
}
