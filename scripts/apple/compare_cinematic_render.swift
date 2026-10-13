// Compare native and regenerated metadata with the public headless renderer.
// Writes derivatives only; it does not import into Photos or download resources.
import AVFoundation
import Cinematic
import CoreImage
import CoreImage.CIFilterBuiltins
import CoreMedia
import CoreVideo
import CryptoKit
import Foundation
import ImageIO
import Metal

func check(_ condition: Bool, _ message: String) throws {
    if !condition { throw NSError(domain: "XDCinematicRenderComparison", code: 1,
                                 userInfo: [NSLocalizedDescriptionKey: message]) }
}

@available(macOS 27.0, *)
func prepare(_ url: URL, _ output: URL) async throws -> CNAssetInfo {
    let asset = AVURLAsset(url: url)
    let info = try await CNAssetInfo(asset: asset)
    try check(info.resourceStatus == .ready, "Cinematic resources are not ready")
    if info.cinematicCapability == .renderable { return info }
    try check(info.cinematicCapability == .needsPreprocessing, "input is not a Cinematic asset")
    let configuration = CNAssetPreprocessConfiguration(destinationAssetURL: output)
    // These test derivatives reference the unchanged input color track. The
    // product pending MOV is self-contained; this saves test-only duplicate data.
    configuration.referenceSourceAssetTracks = true
    let processed = try await info.preprocessAsset(configuration: configuration)
    try check(processed.cinematicCapability == .renderable && processed.isPreprocessed,
              "public preprocessing did not produce a renderable asset")
    return processed
}

struct Selected {
    let color: CVPixelBuffer
    let depth: CVPixelBuffer
    let metadata: AVTimedMetadataGroup
    let pts: CMTime
}

func select(_ info: CNAssetInfo, time: Double) throws -> Selected {
    let reader = try AVAssetReader(asset: info.asset)
    let video = AVAssetReaderTrackOutput(track: info.cinematicVideoTrack, outputSettings: [
        kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_420YpCbCr10BiPlanarVideoRange,
        kCVPixelBufferMetalCompatibilityKey as String: true,
        kCVPixelBufferIOSurfacePropertiesKey as String: [:],
    ])
    let depth = AVAssetReaderTrackOutput(track: info.cinematicDisparityTrack, outputSettings: [
        kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_DisparityFloat16,
        kCVPixelBufferMetalCompatibilityKey as String: true,
        kCVPixelBufferIOSurfacePropertiesKey as String: [:],
    ])
    let metadata = AVAssetReaderTrackOutput(track: info.cinematicMetadataTrack, outputSettings: nil)
    reader.add(video); reader.add(depth); reader.add(metadata)
    let adaptor = AVAssetReaderOutputMetadataAdaptor(assetReaderTrackOutput: metadata)
    try check(reader.startReading(), "cannot read render inputs")
    var nextGroup = adaptor.nextTimedMetadataGroup(), previous: AVTimedMetadataGroup?
    while let sample = video.copyNextSampleBuffer() {
        let pts = CMSampleBufferGetPresentationTimeStamp(sample)
        guard let d = depth.copyNextSampleBuffer() else { throw NSError(domain: "MissingDisparity", code: 1) }
        try check(CMTimeCompare(CMSampleBufferGetPresentationTimeStamp(d), pts) == 0, "video/depth timestamps differ")
        while let group = nextGroup, CMTimeCompare(group.timeRange.start, pts) <= 0 {
            previous = group; nextGroup = adaptor.nextTimedMetadataGroup()
        }
        if pts.seconds >= time {
            guard let color = CMSampleBufferGetImageBuffer(sample), let disparity = CMSampleBufferGetImageBuffer(d), let group = previous else {
                throw NSError(domain: "MissingRenderInput", code: 1)
            }
            reader.cancelReading()
            return Selected(color: color, depth: disparity, metadata: group, pts: pts)
        }
    }
    throw NSError(domain: "FrameNotFound", code: 1)
}

struct Rendered {
    let pixels: [UInt16]
    let linearPixels: [UInt16]
    let focus: Float
    let color: [String: Any]
}

