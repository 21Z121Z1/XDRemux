import CoreGraphics
import CoreMedia
import CoreVideo
import Foundation
import VideoToolbox

/// The Rust runtime supplies the raster and owns the Styles resource and
/// publication policy. This request describes only the VideoToolbox primitive
/// needed to produce Apple's Main10 Linear Thumbnail resource.
struct VideoToolboxMain10Request: Decodable {
    let rawWidth: UInt32
    let rawHeight: UInt32
    let rawBytesPerRow: UInt32
    let quality: Double
    let hvccPath: String

    enum CodingKeys: String, CodingKey {
        case rawWidth = "raw_width"
        case rawHeight = "raw_height"
        case rawBytesPerRow = "raw_bytes_per_row"
        case quality
        case hvccPath = "hvcc_path"
    }
}

struct VideoToolboxMain10Facts: Encodable {
    let width: Int
    let height: Int
    let annexBLength: Int
    let hvccLength: Int
    let hardwareAccelerationAllowed: Bool
    let usingHardwareAcceleratedEncoder: Bool?
    let encoderID: String?

    enum CodingKeys: String, CodingKey {
        case width
        case height
        case annexBLength = "annex_b_length"
        case hvccLength = "hvcc_length"
        case hardwareAccelerationAllowed = "hardware_acceleration_allowed"
        case usingHardwareAcceleratedEncoder = "using_hardware_accelerated_encoder"
        case encoderID = "encoder_id"
    }
}

private final class Main10EncoderState {
    var annexB = Data()
    var hvcc = Data()
    var wroteParameterSets = false
    var error: String?
}

private func main10Error(_ message: String, status: OSStatus = -1) -> NSError {
    NSError(
        domain: "XDRemuxAppleAdapter.VideoToolbox",
        code: Int(status),
        userInfo: [NSLocalizedDescriptionKey: message]
    )
}

private func checkMain10Status(_ status: OSStatus, _ message: String) throws {
    guard status == noErr else {
        throw main10Error("\(message): OSStatus \(status)", status: status)
    }
}

private func copyMain10SessionProperty(
    _ session: VTCompressionSession,
    key: CFString
) -> CFTypeRef? {
    var value: CFTypeRef?
    let status = VTSessionCopyProperty(
        session,
        key: key,
        allocator: kCFAllocatorDefault,
        valueOut: &value
    )
    return status == noErr ? value : nil
}

private func main10HardwareEncoderUsed(_ session: VTCompressionSession) -> Bool? {
    guard let value = copyMain10SessionProperty(
        session,
        key: kVTCompressionPropertyKey_UsingHardwareAcceleratedVideoEncoder
    ) else {
        return nil
    }
    return (value as? NSNumber)?.boolValue
}

private func main10EncoderID(_ session: VTCompressionSession) -> String? {
    copyMain10SessionProperty(
        session,
        key: kVTCompressionPropertyKey_EncoderID
    ) as? String
}

private func appendMain10StartCode(_ data: inout Data) {
    data.append(contentsOf: [0x00, 0x00, 0x00, 0x01])
}

private func appendMain10ParameterSets(
    from formatDescription: CMFormatDescription,
    to data: inout Data
) throws -> Int32 {
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
    try checkMain10Status(firstStatus, "could not read HEVC parameter sets")
    if let pointer = firstPointer, firstSize > 0 {
        appendMain10StartCode(&data)
        data.append(pointer, count: firstSize)
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
            try checkMain10Status(status, "could not read HEVC parameter set \(index)")
            if let pointer, size > 0 {
                appendMain10StartCode(&data)
                data.append(pointer, count: size)
            }
        }
    }
    return nalUnitHeaderLength
}

