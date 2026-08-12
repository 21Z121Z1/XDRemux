import CoreGraphics
import Foundation

extension ColorOS16VisionCoverAlignmentAnalyzer {
    struct MatrixMetrics {
        let scaleX: Double
        let scaleY: Double
        let rotationDegrees: Double
        let translateX: Double
        let translateY: Double
        let perspectiveX: Double
        let perspectiveY: Double
    }

    static func mapToReferenceDimensions(
        _ matrix: [Double],
        floatingSize: CGSize,
        referenceSize: CGSize,
        outputReferenceDimensions: [Float]
    ) -> [Double]? {
        guard matrix.count == 9,
              matrix.allSatisfy(\.isFinite),
              floatingSize.width > 0, floatingSize.height > 0,
              referenceSize.width > 0, referenceSize.height > 0,
              outputReferenceDimensions.count == 2,
              outputReferenceDimensions.allSatisfy({ $0.isFinite && $0 > 0 }) else {
            return nil
        }
        let outputWidth = Double(outputReferenceDimensions[0])
        let outputHeight = Double(outputReferenceDimensions[1])
        let floatingScale = diagonal(
            horizontal: Double(floatingSize.width) / outputWidth,
            vertical: Double(floatingSize.height) / outputHeight
        )
        let referenceScale = diagonal(
            horizontal: Double(referenceSize.width) / outputWidth,
            vertical: Double(referenceSize.height) / outputHeight
        )
        guard let inverseReferenceScale = invert3x3(referenceScale) else { return nil }
        return normalize(
            multiply(multiply(inverseReferenceScale, matrix), floatingScale)
        )
    }

    static func medianMatrix(_ matrices: [[Double]]) -> [Double]? {
        guard !matrices.isEmpty, matrices.allSatisfy({ $0.count == 9 }) else { return nil }
        return normalize((0..<9).map { index in
            let values = matrices.map { $0[index] }.sorted()
            let middle = values.count / 2
            return values.count.isMultiple(of: 2)
                ? (values[middle - 1] + values[middle]) / 2
                : values[middle]
        })
    }

    static func matricesAgree(
        _ lhs: [Double],
        _ rhs: [Double],
        referenceDimensions: [Float]
    ) -> Bool {
        guard referenceDimensions.count == 2,
              let left = metrics(lhs),
              let right = metrics(rhs) else { return false }
        return abs(left.scaleX - right.scaleX) <= 0.04
            && abs(left.scaleY - right.scaleY) <= 0.04
            && abs(left.rotationDegrees - right.rotationDegrees) <= 1.5
            && abs(left.translateX - right.translateX) <= 0.04 * Double(referenceDimensions[0])
            && abs(left.translateY - right.translateY) <= 0.04 * Double(referenceDimensions[1])
    }

    static func metrics(_ matrix: [Double]) -> MatrixMetrics? {
        guard matrix.count == 9, matrix.allSatisfy(\.isFinite) else { return nil }
        return MatrixMetrics(
            scaleX: hypot(matrix[0], matrix[3]),
            scaleY: hypot(matrix[1], matrix[4]),
            rotationDegrees: atan2(matrix[3], matrix[0]) * 180 / .pi,
            translateX: matrix[2],
            translateY: matrix[5],
            perspectiveX: matrix[6],
            perspectiveY: matrix[7]
        )
    }

    private static func diagonal(horizontal: Double, vertical: Double) -> [Double] {
        [horizontal, 0, 0, 0, vertical, 0, 0, 0, 1]
    }

    private static func normalize(_ matrix: [Double]) -> [Double]? {
        guard matrix.count == 9,
              matrix.allSatisfy(\.isFinite),
              abs(matrix[8]) > 1e-12 else { return nil }
        let result = matrix.map { $0 / matrix[8] }
        return result.allSatisfy(\.isFinite) ? result : nil
    }

    private static func multiply(_ lhs: [Double], _ rhs: [Double]) -> [Double] {
        guard lhs.count == 9, rhs.count == 9 else { return lhs }
        return [
            lhs[0]*rhs[0]+lhs[1]*rhs[3]+lhs[2]*rhs[6],
            lhs[0]*rhs[1]+lhs[1]*rhs[4]+lhs[2]*rhs[7],
            lhs[0]*rhs[2]+lhs[1]*rhs[5]+lhs[2]*rhs[8],
            lhs[3]*rhs[0]+lhs[4]*rhs[3]+lhs[5]*rhs[6],
            lhs[3]*rhs[1]+lhs[4]*rhs[4]+lhs[5]*rhs[7],
            lhs[3]*rhs[2]+lhs[4]*rhs[5]+lhs[5]*rhs[8],
            lhs[6]*rhs[0]+lhs[7]*rhs[3]+lhs[8]*rhs[6],
            lhs[6]*rhs[1]+lhs[7]*rhs[4]+lhs[8]*rhs[7],
            lhs[6]*rhs[2]+lhs[7]*rhs[5]+lhs[8]*rhs[8]
        ]
    }

    private static func invert3x3(_ matrix: [Double]) -> [Double]? {
        guard matrix.count == 9 else { return nil }
        let first = matrix[0], second = matrix[1], third = matrix[2]
        let fourth = matrix[3], fifth = matrix[4], sixth = matrix[5]
        let seventh = matrix[6], eighth = matrix[7], ninth = matrix[8]
        let determinant = first * (fifth*ninth-sixth*eighth)
            - second * (fourth*ninth-sixth*seventh)
            + third * (fourth*eighth-fifth*seventh)
        guard determinant.isFinite, abs(determinant) > 1e-12 else { return nil }
        let scale = 1 / determinant
        return [
            (fifth*ninth-sixth*eighth)*scale,
            (third*eighth-second*ninth)*scale,
            (second*sixth-third*fifth)*scale,
            (sixth*seventh-fourth*ninth)*scale,
            (first*ninth-third*seventh)*scale,
            (third*fourth-first*sixth)*scale,
            (fourth*eighth-fifth*seventh)*scale,
            (second*seventh-first*eighth)*scale,
            (first*fifth-second*fourth)*scale
        ]
    }
}
