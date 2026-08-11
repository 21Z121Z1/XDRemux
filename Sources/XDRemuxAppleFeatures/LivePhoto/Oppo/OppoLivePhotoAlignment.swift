import Foundation
import XDRemuxCore

public enum OppoLivePhotoAlignment {
    private static let colorOS16EISCompensationScale = 0.90

    public static func transformMatrix(for metadata: OppoMotionPhotoMetadata) -> [Double]? {
        if metadata.version >= 1 {
            var result: [Double] = [
                colorOS16EISCompensationScale, 0, 0,
                0, colorOS16EISCompensationScale, 0,
                0, 0, 1,
            ]
            if let crop = metadata.photoCropMatrix,
               let inverseCrop = invert3x3(crop) {
                result = multiply(result, inverseCrop)
            }
            if let eis = metadata.photoEisMatrix,
               let inverseEIS = invert3x3(eis) {
                result = multiply(result, inverseEIS)
            }
            return isIdentity(result) ? nil : result
        }

        guard metadata.matrixCount > 0, !metadata.matrices.isEmpty,
              let coverFramePts = metadata.coverFramePtsUs else {
            return nil
        }
        let selected: [Double]?
        if let exact = metadata.matrices[String(coverFramePts)] {
            selected = exact
        } else {
            let closest = metadata.matrices.keys
                .compactMap(Int64.init)
                .min { abs($0 - coverFramePts) < abs($1 - coverFramePts) }
            selected = closest.flatMap { metadata.matrices[String($0)] }
        }
        guard let matrix = selected, matrix.count == 9 else { return nil }
        let result = invert3x3(matrix) ?? matrix
        return isIdentity(result) ? nil : result
    }

    public static func encodeForApple(_ matrix: [Double]) -> Data {
        guard matrix.count == 9 else { return Data() }
        var output = Data(capacity: 72)
        for value in matrix {
            var bits = value.bitPattern.bigEndian
            withUnsafeBytes(of: &bits) { output.append(contentsOf: $0) }
        }
        return output
    }

    public static func referenceDimensions(for metadata: OppoMotionPhotoMetadata) -> [Float]? {
        guard let width = metadata.videoWidth, let height = metadata.videoHeight,
              width > 0, height > 0 else { return nil }
        return [Float(width), Float(height)]
    }

    static func isIdentity(_ matrix: [Double], tolerance: Double = 1e-6) -> Bool {
        guard matrix.count == 9 else { return false }
        let identity: [Double] = [1, 0, 0, 0, 1, 0, 0, 0, 1]
        return zip(matrix, identity).allSatisfy { abs($0 - $1) <= tolerance }
    }

    static func multiply(_ a: [Double], _ b: [Double]) -> [Double] {
        guard a.count == 9, b.count == 9 else { return a }
        return [
            a[0]*b[0] + a[1]*b[3] + a[2]*b[6],
            a[0]*b[1] + a[1]*b[4] + a[2]*b[7],
            a[0]*b[2] + a[1]*b[5] + a[2]*b[8],
            a[3]*b[0] + a[4]*b[3] + a[5]*b[6],
            a[3]*b[1] + a[4]*b[4] + a[5]*b[7],
            a[3]*b[2] + a[4]*b[5] + a[5]*b[8],
            a[6]*b[0] + a[7]*b[3] + a[8]*b[6],
            a[6]*b[1] + a[7]*b[4] + a[8]*b[7],
            a[6]*b[2] + a[7]*b[5] + a[8]*b[8],
        ]
    }

    static func invert3x3(_ m: [Double]) -> [Double]? {
        guard m.count == 9 else { return nil }
        let a = m[0], b = m[1], c = m[2]
        let d = m[3], e = m[4], f = m[5]
        let g = m[6], h = m[7], i = m[8]
        let determinant = a * (e * i - f * h)
            - b * (d * i - f * g)
            + c * (d * h - e * g)
        guard determinant.isFinite, abs(determinant) > 1e-10 else { return nil }
        let inv = 1.0 / determinant
        return [
            (e*i-f*h)*inv, (c*h-b*i)*inv, (b*f-c*e)*inv,
            (f*g-d*i)*inv, (a*i-c*g)*inv, (c*d-a*f)*inv,
            (d*h-e*g)*inv, (b*g-a*h)*inv, (a*e-b*d)*inv,
        ]
    }
}
