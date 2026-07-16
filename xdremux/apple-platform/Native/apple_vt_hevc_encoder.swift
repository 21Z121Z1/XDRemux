#!/usr/bin/env swift
import CoreGraphics
import CoreMedia
import CoreVideo
import Foundation
import ImageIO
import VideoToolbox

final class EncoderState {
    var annexB = Data()
    var hvcc = Data()
    var wroteParameterSets = false
    var error: String?
}

enum PixelMode: String {
    case rgb10
    case mono8
}

func fail(_ message: String) -> Never {
    FileHandle.standardError.write(Data((message + "\n").utf8))
    exit(1)
}

func check(_ status: OSStatus, _ message: String) {
    if status != noErr {
        fail("\(message): OSStatus \(status)")
    }
}

func loadImage(_ path: String) -> CGImage {
    let url = URL(fileURLWithPath: path)
    guard let source = CGImageSourceCreateWithURL(url as CFURL, nil),
          let image = CGImageSourceCreateImageAtIndex(source, 0, nil) else {
        fail("Could not load image: \(path)")
    }
    return image
}

func makePixelBuffer(from image: CGImage, mode: PixelMode) -> CVPixelBuffer {
    let width = image.width
    let height = image.height
    let pixelFormat = mode == .mono8
        ? kCVPixelFormatType_OneComponent8
        : kCVPixelFormatType_32BGRA
    let attrs: CFDictionary = [
        kCVPixelBufferCGImageCompatibilityKey: true,
        kCVPixelBufferCGBitmapContextCompatibilityKey: true,
        kCVPixelBufferIOSurfacePropertiesKey: [:]
    ] as CFDictionary

    var pixelBuffer: CVPixelBuffer?
    check(
        CVPixelBufferCreate(
            kCFAllocatorDefault,
            width,
            height,
            pixelFormat,
            attrs,
            &pixelBuffer
        ),
        "CVPixelBufferCreate failed"
    )
    guard let buffer = pixelBuffer else {
        fail("CVPixelBufferCreate returned nil")
    }

    CVPixelBufferLockBaseAddress(buffer, [])
    defer { CVPixelBufferUnlockBaseAddress(buffer, []) }
    guard let baseAddress = CVPixelBufferGetBaseAddress(buffer) else {
        fail("Pixel buffer has no base address")
    }
    let bytesPerRow = CVPixelBufferGetBytesPerRow(buffer)
    let colorSpace = mode == .mono8
        ? CGColorSpaceCreateDeviceGray()
        : CGColorSpaceCreateDeviceRGB()
    let bitmapInfo = mode == .mono8
        ? CGBitmapInfo(rawValue: 0)
        : CGBitmapInfo(
            rawValue: CGImageAlphaInfo.premultipliedFirst.rawValue
                | CGBitmapInfo.byteOrder32Little.rawValue
        )
    guard let context = CGContext(
        data: baseAddress,
        width: width,
        height: height,
        bitsPerComponent: 8,
        bytesPerRow: bytesPerRow,
        space: colorSpace,
        bitmapInfo: bitmapInfo.rawValue
    ) else {
        fail("Could not create CGContext for pixel buffer")
    }
    context.clear(CGRect(x: 0, y: 0, width: width, height: height))
    context.draw(image, in: CGRect(x: 0, y: 0, width: width, height: height))
    return buffer
}

func appendStartCode(_ data: inout Data) {
    data.append(contentsOf: [0x00, 0x00, 0x00, 0x01])
}

func appendParameterSets(from formatDescription: CMFormatDescription, to data: inout Data) throws -> Int32 {
    var parameterSetCount = 0
    var nalUnitHeaderLength: Int32 = 0
    var firstPointer: UnsafePointer<UInt8>?
    var firstSize = 0
    let firstStatus = CMVideoFormatDescriptionGetHEVCParameterSetAtIndex(
        formatDescription,
        parameterSetIndex: 0,
        parameterSetPointerOut: &firstPointer,
        parameterSetSizeOut: &firstSize,
        parameterSetCountOut: &parameterSetCount,
        nalUnitHeaderLengthOut: &nalUnitHeaderLength
    )
    if firstStatus != noErr {
        throw NSError(domain: "AppleVTEncoder", code: Int(firstStatus), userInfo: [NSLocalizedDescriptionKey: "Could not read HEVC parameter sets"])
    }
    if let ptr = firstPointer, firstSize > 0 {
        appendStartCode(&data)
        data.append(ptr, count: firstSize)
    }
    if parameterSetCount > 1 {
        for index in 1..<parameterSetCount {
            var pointer: UnsafePointer<UInt8>?
            var size = 0
            var count = 0
            var headerLength: Int32 = 0
            let status = CMVideoFormatDescriptionGetHEVCParameterSetAtIndex(
                formatDescription,
                parameterSetIndex: index,
                parameterSetPointerOut: &pointer,
                parameterSetSizeOut: &size,
                parameterSetCountOut: &count,
                nalUnitHeaderLengthOut: &headerLength
            )
            if status != noErr {
                throw NSError(domain: "AppleVTEncoder", code: Int(status), userInfo: [NSLocalizedDescriptionKey: "Could not read HEVC parameter set \(index)"])
            }
            if let ptr = pointer, size > 0 {
                appendStartCode(&data)
                data.append(ptr, count: size)
            }
        }
    }
    return nalUnitHeaderLength
}

