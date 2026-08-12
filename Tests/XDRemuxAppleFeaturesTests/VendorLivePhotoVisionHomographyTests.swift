import CoreGraphics
import XCTest
@testable import XDRemuxAppleFeatures

final class VendorLivePhotoVisionHomographyTests: XCTestCase {
    func testIdenticalTexturedImagesProduceNormalizedNearIdentityHomography() throws {
        let image = try makeTexturedImage(width: 256, height: 192)
        let result = try VendorLivePhotoVisionHomographyEstimator.estimate(
            referenceImage: image,
            floatingImage: image
        )
        let matrix = result.floatingToReference

        XCTAssertEqual(matrix.count, 9)
        XCTAssertTrue(matrix.allSatisfy(\.isFinite))
        XCTAssertEqual(matrix[8], 1.0, accuracy: 1e-9)
        XCTAssertEqual(matrix[0], 1.0, accuracy: 1e-3)
        XCTAssertEqual(matrix[4], 1.0, accuracy: 1e-3)
        XCTAssertEqual(matrix[1], 0.0, accuracy: 1e-3)
        XCTAssertEqual(matrix[3], 0.0, accuracy: 1e-3)
        XCTAssertEqual(matrix[2], 0.0, accuracy: 0.5)
        XCTAssertEqual(matrix[5], 0.0, accuracy: 0.5)
        XCTAssertEqual(matrix[6], 0.0, accuracy: 1e-5)
        XCTAssertEqual(matrix[7], 0.0, accuracy: 1e-5)
    }

    private func makeTexturedImage(width: Int, height: Int) throws -> CGImage {
        let colorSpace = CGColorSpaceCreateDeviceRGB()
        guard let context = CGContext(
            data: nil,
            width: width,
            height: height,
            bitsPerComponent: 8,
            bytesPerRow: 0,
            space: colorSpace,
            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
        ) else {
            throw TestError.cannotCreateImage
        }

        context.setFillColor(CGColor(gray: 0.12, alpha: 1))
        context.fill(CGRect(x: 0, y: 0, width: width, height: height))

        for row in 0..<12 {
            for column in 0..<16 {
                let seed = row * 17 + column * 31
                let red = CGFloat((seed * 13) % 255) / 255.0
                let green = CGFloat((seed * 29 + 41) % 255) / 255.0
                let blue = CGFloat((seed * 47 + 83) % 255) / 255.0
                context.setFillColor(CGColor(red: red, green: green, blue: blue, alpha: 1))
                context.fill(
                    CGRect(
                        x: column * 16 + (row % 3),
                        y: row * 16 + (column % 5),
                        width: 10 + (seed % 6),
                        height: 9 + ((seed / 3) % 7)
                    )
                )
            }
        }

        context.setStrokeColor(CGColor(gray: 0.95, alpha: 1))
        context.setLineWidth(3)
        context.move(to: CGPoint(x: 11, y: 17))
        context.addLine(to: CGPoint(x: width - 19, y: height - 23))
        context.move(to: CGPoint(x: width - 31, y: 13))
        context.addLine(to: CGPoint(x: 23, y: height - 29))
        context.strokePath()

        guard let image = context.makeImage() else {
            throw TestError.cannotCreateImage
        }
        return image
    }

    private enum TestError: Error {
        case cannotCreateImage
    }
}
