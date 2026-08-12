import CoreGraphics
import Foundation

extension ColorOS16VisionCoverAlignmentAnalyzer {
    static func makeEvidence(_ inputs: Inputs) -> Evidence {
        let coverFrames = inputs.primary.frames.filter {
            abs($0.ptsSeconds - inputs.coverSeconds) <= coverWindowSeconds
        }
        let offset = inputs.coverSeconds - inputs.auxiliary.durationSeconds
        let pairs = pairedFrames(
            primary: inputs.primary.frames,
            auxiliary: inputs.auxiliary.frames,
            offset: offset
        )
        let stillMatrices = coverFrames.compactMap {
            mappedMatrix(
                floating: $0.image,
                reference: inputs.still,
                referenceDimensions: inputs.referenceDimensions
            )
        }.filter(isUsable)
        let trajectoryMatrices = pairs.compactMap {
            mappedMatrix(
                floating: $0.primary.image,
                reference: $0.auxiliary.image,
                referenceDimensions: inputs.referenceDimensions
            )
        }.filter(isUsable)
        let coverTrajectoryMatrices = pairs.compactMap { pair -> [Double]? in
            guard abs(pair.primary.ptsSeconds - inputs.coverSeconds) <= coverWindowSeconds else {
                return nil
            }
            return mappedMatrix(
                floating: pair.primary.image,
                reference: pair.auxiliary.image,
                referenceDimensions: inputs.referenceDimensions
            )
        }.filter(isUsable)
        return Evidence(
            pairedFrameCount: pairs.count,
            coverFrameCount: coverFrames.count,
            stillMatrices: stillMatrices,
            trajectoryMatrices: trajectoryMatrices,
            coverTrajectoryMatrices: coverTrajectoryMatrices
        )
    }

    static func makeAlignment(
        evidence: Evidence,
        referenceDimensions: [Float]
    ) throws -> ColorOS16VisionCoverAlignment {
        guard evidence.coverFrameCount >= 3,
              evidence.stillMatrices.count >= 3,
              let stillMedian = medianMatrix(evidence.stillMatrices),
              evidence.coverTrajectoryMatrices.count >= 2,
              let stream2Median = medianMatrix(evidence.coverTrajectoryMatrices) else {
            throw ColorOS16VisionCoverAlignmentError.insufficientCoverEvidence
        }
        let requiredTrajectoryCount = max(
            3,
            Int(ceil(Double(evidence.pairedFrameCount) * 0.60))
        )
        guard evidence.trajectoryMatrices.count >= requiredTrajectoryCount,
              matricesAgree(
                stillMedian,
                stream2Median,
                referenceDimensions: referenceDimensions
              ),
              let transform = AppleLivePhotoStillTransform(
                matrix: stillMedian,
                referenceDimensions: referenceDimensions,
                source: .colorOS16VisionTrajectory
              ) else {
            throw ColorOS16VisionCoverAlignmentError.inconsistentTrajectory
        }
        return ColorOS16VisionCoverAlignment(
            transform: transform,
            pairedFrameCount: evidence.pairedFrameCount,
            usableTrajectoryFrameCount: evidence.trajectoryMatrices.count,
            coverFrameCount: evidence.coverFrameCount,
            usableCoverFrameCount: evidence.stillMatrices.count,
            coverStream1ToStream2: stream2Median
        )
    }

    private static func pairedFrames(
        primary: [Frame],
        auxiliary: [Frame],
        offset: Double
    ) -> [(primary: Frame, auxiliary: Frame)] {
        primary.compactMap { primaryFrame in
            guard let nearest = auxiliary.min(by: {
                abs(($0.ptsSeconds + offset) - primaryFrame.ptsSeconds)
                    < abs(($1.ptsSeconds + offset) - primaryFrame.ptsSeconds)
            }) else { return nil }
            let delta = abs((nearest.ptsSeconds + offset) - primaryFrame.ptsSeconds)
            guard delta <= maximumPairDeltaSeconds else { return nil }
            return (primaryFrame, nearest)
        }
    }

    private static func mappedMatrix(
        floating: CGImage,
        reference: CGImage,
        referenceDimensions: [Float]
    ) -> [Double]? {
        guard let estimate = try? VendorLivePhotoVisionHomographyEstimator.estimate(
            referenceImage: reference,
            floatingImage: floating
        ) else { return nil }
        return mapToReferenceDimensions(
            estimate.floatingToReference,
            floatingSize: CGSize(width: floating.width, height: floating.height),
            referenceSize: CGSize(width: reference.width, height: reference.height),
            outputReferenceDimensions: referenceDimensions
        )
    }

    private static func isUsable(_ matrix: [Double]) -> Bool {
        guard let value = metrics(matrix) else { return false }
        return (0.5...1.5).contains(value.scaleX)
            && (0.5...1.5).contains(value.scaleY)
            && abs(value.perspectiveX) < 0.005
            && abs(value.perspectiveY) < 0.005
    }
}
