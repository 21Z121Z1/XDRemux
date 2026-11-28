import AVFoundation
import CoreGraphics
import CoreImage
import CoreML
import CoreVideo
import CryptoKit
import Foundation
import ImageIO
import Metal
import MetalPerformanceShaders

struct DepthPro2DisparityRequest: Decodable {
    let variant: String?
    let targetWidth: UInt32
    let targetHeight: UInt32
    let orientation: UInt32?
    let focalLength35mm: Double?

    enum CodingKeys: String, CodingKey {
        case variant
        case targetWidth = "target_width"
        case targetHeight = "target_height"
        case orientation
        case focalLength35mm = "focal_length_35mm"
    }
}

struct DepthPro2CameraRequest: Decodable {
    let sampleCount: UInt32?

    enum CodingKeys: String, CodingKey {
        case sampleCount = "sample_count"
    }
}

struct DepthPro2DisparityFacts: Encodable {
    let width: Int
    let height: Int
    let pixelFormat: UInt32
    let minDisparity: Float
    let maxDisparity: Float
    let p05Disparity: Float
    let p50Disparity: Float
    let p95Disparity: Float
    let modelID: String
    let weightsSHA256: String

    enum CodingKeys: String, CodingKey {
        case width
        case height
        case pixelFormat = "pixel_format"
        case minDisparity = "min_disparity"
        case maxDisparity = "max_disparity"
        case p05Disparity = "p05_disparity"
        case p50Disparity = "p50_disparity"
        case p95Disparity = "p95_disparity"
        case modelID = "model_id"
        case weightsSHA256 = "weights_sha256"
    }
}

struct DepthPro2CameraSampleFacts: Encodable {
    let timeSeconds: Double
    let imageSize: [Int]
    let focalLengthPixels: [Double]
    let principalPointPixels: [Double]
    let focalUncertaintyPixels: Double

    enum CodingKeys: String, CodingKey {
        case timeSeconds = "time_seconds"
        case imageSize = "image_size"
        case focalLengthPixels = "focal_length_pixels"
        case principalPointPixels = "principal_point_pixels"
        case focalUncertaintyPixels = "focal_uncertainty_pixels"
    }
}

struct DepthPro2CameraEstimateFacts: Encodable {
    let estimatorName: String
    let revision: String
    let weightsSHA256: String
    let equivalentFocalLength35mm: Double
    let horizontalFOVDegrees: Double
    let verticalFOVDegrees: Double
    let samples: [DepthPro2CameraSampleFacts]
    let sharedIntrinsics: DepthPro2CameraSampleFacts?

    enum CodingKeys: String, CodingKey {
        case estimatorName = "estimator_name"
        case revision
        case weightsSHA256 = "weights_sha256"
        case equivalentFocalLength35mm = "equivalent_focal_length_35mm"
        case horizontalFOVDegrees = "horizontal_fov_degrees"
        case verticalFOVDegrees = "vertical_fov_degrees"
        case samples
        case sharedIntrinsics = "shared_intrinsics"
    }
}

private enum DepthPro2Variant: String {
    case small
    case base

    var packageName: String {
        switch self {
        case .small:
            return "depthpro2-s_w4.mlpackage"
        case .base:
            return "depthpro2-b_w4.mlpackage"
        }
    }

    var modelID: String {
        switch self {
        case .small:
            return "depthpro2-s_w4"
        case .base:
            return "depthpro2-b_w4"
        }
    }
}

private enum DepthPro2Error: LocalizedError {
    case licenseNotAccepted(String)
    case modelMissing(String)
    case invalidInput(String)
    case metalUnavailable(String)
    case inferenceFailed(String)

    var errorDescription: String? {
        switch self {
        case let .licenseNotAccepted(path):
            return "Depth Pro 2 requires explicit Apple AMLR license acceptance in \(path); install models via scripts/distribution/install_depthpro2.py --accept-apple-amlr-license"
        case let .modelMissing(message):
            return message
        case let .invalidInput(message):
            return message
        case let .metalUnavailable(message):
            return message
        case let .inferenceFailed(message):
            return message
        }
    }
}

private enum DepthPro2CoreMLGate {
    private static let lock = NSLock()

    static func withLock<T>(_ body: () throws -> T) rethrows -> T {
        lock.lock()
        defer { lock.unlock() }
        return try body()
    }
}

private enum DepthPro2Catalog {
    static let repoRevisionSHA = "4e607a682f993bbf95035054fc78ba9ffc81faf2"
    static let fovPackageName = "depthpro2-fov_512x896.mlpackage"
    static let licenseReceiptFileName = ".apple-amlr-license-accepted.json"

    static var defaultModelsDirectoryURL: URL {
        if let env = ProcessInfo.processInfo.environment["XDREMUX_MODEL_DIR"]?
            .trimmingCharacters(in: .whitespacesAndNewlines),
           !env.isEmpty
        {
            return URL(fileURLWithPath: env, isDirectory: true)
        }
        let appSupport = FileManager.default.urls(
            for: .applicationSupportDirectory,
            in: .userDomainMask
        ).first ?? FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/Application Support", isDirectory: true)
        return appSupport
            .appendingPathComponent("XDRemux", isDirectory: true)
            .appendingPathComponent("Models", isDirectory: true)
    }

    static func requireLicenseAccepted(in directory: URL) throws {
        if ProcessInfo.processInfo.environment["XDREMUX_ACCEPT_APPLE_AMLR_LICENSE"] == "1" {
            return
        }
        let receipt = directory.appendingPathComponent(licenseReceiptFileName)
        guard FileManager.default.fileExists(atPath: receipt.path) else {
            throw DepthPro2Error.licenseNotAccepted(directory.path)
        }
    }

    static func resolveDisparityPackage(variant requested: String?) throws -> (url: URL, variant: DepthPro2Variant) {
        let root = defaultModelsDirectoryURL
        try requireLicenseAccepted(in: root)
        let fm = FileManager.default

        let candidates: [DepthPro2Variant]
        switch requested?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
        case nil, "", "auto":
            candidates = [.base, .small]
        case "s", "small", "depthpro2-s_w4", "depthpro2-s":
            candidates = [.small]
        case "b", "base", "depthpro2-b_w4", "depthpro2-b":
            candidates = [.base]
        case let other?:
            throw DepthPro2Error.invalidInput("unsupported Depth Pro 2 variant: \(other)")
        }

