#!/usr/bin/env swift

import CoreGraphics
import CoreImage
import CoreVideo
import Foundation
import ImageIO
import UniformTypeIdentifiers
import Vision

struct Point: Codable {
    let x: Double
    let y: Double
}

struct SalientRect: Codable {
    let visionLowerLeftX: Double
    let visionLowerLeftY: Double
    let width: Double
    let height: Double
    let topLeftCenter: Point
}

struct FaceCandidate: Codable {
    let confidence: Float
    let topLeftX: Double
    let topLeftY: Double
    let width: Double
    let height: Double
    let attentionMean: Double
    let attentionPeak: Float
    let centerTopLeft: Point
    let eyesMidpointTopLeft: Point?
}

struct RecommendedFocus: Codable {
    let strategy: String
    let pointTopLeft: Point
}

struct SaliencyResult: Codable {
    let request: String
    let heatmapWidth: Int
    let heatmapHeight: Int
    let maximumValue: Float
    let maximumStoredTopLeft: Point
    let maximumDisplayTopLeft: Point
    let weightedCentroidDisplayTopLeft: Point
    let threshold: Float
    let salientObjects: [SalientRect]
}

struct Report: Codable {
    let input: String
    let storedWidth: Int
    let storedHeight: Int
    let exifOrientation: UInt32
    let displayWidth: Int
    let displayHeight: Int
    let attention: SaliencyResult
    let objectness: SaliencyResult
    let faces: [FaceCandidate]
    let recommendedFocus: RecommendedFocus
}

func displayPoint(storedX: Double, storedY: Double, orientation: UInt32) -> Point {
    switch orientation {
    case 3: return Point(x: 1 - storedX, y: 1 - storedY)
    case 6: return Point(x: 1 - storedY, y: storedX)
    case 8: return Point(x: storedY, y: 1 - storedX)
    default: return Point(x: storedX, y: storedY)
    }
}

func analyze(
    observation: VNSaliencyImageObservation,
    requestName: String,
    orientation: UInt32,
    heatmapURL: URL
) throws -> SaliencyResult {
    let buffer = observation.pixelBuffer
    guard CVPixelBufferGetPixelFormatType(buffer) == kCVPixelFormatType_OneComponent32Float else {
        fatalError("unexpected saliency pixel format")
    }
    CVPixelBufferLockBaseAddress(buffer, .readOnly)
    defer { CVPixelBufferUnlockBaseAddress(buffer, .readOnly) }
    guard let base = CVPixelBufferGetBaseAddress(buffer) else {
        fatalError("saliency buffer has no base address")
    }
    let width = CVPixelBufferGetWidth(buffer)
    let height = CVPixelBufferGetHeight(buffer)
    let stride = CVPixelBufferGetBytesPerRow(buffer) / MemoryLayout<Float>.stride
    let rows = base.assumingMemoryBound(to: Float.self)
    var finite: [Float] = []
    finite.reserveCapacity(width * height)
    var maximum = -Float.infinity
    var maximumX = 0
    var maximumY = 0
    for y in 0..<height {
        for x in 0..<width {
            let value = rows[y * stride + x]
            if value.isFinite {
                finite.append(value)
                if value > maximum {
                    maximum = value
                    maximumX = x
                    maximumY = y
                }
            }
        }
    }
    let sorted = finite.sorted()
    let thresholdIndex = min(sorted.count - 1, Int(Double(sorted.count - 1) * 0.90))
    let threshold = sorted[thresholdIndex]
    let objects = (observation.salientObjects ?? []).map { object in
        let box = object.boundingBox
        return SalientRect(
            visionLowerLeftX: box.minX,
            visionLowerLeftY: box.minY,
            width: box.width,
            height: box.height,
            topLeftCenter: Point(x: box.midX, y: 1 - box.midY)
        )
    }
    func isInsidePrimaryObject(_ point: Point) -> Bool {
        guard let object = objects.first else { return true }
        let top = 1 - object.visionLowerLeftY - object.height
        let bottom = 1 - object.visionLowerLeftY
        return point.x >= object.visionLowerLeftX
            && point.x <= object.visionLowerLeftX + object.width
            && point.y >= top
            && point.y <= bottom
    }
    var weight = 0.0
    var weightedX = 0.0
    var weightedY = 0.0
    var png = Data(count: width * height)
    for y in 0..<height {
        for x in 0..<width {
            let value = rows[y * stride + x]
            let normalized = maximum > 0 ? max(0, min(1, value / maximum)) : 0
            png[y * width + x] = UInt8((normalized * 255).rounded())
            if value.isFinite, value >= threshold, value > 0 {
                let point = displayPoint(
                    storedX: (Double(x) + 0.5) / Double(width),
                    storedY: (Double(y) + 0.5) / Double(height),
                    orientation: orientation
                )
                guard isInsidePrimaryObject(point) else { continue }
                let currentWeight = Double(value)
                weight += currentWeight
                weightedX += point.x * currentWeight
                weightedY += point.y * currentWeight
            }
        }
    }
    guard weight > 0 else { fatalError("saliency map has no positive response") }

    guard let provider = CGDataProvider(data: png as CFData),
          let image = CGImage(
            width: width,
            height: height,
            bitsPerComponent: 8,
            bitsPerPixel: 8,
            bytesPerRow: width,
            space: CGColorSpaceCreateDeviceGray(),
            bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.none.rawValue),
            provider: provider,
            decode: nil,
            shouldInterpolate: true,
            intent: .defaultIntent
          ),
          let destination = CGImageDestinationCreateWithURL(
            heatmapURL as CFURL,
            UTType.png.identifier as CFString,
            1,
            nil
          ) else {
        fatalError("cannot create heatmap")
    }
    CGImageDestinationAddImage(destination, image, nil)
    guard CGImageDestinationFinalize(destination) else { fatalError("cannot write heatmap") }

    let maximumStored = Point(
        x: (Double(maximumX) + 0.5) / Double(width),
        y: (Double(maximumY) + 0.5) / Double(height)
    )
    return SaliencyResult(
        request: requestName,
        heatmapWidth: width,
        heatmapHeight: height,
        maximumValue: maximum,
        maximumStoredTopLeft: maximumStored,
        maximumDisplayTopLeft: displayPoint(
            storedX: maximumStored.x,
            storedY: maximumStored.y,
            orientation: orientation
        ),
        weightedCentroidDisplayTopLeft: Point(
            x: weightedX / weight,
            y: weightedY / weight
        ),
        threshold: threshold,
        salientObjects: objects
    )
}