// CNRenderingSession's RGBA destination retains the source RGB transfer
// function. In particular, HLG output is not a linear sRGB texture. Core Image
// obtains the primaries, transfer function and headroom from the decoded buffer.
func managedImage(_ texture: MTLTexture, source: CVPixelBuffer) throws -> (CIImage, [String: Any]) {
    let sourceImage = CIImage(cvPixelBuffer: source)
    guard let colorSpace = sourceImage.colorSpace,
          let image = CIImage(mtlTexture: texture, options: [.colorSpace: colorSpace]) else {
        throw NSError(domain: "MissingSourceColorSpace", code: 1)
    }
    let headroom = sourceImage.contentHeadroom
    try check(headroom.isFinite && headroom >= 1, "source HDR headroom is unknown")
    let upright = image.transformed(by: CGAffineTransform(translationX: 0, y: image.extent.height).scaledBy(x: 1, y: -1))
        .settingContentHeadroom(headroom)
        .settingContentAverageLightLevel(sourceImage.contentAverageLightLevel)
    return (upright, ["source_color_space": colorSpace.name as String? ?? "unnamed",
                      "source_headroom": headroom, "source_average_light_level": sourceImage.contentAverageLightLevel,
                      "sdr_tone_map": "CISystemToneMap, standard dynamic range, display headroom 1",
                      "sdr_output_color_space": "sRGB", "linear_measurement_color_space": "extended linear ITU-R BT.2020"])
}

func writeSDRPreview(_ image: CIImage, context: CIContext, transform: CGAffineTransform, to url: URL) throws {
    let toneMap = CIFilter.systemToneMap()
    toneMap.inputImage = image; toneMap.displayHeadroom = 1; toneMap.preferredDynamicRange = .standard
    guard let output = toneMap.outputImage else { throw NSError(domain: "CannotToneMapPreview", code: 1) }
    // Track transforms use top-left video coordinates. Core Image uses a
    // bottom-left origin, so conjugate the transform with the two Y flips.
    let height = image.extent.height, displayHeight = image.extent.applying(transform).height
    let display = CGAffineTransform(a: transform.a, b: -transform.b, c: -transform.c, d: transform.d,
        tx: transform.tx + transform.c * height, ty: displayHeight - transform.d * height - transform.ty)
    try context.writePNGRepresentation(of: output.transformed(by: display), to: url, format: .RGBA8,
        colorSpace: CGColorSpace(name: CGColorSpace.sRGB)!, options: [:])
}