        for candidate in candidates {
            let pkg = root.appendingPathComponent(candidate.packageName, isDirectory: true)
            let manifest = pkg.appendingPathComponent("Manifest.json")
            if fm.fileExists(atPath: manifest.path) {
                return (pkg, candidate)
            }
        }
        let expected = candidates.map(\.packageName).joined(separator: " or ")
        throw DepthPro2Error.modelMissing(
            "Depth Pro 2 model package missing in \(root.path) (expected \(expected))"
        )
    }

    static func resolveFOVPackage() throws -> URL {
        let root = defaultModelsDirectoryURL
        try requireLicenseAccepted(in: root)
        let pkg = root.appendingPathComponent(fovPackageName, isDirectory: true)
        let manifest = pkg.appendingPathComponent("Manifest.json")
        guard FileManager.default.fileExists(atPath: manifest.path) else {
            throw DepthPro2Error.modelMissing(
                "Depth Pro 2 FOV model missing in \(root.path) (expected \(fovPackageName))"
            )
        }
        return pkg
    }

    static func compiledModelURL(for packageURL: URL) throws -> URL {
        let fm = FileManager.default
        let compiledRoot = packageURL
            .deletingLastPathComponent()
            .appendingPathComponent(".compiled", isDirectory: true)
        try fm.createDirectory(at: compiledRoot, withIntermediateDirectories: true)

        let cacheKey = cacheDirectoryName(for: packageURL)
        let cachedURL = compiledRoot.appendingPathComponent("\(cacheKey).mlmodelc", isDirectory: true)
        if fm.fileExists(atPath: cachedURL.path) {
            return cachedURL
        }

        let stemPrefix = "\(packageURL.deletingPathExtension().lastPathComponent)-"
        if let existing = try? fm.contentsOfDirectory(
            at: compiledRoot,
            includingPropertiesForKeys: nil,
            options: [.skipsHiddenFiles]
        ) {
            for item in existing where item.lastPathComponent.hasPrefix(stemPrefix) {
                try? fm.removeItem(at: item)
            }
        }

        let temporaryCompiled = try MLModel.compileModel(at: packageURL)
        defer { try? fm.removeItem(at: temporaryCompiled) }
        if fm.fileExists(atPath: cachedURL.path) {
            try? fm.removeItem(at: cachedURL)
        }
        try fm.moveItem(at: temporaryCompiled, to: cachedURL)
        return cachedURL
    }

    static func removeCompiledCache(for packageURL: URL) {
        let compiledRoot = packageURL
            .deletingLastPathComponent()
            .appendingPathComponent(".compiled", isDirectory: true)
        let candidate = compiledRoot.appendingPathComponent(
            "\(cacheDirectoryName(for: packageURL)).mlmodelc",
            isDirectory: true
        )
        try? FileManager.default.removeItem(at: candidate)
    }

    static func listFunctions(of compiledURL: URL) -> [String] {
        final class Box: @unchecked Sendable {
            var names: [String] = []
        }
        let box = Box()
        let semaphore = DispatchSemaphore(value: 0)
        Task.detached {
            box.names = (try? await MLModelAsset(url: compiledURL).functionNames) ?? []
            semaphore.signal()
        }
        semaphore.wait()
        return box.names
    }

    static func selectFunctionAndSize(
        sourceWidth: Int,
        sourceHeight: Int,
        availableFunctions: [String],
        preferredMaxLongEdge: Int
    ) -> (width: Int, height: Int, functionName: String)? {
        let parsed: [(width: Int, height: Int, functionName: String)] = availableFunctions.compactMap { fn in
            guard fn.hasPrefix("res_") else { return nil }
            let parts = fn.dropFirst(4).split(separator: "x")
            guard parts.count == 2, let w = Int(parts[0]), let h = Int(parts[1]) else {
                return nil
            }
            return (w, h, fn)
        }.sorted { ($0.width * $0.height) < ($1.width * $1.height) }

        guard !parsed.isEmpty else { return nil }

        let targetAspect = Double(max(sourceWidth, 1)) / Double(max(sourceHeight, 1))
        let candidates = parsed.filter { max($0.width, $0.height) <= preferredMaxLongEdge }
        let pool = candidates.isEmpty ? [parsed[0]] : candidates

        return pool.max { lhs, rhs in
            let lhsAspect = Double(lhs.width) / Double(lhs.height)
            let rhsAspect = Double(rhs.width) / Double(rhs.height)
            let lhsPenalty = abs(log(max(lhsAspect, 0.01) / max(targetAspect, 0.01)))
            let rhsPenalty = abs(log(max(rhsAspect, 0.01) / max(targetAspect, 0.01)))
            if abs(lhsPenalty - rhsPenalty) > 0.05 {
                return lhsPenalty > rhsPenalty
            }
            return (lhs.width * lhs.height) < (rhs.width * rhs.height)
        }
    }

    static func weightsSHA256(for packageURL: URL) throws -> String {
        let weightFile = packageURL
            .appendingPathComponent("Data", isDirectory: true)
            .appendingPathComponent("com.apple.CoreML", isDirectory: true)
            .appendingPathComponent("weights", isDirectory: true)
            .appendingPathComponent("weight.bin")
        let targetURL = FileManager.default.fileExists(atPath: weightFile.path)
            ? weightFile
            : packageURL.appendingPathComponent("Manifest.json")
        guard let stream = InputStream(url: targetURL) else {
            throw DepthPro2Error.invalidInput("Cannot open weights file at \(targetURL.path)")
        }
        stream.open()
        defer { stream.close() }
        var hasher = SHA256()
        let bufferSize = 1024 * 1024
        var buffer = [UInt8](repeating: 0, count: bufferSize)
        while stream.hasBytesAvailable {
            let count = stream.read(&buffer, maxLength: bufferSize)
            if count < 0 {
                throw stream.streamError ?? DepthPro2Error.invalidInput("Failed reading \(targetURL.path)")
            }
            if count == 0 {
                break
            }
            hasher.update(data: buffer.prefix(count))
        }
        return hasher.finalize().map { String(format: "%02x", $0) }.joined()
    }

    private static func cacheDirectoryName(for packageURL: URL) -> String {
        let stem = packageURL.deletingPathExtension().lastPathComponent
        let manifest = packageURL.appendingPathComponent("Manifest.json")
        let attrs = (try? FileManager.default.attributesOfItem(atPath: manifest.path))
            ?? (try? FileManager.default.attributesOfItem(atPath: packageURL.path))
        let mtime = (attrs?[.modificationDate] as? Date)?.timeIntervalSince1970 ?? 0
        let stamp = Int64(mtime * 1000.0)
        return "\(stem)-\(stamp)"
    }
}

private final class DepthPro2MetalContext: @unchecked Sendable {
    static let shared: DepthPro2MetalContext? = try? DepthPro2MetalContext()

    let device: MTLDevice
    let queue: MTLCommandQueue
    let textureCache: CVMetalTextureCache
    let lanczosScaler: MPSImageLanczosScale
    let ciContext: CIContext
    let sRGBColorSpace: CGColorSpace

    let splitPlanarPipeline: MTLComputePipelineState
    let aspectFitReflectPadPipeline: MTLComputePipelineState
    let resizePlanarAntialiasedPipeline: MTLComputePipelineState
    let inverseDepthToDisparityPipeline: MTLComputePipelineState

    struct DisparityConvertParams {
        var contentRect: SIMD4<UInt32>
        var disparityScale: Float
    }