func landmarkCenter(_ region: VNFaceLandmarkRegion2D, face: VNFaceObservation) -> Point? {
    guard region.pointCount > 0 else { return nil }
    var x = 0.0
    var y = 0.0
    for point in region.normalizedPoints {
        x += face.boundingBox.minX + CGFloat(point.x) * face.boundingBox.width
        y += face.boundingBox.minY + CGFloat(point.y) * face.boundingBox.height
    }
    let count = CGFloat(region.pointCount)
    return Point(x: x / count, y: 1 - y / count)
}

func rankFaces(
    _ faces: [VNFaceObservation],
    saliency: VNSaliencyImageObservation,
    orientation: UInt32
) -> [FaceCandidate] {
    let buffer = saliency.pixelBuffer
    CVPixelBufferLockBaseAddress(buffer, .readOnly)
    defer { CVPixelBufferUnlockBaseAddress(buffer, .readOnly) }
    guard let base = CVPixelBufferGetBaseAddress(buffer) else { return [] }
    let mapWidth = CVPixelBufferGetWidth(buffer)
    let mapHeight = CVPixelBufferGetHeight(buffer)
    let stride = CVPixelBufferGetBytesPerRow(buffer) / MemoryLayout<Float>.stride
    let values = base.assumingMemoryBound(to: Float.self)
    return faces.map { face in
        let box = face.boundingBox
        let top = 1 - box.maxY
        let bottom = 1 - box.minY
        var sum = 0.0
        var count = 0
        var peak: Float = 0
        for y in 0..<mapHeight {
            for x in 0..<mapWidth {
                let point = displayPoint(
                    storedX: (Double(x) + 0.5) / Double(mapWidth),
                    storedY: (Double(y) + 0.5) / Double(mapHeight),
                    orientation: orientation
                )
                guard point.x >= box.minX, point.x <= box.maxX,
                      point.y >= top, point.y <= bottom else { continue }
                let value = values[y * stride + x]
                guard value.isFinite else { continue }
                sum += Double(value)
                count += 1
                peak = max(peak, value)
            }
        }
        let leftEye = face.landmarks?.leftEye.flatMap { landmarkCenter($0, face: face) }
        let rightEye = face.landmarks?.rightEye.flatMap { landmarkCenter($0, face: face) }
        let eyes: Point?
        if let leftEye, let rightEye {
            eyes = Point(
                x: (leftEye.x + rightEye.x) / 2,
                y: (leftEye.y + rightEye.y) / 2
            )
        } else {
            eyes = nil
        }
        return FaceCandidate(
            confidence: face.confidence,
            topLeftX: box.minX,
            topLeftY: top,
            width: box.width,
            height: box.height,
            attentionMean: count > 0 ? sum / Double(count) : 0,
            attentionPeak: peak,
            centerTopLeft: Point(x: box.midX, y: (top + bottom) / 2),
            eyesMidpointTopLeft: eyes
        )
    }.sorted {
        $0.attentionMean * Double($0.confidence) > $1.attentionMean * Double($1.confidence)
    }
}

