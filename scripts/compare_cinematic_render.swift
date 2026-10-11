// Compare native and regenerated metadata with the public headless renderer.
// Writes derivatives only; it does not import into Photos or download resources.
import AVFoundation
import Cinematic
import CoreImage
import CoreMedia
import CoreVideo
import Foundation
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
    let focus: Float
}

func render(_ session: CNRenderingSession, _ attributes: CNRenderingSession.Attributes,
            _ selected: Selected, _ fNumber: Float, _ queue: MTLCommandQueue,
            _ device: MTLDevice, _ png: URL?, focusOverride: Float? = nil) throws -> Rendered {
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
    if let png {
        guard let ci = CIImage(mtlTexture: destination, options: [.colorSpace: CGColorSpace(name: CGColorSpace.linearSRGB)!]) else {
            throw NSError(domain: "CannotCreatePreview", code: 1)
        }
        let display = ci.transformed(by: CGAffineTransform(translationX: 0, y: ci.extent.height).scaledBy(x: 1, y: -1))
        try CIContext(mtlDevice: device).writePNGRepresentation(of: display, to: png, format: .RGBA8,
            colorSpace: CGColorSpace(name: CGColorSpace.sRGB)!, options: [:])
    }
    return Rendered(pixels: pixels, focus: frame.focusDisparity)
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

func effectDifference(_ reference: [[UInt16]], _ candidate: [[UInt16]]) throws -> [String: Any] {
    try check(reference.count == 2 && candidate.count == 2 && reference[0].count == candidate[0].count,
              "effect comparison geometry differs")
    var errorEnergy = 0.0, nativeEnergy = 0.0, generatedEnergy = 0.0, count = 0
    for index in reference[0].indices where index % 4 != 3 {
        let a = Double(Float(Float16(bitPattern: reference[0][index]))) - Double(Float(Float16(bitPattern: reference[1][index])))
        let b = Double(Float(Float16(bitPattern: candidate[0][index]))) - Double(Float(Float16(bitPattern: candidate[1][index])))
        errorEnergy += (a - b) * (a - b); nativeEnergy += a * a; generatedEnergy += b * b; count += 1
    }
    var result: [String: Any] = ["definition": "(native f2 - native f16) versus (generated f2 - generated f16)",
                               "effect_error_rmse": sqrt(errorEnergy / Double(count)),
                               "native_effect_rmse": sqrt(nativeEnergy / Double(count)),
                               "generated_effect_rmse": sqrt(generatedEnergy / Double(count))]
    if nativeEnergy > 0 {
        result["relative_effect_error"] = sqrt(errorEnergy / nativeEnergy)
        result["generated_to_native_effect_strength"] = sqrt(generatedEnergy / nativeEnergy)
    }
    return result
}

@available(macOS 27.0, *)
func compare(_ nativeURL: URL, _ generatedURL: URL, _ directory: URL) async throws -> [String: Any] {
    let native = try await prepare(nativeURL, directory.appendingPathComponent("native-processed.mov"))
    let generated = try await prepare(generatedURL, directory.appendingPathComponent("generated-processed.mov"))
    let nativeAttributes = try await CNRenderingSession.Attributes(asset: native.asset)
    let generatedAttributes = try await CNRenderingSession.Attributes(asset: generated.asset)
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
        let depthDifference = try difference(disparityPixels(a.depth), disparityPixels(b.depth), channels: 1)
        var referenceRenders: [[UInt16]] = [], candidateRenders: [[UInt16]] = []
        for fNumber: Float in [2, 16] {
            let prefix = "t\(index + 1)-f\(Int(fNumber))"
            let nativePNG = directory.appendingPathComponent(prefix + "-native.png")
            let generatedPNG = directory.appendingPathComponent(prefix + "-generated.png")
            let reference = try render(nativeSession, nativeAttributes, a, fNumber, queue, device, nativePNG)
            let candidate = try render(generatedSession, generatedAttributes, b, fNumber, queue, device, generatedPNG)
            var row: [String: Any] = ["pts": a.pts.seconds, "f_number": fNumber,
                "native_focus": reference.focus, "generated_focus": candidate.focus,
                "native_png": nativePNG.path, "generated_png": generatedPNG.path,
                "render_difference": try difference(reference.pixels, candidate.pixels), "disparity_difference": depthDifference]
            if index == 0 && fNumber == 2 {
                let repeatRender = try render(nativeSession, nativeAttributes, a, fNumber, queue, device, nil)
                row["native_repeat_difference"] = try difference(reference.pixels, repeatRender.pixels)
                let fixedFocus = try render(generatedSession, generatedAttributes, b, fNumber, queue, device, nil, focusOverride: reference.focus)
                row["generated_with_native_focus_difference"] = try difference(reference.pixels, fixedFocus.pixels)
            }
            referenceRenders.append(reference.pixels); candidateRenders.append(candidate.pixels)
            results.append(row)
        }
        results[results.count - 1]["native_f2_f16_difference"] = try difference(referenceRenders[0], referenceRenders[1])
        results[results.count - 1]["generated_f2_f16_difference"] = try difference(candidateRenders[0], candidateRenders[1])
        results[results.count - 1]["effect_comparison"] = try effectDifference(referenceRenders, candidateRenders)
    }
    return ["schema": "xdremux-cinematic-headless-comparison-v1", "renderer": "public CNRenderingSession",
            "quality": "export", "texture_format": "RGBA16Float", "input_pixel_format": "10-bit bi-planar video range",
            "comparison_space": "raw linear RGB, full resolution, alpha excluded", "source_transform": "identity for both assets",
            "preview_png": "8-bit sRGB preview; measurements use the original floating point texture",
            "public_preprocessing": true, "test_processed_color_tracks_reference_inputs": true,
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
            try check(args.count == 3, "usage: compare_cinematic_render NATIVE.mov GENERATED.mov NEW_OUTPUT_DIRECTORY")
            guard #available(macOS 27.0, *) else { throw NSError(domain: "RequiresMacOS27", code: 1) }
            let directory = URL(fileURLWithPath: args[2])
            try check(!FileManager.default.fileExists(atPath: directory.path), "comparison output directory exists")
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            let report = try await compare(URL(fileURLWithPath: args[0]), URL(fileURLWithPath: args[1]), directory)
            let data = try JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys])
            try data.write(to: directory.appendingPathComponent("comparison.json"), options: .atomic)
            print("headless comparison complete; identical=\(report["all_compared_frames_identical"]!)")
        } catch { fputs("headless comparison failed: \(error.localizedDescription)\n", stderr); exit(1) }
    }
}