    private static let metalShaderSource = """
        #include <metal_stdlib>
        using namespace metal;

        kernel void split_planar(
            texture2d<half, access::read> rgb [[texture(0)]],
            texture2d<half, access::write> planar [[texture(1)]],
            uint2 gid [[thread_position_in_grid]]
        ) {
            uint h = rgb.get_height();
            if (gid.x >= rgb.get_width() || gid.y >= h) return;
            half4 c = clamp(rgb.read(gid), 0.0h, 1.0h);
            planar.write(half4(c.r), gid);
            planar.write(half4(c.g), uint2(gid.x, gid.y + h));
            planar.write(half4(c.b), uint2(gid.x, gid.y + 2 * h));
        }

        inline int reflect_coord(int i, int n) {
            if (n <= 1) return 0;
            int period = 2 * (n - 1);
            int r = i % period;
            if (r < 0) r += period;
            return r < n ? r : period - r;
        }

        kernel void aspect_fit_reflect_pad_planar(
            texture2d<half, access::read> contentRGB [[texture(0)]],
            texture2d<half, access::write> planar [[texture(1)]],
            constant uint4& contentRect [[buffer(0)]],
            uint2 gid [[thread_position_in_grid]]
        ) {
            uint modelW = planar.get_width();
            uint modelH = planar.get_height() / 3;
            if (gid.x >= modelW || gid.y >= modelH) return;

            int sx = reflect_coord(int(gid.x) - int(contentRect.x), int(contentRect.z));
            int sy = reflect_coord(int(gid.y) - int(contentRect.y), int(contentRect.w));
            half4 c = clamp(contentRGB.read(uint2(sx, sy)), 0.0h, 1.0h);
            planar.write(half4(c.r), gid);
            planar.write(half4(c.g), uint2(gid.x, gid.y + modelH));
            planar.write(half4(c.b), uint2(gid.x, gid.y + 2 * modelH));
        }

        kernel void resize_planar_antialiased(
            texture2d<half, access::read> rgb [[texture(0)]],
            texture2d<half, access::write> planar [[texture(1)]],
            uint2 gid [[thread_position_in_grid]]
        ) {
            uint2 size = uint2(planar.get_width(), planar.get_height() / 3);
            if (gid.x >= size.x || gid.y >= 3 * size.y) return;
            uint channel = gid.y / size.y;
            int2 frame = int2(rgb.get_width(), rgb.get_height());
            int2 crop = min(frame, int2(frame.y * 16 / 9, frame.x * 9 / 16));
            int2 origin = (frame - crop) / 2;
            float2 scale = float2(crop) / float2(size);
            float2 support = max(scale, 1.0);
            float2 centre = scale * (float2(gid.x, gid.y % size.y) + 0.5);
            int2 low = max(int2(centre - support + 0.5), int2(0));
            int2 high = min(int2(centre + support + 0.5), crop);
            float sum = 0.0, total = 0.0;
            for (int y = low.y; y < high.y; y++) {
                float wy = max(1.0 - abs((float(y) - centre.y + 0.5) / support.y), 0.0);
                for (int x = low.x; x < high.x; x++) {
                    float w = wy * max(1.0 - abs((float(x) - centre.x + 0.5) / support.x), 0.0);
                    sum += w * float(rgb.read(uint2(origin + int2(x, y)))[channel]);
                    total += w;
                }
            }
            planar.write(half4(total > 0.0 ? half(sum / total) : 0.0h), gid);
        }

        struct DisparityConvertParams {
            uint4 contentRect;
            float disparityScale;
        };

        kernel void inverse_depth_to_disparity(
            texture2d<half, access::read> inverseDepth [[texture(0)]],
            texture2d<half, access::write> outDisparity [[texture(1)]],
            constant DisparityConvertParams& params [[buffer(0)]],
            uint2 gid [[thread_position_in_grid]]
        ) {
            uint outW = outDisparity.get_width();
            uint outH = outDisparity.get_height();
            if (gid.x >= outW || gid.y >= outH) return;

            float cw = float(max(params.contentRect.z, 1u));
            float ch = float(max(params.contentRect.w, 1u));
            float fx = (float(gid.x) + 0.5f) * cw / float(outW) - 0.5f;
            float fy = (float(gid.y) + 0.5f) * ch / float(outH) - 0.5f;

            float sx = clamp(fx, 0.0f, max(0.0f, cw - 1.0f)) + float(params.contentRect.x);
            float sy = clamp(fy, 0.0f, max(0.0f, ch - 1.0f)) + float(params.contentRect.y);

            int maxX = int(params.contentRect.x + params.contentRect.z) - 1;
            int maxY = int(params.contentRect.y + params.contentRect.w) - 1;
            int x0 = clamp(int(floor(sx)), int(params.contentRect.x), maxX);
            int y0 = clamp(int(floor(sy)), int(params.contentRect.y), maxY);
            int x1 = min(x0 + 1, maxX);
            int y1 = min(y0 + 1, maxY);
            float tx = sx - float(x0);
            float ty = sy - float(y0);

            float d00 = float(inverseDepth.read(uint2(x0, y0)).r);
            float d10 = float(inverseDepth.read(uint2(x1, y0)).r);
            float d01 = float(inverseDepth.read(uint2(x0, y1)).r);
            float d11 = float(inverseDepth.read(uint2(x1, y1)).r);
            float interp = mix(mix(d00, d10, tx), mix(d01, d11, tx), ty);
            float disp = isfinite(interp) ? max(0.0f, interp * params.disparityScale) : 0.0f;
            outDisparity.write(half4(half(disp)), gid);
        }
        """

    private init() throws {
        guard let device = MTLCreateSystemDefaultDevice(),
              let queue = device.makeCommandQueue()
        else {
            throw DepthPro2Error.metalUnavailable("MTLCreateSystemDefaultDevice or queue unavailable")
        }
        self.device = device
        self.queue = queue
        let library = try device.makeLibrary(source: Self.metalShaderSource, options: nil)
        self.lanczosScaler = MPSImageLanczosScale(device: device)

        var cache: CVMetalTextureCache?
        let cacheStatus = CVMetalTextureCacheCreate(kCFAllocatorDefault, nil, device, nil, &cache)
        guard cacheStatus == kCVReturnSuccess, let cache else {
            throw DepthPro2Error.metalUnavailable("CVMetalTextureCacheCreate failed (\(cacheStatus))")
        }
        self.textureCache = cache

        guard let sRGB = CGColorSpace(name: CGColorSpace.sRGB) else {
            throw DepthPro2Error.metalUnavailable("sRGB CGColorSpace unavailable")
        }
        self.sRGBColorSpace = sRGB
        self.ciContext = CIContext(
            mtlDevice: device,
            options: [
                .workingColorSpace: sRGB,
                .cacheIntermediates: false,
            ]
        )

        func makePipeline(_ name: String) throws -> MTLComputePipelineState {
            guard let fn = library.makeFunction(name: name) else {
                throw DepthPro2Error.metalUnavailable("Missing Metal kernel '\(name)'")
            }
            return try device.makeComputePipelineState(function: fn)
        }

        self.splitPlanarPipeline = try makePipeline("split_planar")
        self.aspectFitReflectPadPipeline = try makePipeline("aspect_fit_reflect_pad_planar")
        self.resizePlanarAntialiasedPipeline = try makePipeline("resize_planar_antialiased")
        self.inverseDepthToDisparityPipeline = try makePipeline("inverse_depth_to_disparity")
    }

    func privateTexture(_ format: MTLPixelFormat, width: Int, height: Int) throws -> MTLTexture {
        let descriptor = MTLTextureDescriptor.texture2DDescriptor(
            pixelFormat: format,
            width: width,
            height: height,
            mipmapped: false
        )
        descriptor.usage = [.shaderRead, .shaderWrite]
        descriptor.storageMode = .private
        guard let texture = device.makeTexture(descriptor: descriptor) else {
            throw DepthPro2Error.metalUnavailable("Failed to allocate private texture \(width)x\(height)")
        }
        return texture
    }

    func wrapIOSurfaceTexture(
        _ pixelBuffer: CVPixelBuffer,
        pixelFormat: MTLPixelFormat,
        usage: MTLTextureUsage
    ) throws -> MTLTexture {
        let descriptor = MTLTextureDescriptor.texture2DDescriptor(
            pixelFormat: pixelFormat,
            width: CVPixelBufferGetWidth(pixelBuffer),
            height: CVPixelBufferGetHeight(pixelBuffer),
            mipmapped: false
        )
        descriptor.usage = usage
        guard let surface = CVPixelBufferGetIOSurface(pixelBuffer)?.takeUnretainedValue(),
              let texture = device.makeTexture(descriptor: descriptor, iosurface: surface, plane: 0)
        else {
            throw DepthPro2Error.metalUnavailable("CVPixelBuffer is not IOSurface-backed for Metal")
        }
        return texture
    }