func verifyColorManagement(_ directory: URL) throws -> [String: Any] {
    guard let device = MTLCreateSystemDefaultDevice() else { throw NSError(domain: "NoMetalDevice", code: 1) }
    let linear = CGColorSpace(name: CGColorSpace.extendedLinearITUR_2020)!
    let hlg = CGColorSpace(name: CGColorSpace.itur_2100_HLG)!
    let sRGB = CGColorSpace(name: CGColorSpace.sRGB)!
    let context = CIContext(mtlDevice: device, options: [.workingColorSpace: linear])
    var source: CVPixelBuffer?
    try check(CVPixelBufferCreate(nil, 16, 16, kCVPixelFormatType_420YpCbCr10BiPlanarVideoRange,
        [kCVPixelBufferIOSurfacePropertiesKey: [:]] as CFDictionary, &source) == kCVReturnSuccess, "cannot allocate color fixture")
    CVBufferSetAttachment(source!, kCVImageBufferColorPrimariesKey, kCVImageBufferColorPrimaries_ITU_R_2020, .shouldPropagate)
    CVBufferSetAttachment(source!, kCVImageBufferTransferFunctionKey, kCVImageBufferTransferFunction_ITU_R_2100_HLG, .shouldPropagate)
    CVBufferSetAttachment(source!, kCVImageBufferYCbCrMatrixKey, kCVImageBufferYCbCrMatrix_ITU_R_2020, .shouldPropagate)
    let descriptor = MTLTextureDescriptor.texture2DDescriptor(pixelFormat: .rgba16Float, width: 16, height: 16, mipmapped: false)
    descriptor.storageMode = .shared; descriptor.usage = [.shaderRead]
    let texture = device.makeTexture(descriptor: descriptor)!
    var records: [[String: Any]] = [], grayOutputs: [Double] = []
    for (index, rgb) in [[Float(0.5),0.5,0.5], [0.75,0.75,0.75], [0.9,0.9,0.9], [1,1,1], [0.65,0.4,0.25]].enumerated() {
        let values = (0..<256).flatMap { _ in (rgb + [1]).map { Float16($0).bitPattern } }
        values.withUnsafeBytes { texture.replace(region: MTLRegionMake2D(0,0,16,16), mipmapLevel: 0, withBytes: $0.baseAddress!, bytesPerRow: 16*8) }
        let (image, color) = try managedImage(texture, source: source!)
        let reference = CIImage(color: CIColor(red: CGFloat(rgb[0]), green: CGFloat(rgb[1]), blue: CGFloat(rgb[2]), alpha: 1, colorSpace: hlg)!)
            .cropped(to: image.extent).settingContentHeadroom(color["source_headroom"] as! Float)
        func pixels(_ image: CIImage, _ space: CGColorSpace) -> [UInt16] {
            var p = [UInt16](repeating: 0, count: 256*4)
            context.render(image, toBitmap: &p, rowBytes: 16*8, bounds: image.extent, format: .RGBAh, colorSpace: space)
            return p
        }
        let error = try difference(pixels(image, linear), pixels(reference, linear))
        try check((error["rmse"] as! Double) < 0.003, "HLG texture was interpreted in the wrong color space")
        let wrong = CIImage(mtlTexture: texture, options: [.colorSpace: CGColorSpace(name: CGColorSpace.linearSRGB)!])!
        let wrongError = try difference(pixels(wrong, linear), pixels(reference, linear))
        try check(wrongError["rmse"] as! Double > 0.05, "negative color-space control did not fail")
        let png = directory.appendingPathComponent("hlg-patch-\(index).png")
        try writeSDRPreview(image, context: context, transform: .identity, to: png)
        guard let decoded = CIImage(contentsOf: png), let actualSpace = decoded.colorSpace else { throw NSError(domain: "CannotReadColorFixture", code: 1) }
        try check(actualSpace.name == sRGB.name, "SDR PNG has no sRGB color profile")
        let tone = CIFilter.systemToneMap(); tone.inputImage = reference; tone.displayHeadroom = 1; tone.preferredDynamicRange = .standard
        let sdrError = try difference(pixels(decoded, sRGB), pixels(tone.outputImage!, sRGB))
        try check((sdrError["rmse"] as! Double) < 0.006, "SDR preview differs from system tone-mapped reference")
        let channels = pixels(decoded, sRGB).prefix(3).map { Double(Float(Float16(bitPattern: $0))) }
        if index < 4 { grayOutputs.append(channels[0]) }
        records.append(["input_hlg_rgb": rgb, "sdr_rgb": channels, "linear_reference_error": error,
                        "wrong_linear_srgb_error": wrongError, "sdr_reference_error": sdrError, "color_management": color])
    }
    try check(zip(grayOutputs, grayOutputs.dropFirst()).allSatisfy { $1 > $0 + 0.005 }, "HDR highlights were clipped instead of tone-mapped")
    let marker = (0..<256).flatMap { index -> [UInt16] in
        let x = index % 16, y = index / 16
        let colors: [[Float]] = [[0.8,0.2,0.2], [0.2,0.8,0.2], [0.2,0.2,0.8], [0.8,0.8,0.8]]
        return (colors[(y / 8) * 2 + x / 8] + [1]).map { Float16($0).bitPattern }
    }
    marker.withUnsafeBytes { texture.replace(region: MTLRegionMake2D(0,0,16,16), mipmapLevel: 0, withBytes: $0.baseAddress!, bytesPerRow: 16*8) }
    let (markerImage, _) = try managedImage(texture, source: source!)
    let transforms: [(String, CGAffineTransform, CGImagePropertyOrientation)] = [
        ("0", .identity, .up), ("90", CGAffineTransform(a: 0,b: 1,c: -1,d: 0,tx: 16,ty: 0), .right),
        ("180", CGAffineTransform(a: -1,b: 0,c: 0,d: -1,tx: 16,ty: 16), .down),
        ("270", CGAffineTransform(a: 0,b: -1,c: 1,d: 0,tx: 0,ty: 16), .left)]
    var orientationRecords: [[String: Any]] = []
    for (name, transform, exif) in transforms {
        let png = directory.appendingPathComponent("orientation-\(name).png")
        try writeSDRPreview(markerImage, context: context, transform: transform, to: png)
        let tone = CIFilter.systemToneMap(); tone.inputImage = markerImage; tone.displayHeadroom = 1; tone.preferredDynamicRange = .standard
        let expected = tone.outputImage!.oriented(exif), decoded = CIImage(contentsOf: png)!
        func pixels(_ image: CIImage) -> [UInt16] {
            var values = [UInt16](repeating: 0, count: 256*4)
            context.render(image, toBitmap: &values, rowBytes: 16*8, bounds: image.extent, format: .RGBAh, colorSpace: sRGB)
            return values
        }
        let referencePNG = directory.appendingPathComponent("orientation-\(name)-reference.png")
        try context.writePNGRepresentation(of: expected, to: referencePNG, format: .RGBA8, colorSpace: sRGB, options: [:])
        let error = try difference(pixels(decoded), pixels(CIImage(contentsOf: referencePNG)!))
        try check((error["rmse"] as! Double) < 0.006, "video preview orientation \(name) differs from the EXIF reference: \(error)")
        orientationRecords.append(["rotation_degrees": name, "reference_difference": error])
    }
    return ["schema": "xdremux-cinematic-color-regression-v1", "patches": records, "highlights_distinct": true,
            "orientation_checks": orientationRecords]
}