private func appendMain10SampleData(
    _ sampleBuffer: CMSampleBuffer,
    nalUnitHeaderLength: Int32,
    to data: inout Data
) throws {
    guard let blockBuffer = CMSampleBufferGetDataBuffer(sampleBuffer) else {
        throw main10Error("compressed sample has no block buffer")
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
    try checkMain10Status(status, "could not access compressed sample data")
    guard let dataPointer else {
        throw main10Error("compressed sample data pointer is nil")
    }
    let bytes = UnsafeRawPointer(dataPointer).assumingMemoryBound(to: UInt8.self)
    let headerLength = Int(nalUnitHeaderLength)
    guard (1...4).contains(headerLength) else {
        throw main10Error("VideoToolbox returned an invalid NAL header length")
    }
    var offset = 0
    while offset + headerLength <= totalLength {
        var nalLength = 0
        for index in 0..<headerLength {
            nalLength = (nalLength << 8) | Int(bytes[offset + index])
        }
        offset += headerLength
        guard nalLength > 0, offset + nalLength <= totalLength else {
            throw main10Error("compressed sample contains an invalid NAL length")
        }
        appendMain10StartCode(&data)
        data.append(bytes + offset, count: nalLength)
        offset += nalLength
    }
    guard offset == totalLength else {
        throw main10Error("compressed sample contains a truncated NAL header")
    }
}

private let main10CompressionCallback: VTCompressionOutputCallback = {
    refcon, _, status, _, sampleBuffer in
    guard let refcon else { return }
    let state = Unmanaged<Main10EncoderState>
        .fromOpaque(refcon)
        .takeUnretainedValue()
    guard status == noErr else {
        state.error = "VideoToolbox callback status \(status)"
        return
    }
    guard let sampleBuffer,
          CMSampleBufferDataIsReady(sampleBuffer),
          let formatDescription = CMSampleBufferGetFormatDescription(sampleBuffer) else {
        state.error = "VideoToolbox callback returned an incomplete sample"
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
            nalUnitHeaderLength = try appendMain10ParameterSets(
                from: formatDescription,
                to: &state.annexB
            )
            state.wroteParameterSets = true
        } else {
            var pointer: UnsafePointer<UInt8>?
            var size = 0
            var count = 0
            var headerLength: Int32 = 0
            let parameterStatus = CMVideoFormatDescriptionGetHEVCParameterSetAtIndex(
                formatDescription,
                parameterSetIndex: 0,
                parameterSetPointerOut: &pointer,
                parameterSetSizeOut: &size,
                parameterSetCountOut: &count,
                nalUnitHeaderLengthOut: &headerLength
            )
            try checkMain10Status(
                parameterStatus,
                "could not read VideoToolbox NAL header length"
            )
            nalUnitHeaderLength = headerLength
        }
        try appendMain10SampleData(
            sampleBuffer,
            nalUnitHeaderLength: nalUnitHeaderLength,
            to: &state.annexB
        )
    } catch {
        state.error = error.localizedDescription
    }
}

private func makeMain10PixelBuffer(
    data: Data,
    width: Int,
    height: Int,
    bytesPerRow: Int
) throws -> CVPixelBuffer {
    let expectedBytes = bytesPerRow.multipliedReportingOverflow(by: height)
    guard !expectedBytes.overflow, data.count == expectedBytes.partialValue else {
        throw main10Error("raw RGB8 raster length does not match its geometry")
    }
    guard bytesPerRow >= width * 3 else {
        throw main10Error("raw RGB8 raster row is shorter than width * 3")
    }

    let attributes: CFDictionary = [
        kCVPixelBufferPixelFormatTypeKey: kCVPixelFormatType_32BGRA,
        kCVPixelBufferWidthKey: width,
        kCVPixelBufferHeightKey: height,
        kCVPixelBufferIOSurfacePropertiesKey: [:],
    ] as CFDictionary
    var pixelBuffer: CVPixelBuffer?
    try checkMain10Status(
        CVPixelBufferCreate(
            kCFAllocatorDefault,
            width,
            height,
            kCVPixelFormatType_32BGRA,
            attributes,
            &pixelBuffer
        ),
        "CVPixelBufferCreate failed"
    )
    guard let pixelBuffer else {
        throw main10Error("CVPixelBufferCreate returned nil")
    }

    let lockStatus = CVPixelBufferLockBaseAddress(pixelBuffer, [])
    try checkMain10Status(lockStatus, "CVPixelBufferLockBaseAddress failed")
    defer { CVPixelBufferUnlockBaseAddress(pixelBuffer, []) }
    guard let baseAddress = CVPixelBufferGetBaseAddress(pixelBuffer) else {
        throw main10Error("VideoToolbox pixel buffer has no base address")
    }
    let destination = baseAddress.assumingMemoryBound(to: UInt8.self)
    let destinationBytesPerRow = CVPixelBufferGetBytesPerRow(pixelBuffer)
    data.withUnsafeBytes { rawBuffer in
        let source = rawBuffer.baseAddress!.assumingMemoryBound(to: UInt8.self)
        for y in 0..<height {
            let sourceRow = source.advanced(by: y * bytesPerRow)
            let destinationRow = destination.advanced(by: y * destinationBytesPerRow)
            for x in 0..<width {
                let sourcePixel = sourceRow.advanced(by: x * 3)
                let destinationPixel = destinationRow.advanced(by: x * 4)
                destinationPixel[0] = sourcePixel[2]
                destinationPixel[1] = sourcePixel[1]
                destinationPixel[2] = sourcePixel[0]
                destinationPixel[3] = 0xff
            }
        }
    }
    return pixelBuffer
}