    func makeHalfSurface(width: Int, height: Int) throws -> CVPixelBuffer {
        var buffer: CVPixelBuffer?
        let attributes: [CFString: Any] = [
            kCVPixelBufferIOSurfacePropertiesKey: [String: Any](),
            kCVPixelBufferMetalCompatibilityKey: true,
        ]
        let status = CVPixelBufferCreate(
            kCFAllocatorDefault,
            width,
            height,
            kCVPixelFormatType_OneComponent16Half,
            attributes as CFDictionary,
            &buffer
        )
        guard status == kCVReturnSuccess, let buffer else {
            throw DepthPro2Error.metalUnavailable("CVPixelBufferCreate half surface failed (\(status))")
        }
        return buffer
    }

    func makeBGRA8Surface(from cgImage: CGImage) throws -> CVPixelBuffer {
        let width = cgImage.width
        let height = cgImage.height
        guard width > 0, height > 0 else {
            throw DepthPro2Error.invalidInput("CGImage has zero dimensions")
        }
        var buffer: CVPixelBuffer?
        let attributes: [CFString: Any] = [
            kCVPixelBufferIOSurfacePropertiesKey: [String: Any](),
            kCVPixelBufferMetalCompatibilityKey: true,
            kCVPixelBufferCGImageCompatibilityKey: true,
            kCVPixelBufferCGBitmapContextCompatibilityKey: true,
        ]
        let status = CVPixelBufferCreate(
            kCFAllocatorDefault,
            width,
            height,
            kCVPixelFormatType_32BGRA,
            attributes as CFDictionary,
            &buffer
        )
        guard status == kCVReturnSuccess, let destination = buffer else {
            throw DepthPro2Error.metalUnavailable("CVPixelBufferCreate BGRA8 surface failed (\(status))")
        }

        CVPixelBufferLockBaseAddress(destination, [])
        defer { CVPixelBufferUnlockBaseAddress(destination, []) }
        guard let baseAddress = CVPixelBufferGetBaseAddress(destination) else {
            throw DepthPro2Error.invalidInput("Failed to lock BGRA surface base address")
        }
        let bytesPerRow = CVPixelBufferGetBytesPerRow(destination)
        let bitmapInfo = CGImageAlphaInfo.premultipliedFirst.rawValue | CGBitmapInfo.byteOrder32Little.rawValue
        guard let context = CGContext(
            data: baseAddress,
            width: width,
            height: height,
            bitsPerComponent: 8,
            bytesPerRow: bytesPerRow,
            space: sRGBColorSpace,
            bitmapInfo: bitmapInfo
        ) else {
            throw DepthPro2Error.invalidInput("Failed to create sRGB bitmap context")
        }
        context.draw(cgImage, in: CGRect(x: 0, y: 0, width: width, height: height))
        return destination
    }
}

private final class DepthPro2FOVSession {
    let packageURL: URL
    let weightsSHA256: String
    let inputWidth: Int
    let inputHeight: Int

    private let model: MLModel
    private let metal: DepthPro2MetalContext
    private let inputBuffer: CVPixelBuffer
    private let planarTexture: MTLTexture
    private let features: MLFeatureProvider

    init() throws {
        guard let metal = DepthPro2MetalContext.shared else {
            throw DepthPro2Error.metalUnavailable("Metal context initialization failed")
        }
        self.metal = metal
        let resolvedPackage = try DepthPro2Catalog.resolveFOVPackage()
        self.packageURL = resolvedPackage
        self.weightsSHA256 = try DepthPro2Catalog.weightsSHA256(for: resolvedPackage)

        let compiled = try DepthPro2CoreMLGate.withLock {
            try DepthPro2Catalog.compiledModelURL(for: resolvedPackage)
        }
        let config = MLModelConfiguration()
        config.computeUnits = .cpuAndGPU

        let loadedModel: MLModel
        do {
            loadedModel = try DepthPro2CoreMLGate.withLock {
                try MLModel(contentsOf: compiled, configuration: config)
            }
        } catch {
            DepthPro2Catalog.removeCompiledCache(for: resolvedPackage)
            let recompiled = try DepthPro2CoreMLGate.withLock {
                try DepthPro2Catalog.compiledModelURL(for: resolvedPackage)
            }
            loadedModel = try DepthPro2CoreMLGate.withLock {
                try MLModel(contentsOf: recompiled, configuration: config)
            }
        }
        self.model = loadedModel

        guard let shape = loadedModel.modelDescription.inputDescriptionsByName["frame"]?.multiArrayConstraint?.shape,
              shape.count == 4
        else {
            throw DepthPro2Error.invalidInput("FOV model missing 'frame' input (1, 3, H, W)")
        }
        self.inputHeight = shape[2].intValue
        self.inputWidth = shape[3].intValue

        self.inputBuffer = try metal.makeHalfSurface(width: inputWidth, height: 3 * inputHeight)
        self.planarTexture = try metal.wrapIOSurfaceTexture(
            inputBuffer,
            pixelFormat: .r16Float,
            usage: .shaderWrite
        )
        let multiArray = MLMultiArray(
            pixelBuffer: inputBuffer,
            shape: [1, 3, inputHeight, inputWidth].map { NSNumber(value: $0) }
        )
        self.features = try MLDictionaryFeatureProvider(dictionary: [
            "frame": MLFeatureValue(multiArray: multiArray),
        ])
    }

    func predictFOVDegrees(cgImage: CGImage) throws -> (hfov: Double, vfov: Double) {
        let bgraBuffer = try metal.makeBGRA8Surface(from: cgImage)
        let srcWidth = CVPixelBufferGetWidth(bgraBuffer)
        let srcHeight = CVPixelBufferGetHeight(bgraBuffer)

        var cvTexture: CVMetalTexture?
        let status = CVMetalTextureCacheCreateTextureFromImage(
            kCFAllocatorDefault,
            metal.textureCache,
            bgraBuffer,
            nil,
            .bgra8Unorm,
            srcWidth,
            srcHeight,
            0,
            &cvTexture
        )
        guard status == kCVReturnSuccess,
              let cvTexture,
              let sourceTexture = CVMetalTextureGetTexture(cvTexture),
              let commands = metal.queue.makeCommandBuffer(),
              let encoder = commands.makeComputeCommandEncoder()
        else {
            throw DepthPro2Error.metalUnavailable("Failed to encode FOV resize Metal command")
        }

        encoder.setComputePipelineState(metal.resizePlanarAntialiasedPipeline)
        encoder.setTexture(sourceTexture, index: 0)
        encoder.setTexture(planarTexture, index: 1)
        encoder.dispatchThreads(
            MTLSize(width: inputWidth, height: 3 * inputHeight, depth: 1),
            threadsPerThreadgroup: MTLSize(width: 16, height: 16, depth: 1)
        )
        encoder.endEncoding()
        commands.commit()
        commands.waitUntilCompleted()
        CVMetalTextureCacheFlush(metal.textureCache, 0)
        if let error = commands.error {
            throw DepthPro2Error.inferenceFailed("FOV preprocess failed: \(error.localizedDescription)")
        }

        let prediction = try autoreleasepool {
            try DepthPro2CoreMLGate.withLock {
                try model.prediction(from: features)
            }
        }
        guard let fovArray = prediction.featureValue(for: "fov")?.multiArrayValue,
              fovArray.count >= 2
        else {
            throw DepthPro2Error.inferenceFailed("FOV model output missing 'fov' array")
        }
        let hfov = fovArray[0].doubleValue
        let vfov = fovArray[1].doubleValue
        guard hfov.isFinite, vfov.isFinite, hfov > 1.0, hfov < 175.0, vfov > 1.0, vfov < 175.0 else {
            throw DepthPro2Error.inferenceFailed("FOV model produced non-finite values (\(hfov), \(vfov))")
        }
        return (hfov, vfov)
    }