func render(_ session: CNRenderingSession, _ attributes: CNRenderingSession.Attributes,
            _ selected: Selected, _ fNumber: Float, _ queue: MTLCommandQueue,
            _ device: MTLDevice, _ png: URL?, focusOverride: Float? = nil,
            displayTransform: CGAffineTransform = .identity) throws -> Rendered {
    guard var frame = CNRenderingSession.FrameAttributes(timedMetadataGroup: selected.metadata, sessionAttributes: attributes) else {
        throw NSError(domain: "InvalidFrameAttributes", code: 1)
    }
    frame.fNumber = fNumber
    if let focusOverride { frame.focusDisparity = focusOverride }
    let descriptor = MTLTextureDescriptor.texture2DDescriptor(pixelFormat: .rgba16Float,
        width: CVPixelBufferGetWidth(selected.color), height: CVPixelBufferGetHeight(selected.color), mipmapped: false)
    descriptor.storageMode = .shared; descriptor.usage = [.shaderRead, .shaderWrite, .renderTarget]
    guard let destination = device.makeTexture(descriptor: descriptor), let command = queue.makeCommandBuffer() else {
        throw NSError(domain: "MetalAllocationFailed", code: 1)
    }
    try check(session.encodeRender(to: command, frameAttributes: frame,
        sourceImage: CVReadOnlyPixelBuffer(unsafeBuffer: selected.color),
        sourceDisparity: CVReadOnlyPixelBuffer(unsafeBuffer: selected.depth), destinationRGBA: destination),
        "public renderer rejected the frame")
    command.commit(); command.waitUntilCompleted()
    try check(command.status == .completed, "Metal render did not complete: \(String(describing: command.error))")
    var pixels = [UInt16](repeating: 0, count: destination.width * destination.height * 4)
    destination.getBytes(&pixels, bytesPerRow: destination.width * 8,
                         from: MTLRegionMake2D(0, 0, destination.width, destination.height), mipmapLevel: 0)
    let linearSpace = CGColorSpace(name: CGColorSpace.extendedLinearITUR_2020)!
    let context = CIContext(mtlDevice: device, options: [.workingColorSpace: linearSpace])
    let (image, color) = try managedImage(destination, source: selected.color)
    var linear = [UInt16](repeating: 0, count: pixels.count)
    context.render(image, toBitmap: &linear, rowBytes: destination.width * 8, bounds: image.extent, format: .RGBAh, colorSpace: linearSpace)
    if let png { try writeSDRPreview(image, context: context, transform: displayTransform, to: png) }
    return Rendered(pixels: pixels, linearPixels: linear, focus: frame.focusDisparity, color: color)
}

func difference(_ a: [UInt16], _ b: [UInt16], channels: Int = 4) throws -> [String: Any] {
    try check(a.count == b.count && !a.isEmpty, "comparison geometry differs")
    var absolute = 0.0, square = 0.0, peak = 1.0, maximum = 0.0, changed = 0, finite = 0
    var aboveTolerance = 0
    for index in a.indices where channels != 4 || index % 4 != 3 {
        let left = Double(Float(Float16(bitPattern: a[index]))), right = Double(Float(Float16(bitPattern: b[index])))
        try check(left.isFinite && right.isFinite, "render contains non-finite pixels")
        let d = abs(left - right)
        absolute += d; square += d * d; peak = max(peak, abs(left)); maximum = max(maximum, d)
        if a[index] != b[index] { changed += 1 }
        if d > 0.001 { aboveTolerance += 1 }
        finite += 1
    }
    let mse = square / Double(finite)
    return ["identical": changed == 0, "components": finite, "changed_components": changed,
            "fraction_above_0_001": Double(aboveTolerance) / Double(finite),
            "mae": absolute / Double(finite), "rmse": sqrt(mse), "max_abs_error": maximum,
            "reference_peak": peak, "psnr_db": mse == 0 ? "infinity" : String(format: "%.4f", 10 * log10(peak * peak / mse))]
}

func disparityPixels(_ buffer: CVPixelBuffer) throws -> [UInt16] {
    try check(CVPixelBufferLockBaseAddress(buffer, .readOnly) == kCVReturnSuccess, "cannot read disparity")
    defer { CVPixelBufferUnlockBaseAddress(buffer, .readOnly) }
    guard let base = CVPixelBufferGetBaseAddress(buffer) else { throw NSError(domain: "MissingDisparityBytes", code: 1) }
    let width = CVPixelBufferGetWidth(buffer), height = CVPixelBufferGetHeight(buffer)
    let stride = CVPixelBufferGetBytesPerRow(buffer)
    var pixels = [UInt16](); pixels.reserveCapacity(width * height)
    for y in 0..<height { pixels.append(contentsOf: UnsafeBufferPointer(start: base.advanced(by: y * stride).assumingMemoryBound(to: UInt16.self), count: width)) }
    return pixels
}