func appendSampleData(_ sampleBuffer: CMSampleBuffer, nalUnitHeaderLength: Int32, to data: inout Data) throws {
    guard let blockBuffer = CMSampleBufferGetDataBuffer(sampleBuffer) else {
        throw NSError(domain: "AppleVTEncoder", code: -1, userInfo: [NSLocalizedDescriptionKey: "Sample has no block buffer"])
    }
    var totalLength = 0
    var dataPointer: UnsafeMutablePointer<Int8>?
    let status = CMBlockBufferGetDataPointer(
        blockBuffer,
        atOffset: 0,
        lengthAtOffsetOut: nil,
        totalLengthOut: &totalLength,
        dataPointerOut: &dataPointer
    )
    if status != noErr {
        throw NSError(domain: "AppleVTEncoder", code: Int(status), userInfo: [NSLocalizedDescriptionKey: "Could not access compressed sample data"])
    }
    guard let rawPointer = dataPointer else {
        throw NSError(domain: "AppleVTEncoder", code: -2, userInfo: [NSLocalizedDescriptionKey: "Compressed sample data pointer is nil"])
    }
    let bytes = UnsafeRawPointer(rawPointer).assumingMemoryBound(to: UInt8.self)
    let headerLength = Int(nalUnitHeaderLength)
    var offset = 0
    while offset + headerLength <= totalLength {
        var nalLength = 0
        for idx in 0..<headerLength {
            nalLength = (nalLength << 8) | Int(bytes[offset + idx])
        }
        offset += headerLength
        if nalLength <= 0 || offset + nalLength > totalLength {
            throw NSError(domain: "AppleVTEncoder", code: -3, userInfo: [NSLocalizedDescriptionKey: "Invalid NAL length in compressed sample"])
        }
        appendStartCode(&data)
        data.append(bytes + offset, count: nalLength)
        offset += nalLength
    }
}

let callback: VTCompressionOutputCallback = { refcon, _, status, _, sampleBuffer in
    guard let refcon else { return }
    let state = Unmanaged<EncoderState>.fromOpaque(refcon).takeUnretainedValue()
    if status != noErr {
        state.error = "Compression callback status \(status)"
        return
    }
    guard let sampleBuffer else {
        state.error = "Compression callback did not provide a sample buffer"
        return
    }
    guard CMSampleBufferDataIsReady(sampleBuffer) else {
        state.error = "Compressed sample buffer is not ready"
        return
    }
    guard let formatDescription = CMSampleBufferGetFormatDescription(sampleBuffer) else {
        state.error = "Compressed sample has no format description"
        return
    }
    if state.hvcc.isEmpty,
       let extensions = CMFormatDescriptionGetExtensions(formatDescription) as? [String: Any],
       let atoms = extensions[kCMFormatDescriptionExtension_SampleDescriptionExtensionAtoms as String]
            as? [String: Any],
       let hvcc = atoms["hvcC"] as? Data {
        state.hvcc = hvcc
    }
    do {
        let nalUnitHeaderLength: Int32
        if !state.wroteParameterSets {
            nalUnitHeaderLength = try appendParameterSets(from: formatDescription, to: &state.annexB)
            state.wroteParameterSets = true
        } else {
            var pointer: UnsafePointer<UInt8>?
            var size = 0
            var count = 0
            var headerLength: Int32 = 0
            let psStatus = CMVideoFormatDescriptionGetHEVCParameterSetAtIndex(
                formatDescription,
                parameterSetIndex: 0,
                parameterSetPointerOut: &pointer,
                parameterSetSizeOut: &size,
                parameterSetCountOut: &count,
                nalUnitHeaderLengthOut: &headerLength
            )
            if psStatus != noErr {
                throw NSError(domain: "AppleVTEncoder", code: Int(psStatus), userInfo: [NSLocalizedDescriptionKey: "Could not read HEVC NAL header length"])
            }
            nalUnitHeaderLength = headerLength
        }
        try appendSampleData(sampleBuffer, nalUnitHeaderLength: nalUnitHeaderLength, to: &state.annexB)
    } catch {
        state.error = error.localizedDescription
    }
}