    static func focalPixels(verticalFOVDegrees: Double, width: Int, height: Int) -> Double {
        let clampedVFOV = min(max(verticalFOVDegrees, 5.0), 170.0)
        let cropHeight = Double(min(height, width * 9 / 16))
        return 0.5 * cropHeight / tan(clampedVFOV * .pi / 360.0)
    }

    static func horizontalEquivalent35mm(focalPixels: Double, width: Int, height: Int) -> Double {
        36.0 * focalPixels / Double(max(1, max(width, height)))
    }
}

private enum DepthPro2PreprocessMode {
    case fillCrop
    case aspectFitReflectPad
}

private struct DepthPro2LayoutPlan {
    let mode: DepthPro2PreprocessMode
    let contentOffsetX: Int
    let contentOffsetY: Int
    let contentWidth: Int
    let contentHeight: Int
    let effectiveScale: Double
}

private final class DepthPro2DisparitySession {
    let variant: DepthPro2Variant
    let packageURL: URL
    let weightsSHA256: String
    let modelWidth: Int
    let modelHeight: Int

    private let metal: DepthPro2MetalContext
    private let model: MLModel
    private let inputBuffer: CVPixelBuffer
    private let outputBuffer: CVPixelBuffer
    private let inputPlanarTexture: MTLTexture
    private let outputInverseDepthTexture: MTLTexture
    private let fillResizedTexture: MTLTexture
    private let features: MLFeatureProvider
    private let predictionOptions: MLPredictionOptions
    private var state: MLState?

    init(
        variant requestedVariant: String?,
        sourceWidth: Int,
        sourceHeight: Int,
        preferredMaxLongEdge: Int = 1280
    ) throws {
        guard let metal = DepthPro2MetalContext.shared else {
            throw DepthPro2Error.metalUnavailable("Metal context initialization failed")
        }
        self.metal = metal

        let resolved = try DepthPro2Catalog.resolveDisparityPackage(variant: requestedVariant)
        self.packageURL = resolved.url
        self.variant = resolved.variant
        self.weightsSHA256 = try DepthPro2Catalog.weightsSHA256(for: resolved.url)

        let compiledURL = try DepthPro2CoreMLGate.withLock {
            try DepthPro2Catalog.compiledModelURL(for: resolved.url)
        }
        let functions = DepthPro2Catalog.listFunctions(of: compiledURL)
        let chosen = DepthPro2Catalog.selectFunctionAndSize(
            sourceWidth: sourceWidth,
            sourceHeight: sourceHeight,
            availableFunctions: functions,
            preferredMaxLongEdge: preferredMaxLongEdge
        )

        // Small model targets Neural Engine with GPU fallback; Base uses CPU+GPU to avoid Float16 ANE overflow.
        let preferredUnits: MLComputeUnits = (resolved.variant == .small) ? .cpuAndNeuralEngine : .cpuAndGPU
        func makeConfig(units: MLComputeUnits) -> MLModelConfiguration {
            let config = MLModelConfiguration()
            config.computeUnits = units
            config.functionName = chosen?.functionName
            return config
        }

        let loadedModel: MLModel
        do {
            loadedModel = try DepthPro2CoreMLGate.withLock {
                try MLModel(contentsOf: compiledURL, configuration: makeConfig(units: preferredUnits))
            }
        } catch {
            if preferredUnits == .cpuAndNeuralEngine {
                loadedModel = try DepthPro2CoreMLGate.withLock {
                    try MLModel(contentsOf: compiledURL, configuration: makeConfig(units: .cpuAndGPU))
                }
            } else {
                DepthPro2Catalog.removeCompiledCache(for: resolved.url)
                let recompiled = try DepthPro2CoreMLGate.withLock {
                    try DepthPro2Catalog.compiledModelURL(for: resolved.url)
                }
                loadedModel = try DepthPro2CoreMLGate.withLock {
                    try MLModel(contentsOf: recompiled, configuration: makeConfig(units: preferredUnits))
                }
            }
        }
        self.model = loadedModel

        guard let shape = loadedModel.modelDescription.inputDescriptionsByName["frame"]?.multiArrayConstraint?.shape,
              shape.count == 4
        else {
            throw DepthPro2Error.invalidInput("Depth Pro 2 model missing 'frame' input")
        }
        guard loadedModel.modelDescription.outputDescriptionsByName["inverse_depth"] != nil else {
            throw DepthPro2Error.invalidInput("Depth Pro 2 model missing 'inverse_depth' output")
        }

        self.modelHeight = shape[2].intValue
        self.modelWidth = shape[3].intValue
        self.inputBuffer = try metal.makeHalfSurface(width: modelWidth, height: 3 * modelHeight)
        self.outputBuffer = try metal.makeHalfSurface(width: modelWidth, height: modelHeight)
        self.inputPlanarTexture = try metal.wrapIOSurfaceTexture(
            inputBuffer,
            pixelFormat: .r16Float,
            usage: .shaderWrite
        )
        self.outputInverseDepthTexture = try metal.wrapIOSurfaceTexture(
            outputBuffer,
            pixelFormat: .r16Float,
            usage: .shaderRead
        )
        self.fillResizedTexture = try metal.privateTexture(
            .rgba16Float,
            width: modelWidth,
            height: modelHeight
        )

        let multiArray = MLMultiArray(
            pixelBuffer: inputBuffer,
            shape: [1, 3, modelHeight, modelWidth].map { NSNumber(value: $0) }
        )
        self.features = try MLDictionaryFeatureProvider(dictionary: [
            "frame": MLFeatureValue(multiArray: multiArray),
        ])
        let outputArray = MLMultiArray(
            pixelBuffer: outputBuffer,
            shape: [1, modelHeight, modelWidth].map { NSNumber(value: $0) }
        )
        let options = MLPredictionOptions()
        options.outputBackings = ["inverse_depth": outputArray]
        self.predictionOptions = options

        if !loadedModel.modelDescription.stateDescriptionsByName.isEmpty {
            self.state = loadedModel.makeState()
        }
    }

    func estimateDisparity(
        cgImage: CGImage,
        focalLengthPixels: Double,
        outputWidth: Int,
        outputHeight: Int
    ) throws -> (values: [Float], stats: (min: Float, max: Float, p05: Float, p50: Float, p95: Float)) {
        guard focalLengthPixels.isFinite, focalLengthPixels > 1.0 else {
            throw DepthPro2Error.invalidInput("Focal length in pixels must be > 1.0")
        }
        let bgra = try metal.makeBGRA8Surface(from: cgImage)
        let srcWidth = CVPixelBufferGetWidth(bgra)
        let srcHeight = CVPixelBufferGetHeight(bgra)
        let plan = makeLayoutPlan(sourceWidth: srcWidth, sourceHeight: srcHeight)

        try encodePreprocess(bgraBuffer: bgra, plan: plan)

        try autoreleasepool {
            try DepthPro2CoreMLGate.withLock {
                if let state {
                    _ = try model.prediction(from: features, using: state, options: predictionOptions)
                } else {
                    _ = try model.prediction(from: features, options: predictionOptions)
                }
            }
        }

        let focalInModelPixels = focalLengthPixels * plan.effectiveScale
        let disparityScale = Float(Double(modelWidth) / max(focalInModelPixels, 1e-3))

        let outHalf = try encodeDisparityConversion(
            outputWidth: outputWidth,
            outputHeight: outputHeight,
            plan: plan,
            disparityScale: disparityScale
        )
        let values = readFloatValues(from: outHalf)
        let stats = computeStats(values: values)
        return (values, stats)
    }