func colorDigest(_ buffer: CVPixelBuffer) throws -> String {
    try check(CVPixelBufferLockBaseAddress(buffer, .readOnly) == kCVReturnSuccess, "cannot read color")
    defer { CVPixelBufferUnlockBaseAddress(buffer, .readOnly) }
    var digest = SHA256()
    for plane in 0..<CVPixelBufferGetPlaneCount(buffer) {
        guard let base = CVPixelBufferGetBaseAddressOfPlane(buffer, plane) else { throw NSError(domain: "MissingColorBytes", code: 1) }
        let width = CVPixelBufferGetWidthOfPlane(buffer, plane) * (plane == 0 ? 2 : 4)
        let stride = CVPixelBufferGetBytesPerRowOfPlane(buffer, plane)
        for row in 0..<CVPixelBufferGetHeightOfPlane(buffer, plane) {
            digest.update(data: Data(bytes: base.advanced(by: row * stride), count: width))
        }
    }
    return digest.finalize().map { String(format: "%02x", $0) }.joined()
}

func detectionRecord(_ detection: CNDetection) -> [String: Any] {
    let rect = detection.normalizedRect
    return ["type": detection.detectionType.rawValue, "rect": [rect.minX, rect.minY, rect.width, rect.height],
            "focus_disparity": detection.focusDisparity]
}

func effectDifference(_ reference: [[UInt16]], _ candidate: [[UInt16]], width: Int? = nil, region: CGRect? = nil) throws -> [String: Any] {
    try check(reference.count == 2 && candidate.count == 2 && reference[0].count == candidate[0].count,
              "effect comparison geometry differs")
    var errorEnergy = 0.0, nativeEnergy = 0.0, generatedEnergy = 0.0, dotProduct = 0.0, count = 0
    for index in reference[0].indices where index % 4 != 3 {
        if let width, let region {
            let pixel = index / 4, height = reference[0].count / (4 * width)
            let point = CGPoint(x: (Double(pixel % width) + 0.5) / Double(width),
                                y: (Double(pixel / width) + 0.5) / Double(height))
            if !region.contains(point) { continue }
        }
        let a = Double(Float(Float16(bitPattern: reference[0][index]))) - Double(Float(Float16(bitPattern: reference[1][index])))
        let b = Double(Float(Float16(bitPattern: candidate[0][index]))) - Double(Float(Float16(bitPattern: candidate[1][index])))
        errorEnergy += (a - b) * (a - b); nativeEnergy += a * a; generatedEnergy += b * b; dotProduct += a * b; count += 1
    }
    try check(count > 0, "empty effect comparison region")
    var result: [String: Any] = ["definition": "(native f2 - native f16) versus (generated f2 - generated f16)",
                               "effect_error_rmse": sqrt(errorEnergy / Double(count)),
                               "native_effect_rmse": sqrt(nativeEnergy / Double(count)),
                               "generated_effect_rmse": sqrt(generatedEnergy / Double(count))]
    if nativeEnergy > 0 {
        result["relative_effect_error"] = sqrt(errorEnergy / nativeEnergy)
        result["generated_to_native_effect_strength"] = sqrt(generatedEnergy / nativeEnergy)
    }
    if nativeEnergy > 0 && generatedEnergy > 0 { result["effect_cosine_similarity"] = dotProduct / sqrt(nativeEnergy * generatedEnergy) }
    result["components"] = count
    return result
}

