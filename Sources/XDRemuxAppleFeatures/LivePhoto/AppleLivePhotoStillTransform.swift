import Foundation

public struct AppleLivePhotoStillTransform: Sendable, Equatable {
    public enum Source: String, Sendable, Equatable {
        case colorOS16VisionTrajectory
        case oppoMetadata
    }

    public let matrix: [Double]
    public let referenceDimensions: [Float]
    public let source: Source

    public init?(
        matrix: [Double],
        referenceDimensions: [Float],
        source: Source
    ) {
        guard matrix.count == 9,
              matrix.allSatisfy(\.isFinite),
              abs(matrix[8]) > 1e-12,
              referenceDimensions.count == 2,
              referenceDimensions.allSatisfy({ $0.isFinite && $0 > 0 }) else {
            return nil
        }
        self.matrix = matrix.map { $0 / matrix[8] }
        self.referenceDimensions = referenceDimensions
        self.source = source
    }
}