    private func makeLayoutPlan(sourceWidth: Int, sourceHeight: Int) -> DepthPro2LayoutPlan {
        let srcAspect = Double(max(sourceWidth, 1)) / Double(max(sourceHeight, 1))
        let modelAspect = Double(max(modelWidth, 1)) / Double(max(modelHeight, 1))
        let relDiff = abs(srcAspect - modelAspect) / max(modelAspect, 0.001)
        if relDiff <= 0.03 {
            let fillScale = max(
                Double(modelWidth) / Double(sourceWidth),
                Double(modelHeight) / Double(sourceHeight)
            )
            return DepthPro2LayoutPlan(
                mode: .fillCrop,
                contentOffsetX: 0,
                contentOffsetY: 0,
                contentWidth: modelWidth,
                contentHeight: modelHeight,
                effectiveScale: fillScale
            )
        }

        let fitScale = min(
            Double(modelWidth) / Double(sourceWidth),
            Double(modelHeight) / Double(sourceHeight)
        )
        let cw = min(modelWidth, max(1, Int((Double(sourceWidth) * fitScale).rounded())))
        let ch = min(modelHeight, max(1, Int((Double(sourceHeight) * fitScale).rounded())))
        let ox = max(0, (modelWidth - cw) / 2)
        let oy = max(0, (modelHeight - ch) / 2)
        return DepthPro2LayoutPlan(
            mode: .aspectFitReflectPad,
            contentOffsetX: ox,
            contentOffsetY: oy,
            contentWidth: cw,
            contentHeight: ch,
            effectiveScale: fitScale
        )
    }

    private func encodePreprocess(bgraBuffer: CVPixelBuffer, plan: DepthPro2LayoutPlan) throws {
        let srcWidth = CVPixelBufferGetWidth(bgraBuffer)
        let srcHeight = CVPixelBufferGetHeight(bgraBuffer)

        var cvTexture: CVMetalTexture?
        let status = CVMetalTextureCacheCreateTextureFromImage(
            kCFAllocatorDefault,
            metal.textureCache,
            bgraBuffer,
            nil,
            .bgra8Unorm,
            srcWidth,
            srcHeight,
            0,
            &cvTexture
        )
        guard status == kCVReturnSuccess,
              let cvTexture,
              let sourceTexture = CVMetalTextureGetTexture(cvTexture),
              let commands = metal.queue.makeCommandBuffer()
        else {
            throw DepthPro2Error.metalUnavailable("Failed to wrap BGRA buffer in Metal texture")
        }

        let destinationTexture: MTLTexture
        let scaleX: Double
        let scaleY: Double
        let translateX: Double
        let translateY: Double

        switch plan.mode {
        case .fillCrop:
            destinationTexture = fillResizedTexture
            scaleX = plan.effectiveScale
            scaleY = plan.effectiveScale
            translateX = (Double(modelWidth) - Double(srcWidth) * plan.effectiveScale) / 2.0
            translateY = (Double(modelHeight) - Double(srcHeight) * plan.effectiveScale) / 2.0
        case .aspectFitReflectPad:
            destinationTexture = try metal.privateTexture(
                .rgba16Float,
                width: plan.contentWidth,
                height: plan.contentHeight
            )
            scaleX = Double(plan.contentWidth) / Double(srcWidth)
            scaleY = Double(plan.contentHeight) / Double(srcHeight)
            translateX = 0.0
            translateY = 0.0
        }

        var transform = MPSScaleTransform(
            scaleX: scaleX,
            scaleY: scaleY,
            translateX: translateX,
            translateY: translateY
        )
        withUnsafePointer(to: &transform) { pointer in
            metal.lanczosScaler.scaleTransform = pointer
            metal.lanczosScaler.encode(
                commandBuffer: commands,
                sourceTexture: sourceTexture,
                destinationTexture: destinationTexture
            )
            metal.lanczosScaler.scaleTransform = nil
        }

        guard let encoder = commands.makeComputeCommandEncoder() else {
            throw DepthPro2Error.metalUnavailable("Failed to create compute command encoder")
        }

        switch plan.mode {
        case .fillCrop:
            encoder.setComputePipelineState(metal.splitPlanarPipeline)
            encoder.setTexture(destinationTexture, index: 0)
            encoder.setTexture(inputPlanarTexture, index: 1)
        case .aspectFitReflectPad:
            var rect = SIMD4<UInt32>(
                UInt32(plan.contentOffsetX),
                UInt32(plan.contentOffsetY),
                UInt32(plan.contentWidth),
                UInt32(plan.contentHeight)
            )
            encoder.setComputePipelineState(metal.aspectFitReflectPadPipeline)
            encoder.setTexture(destinationTexture, index: 0)
            encoder.setTexture(inputPlanarTexture, index: 1)
            encoder.setBytes(&rect, length: MemoryLayout<SIMD4<UInt32>>.size, index: 0)
        }

        encoder.dispatchThreads(
            MTLSize(width: modelWidth, height: modelHeight, depth: 1),
            threadsPerThreadgroup: MTLSize(width: 16, height: 16, depth: 1)
        )
        encoder.endEncoding()
        commands.commit()
        commands.waitUntilCompleted()
        CVMetalTextureCacheFlush(metal.textureCache, 0)
        if let error = commands.error {
            throw DepthPro2Error.inferenceFailed("Preprocess kernel failed: \(error.localizedDescription)")
        }
    }

    private func encodeDisparityConversion(
        outputWidth: Int,
        outputHeight: Int,
        plan: DepthPro2LayoutPlan,
        disparityScale: Float
    ) throws -> CVPixelBuffer {
        let outHalf = try metal.makeHalfSurface(width: outputWidth, height: outputHeight)
        let outTexture = try metal.wrapIOSurfaceTexture(
            outHalf,
            pixelFormat: .r16Float,
            usage: .shaderWrite
        )
        guard let commands = metal.queue.makeCommandBuffer(),
              let encoder = commands.makeComputeCommandEncoder()
        else {
            throw DepthPro2Error.metalUnavailable("Failed to encode disparity conversion kernel")
        }

        var params = DepthPro2MetalContext.DisparityConvertParams(
            contentRect: SIMD4<UInt32>(
                UInt32(plan.contentOffsetX),
                UInt32(plan.contentOffsetY),
                UInt32(plan.contentWidth),
                UInt32(plan.contentHeight)
            ),
            disparityScale: disparityScale
        )
        encoder.setComputePipelineState(metal.inverseDepthToDisparityPipeline)
        encoder.setTexture(outputInverseDepthTexture, index: 0)
        encoder.setTexture(outTexture, index: 1)
        encoder.setBytes(&params, length: MemoryLayout<DepthPro2MetalContext.DisparityConvertParams>.size, index: 0)
        encoder.dispatchThreads(
            MTLSize(width: outputWidth, height: outputHeight, depth: 1),
            threadsPerThreadgroup: MTLSize(width: 16, height: 16, depth: 1)
        )
        encoder.endEncoding()
        commands.commit()
        commands.waitUntilCompleted()
        if let error = commands.error {
            throw DepthPro2Error.inferenceFailed("Disparity conversion failed: \(error.localizedDescription)")
        }
        return outHalf
    }