@available(macOS 27.0, *)
func verifyEditing(_ info: CNAssetInfo) async throws -> [String: Any] {
    try check(info.cinematicCapability == .renderable && info.isPreprocessed,
              "editing verification requires preprocessed resources")
    let script = try await CNScript(asset: info.asset)
    let frames = script.frames(in: script.timeRange)
    try check(frames.count >= 2, "editing verification needs two frames")
    let recordedIDs = Set(frames.flatMap { $0.allDetections.compactMap { $0.detectionID?.rawValue } })
    var editIDs: [CNDetectionID] = []
    for focus: Float in [0.05, 0.08] {
        let detections = frames.map { CNDetection(time: $0.time, detectionType: .custom,
            normalizedRect: CGRect(x: 0.4, y: 0.4, width: 0.2, height: 0.2), focusDisparity: focus) }
        let id = script.addDetectionTrack(CNCustomDetectionTrack(detections: detections, smooth: false))
        try check(!recordedIDs.contains(id.rawValue) && !editIDs.contains(id),
                  "processed public editor allocated a colliding track ID")
        editIDs.append(id)
    }
    for (time, id) in [(frames[0].time, editIDs[0]), (frames[frames.count - 1].time, editIDs[1])] {
        try check(script.addUserDecision(CNDecision(time: time, detectionID: id, strong: true)),
                  "public editor rejected a custom-focus decision")
    }
    script.fNumber = 4
    let editedFrames = script.frames(in: script.timeRange)
    try check(abs(editedFrames[0].focusDisparity - 0.05) < 1e-6 &&
              abs(editedFrames[editedFrames.count - 1].focusDisparity - 0.08) < 1e-6,
              "public editor did not apply the requested focus")
    guard let changes = CNScript.Changes(dataRepresentation: script.changes().dataRepresentation) else {
        throw NSError(domain: "CannotSerializeCinematicEdits", code: 1)
    }
    let reopened = try await CNScript(asset: info.asset, changes: changes)
    let reloadedFrames = reopened.frames(in: reopened.timeRange)
    let restoredIDs = reopened.addedDetectionTracks.map { $0.detectionID }
    try check(restoredIDs == editIDs && restoredIDs.allSatisfy { !recordedIDs.contains($0.rawValue) },
              "public custom track IDs changed or collided after reload")
    try check(reopened.fNumber == 4 && reloadedFrames.count == editedFrames.count,
              "public aperture/frame count did not survive reload (aperture=\(reopened.fNumber))")
    try check(abs(reloadedFrames[0].focusDisparity - 0.05) < 1e-6 &&
              abs(reloadedFrames[reloadedFrames.count - 1].focusDisparity - 0.08) < 1e-6,
              "public focus decisions did not survive reload")
    for (index, track) in reopened.addedDetectionTracks.enumerated() {
        let restored = track.detections(in: reopened.timeRange)
        let expectedFocus: Float = index == 0 ? 0.05 : 0.08
        try check(restored.count == frames.count && zip(restored, frames).allSatisfy {
            CMTimeCompare($0.time, $1.time) == 0 && $0.focusDisparity.isFinite &&
            abs($0.focusDisparity - expectedFocus) < 1e-6
        }, "public custom detections did not survive reload")
    }
    var transitionChanges = 0, maximumTransitionDifference: Float = 0
    for (expected, actual) in zip(editedFrames, reloadedFrames) {
        try check(CMTimeCompare(expected.time, actual.time) == 0 && actual.focusDisparity.isFinite,
                  "public reload returned invalid focus or timing")
        let error = abs(expected.focusDisparity - actual.focusDisparity)
        if error >= 1e-6 { transitionChanges += 1 }
        maximumTransitionDifference = max(maximumTransitionDifference, error)
    }
    // The public loader recalculates focus transitions. Native camera controls
    // also differ from the immediate in-memory curve around the last decision.
    // Require the persisted decisions/detections, not bit-identical transitions.
    return ["serialized_changes_reload": true, "aperture": reopened.fNumber,
            "custom_track_ids": restoredIDs.map { $0.rawValue }, "recorded_ids_disjoint": true,
            "focus_frames_checked": reloadedFrames.count, "all_focus_finite": true,
            "transition_frames_changed_after_reload": transitionChanges,
            "transition_max_abs_difference_after_reload": maximumTransitionDifference,
            "first_focus": reloadedFrames[0].focusDisparity,
            "last_focus": reloadedFrames[reloadedFrames.count - 1].focusDisparity]
}

@available(macOS 27.0, *)
func verifyGenerated(_ url: URL, _ directory: URL) async throws -> [String: Any] {
    let info = try await prepare(url, directory.appendingPathComponent("generated-processed.mov"))
    return try await verifyPrepared(info, directory)
}

@available(macOS 27.0, *)
func verifyCompatibility(_ url: URL, _ directory: URL) async throws -> [String: Any] {
    let info: CNAssetInfo
    do {
        info = try await prepare(url, directory.appendingPathComponent("generated-processed.mov"))
    } catch {
        let nativeError = error as NSError
        // The SDK defines CNCinematicErrorCodeUnsupported as 5. This exception
        // is allowed only for the explicit compatibility probe, never for the
        // ordinary native/reference or generated render acceptance checks.
        guard nativeError.domain == "CNCinematicErrorDomain", nativeError.code == 5 else { throw error }
        return ["schema": "xdremux-cinematic-compatibility-probe-v1",
                "public_preprocessing": false, "native_preprocessing_supported": false,
                "error_domain": nativeError.domain, "error_code": nativeError.code,
                "render_and_edit_verified": false]
    }
    var report = try await verifyPrepared(info, directory)
    report["native_preprocessing_supported"] = true
    return report
}