let args = CommandLine.arguments
if args.count < 3 || args.count > 6 {
    fail(
        "usage: apple_vt_hevc_encoder.swift input.png output.hevc "
            + "[quality] [rgb10|mono8] [output.hvcc]"
    )
}

let quality: Double
if args.count == 4 {
    guard let parsedQuality = Double(args[3]), parsedQuality > 0.0, parsedQuality <= 1.0 else {
        fail("quality must be in the range (0.0, 1.0]")
    }
    quality = parsedQuality
} else {
    quality = 0.45
}

let mode: PixelMode
if args.count >= 5 {
    guard let parsedMode = PixelMode(rawValue: args[4]) else {
        fail("pixel mode must be rgb10 or mono8")
    }
    mode = parsedMode
} else {
    mode = .rgb10
}
let hvccOutputPath = args.count >= 6 ? args[5] : nil

let image = loadImage(args[1])
let pixelBuffer = makePixelBuffer(from: image, mode: mode)
let state = EncoderState()

let imageBufferAttributes: CFDictionary = [
    kCVPixelBufferPixelFormatTypeKey: CVPixelBufferGetPixelFormatType(pixelBuffer),
    kCVPixelBufferWidthKey: image.width,
    kCVPixelBufferHeightKey: image.height,
    kCVPixelBufferIOSurfacePropertiesKey: [:],
] as CFDictionary

var session: VTCompressionSession?
check(
    VTCompressionSessionCreate(
        allocator: kCFAllocatorDefault,
        width: Int32(image.width),
        height: Int32(image.height),
        codecType: kCMVideoCodecType_HEVC,
        encoderSpecification: nil,
        imageBufferAttributes: imageBufferAttributes,
        compressedDataAllocator: nil,
        outputCallback: callback,
        refcon: UnsafeMutableRawPointer(Unmanaged.passUnretained(state).toOpaque()),
        compressionSessionOut: &session
    ),
    "VTCompressionSessionCreate failed"
)
guard let session else {
    fail("VTCompressionSessionCreate returned nil")
}

let profile = mode == .mono8
    ? kVTProfileLevel_HEVC_Monochrome_AutoLevel
    : kVTProfileLevel_HEVC_Main10_AutoLevel
VTSessionSetProperty(session, key: kVTCompressionPropertyKey_ProfileLevel, value: profile)
VTSessionSetProperty(session, key: kVTCompressionPropertyKey_AllowFrameReordering, value: kCFBooleanFalse)
VTSessionSetProperty(session, key: kVTCompressionPropertyKey_MaxKeyFrameInterval, value: 1 as CFTypeRef)
VTSessionSetProperty(session, key: kVTCompressionPropertyKey_MaxKeyFrameIntervalDuration, value: 1 as CFTypeRef)
VTSessionSetProperty(session, key: kVTCompressionPropertyKey_RealTime, value: kCFBooleanFalse)
VTSessionSetProperty(session, key: kVTCompressionPropertyKey_Quality, value: quality as CFTypeRef)
VTSessionSetProperty(session, key: kVTCompressionPropertyKey_ColorPrimaries, value: kCVImageBufferColorPrimaries_ITU_R_709_2)
VTSessionSetProperty(session, key: kVTCompressionPropertyKey_TransferFunction, value: kCVImageBufferTransferFunction_ITU_R_709_2)
VTSessionSetProperty(session, key: kVTCompressionPropertyKey_YCbCrMatrix, value: kCVImageBufferYCbCrMatrix_ITU_R_709_2)

check(VTCompressionSessionPrepareToEncodeFrames(session), "VTCompressionSessionPrepareToEncodeFrames failed")
let frameProperties: CFDictionary = [kVTEncodeFrameOptionKey_ForceKeyFrame: true] as CFDictionary
check(
    VTCompressionSessionEncodeFrame(
        session,
        imageBuffer: pixelBuffer,
        presentationTimeStamp: CMTime(value: 0, timescale: 1),
        duration: CMTime(value: 1, timescale: 1),
        frameProperties: frameProperties,
        sourceFrameRefcon: nil,
        infoFlagsOut: nil
    ),
    "VTCompressionSessionEncodeFrame failed"
)
check(VTCompressionSessionCompleteFrames(session, untilPresentationTimeStamp: .invalid), "VTCompressionSessionCompleteFrames failed")
VTCompressionSessionInvalidate(session)

if let error = state.error {
    fail(error)
}
if state.annexB.isEmpty {
    fail("VideoToolbox produced no HEVC data")
}
try state.annexB.write(to: URL(fileURLWithPath: args[2]))
if let hvccOutputPath {
    if state.hvcc.isEmpty {
        fail("VideoToolbox format description did not expose an hvcC atom")
    }
    try state.hvcc.write(to: URL(fileURLWithPath: hvccOutputPath))
}