    private func readFloatValues(from halfBuffer: CVPixelBuffer) -> [Float] {
        let width = CVPixelBufferGetWidth(halfBuffer)
        let height = CVPixelBufferGetHeight(halfBuffer)
        CVPixelBufferLockBaseAddress(halfBuffer, .readOnly)
        defer { CVPixelBufferUnlockBaseAddress(halfBuffer, .readOnly) }
        guard let base = CVPixelBufferGetBaseAddress(halfBuffer) else { return [] }
        let bytesPerRow = CVPixelBufferGetBytesPerRow(halfBuffer)

        var result = [Float](repeating: 0, count: width * height)
        for y in 0..<height {
            let row = (base + y * bytesPerRow).assumingMemoryBound(to: Float16.self)
            let rowBase = y * width
            for x in 0..<width {
                let v = Float(row[x])
                result[rowBase + x] = (v.isFinite && v >= 0) ? v : 0
            }
        }
        return result
    }

    private func computeStats(values: [Float]) -> (min: Float, max: Float, p05: Float, p50: Float, p95: Float) {
        let finite = values.filter { $0.isFinite && $0 >= 0 }
        guard !finite.isEmpty else { return (0, 0, 0, 0, 0) }
        let sorted = finite.sorted()
        let last = Double(sorted.count - 1)
        return (
            sorted[0],
            sorted[sorted.count - 1],
            sorted[Int(0.05 * last)],
            sorted[Int(0.50 * last)],
            sorted[Int(0.95 * last)]
        )
    }
}

private func loadOrientedCGImage(
    inputPath: String,
    orientationOverride: UInt32?
) throws -> (oriented: CGImage, properties: [CFString: Any], orientation: UInt32) {
    let url = URL(fileURLWithPath: inputPath)
    guard let source = CGImageSourceCreateWithURL(url as CFURL, nil) else {
        throw DepthPro2Error.invalidInput("ImageIO cannot open \(inputPath)")
    }
    let properties = (CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any]) ?? [:]
    guard let baseImage = CGImageSourceCreateImageAtIndex(source, 0, nil) else {
        throw DepthPro2Error.invalidInput("ImageIO cannot decode primary image at \(inputPath)")
    }
    let metadataOrientation = (properties[kCGImagePropertyOrientation] as? NSNumber)?.uint32Value ?? 1
    let rawOrientation = orientationOverride ?? metadataOrientation
    guard (1...8).contains(rawOrientation),
          let cgOrientation = CGImagePropertyOrientation(rawValue: rawOrientation)
    else {
        throw DepthPro2Error.invalidInput("Invalid EXIF orientation \(rawOrientation)")
    }
    if rawOrientation == 1 {
        return (baseImage, properties, rawOrientation)
    }
    guard let metal = DepthPro2MetalContext.shared else {
        throw DepthPro2Error.metalUnavailable("Metal context unavailable for orientation transform")
    }
    let ciImage = CIImage(cgImage: baseImage).oriented(cgOrientation)
    guard let oriented = metal.ciContext.createCGImage(ciImage, from: ciImage.extent) else {
        throw DepthPro2Error.invalidInput("Failed to apply EXIF orientation \(rawOrientation)")
    }
    return (oriented, properties, rawOrientation)
}

private func inverseOrientFloatGrid(
    _ values: [Float],
    orientedWidth w: Int,
    orientedHeight h: Int,
    orientation: UInt32
) -> (width: Int, height: Int, values: [Float]) {
    guard values.count == w * h else {
        return (w, h, values)
    }
    switch orientation {
    case 1:
        return (w, h, values)
    case 2:
        var out = [Float](repeating: 0, count: w * h)
        for y in 0..<h {
            for x in 0..<w {
                out[y * w + (w - 1 - x)] = values[y * w + x]
            }
        }
        return (w, h, out)
    case 3:
        var out = [Float](repeating: 0, count: w * h)
        for y in 0..<h {
            for x in 0..<w {
                out[(h - 1 - y) * w + (w - 1 - x)] = values[y * w + x]
            }
        }
        return (w, h, out)
    case 4:
        var out = [Float](repeating: 0, count: w * h)
        for y in 0..<h {
            for x in 0..<w {
                out[(h - 1 - y) * w + x] = values[y * w + x]
            }
        }
        return (w, h, out)
    case 5:
        let outW = h
        let outH = w
        var out = [Float](repeating: 0, count: outW * outH)
        for y in 0..<h {
            for x in 0..<w {
                out[x * outW + y] = values[y * w + x]
            }
        }
        return (outW, outH, out)
    case 6:
        let outW = h
        let outH = w
        var out = [Float](repeating: 0, count: outW * outH)
        for y in 0..<h {
            for x in 0..<w {
                let rx = y
                let ry = w - 1 - x
                out[ry * outW + rx] = values[y * w + x]
            }
        }
        return (outW, outH, out)
    case 7:
        let outW = h
        let outH = w
        var out = [Float](repeating: 0, count: outW * outH)
        for y in 0..<h {
            for x in 0..<w {
                let rx = h - 1 - y
                let ry = w - 1 - x
                out[ry * outW + rx] = values[y * w + x]
            }
        }
        return (outW, outH, out)
    case 8:
        let outW = h
        let outH = w
        var out = [Float](repeating: 0, count: outW * outH)
        for y in 0..<h {
            for x in 0..<w {
                let rx = h - 1 - y
                let ry = x
                out[ry * outW + rx] = values[y * w + x]
            }
        }
        return (outW, outH, out)
    default:
        return (w, h, values)
    }
}

func runDepthPro2Disparity(
    inputPath: String,
    outputPath: String,
    configuration: DepthPro2DisparityRequest
) throws -> DepthPro2DisparityFacts {
    let targetW = Int(configuration.targetWidth)
    let targetH = Int(configuration.targetHeight)
    guard targetW > 0, targetH > 0 else {
        throw DepthPro2Error.invalidInput("Depth Pro 2 target geometry must be positive")
    }

    let (orientedImage, properties, rawOrientation) = try loadOrientedCGImage(
        inputPath: inputPath,
        orientationOverride: configuration.orientation
    )
    let orientedW = orientedImage.width
    let orientedH = orientedImage.height

    let focalLength35mm: Double
    if let explicit35 = configuration.focalLength35mm, explicit35.isFinite, explicit35 > 1.0 {
        focalLength35mm = explicit35
    } else if let exif = properties[kCGImagePropertyExifDictionary] as? [CFString: Any],
              let exif35 = (exif[kCGImagePropertyExifFocalLenIn35mmFilm] as? NSNumber)?.doubleValue,
              exif35.isFinite, exif35 > 1.0
    {
        focalLength35mm = exif35
    } else {
        let fovSession = try DepthPro2FOVSession()
        let (_, vfov) = try fovSession.predictFOVDegrees(cgImage: orientedImage)
        let fovPx = DepthPro2FOVSession.focalPixels(
            verticalFOVDegrees: vfov,
            width: orientedW,
            height: orientedH
        )
        focalLength35mm = DepthPro2FOVSession.horizontalEquivalent35mm(
            focalPixels: fovPx,
            width: orientedW,
            height: orientedH
        )
    }

    let focalPixels = Double(max(orientedW, orientedH)) * (focalLength35mm / 36.0)
    let axesSwapped = (5...8).contains(rawOrientation)
    let outOrientedW = axesSwapped ? targetH : targetW
    let outOrientedH = axesSwapped ? targetW : targetH

    let session = try DepthPro2DisparitySession(
        variant: configuration.variant,
        sourceWidth: orientedW,
        sourceHeight: orientedH
    )
    let result = try session.estimateDisparity(
        cgImage: orientedImage,
        focalLengthPixels: focalPixels,
        outputWidth: outOrientedW,
        outputHeight: outOrientedH
    )
    let stored = inverseOrientFloatGrid(
        result.values,
        orientedWidth: outOrientedW,
        orientedHeight: outOrientedH,
        orientation: rawOrientation
    )
    guard stored.width == targetW, stored.height == targetH, stored.values.count == targetW * targetH else {
        throw DepthPro2Error.inferenceFailed("Disparity orientation inverse transform size mismatch")
    }

    var rawBytes = Data(capacity: targetW * targetH * 2)
    for value in stored.values {
        let clamped = (value.isFinite && value > 0) ? value : 0
        var le = Float16(clamped).bitPattern.littleEndian
        withUnsafeBytes(of: &le) { rawBytes.append(contentsOf: $0) }
    }
    try rawBytes.write(to: URL(fileURLWithPath: outputPath), options: .atomic)

    return DepthPro2DisparityFacts(
        width: targetW,
        height: targetH,
        pixelFormat: 0x68646973,
        minDisparity: result.stats.min,
        maxDisparity: result.stats.max,
        p05Disparity: result.stats.p05,
        p50Disparity: result.stats.p50,
        p95Disparity: result.stats.p95,
        modelID: session.variant.modelID,
        weightsSHA256: session.weightsSHA256
    )
}