@available(macOS 27.0, *)
func verifyPrepared(_ info: CNAssetInfo, _ directory: URL) async throws -> [String: Any] {
    let attributes = try await CNRenderingSession.Attributes(asset: info.asset)
    guard let device = MTLCreateSystemDefaultDevice(), let queue = device.makeCommandQueue() else {
        throw NSError(domain: "NoMetalDevice", code: 1)
    }
    let session = CNRenderingSession(commandQueue: queue, sessionAttributes: attributes,
                                    preferredTransform: .identity, quality: .export)
    let selected = try select(info, time: info.timeRange.start.seconds + info.timeRange.duration.seconds / 2)
    let a = try render(session, attributes, selected, 2, queue, device,
                       directory.appendingPathComponent("f2.png"), displayTransform: info.preferredTransform)
    let b = try render(session, attributes, selected, 16, queue, device,
                       directory.appendingPathComponent("f16.png"), displayTransform: info.preferredTransform)
    // Finite data and native completion are required; a flat scene may have no
    // measurable blur difference even when its resources are renderable.
    return ["schema": "xdremux-cinematic-generated-render-verification-v1",
            "public_preprocessing": true, "output_capability": "renderable",
            "renderer": "public CNRenderingSession", "quality": "export", "pts": selected.pts.seconds,
            "disparity_finite": try difference(disparityPixels(selected.depth), disparityPixels(selected.depth), channels: 1),
            "f2_f16_difference": try difference(a.pixels, b.pixels),
            "linear_f2_f16_difference": try difference(a.linearPixels, b.linearPixels),
            "public_editing": try await verifyEditing(info)]
}

@available(macOS 27.0, *)
func compare(_ nativeURL: URL, _ generatedURL: URL, _ directory: URL) async throws -> [String: Any] {
    let native = try await prepare(nativeURL, directory.appendingPathComponent("native-processed.mov"))
    let generated = try await prepare(generatedURL, directory.appendingPathComponent("generated-processed.mov"))
    let nativeAttributes = try await CNRenderingSession.Attributes(asset: native.asset)
    let generatedAttributes = try await CNRenderingSession.Attributes(asset: generated.asset)
    let nativeScript = try await CNScript(asset: native.asset)
    let generatedScript = try await CNScript(asset: generated.asset)
    guard let device = MTLCreateSystemDefaultDevice(), let queue = device.makeCommandQueue() else {
        throw NSError(domain: "NoMetalDevice", code: 1)
    }
    let nativeSession = CNRenderingSession(commandQueue: queue, sessionAttributes: nativeAttributes, preferredTransform: .identity, quality: .export)
    let generatedSession = CNRenderingSession(commandQueue: queue, sessionAttributes: generatedAttributes, preferredTransform: .identity, quality: .export)
    let duration = min(native.timeRange.duration.seconds, generated.timeRange.duration.seconds)
    var results: [[String: Any]] = []
    for (index, fraction) in [0.25, 0.5, 0.75].enumerated() {
        let a = try select(native, time: duration * fraction), b = try select(generated, time: duration * fraction)
        try check(CMTimeCompare(a.pts, b.pts) == 0, "selected native/generated timestamps differ")
        let nativeColorHash = try colorDigest(a.color), generatedColorHash = try colorDigest(b.color)
        try check(nativeColorHash == generatedColorHash, "decoded color inputs differ")
        guard let nativeFrame = nativeScript.frame(at: a.pts, tolerance: .zero),
              let generatedFrame = generatedScript.frame(at: b.pts, tolerance: .zero) else {
            throw NSError(domain: "MissingScriptFrame", code: 1)
        }
        let depthDifference = try difference(disparityPixels(a.depth), disparityPixels(b.depth), channels: 1)
        var referenceRenders: [[UInt16]] = [], candidateRenders: [[UInt16]] = []
        var referenceLinear: [[UInt16]] = [], candidateLinear: [[UInt16]] = []
        for fNumber: Float in [2, 16] {
            let prefix = "t\(index + 1)-f\(Int(fNumber))"
            let nativePNG = directory.appendingPathComponent(prefix + "-native.png")
            let generatedPNG = directory.appendingPathComponent(prefix + "-generated.png")
            let reference = try render(nativeSession, nativeAttributes, a, fNumber, queue, device, nativePNG, displayTransform: native.preferredTransform)
            let candidate = try render(generatedSession, generatedAttributes, b, fNumber, queue, device, generatedPNG, displayTransform: generated.preferredTransform)
            try check(NSDictionary(dictionary: reference.color).isEqual(to: candidate.color), "source color management differs")
            var row: [String: Any] = ["pts": a.pts.seconds, "f_number": fNumber,
                "native_focus": reference.focus, "generated_focus": candidate.focus,
                "native_focus_detection": detectionRecord(nativeFrame.focusDetection),
                "generated_focus_detection": detectionRecord(generatedFrame.focusDetection),
                "decoded_color_sha256": nativeColorHash, "decoded_color_identical": true,
                "native_png": nativePNG.path, "generated_png": generatedPNG.path,
                "color_management": reference.color,
                "render_difference": try difference(reference.pixels, candidate.pixels),
                "linear_render_difference": try difference(reference.linearPixels, candidate.linearPixels),
                "disparity_difference": depthDifference]
            if index == 0 && fNumber == 2 {
                let repeatRender = try render(nativeSession, nativeAttributes, a, fNumber, queue, device, nil)
                row["native_repeat_difference"] = try difference(reference.pixels, repeatRender.pixels)
                row["native_linear_repeat_difference"] = try difference(reference.linearPixels, repeatRender.linearPixels)
                let fixedFocus = try render(generatedSession, generatedAttributes, b, fNumber, queue, device, nil, focusOverride: reference.focus)
                row["generated_with_native_focus_difference"] = try difference(reference.pixels, fixedFocus.pixels)
            }
            referenceRenders.append(reference.pixels); candidateRenders.append(candidate.pixels)
            referenceLinear.append(reference.linearPixels); candidateLinear.append(candidate.linearPixels)
            results.append(row)
        }
        results[results.count - 1]["native_f2_f16_difference"] = try difference(referenceRenders[0], referenceRenders[1])
        results[results.count - 1]["generated_f2_f16_difference"] = try difference(candidateRenders[0], candidateRenders[1])
        results[results.count - 1]["effect_comparison"] = try effectDifference(referenceRenders, candidateRenders)
        results[results.count - 1]["linear_effect_comparison"] = try effectDifference(referenceLinear, candidateLinear)
        results[results.count - 1]["native_focus_region_effect_comparison"] = try effectDifference(referenceRenders, candidateRenders,
            width: CVPixelBufferGetWidth(a.color), region: nativeFrame.focusDetection.normalizedRect)
    }
    return ["schema": "xdremux-cinematic-headless-comparison-v1", "renderer": "public CNRenderingSession",
            "quality": "export", "texture_format": "RGBA16Float", "input_pixel_format": "10-bit bi-planar video range",
            "comparison_space": "source-transfer RGB texture plus color-managed extended linear BT.2020, full resolution, alpha excluded",
            "source_transform": "identity for measurements; source preferred transform for previews",
            "preview_png": "HDR source color space -> CISystemToneMap headroom 1 -> 8-bit sRGB PNG with color profile",
            "public_preprocessing": true, "test_processed_color_tracks_reference_inputs": true,
            "public_editing": try await verifyEditing(generated),
            "native_rendering_version": nativeAttributes.renderingVersion,
            "generated_rendering_version": generatedAttributes.renderingVersion,
            "all_compared_frames_identical": results.allSatisfy { ($0["render_difference"] as? [String: Any])?["identical"] as? Bool == true },
            "frames": results]
}