/// Run exactly the non-tiled VideoToolbox Main10 primitive used by the
/// historical Styles path. No Styles policy or container assembly lives here.
func encodeVideoToolboxMain10(
    inputPath: String,
    outputPath: String,
    configuration: VideoToolboxMain10Request
) throws -> VideoToolboxMain10Facts {
    guard configuration.rawWidth > 0, configuration.rawHeight > 0 else {
        throw main10Error("raw RGB8 raster geometry must be non-zero")
    }
    guard configuration.quality.isFinite, (0.0...1.0).contains(configuration.quality) else {
        throw main10Error("VideoToolbox quality must be finite and within 0 through 1")
    }
    guard !configuration.hvccPath.isEmpty else {
        throw main10Error("VideoToolbox hvcC output path is empty")
    }
    let width = Int(configuration.rawWidth)
    let height = Int(configuration.rawHeight)
    let bytesPerRow = Int(configuration.rawBytesPerRow)
    guard width <= Int.max / 4, height <= Int.max / max(bytesPerRow, 1) else {
        throw main10Error("raw RGB8 raster geometry overflows")
    }
    let data = try Data(contentsOf: URL(fileURLWithPath: inputPath), options: [.mappedIfSafe])
    let pixelBuffer = try makeMain10PixelBuffer(
        data: data,
        width: width,
        height: height,
        bytesPerRow: bytesPerRow
    )
    let state = Main10EncoderState()
    let encoderSpecification: CFDictionary = [
        kVTVideoEncoderSpecification_EnableHardwareAcceleratedVideoEncoder: true,
    ] as CFDictionary
    let imageBufferAttributes: CFDictionary = [
        kCVPixelBufferPixelFormatTypeKey: CVPixelBufferGetPixelFormatType(pixelBuffer),
        kCVPixelBufferWidthKey: width,
        kCVPixelBufferHeightKey: height,
        kCVPixelBufferIOSurfacePropertiesKey: [:],
    ] as CFDictionary

    var session: VTCompressionSession?
    try checkMain10Status(
        VTCompressionSessionCreate(
            allocator: kCFAllocatorDefault,
            width: Int32(width),
            height: Int32(height),
            codecType: kCMVideoCodecType_HEVC,
            encoderSpecification: encoderSpecification,
            imageBufferAttributes: imageBufferAttributes,
            compressedDataAllocator: nil,
            outputCallback: main10CompressionCallback,
            refcon: Unmanaged.passUnretained(state).toOpaque(),
            compressionSessionOut: &session
        ),
        "VTCompressionSessionCreate failed"
    )
    guard let session else {
        throw main10Error("VTCompressionSessionCreate returned nil")
    }
    defer { VTCompressionSessionInvalidate(session) }

    try checkMain10Status(
        VTSessionSetProperty(
            session,
            key: kVTCompressionPropertyKey_ProfileLevel,
            value: kVTProfileLevel_HEVC_Main10_AutoLevel
        ),
        "setting HEVC Main10 profile failed"
    )
    _ = VTSessionSetProperty(
        session,
        key: kVTCompressionPropertyKey_AllowFrameReordering,
        value: kCFBooleanFalse
    )
    _ = VTSessionSetProperty(
        session,
        key: kVTCompressionPropertyKey_MaxKeyFrameInterval,
        value: 1 as CFTypeRef
    )
    _ = VTSessionSetProperty(
        session,
        key: kVTCompressionPropertyKey_MaxKeyFrameIntervalDuration,
        value: 1 as CFTypeRef
    )
    _ = VTSessionSetProperty(
        session,
        key: kVTCompressionPropertyKey_RealTime,
        value: kCFBooleanFalse
    )
    _ = VTSessionSetProperty(
        session,
        key: kVTCompressionPropertyKey_Quality,
        value: configuration.quality as CFTypeRef
    )
    _ = VTSessionSetProperty(
        session,
        key: kVTCompressionPropertyKey_ColorPrimaries,
        value: kCVImageBufferColorPrimaries_ITU_R_709_2
    )
    _ = VTSessionSetProperty(
        session,
        key: kVTCompressionPropertyKey_TransferFunction,
        value: kCVImageBufferTransferFunction_ITU_R_709_2
    )
    _ = VTSessionSetProperty(
        session,
        key: kVTCompressionPropertyKey_YCbCrMatrix,
        value: kCVImageBufferYCbCrMatrix_ITU_R_601_4
    )

    try checkMain10Status(
        VTCompressionSessionPrepareToEncodeFrames(session),
        "VTCompressionSessionPrepareToEncodeFrames failed"
    )
    let frameProperties: CFDictionary = [
        kVTEncodeFrameOptionKey_ForceKeyFrame: true,
    ] as CFDictionary
    try checkMain10Status(
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
    try checkMain10Status(
        VTCompressionSessionCompleteFrames(session, untilPresentationTimeStamp: .invalid),
        "VTCompressionSessionCompleteFrames failed"
    )
    if let error = state.error {
        throw main10Error(error)
    }
    guard !state.annexB.isEmpty else {
        throw main10Error("VideoToolbox produced no HEVC data")
    }
    guard !state.hvcc.isEmpty else {
        throw main10Error("VideoToolbox format description did not expose an hvcC atom")
    }
    let usingHardwareAcceleratedEncoder = main10HardwareEncoderUsed(session)
    let encoderID = main10EncoderID(session)
    try state.annexB.write(to: URL(fileURLWithPath: outputPath), options: [.atomic])
    try state.hvcc.write(to: URL(fileURLWithPath: configuration.hvccPath), options: [.atomic])
    return VideoToolboxMain10Facts(
        width: width,
        height: height,
        annexBLength: state.annexB.count,
        hvccLength: state.hvcc.count,
        hardwareAccelerationAllowed: true,
        usingHardwareAcceleratedEncoder: usingHardwareAcceleratedEncoder,
        encoderID: encoderID
    )
}