private func isVideoAsset(at url: URL) -> Bool {
    let asset = AVURLAsset(url: url)
    final class Box: @unchecked Sendable {
        var hasVideo = false
    }
    let box = Box()
    let semaphore = DispatchSemaphore(value: 0)
    Task.detached {
        let tracks = (try? await asset.loadTracks(withMediaType: .video)) ?? []
        box.hasVideo = !tracks.isEmpty
        semaphore.signal()
    }
    semaphore.wait()
    return box.hasVideo
}

private func generateVideoFrame(generator: AVAssetImageGenerator, time: CMTime) throws -> CGImage {
    final class ImageBox: @unchecked Sendable {
        var image: CGImage?
        var error: Error?
    }
    let box = ImageBox()
    let semaphore = DispatchSemaphore(value: 0)
    Task.detached {
        do {
            let result = try await generator.image(at: time)
            box.image = result.image
        } catch {
            box.error = error
        }
        semaphore.signal()
    }
    semaphore.wait()
    if let error = box.error {
        throw error
    }
    guard let image = box.image else {
        throw DepthPro2Error.inferenceFailed("AVAssetImageGenerator produced no image")
    }
    return image
}

private func sampleVideoFrames(at url: URL, count: Int) throws -> [(timeSeconds: Double, image: CGImage)] {
    let asset = AVURLAsset(url: url)
    final class Box: @unchecked Sendable {
        var durationSeconds: Double = 0
        var error: Error?
    }
    let box = Box()
    let semaphore = DispatchSemaphore(value: 0)
    Task.detached {
        do {
            let tracks = try await asset.loadTracks(withMediaType: .video)
            guard !tracks.isEmpty else {
                throw DepthPro2Error.invalidInput("Input video has no video track")
            }
            let duration = try await asset.load(.duration)
            box.durationSeconds = duration.seconds
        } catch {
            box.error = error
        }
        semaphore.signal()
    }
    semaphore.wait()
    if let error = box.error {
        throw error
    }
    guard box.durationSeconds.isFinite, box.durationSeconds > 0 else {
        throw DepthPro2Error.invalidInput("Video duration must be finite and positive")
    }

    let generator = AVAssetImageGenerator(asset: asset)
    generator.appliesPreferredTrackTransform = true
    let tolerance = CMTime(seconds: 0.25, preferredTimescale: 600)
    generator.requestedTimeToleranceBefore = tolerance
    generator.requestedTimeToleranceAfter = tolerance

    var frames: [(Double, CGImage)] = []
    var previousTime = -1.0
    for index in 0..<count {
        let target = box.durationSeconds * (Double(index) + 0.5) / Double(count)
        let cmTime = CMTime(seconds: target, preferredTimescale: 600)
        let cgImage: CGImage
        do {
            cgImage = try generateVideoFrame(generator: generator, time: cmTime)
        } catch {
            generator.requestedTimeToleranceBefore = .positiveInfinity
            generator.requestedTimeToleranceAfter = .positiveInfinity
            cgImage = try generateVideoFrame(generator: generator, time: .zero)
        }
        let timestamp = max(target, previousTime + 0.001)
        previousTime = timestamp
        frames.append((timestamp, cgImage))
    }
    return frames
}

func runDepthPro2CameraEstimate(
    inputPath: String,
    configuration: DepthPro2CameraRequest?
) throws -> DepthPro2CameraEstimateFacts {
    let inputURL = URL(fileURLWithPath: inputPath)
    guard FileManager.default.fileExists(atPath: inputURL.path) else {
        throw DepthPro2Error.invalidInput("Input file not found: \(inputPath)")
    }

    let requestedCount = Int(configuration?.sampleCount ?? 5)
    let sampleCount = min(32, max(3, requestedCount))
    let session = try DepthPro2FOVSession()

    let frames: [(timeSeconds: Double, image: CGImage)]
    if isVideoAsset(at: inputURL) {
        frames = try sampleVideoFrames(at: inputURL, count: sampleCount)
    } else {
        let (oriented, _, _) = try loadOrientedCGImage(inputPath: inputPath, orientationOverride: nil)
        frames = [(0.0, oriented)]
    }

    var samples: [DepthPro2CameraSampleFacts] = []
    var focalPxValues: [Double] = []
    var hfovValues: [Double] = []
    var vfovValues: [Double] = []

    for (timeSeconds, cgImage) in frames {
        let width = cgImage.width
        let height = cgImage.height
        let (hfov, vfov) = try session.predictFOVDegrees(cgImage: cgImage)
        let focalPx = DepthPro2FOVSession.focalPixels(
            verticalFOVDegrees: vfov,
            width: width,
            height: height
        )
        let uncertainty = max(1.0, focalPx * 0.05)
        samples.append(
            DepthPro2CameraSampleFacts(
                timeSeconds: timeSeconds,
                imageSize: [width, height],
                focalLengthPixels: [focalPx, focalPx],
                principalPointPixels: [Double(width) / 2.0, Double(height) / 2.0],
                focalUncertaintyPixels: uncertainty
            )
        )
        focalPxValues.append(focalPx)
        hfovValues.append(hfov)
        vfovValues.append(vfov)
    }

    func median(_ values: [Double]) -> Double {
        let sorted = values.sorted()
        return sorted[(sorted.count - 1) / 2]
    }

    let refWidth = frames[0].image.width
    let refHeight = frames[0].image.height
    let medianFocalPx = median(focalPxValues)
    let medianHFOV = median(hfovValues)
    let medianVFOV = median(vfovValues)
    let equivalent35 = DepthPro2FOVSession.horizontalEquivalent35mm(
        focalPixels: medianFocalPx,
        width: refWidth,
        height: refHeight
    )

    let shared = samples.count >= 3
        ? DepthPro2CameraSampleFacts(
            timeSeconds: 0.0,
            imageSize: [refWidth, refHeight],
            focalLengthPixels: [medianFocalPx, medianFocalPx],
            principalPointPixels: [Double(refWidth) / 2.0, Double(refHeight) / 2.0],
            focalUncertaintyPixels: max(1.0, medianFocalPx * 0.05)
        )
        : nil

    return DepthPro2CameraEstimateFacts(
        estimatorName: "depthpro2-fov",
        revision: DepthPro2Catalog.repoRevisionSHA,
        weightsSHA256: session.weightsSHA256,
        equivalentFocalLength35mm: equivalent35,
        horizontalFOVDegrees: medianHFOV,
        verticalFOVDegrees: medianVFOV,
        samples: samples,
        sharedIntrinsics: shared
    )
}