@main
struct HeadlessComparison {
    static func main() async {
        do {
            let args = Array(CommandLine.arguments.dropFirst())
            guard #available(macOS 27.0, *) else { throw NSError(domain: "RequiresMacOS27", code: 1) }
            if args.count == 2 && args[0] == "--verify-color" {
                let directory = URL(fileURLWithPath: args[1])
                try check(!FileManager.default.fileExists(atPath: directory.path), "color output directory exists")
                try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
                let report = try verifyColorManagement(directory)
                try JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys])
                    .write(to: directory.appendingPathComponent("color-regression.json"), options: .atomic)
                print("HLG color-space, system SDR tone-map, profile and highlight regression passed")
                return
            }
            try check(args.count == 3, "usage: compare_cinematic_render NATIVE.mov GENERATED.mov NEW_OUTPUT_DIRECTORY | --verify-generated GENERATED.mov NEW_OUTPUT_DIRECTORY | --probe-compatibility GENERATED.mov NEW_OUTPUT_DIRECTORY | --verify-color NEW_OUTPUT_DIRECTORY")
            let directory = URL(fileURLWithPath: args[2])
            try check(!FileManager.default.fileExists(atPath: directory.path), "comparison output directory exists")
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            let generatedOnly = args[0] == "--verify-generated"
            let compatibility = args[0] == "--probe-compatibility"
            let report = compatibility
                ? try await verifyCompatibility(URL(fileURLWithPath: args[1]), directory)
                : generatedOnly
                ? try await verifyGenerated(URL(fileURLWithPath: args[1]), directory)
                : try await compare(URL(fileURLWithPath: args[0]), URL(fileURLWithPath: args[1]), directory)
            let data = try JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys])
            let reportName = compatibility ? "compatibility.json" : generatedOnly ? "generated-verification.json" : "comparison.json"
            try data.write(to: directory.appendingPathComponent(reportName), options: .atomic)
            if compatibility {
                print("compatibility probe: native preprocessing supported=\(report["native_preprocessing_supported"]!)")
                return
            }
            print(generatedOnly ? "generated native preprocessing, render and edit/reload passed" :
                  "headless comparison complete; identical=\(report["all_compared_frames_identical"]!)")
        } catch { fputs("headless comparison failed: \(error.localizedDescription)\n", stderr); exit(1) }
    }
}