guard CommandLine.arguments.count == 4 else {
    fatalError("usage: VisionSaliencyFocus.swift INPUT OUTPUT.json HEATMAP_DIR")
}
let inputURL = URL(fileURLWithPath: CommandLine.arguments[1])
let reportURL = URL(fileURLWithPath: CommandLine.arguments[2])
let heatmapDirectory = URL(fileURLWithPath: CommandLine.arguments[3], isDirectory: true)
try FileManager.default.createDirectory(at: heatmapDirectory, withIntermediateDirectories: true)

guard let source = CGImageSourceCreateWithURL(inputURL as CFURL, nil),
      let image = CGImageSourceCreateImageAtIndex(source, 0, nil) else {
    fatalError("cannot decode input")
}
let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any]
let orientationRaw = (properties?[kCGImagePropertyOrientation] as? NSNumber)?.uint32Value ?? 1
guard let orientation = CGImagePropertyOrientation(rawValue: orientationRaw) else {
    fatalError("unsupported EXIF orientation \(orientationRaw)")
}

let attentionRequest = VNGenerateAttentionBasedSaliencyImageRequest()
let objectnessRequest = VNGenerateObjectnessBasedSaliencyImageRequest()
let faceRequest = VNDetectFaceLandmarksRequest()
let handler = VNImageRequestHandler(cgImage: image, orientation: orientation, options: [:])
try handler.perform([attentionRequest, objectnessRequest, faceRequest])
guard let attentionObservation = attentionRequest.results?.first,
      let objectnessObservation = objectnessRequest.results?.first else {
    fatalError("Vision returned no saliency result")
}

let attention = try analyze(
    observation: attentionObservation,
    requestName: "attention",
    orientation: orientationRaw,
    heatmapURL: heatmapDirectory.appendingPathComponent("attention.png")
)
let objectness = try analyze(
    observation: objectnessObservation,
    requestName: "objectness",
    orientation: orientationRaw,
    heatmapURL: heatmapDirectory.appendingPathComponent("objectness.png")
)
let faces = rankFaces(faceRequest.results ?? [], saliency: attentionObservation, orientation: orientationRaw)
let recommendedFocus: RecommendedFocus
if let face = faces.first, let eyes = face.eyesMidpointTopLeft {
    recommendedFocus = RecommendedFocus(strategy: "attention-ranked-face-eyes", pointTopLeft: eyes)
} else if let face = faces.first {
    recommendedFocus = RecommendedFocus(strategy: "attention-ranked-face-center", pointTopLeft: face.centerTopLeft)
} else {
    recommendedFocus = RecommendedFocus(
        strategy: "attention-weighted-centroid",
        pointTopLeft: attention.weightedCentroidDisplayTopLeft
    )
}
let swapsDimensions = [5, 6, 7, 8].contains(orientationRaw)
let report = Report(
    input: inputURL.path,
    storedWidth: image.width,
    storedHeight: image.height,
    exifOrientation: orientationRaw,
    displayWidth: swapsDimensions ? image.height : image.width,
    displayHeight: swapsDimensions ? image.width : image.height,
    attention: attention,
    objectness: objectness,
    faces: faces,
    recommendedFocus: recommendedFocus
)
let encoder = JSONEncoder()
encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
try encoder.encode(report).write(to: reportURL)
print(String(data: try encoder.encode(report), encoding: .utf8)!)
