#import "xdremux_cinematic.h"

#import <AVFoundation/AVFoundation.h>
#import <Cinematic/Cinematic.h>
#import <CoreMedia/CoreMedia.h>
#import <CoreVideo/CoreVideo.h>
#import <simd/simd.h>
#import <Vision/Vision.h>
#import <ImageIO/ImageIO.h>
#import <objc/message.h>
#import <objc/runtime.h>
#import <math.h>
#import <stdint.h>
#import <stdio.h>
#import <string.h>
#import <unistd.h>

typedef struct { int32_t width; int32_t height; } PTDisparitySize32;
typedef id (*PTProviderInit)(id, SEL, int, CGSize);
typedef BOOL (*PTProviderSupported)(id, SEL);
typedef PTDisparitySize32 (*PTDisparitySize)(id, SEL);
typedef OSType (*PTDisparityFormat)(id, SEL);
typedef CVPixelBufferRef (*PTInferDisparity)(id, SEL, CVPixelBufferRef, float);
typedef void (*PTSetUInt32)(id, SEL, uint32_t);
typedef id (*PTInitDetection)(id, SEL, CMTime, CGRect, float);
typedef id (*PTInitDecision)(id, SEL, CMTime, long long, long long, unsigned long long);
typedef id (*PTInitRefinement)(id, SEL, id, BOOL);
typedef CMTime (*PTProcessDisparity)(id, SEL, CVPixelBufferRef);
typedef long long (*PTNextTrackIdentifier)(id, SEL);
typedef id (*PTInternalizeFocusTrack)(id, SEL, id);
typedef void (*PTInternalizeFrames)(id, SEL, id);
typedef BOOL (*PTBoolGetter)(id, SEL);
typedef id (*PTObjectGetter)(id, SEL);
typedef CMTime (*PTTimeGetter)(id, SEL);
typedef float (*PTFloatGetter)(id, SEL);
typedef void (*PTSetFloat)(id, SEL, float);
typedef void (*PTSetFloat2)(id, SEL, vector_float2);
typedef uint32_t (*PTSerializedSize)(id, SEL, id);
typedef BOOL (*PTWriteData)(id, SEL, id, id);

static int XDFail(char *buffer, size_t capacity, int code, NSString *message) {
    if (buffer && capacity) snprintf(buffer, capacity, "%s", message.UTF8String ?: "Cinematic envelope failed");
    return code;
}

// The public consumer decides whether a generated envelope is accepted. Do
// not publish a MOV merely because the private serializers returned bytes.
static BOOL XDValidatePendingAsset(NSString *path, char *errorBuffer, size_t errorCapacity) {
    if (@available(macOS 27.0, *)) {
        AVURLAsset *asset = [AVURLAsset URLAssetWithURL:[NSURL fileURLWithPath:path] options:nil];
        dispatch_semaphore_t finished = dispatch_semaphore_create(0);
        __block CNAssetInfo *info = nil;
        __block NSError *loadError = nil;
        [CNAssetInfo loadFromAsset:asset completionHandler:^(CNAssetInfo *value, NSError *error) {
            info = value;
            loadError = error;
            dispatch_semaphore_signal(finished);
        }];
        if (dispatch_semaphore_wait(finished, dispatch_time(DISPATCH_TIME_NOW, 60 * NSEC_PER_SEC)) != 0) {
            XDFail(errorBuffer, errorCapacity, 5, @"public Cinematic asset validation timed out");
            return NO;
        }
        BOOL valid = info && info.cinematicCapability == CNCinematicCapabilityNeedsPreprocessing &&
                     info.resourceStatus == CNResourceStatusReady && !info.isPreprocessed;
        if (!valid) {
            NSString *detail = info
                ? [NSString stringWithFormat:@"public Cinematic validation rejected the envelope: capability=%ld, resourceStatus=%ld, preprocessed=%d",
                   (long)info.cinematicCapability, (long)info.resourceStatus, info.isPreprocessed]
                : [NSString stringWithFormat:@"cannot load generated Cinematic asset: %@", loadError.localizedDescription];
            XDFail(errorBuffer, errorCapacity, 5, detail);
        }
        return valid;
    }
    XDFail(errorBuffer, errorCapacity, 5, @"Cinematic envelope validation requires macOS 27 or later");
    return NO;
}

static Method XDRequireMethod(Class cls, const char *name, BOOL classMethod, const char *encoding,
                              char *errorBuffer, size_t errorCapacity) {
    SEL selector = sel_registerName(name);
    Method method = classMethod ? class_getClassMethod(cls, selector) : class_getInstanceMethod(cls, selector);
    const char *actual = method ? method_getTypeEncoding(method) : NULL;
    if (!actual || strcmp(actual, encoding) != 0) {
        NSString *detail = [NSString stringWithFormat:@"runtime ABI mismatch %s.%s: got %s, expected %s",
                            cls ? class_getName(cls) : "<missing class>", name, actual ?: "<missing>", encoding];
        if (errorBuffer && errorCapacity) snprintf(errorBuffer, errorCapacity, "%s", detail.UTF8String);
        return NULL;
    }
    return method;
}

static NSData *XDSerialize(id object, Class cls, BOOL acceptFalseWriteResult,
                           char *errorBuffer, size_t errorCapacity) {
    Method sizeMethod = XDRequireMethod(cls, "sizeOfSerializedObjectWithOptions:", NO, "I24@0:8@16", errorBuffer, errorCapacity);
    Method writeMethod = XDRequireMethod(cls, "writeToData:withOptions:", NO, "B32@0:8@16@24", errorBuffer, errorCapacity);
    if (!object || !sizeMethod || !writeMethod) return nil;
    uint32_t size = ((PTSerializedSize)method_getImplementation(sizeMethod))(object,
                            sel_registerName("sizeOfSerializedObjectWithOptions:"), nil);
    if (size < 8 || size > 4096) {
        XDFail(errorBuffer, errorCapacity, 2, [NSString stringWithFormat:@"invalid Portrait serialization size %u", size]);
        return nil;
    }
    NSMutableData *data = [NSMutableData dataWithLength:size];
    BOOL wrote = ((PTWriteData)method_getImplementation(writeMethod))(object,
                    sel_registerName("writeToData:withOptions:"), data, nil);
    if (!wrote && !acceptFalseWriteResult) {
        XDFail(errorBuffer, errorCapacity, 2, @"Portrait serializer rejected generated metadata");
        return nil;
    }
    return data;
}

static void XDAppendBE32(NSMutableData *data, uint32_t value) {
    uint8_t bytes[4] = {(uint8_t)(value >> 24), (uint8_t)(value >> 16), (uint8_t)(value >> 8), (uint8_t)value};
    [data appendBytes:bytes length:sizeof(bytes)];
}

static NSData *XDAtom(const char type[4], NSData *payload) {
    if (payload.length > UINT32_MAX - 8) return nil;
    NSMutableData *data = [NSMutableData dataWithCapacity:payload.length + 8];
    XDAppendBE32(data, (uint32_t)payload.length + 8);
    [data appendBytes:type length:4];
    [data appendData:payload];
    return data;
}

static NSData *XDGlobalCinematicVideoData(char *errorBuffer, size_t errorCapacity) {
    NSBundle *portrait = [NSBundle bundleWithPath:@"/System/Library/PrivateFrameworks/Portrait.framework"];
    NSError *loadError = nil;
    if (![portrait loadAndReturnError:&loadError]) {
        XDFail(errorBuffer, errorCapacity, 3, [NSString stringWithFormat:@"cannot load Portrait.framework: %@", loadError.localizedDescription]);
        return nil;
    }
    Class providerClass = NSClassFromString(@"PTMonocularDisparityProvider");
    Class globalCinematographyClass = NSClassFromString(@"PTGlobalCinematographyMetadataVersion1");
    if (!providerClass || !globalCinematographyClass) {
        XDFail(errorBuffer, errorCapacity, 3, @"required Portrait global metadata classes are unavailable");
        return nil;
    }
    Method defaultMethod = XDRequireMethod(providerClass, "defaultRenderingMetadataWithRenderVersion:", YES,
                                          "@24@0:8Q16", errorBuffer, errorCapacity);
    Method supportedMethod = XDRequireMethod(providerClass, "isSupported", YES,
                                             "B16@0:8", errorBuffer, errorCapacity);
    if (!defaultMethod || !supportedMethod ||
        !((PTProviderSupported)method_getImplementation(supportedMethod))(providerClass, sel_registerName("isSupported"))) {
        XDFail(errorBuffer, errorCapacity, 3, @"PTMonocularDisparityProvider is unavailable on this system");
        return nil;
    }
    id rendering = ((id (*)(id, SEL, uint64_t))method_getImplementation(defaultMethod))(
        providerClass, sel_registerName("defaultRenderingMetadataWithRenderVersion:"), 2ULL);
    if (!rendering) {
        XDFail(errorBuffer, errorCapacity, 3, @"Portrait returned no default global rendering metadata");
        return nil;
    }
    Class renderingClass = object_getClass(rendering);
    Method setRenderingVersion = XDRequireMethod(renderingClass, "setRenderingVersion:", NO, "v20@0:8I16", errorBuffer, errorCapacity);
    Method setHardware = XDRequireMethod(renderingClass, "setHwModelID:", NO, "v20@0:8I16", errorBuffer, errorCapacity);
    Method setSensor = XDRequireMethod(renderingClass, "setSensorID:", NO, "v20@0:8I16", errorBuffer, errorCapacity);
    Method setDepth = XDRequireMethod(renderingClass, "setSourceColorBitDepth:", NO, "v20@0:8I16", errorBuffer, errorCapacity);
    if (!setRenderingVersion || !setHardware || !setSensor || !setDepth) return nil;
    // Native iPhone 18 Pro captures on iOS 27 use rendering version 5.
    ((PTSetUInt32)method_getImplementation(setRenderingVersion))(rendering, sel_registerName("setRenderingVersion:"), 5);
    ((PTSetUInt32)method_getImplementation(setHardware))(rendering, sel_registerName("setHwModelID:"), 32);
    ((PTSetUInt32)method_getImplementation(setSensor))(rendering, sel_registerName("setSensorID:"), 2309);
    ((PTSetUInt32)method_getImplementation(setDepth))(rendering, sel_registerName("setSourceColorBitDepth:"), 10);
    NSData *renderingData = XDSerialize(rendering, renderingClass, YES, errorBuffer, errorCapacity);
    if (!renderingData || renderingData.length != 56) {
        XDFail(errorBuffer, errorCapacity, 3, @"global rendering metadata did not serialize to the supported V2 size");
        return nil;
    }

    Method initCinematography = XDRequireMethod(globalCinematographyClass, "initWithMinorVersion:", NO,
                                                "@20@0:8I16", errorBuffer, errorCapacity);
    if (!initCinematography) return nil;
    id cinematography = ((id (*)(id, SEL, uint32_t))method_getImplementation(initCinematography))(
        [globalCinematographyClass alloc], sel_registerName("initWithMinorVersion:"), 1);
    NSData *cinematographyData = XDSerialize(cinematography, globalCinematographyClass, NO, errorBuffer, errorCapacity);
    if (!cinematographyData || cinematographyData.length != 64) {
        XDFail(errorBuffer, errorCapacity, 3, @"global cinematography metadata did not serialize to V1 size");
        return nil;
    }

    NSMutableData *headerPayload = [NSMutableData data];
    XDAppendBE32(headerPayload, 1);
    XDAppendBE32(headerPayload, 1);
    NSData *header = XDAtom("vhdr", headerPayload);
    NSData *wrappedHeader = header ? XDAtom("vhdr", header) : nil;
    NSData *wrappedRendering = XDAtom("rend", renderingData);
    NSData *wrappedCinematography = XDAtom("cntg", cinematographyData);
    if (!wrappedHeader || !wrappedRendering || !wrappedCinematography) {
        XDFail(errorBuffer, errorCapacity, 3, @"could not build global Cinematic atom tree");
        return nil;
    }
    NSMutableData *payload = [NSMutableData data];
    [payload appendData:wrappedHeader];
    [payload appendData:wrappedRendering];
    [payload appendData:wrappedCinematography];
    NSData *cvid = XDAtom("cvid", payload);
    if (cvid.length != 168) {
        XDFail(errorBuffer, errorCapacity, 3, @"generated cvid atom does not match the supported 168-byte envelope");
        return nil;
    }
    return cvid;
}

static NSData *XDSerializeCinematographyFrame(id frame, char *errorBuffer, size_t errorCapacity) {
    Class byteWriterClass = NSClassFromString(@"PTDataByteWriter");
    Class atomWriterClass = NSClassFromString(@"PTAtomWriter");
    Class frameClass = object_getClass(frame);
    Method byteInit = byteWriterClass ? XDRequireMethod(byteWriterClass, "initWithMutableData:", NO,
                                                       "@24@0:8@16", errorBuffer, errorCapacity) : NULL;
    Method atomInit = atomWriterClass ? XDRequireMethod(atomWriterClass, "initWithByteWriter:", NO,
                                                       "@24@0:8@16", errorBuffer, errorCapacity) : NULL;
    Method writeFrame = frameClass ? XDRequireMethod(frameClass, "writeToAtomWriter:options:", NO,
                                                     "B32@0:8@16@24", errorBuffer, errorCapacity) : NULL;
    if (!byteWriterClass || !atomWriterClass || !byteInit || !atomInit || !writeFrame) return nil;
    NSMutableData *data = [NSMutableData data];
    id byteWriter = ((id (*)(id, SEL, id))method_getImplementation(byteInit))(
        [byteWriterClass alloc], sel_registerName("initWithMutableData:"), data);
    id atomWriter = ((id (*)(id, SEL, id))method_getImplementation(atomInit))(
        [atomWriterClass alloc], sel_registerName("initWithByteWriter:"), byteWriter);
    BOOL wrote = ((BOOL (*)(id, SEL, id, id))method_getImplementation(writeFrame))(
        frame, sel_registerName("writeToAtomWriter:options:"), atomWriter, nil);
    Method dataMethod = XDRequireMethod(object_getClass(byteWriter), "data", NO, "@16@0:8", errorBuffer, errorCapacity);
    NSData *serialized = dataMethod ? ((PTObjectGetter)method_getImplementation(dataMethod))(byteWriter, sel_registerName("data")) : nil;
    if (!wrote || serialized.length < 32 || memcmp((const uint8_t *)serialized.bytes + 4, "frme", 4) != 0) {
        XDFail(errorBuffer, errorCapacity, 3, @"Apple failed to serialize a refined Cinematography frame");
        return nil;
    }
    return serialized;
}

static uint32_t XDReadBE32(const uint8_t *bytes, size_t offset) {
    return ((uint32_t)bytes[offset] << 24) | ((uint32_t)bytes[offset + 1] << 16) |
           ((uint32_t)bytes[offset + 2] << 8) | (uint32_t)bytes[offset + 3];
}

static void XDWriteBE32(uint8_t *bytes, size_t offset, uint32_t value) {
    bytes[offset] = (uint8_t)(value >> 24);
    bytes[offset + 1] = (uint8_t)(value >> 16);
    bytes[offset + 2] = (uint8_t)(value >> 8);
    bytes[offset + 3] = (uint8_t)value;
}

static NSData *XDResetPreprocessingDetectionFocusDistances(NSData *serialized,
                                                            char *errorBuffer,
                                                            size_t errorCapacity) {
    if (serialized.length < 20 ||
        XDReadBE32(serialized.bytes, 0) != serialized.length ||
        memcmp((const uint8_t *)serialized.bytes + 4, "frme", 4) != 0) {
        XDFail(errorBuffer, errorCapacity, 3, @"invalid serialized Cinematography frame before detection focus reset");
        return nil;
    }
    NSMutableData *result = [serialized mutableCopy];
    uint8_t *bytes = result.mutableBytes;
    size_t cursor = 12;
    size_t detectionCount = 0;
    while (cursor + 8 <= result.length) {
        uint32_t atomSize = XDReadBE32(bytes, cursor);
        if (atomSize < 8 || atomSize > result.length - cursor) {
            XDFail(errorBuffer, errorCapacity, 3, @"invalid child atom in serialized Cinematography frame");
            return nil;
        }
        if (memcmp(bytes + cursor + 4, "dtcs", 4) == 0) {
            if (atomSize < 12) {
                XDFail(errorBuffer, errorCapacity, 3, @"invalid detection container in serialized Cinematography frame");
                return nil;
            }
            size_t child = cursor + 12;
            size_t end = cursor + atomSize;
            while (child + 8 <= end) {
                uint32_t childSize = XDReadBE32(bytes, child);
                if (childSize < 8 || childSize > end - child) {
                    XDFail(errorBuffer, errorCapacity, 3, @"invalid detection atom in serialized Cinematography frame");
                    return nil;
                }
                if (memcmp(bytes + child + 4, "dtct", 4) == 0) {
                    if (childSize != 52) {
                        XDFail(errorBuffer, errorCapacity, 3, @"unexpected serialized Cinematography detection size");
                        return nil;
                    }
                    // Genuine capture-time detections carry unknown focus distance. Keep
                    // the SPI refinement in timed rendering, but let Photos infer each
                    // detection's distance during .needsPreprocessing.
                    XDWriteBE32(bytes, child + 32, 0xbf800000U);
                    detectionCount++;
                }
                child += childSize;
            }
            if (child != end) {
                XDFail(errorBuffer, errorCapacity, 3, @"misaligned detection container in serialized Cinematography frame");
                return nil;
            }
        }
        cursor += atomSize;
    }
    if (cursor != result.length || detectionCount == 0) {
        XDFail(errorBuffer, errorCapacity, 3, @"serialized Cinematography frame had no resettable detections");
        return nil;
    }
    return result;
}

static CGImagePropertyOrientation XDVisionOrientation(CGAffineTransform transform) {
    // AVFoundation keeps encoded buffers unrotated. Give Vision the display
    // orientation, then map its rectangles back to the encoded image.
    if (fabs(transform.b) > fabs(transform.a)) {
        if (transform.b > 0) return transform.c < 0 ? kCGImagePropertyOrientationRight : kCGImagePropertyOrientationLeftMirrored;
        return transform.c > 0 ? kCGImagePropertyOrientationLeft : kCGImagePropertyOrientationRightMirrored;
    }
    if (transform.a < 0) return transform.d < 0 ? kCGImagePropertyOrientationDown : kCGImagePropertyOrientationUpMirrored;
    return transform.d < 0 ? kCGImagePropertyOrientationDownMirrored : kCGImagePropertyOrientationUp;
}

static CGRect XDEncodedVisionRect(CGRect vision, CGImagePropertyOrientation orientation) {
    CGRect rect = CGRectMake(vision.origin.x, 1.0 - CGRectGetMaxY(vision), vision.size.width, vision.size.height);
    switch (orientation) {
        case kCGImagePropertyOrientationDown:
            rect = CGRectMake(1.0 - CGRectGetMaxX(rect), 1.0 - CGRectGetMaxY(rect), rect.size.width, rect.size.height); break;
        case kCGImagePropertyOrientationRight:
            rect = CGRectMake(rect.origin.y, 1.0 - CGRectGetMaxX(rect), rect.size.height, rect.size.width); break;
        case kCGImagePropertyOrientationLeft:
            rect = CGRectMake(1.0 - CGRectGetMaxY(rect), rect.origin.x, rect.size.height, rect.size.width); break;
        case kCGImagePropertyOrientationUpMirrored:
            rect.origin.x = 1.0 - CGRectGetMaxX(rect); break;
        case kCGImagePropertyOrientationDownMirrored:
            rect.origin.y = 1.0 - CGRectGetMaxY(rect); break;
        case kCGImagePropertyOrientationLeftMirrored:
            rect = CGRectMake(rect.origin.y, rect.origin.x, rect.size.height, rect.size.width); break;
        case kCGImagePropertyOrientationRightMirrored:
            rect = CGRectMake(1.0 - CGRectGetMaxY(rect), 1.0 - CGRectGetMaxX(rect), rect.size.height, rect.size.width); break;
        default: break;
    }
    return CGRectIntersection(rect, CGRectMake(0, 0, 1, 1));
}

static CGFloat XDRectOverlap(CGRect a, CGRect b) {
    CGRect intersection = CGRectIntersection(a, b);
    CGFloat common = CGRectIsNull(intersection) ? 0 : intersection.size.width * intersection.size.height;
    CGFloat total = a.size.width * a.size.height + b.size.width * b.size.height - common;
    return total > 0 ? common / total : 0;
}

static id XDRegistrationLuma(CVPixelBufferRef source) {
    if (CVPixelBufferGetPixelFormatType(source) != kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange ||
        CVPixelBufferLockBaseAddress(source, kCVPixelBufferLock_ReadOnly) != kCVReturnSuccess) return nil;
    size_t sourceWidth = CVPixelBufferGetWidthOfPlane(source, 0), sourceHeight = CVPixelBufferGetHeightOfPlane(source, 0);
    size_t width = MIN(sourceWidth, 960), height = MAX(1, sourceHeight * width / sourceWidth);
    CVPixelBufferRef result = NULL;
    CVReturn status = CVPixelBufferCreate(NULL, width, height, kCVPixelFormatType_OneComponent8,
        (__bridge CFDictionaryRef)@{(id)kCVPixelBufferIOSurfacePropertiesKey: @{}}, &result);
    if (status == kCVReturnSuccess && CVPixelBufferLockBaseAddress(result, 0) == kCVReturnSuccess) {
        const uint8_t *input = CVPixelBufferGetBaseAddressOfPlane(source, 0);
        size_t inputStride = CVPixelBufferGetBytesPerRowOfPlane(source, 0), outputStride = CVPixelBufferGetBytesPerRow(result);
        uint8_t *output = CVPixelBufferGetBaseAddress(result);
        for (size_t y = 0; y < height; ++y) {
            size_t sy = MIN(sourceHeight - 1, (2 * y + 1) * sourceHeight / (2 * height));
            for (size_t x = 0; x < width; ++x) {
                size_t sx = MIN(sourceWidth - 1, (2 * x + 1) * sourceWidth / (2 * width));
                output[y * outputStride + x] = input[sy * inputStride + sx];
            }
        }
        CVPixelBufferUnlockBaseAddress(result, 0);
    } else if (result) { CVPixelBufferRelease(result); result = NULL; }
    CVPixelBufferUnlockBaseAddress(source, kCVPixelBufferLock_ReadOnly);
    return CFBridgingRelease(result);
}

static double XDRegisteredScale(matrix_float3x3 matrix, float x, float y) {
    vector_float3 projected = simd_mul(matrix, (vector_float3){x, y, 1});
    if (!isfinite(projected.z) || fabsf(projected.z) < 0.01f) return NAN;
    float divisor = projected.z * projected.z;
    float a = (matrix.columns[0].x * projected.z - projected.x * matrix.columns[0].z) / divisor;
    float b = (matrix.columns[1].x * projected.z - projected.x * matrix.columns[1].z) / divisor;
    float c = (matrix.columns[0].y * projected.z - projected.y * matrix.columns[0].z) / divisor;
    float d = (matrix.columns[1].y * projected.z - projected.y * matrix.columns[1].z) / divisor;
    return sqrt(fabs((double)a * d - (double)b * c));
}

static double XDRelativeZoom(id previous, id current) {
    if (!previous || !current) return 1;
    CVPixelBufferRef buffer = (__bridge CVPixelBufferRef)current;
    VNHomographicImageRegistrationRequest *request = [[VNHomographicImageRegistrationRequest alloc]
        initWithTargetedCVPixelBuffer:(__bridge CVPixelBufferRef)previous options:@{}];
    NSError *error = nil;
    if (![[[VNImageRequestHandler alloc] initWithCVPixelBuffer:buffer options:@{}]
          performRequests:@[request] error:&error] || !request.results.firstObject) return 1;
    matrix_float3x3 matrix = request.results.firstObject.warpTransform;
    double minimum = INFINITY, maximum = 0, scales[9];
    NSUInteger count = 0;
    for (float x = 0.25f; x <= 0.75f; x += 0.25f) {
        for (float y = 0.25f; y <= 0.75f; y += 0.25f) {
            double scale = XDRegisteredScale(matrix, x * CVPixelBufferGetWidth(buffer), y * CVPixelBufferGetHeight(buffer));
            if (!isfinite(scale) || scale <= 0) return 1;
            minimum = MIN(minimum, scale); maximum = MAX(maximum, scale); scales[count++] = scale;
        }
    }
    for (NSUInteger i = 0; i < count; ++i) for (NSUInteger j = i + 1; j < count; ++j) {
        if (scales[j] < scales[i]) { double saved = scales[i]; scales[i] = scales[j]; scales[j] = saved; }
    }
    double scale = scales[4];
    // Use only substantial, spatially consistent magnification. Small scale
    // drift from walking and perspective changes must not accumulate as zoom.
    if (scale < 0.8 || scale > 1.25 || (maximum - minimum) / scale > 0.08 || fabs(log(scale)) < 0.02) return 1;
    return scale;
}

static NSData *XDSerializeTimedRendering(float focusDisparity, float aperture, float focalLength,
                                         CGSize encodedSize, uint32_t frameId,
                                         char *errorBuffer, size_t errorCapacity) {
    Class cls = NSClassFromString(@"PTTimedRenderingMetadataVersion2");
    if (!cls) {
        XDFail(errorBuffer, errorCapacity, 3, @"PTTimedRenderingMetadataVersion2 is unavailable");
        return nil;
    }
    Method initMethod = XDRequireMethod(cls, "initWithMinorVersion:", NO, "@20@0:8I16", errorBuffer, errorCapacity);
    if (!initMethod) return nil;
    id object = ((id (*)(id, SEL, uint32_t))method_getImplementation(initMethod))(
        [cls alloc], sel_registerName("initWithMinorVersion:"), 1);
    Method setVisibleCrop = XDRequireMethod(cls, "setVisCropFactor:", NO, "v24@0:816", errorBuffer, errorCapacity);
    if (!object || !setVisibleCrop) return nil;
    // PTCinematographyFrame derives its focal length from the timed-rendering
    // focal and this two-axis visible crop. The class default is (0, 0), which
    // leaves the parsed frame focal at zero even when a focal length is set.
    // Synthetic RGB has no optical-crop measurement, so use the identity crop.
    ((PTSetFloat2)method_getImplementation(setVisibleCrop))(
        object, sel_registerName("setVisCropFactor:"), (vector_float2){1.0f, 1.0f});
    Method setSensorCrop = XDRequireMethod(cls, "setTotalSensorCropRectSize:", NO,
                                          "v32@0:8{CGSize=dd}16", errorBuffer, errorCapacity);
    if (!setSensorCrop || !isfinite(encodedSize.width) || !isfinite(encodedSize.height) ||
        encodedSize.width <= 0 || encodedSize.height <= 0) return nil;
    // Ordinary video has no measured sensor region. A virtual region with the
    // encoded image size keeps rendering geometry valid without borrowing a
    // camera sample's overscan or stabilization crop.
    ((void (*)(id, SEL, CGSize))method_getImplementation(setSensorCrop))(
        object, sel_registerName("setTotalSensorCropRectSize:"), encodedSize);
    const struct { const char *name; float value; } floatFields[] = {
        {"setFocusDisparity:", focusDisparity}, {"setFNumber:", aperture},
        {"setFocalLenIn35mmFilm:", focalLength}, {"setAlphaLowLight:", 1.0f},
    };
    for (size_t i = 0; i < sizeof(floatFields) / sizeof(floatFields[0]); ++i) {
        Method method = XDRequireMethod(cls, floatFields[i].name, NO, "v20@0:8f16", errorBuffer, errorCapacity);
        if (!method) return nil;
        ((PTSetFloat)method_getImplementation(method))(object, sel_registerName(floatFields[i].name), floatFields[i].value);
    }
    const struct { const char *name; uint32_t value; } uintFields[] = {
        {"setAGC:", 1}, {"setConversionGain:", 1}, {"setReadNoise_1x:", 0},
        {"setReadNoise_8x:", 0}, {"setFrameId:", frameId},
    };
    for (size_t i = 0; i < sizeof(uintFields) / sizeof(uintFields[0]); ++i) {
        Method method = XDRequireMethod(cls, uintFields[i].name, NO, "v20@0:8I16", errorBuffer, errorCapacity);
        if (!method) return nil;
        ((PTSetUInt32)method_getImplementation(method))(object, sel_registerName(uintFields[i].name), uintFields[i].value);
    }
    return XDSerialize(object, cls, NO, errorBuffer, errorCapacity);
}

static BOOL XDSavePixelBuffer(CVPixelBufferRef pixelBuffer, NSString *path) {
    if (!pixelBuffer || CVPixelBufferLockBaseAddress(pixelBuffer, kCVPixelBufferLock_ReadOnly) != kCVReturnSuccess) return NO;
    size_t byteCount = CVPixelBufferGetBytesPerRow(pixelBuffer) * CVPixelBufferGetHeight(pixelBuffer);
    void *base = CVPixelBufferGetBaseAddress(pixelBuffer);
    FILE *file = base ? fopen(path.fileSystemRepresentation, "wb") : NULL;
    BOOL success = file && fwrite(base, 1, byteCount, file) == byteCount;
    if (file) fclose(file);
    CVPixelBufferUnlockBaseAddress(pixelBuffer, kCVPixelBufferLock_ReadOnly);
    return success;
}

static uint32_t XDApproximateSceneIlluminanceMilliLux(CVPixelBufferRef pixelBuffer) {
    // The ordinary source has no sensor illuminance. Approximate a stable
    // per-frame value from the video-range luma histogram; this is a proxy,
    // not a recovered physical lux measurement.
    if (!pixelBuffer || CVPixelBufferGetPlaneCount(pixelBuffer) < 1 ||
        CVPixelBufferLockBaseAddress(pixelBuffer, kCVPixelBufferLock_ReadOnly) != kCVReturnSuccess) {
        return 1;
    }
    const uint8_t *luma = CVPixelBufferGetBaseAddressOfPlane(pixelBuffer, 0);
    size_t width = CVPixelBufferGetWidthOfPlane(pixelBuffer, 0);
    size_t height = CVPixelBufferGetHeightOfPlane(pixelBuffer, 0);
    size_t stride = CVPixelBufferGetBytesPerRowOfPlane(pixelBuffer, 0);
    if (!luma || width == 0 || height == 0 || stride < width) {
        CVPixelBufferUnlockBaseAddress(pixelBuffer, kCVPixelBufferLock_ReadOnly);
        return 1;
    }
    uint64_t histogram[256] = {0};
    uint64_t count = 0;
    size_t step = MAX((size_t)1, MAX(width, height) / 320);
    for (size_t y = 0; y < height; y += step) {
        const uint8_t *row = luma + y * stride;
        for (size_t x = 0; x < width; x += step) {
            histogram[row[x]]++;
            count++;
        }
    }
    CVPixelBufferUnlockBaseAddress(pixelBuffer, kCVPixelBufferLock_ReadOnly);
    if (count == 0) return 1;

    // Ignore the darkest and brightest 1% so a small black border, specular
    // pixel, or subtitle does not dominate the scene-level estimate.
    uint64_t trim = count / 100;
    uint64_t skip = trim;
    uint64_t take = count - 2 * trim;
    double normalizedSum = 0.0;
    for (NSUInteger value = 0; value < 256 && take > 0; ++value) {
        uint64_t binCount = histogram[value];
        uint64_t skippedHere = MIN(skip, binCount);
        skip -= skippedHere;
        binCount -= skippedHere;
        uint64_t usedHere = MIN(take, binCount);
        double normalized = ((double)value - 16.0) / 219.0;
        normalized = fmin(1.0, fmax(0.0, normalized));
        normalizedSum += normalized * (double)usedHere;
        take -= usedHere;
    }
    double normalizedMean = normalizedSum / (double)(count - 2 * trim);
    // Approximate the capture lux scale using the luma level of the genuine
    // iPhone reference clips as an anchor (~0.38 video-range luma ~= 76 lux).
    double milliLux = normalizedMean * 200000.0;
    return (uint32_t)llround(fmax(1.0, fmin(200000.0, milliLux)));
}

static CVPixelBufferRef XDReadDisparity(NSString *path, size_t width, size_t height, OSType pixelFormat,
                                        char *errorBuffer, size_t errorCapacity) {
    NSDictionary *attributes = @{
        (id)kCVPixelBufferIOSurfacePropertiesKey: @{},
        (id)kCVPixelBufferMetalCompatibilityKey: @YES,
    };
    CVPixelBufferRef buffer = NULL;
    CVReturn result = CVPixelBufferCreate(kCFAllocatorDefault, width, height, pixelFormat,
                                          (__bridge CFDictionaryRef)attributes, &buffer);
    if (result != kCVReturnSuccess || !buffer) {
        XDFail(errorBuffer, errorCapacity, 4, [NSString stringWithFormat:@"cannot allocate transient hdis buffer (%d)", result]);
        return NULL;
    }
    NSData *raw = [NSData dataWithContentsOfFile:path];
    if (!raw || CVPixelBufferLockBaseAddress(buffer, 0) != kCVReturnSuccess) {
        CVPixelBufferRelease(buffer);
        XDFail(errorBuffer, errorCapacity, 4, @"cannot read transient hdis buffer");
        return NULL;
    }
    size_t sourceStride = raw.length / height;
    size_t destinationStride = CVPixelBufferGetBytesPerRow(buffer);
    if (raw.length != sourceStride * height || sourceStride < width * 2 || destinationStride < width * 2) {
        CVPixelBufferUnlockBaseAddress(buffer, 0);
        CVPixelBufferRelease(buffer);
        XDFail(errorBuffer, errorCapacity, 4, @"transient hdis buffer geometry is invalid");
        return NULL;
    }
    const uint8_t *source = raw.bytes;
    uint8_t *destination = CVPixelBufferGetBaseAddress(buffer);
    for (size_t row = 0; row < height; ++row) memcpy(destination + row * destinationStride, source + row * sourceStride, width * 2);
    CVPixelBufferUnlockBaseAddress(buffer, 0);
    return buffer;
}

static AVMutableMetadataItem *XDMetadataItem(NSString *identifier, NSString *dataType, id value) {
    AVMutableMetadataItem *item = [AVMutableMetadataItem metadataItem];
    item.identifier = identifier;
    item.dataType = dataType;
    item.value = value;
    return item;
}

static BOOL XDWriteMediaAndMetadata(AVURLAsset *asset, AVAssetTrack *videoTrack,
                                    NSArray<NSDictionary *> *frameRows,
                                    NSArray<NSData *> *cinematographyPayloads,
                                    NSArray<NSData *> *renderingPayloads,
                                    NSData *globalCinematicData,
                                    NSString *outputPath,
                                    char *errorBuffer, size_t errorCapacity) {
    NSError *error = nil;
    NSURL *outputURL = [NSURL fileURLWithPath:outputPath];
    AVAssetWriter *writer = [[AVAssetWriter alloc] initWithURL:outputURL fileType:AVFileTypeQuickTimeMovie error:&error];
    if (!writer) {
        XDFail(errorBuffer, errorCapacity, 5, [NSString stringWithFormat:@"cannot create AVAssetWriter: %@", error.localizedDescription]);
        return NO;
    }
    // The default movie time scale (600) also rounds the last edit duration
    // of fractional-rate sources even when the video track retains its scale.
    if (videoTrack.naturalTimeScale > 0) writer.movieTimeScale = videoTrack.naturalTimeScale;
    writer.metadata = @[
        XDMetadataItem(@"mdta/com.apple.quicktime.cinematic-video", @"com.apple.metadata.datatype.raw-data", globalCinematicData),
        XDMetadataItem(@"mdta/com.apple.quicktime.cinematic-video-intent", @"com.apple.metadata.datatype.int8", @0),
        XDMetadataItem(@"mdta/com.apple.quicktime.metadata.1", @"com.apple.metadata.datatype.UTF-8", @"1"),
    ];

    AVAssetReader *reader = [[AVAssetReader alloc] initWithAsset:asset error:&error];
    if (!reader) {
        XDFail(errorBuffer, errorCapacity, 5, [NSString stringWithFormat:@"cannot create AVAssetReader: %@", error.localizedDescription]);
        return NO;
    }

    NSMutableArray<NSDictionary *> *mediaPumps = [NSMutableArray array];
    AVAssetWriterInput *writerVideoInput = nil;
    for (AVAssetTrack *track in [asset tracksWithMediaType:AVMediaTypeVideo]) {
        id formatValue = track.formatDescriptions.firstObject;
        CMFormatDescriptionRef description = formatValue ? (__bridge CMFormatDescriptionRef)formatValue : NULL;
        if (description) CFRetain(description);
        AVAssetReaderTrackOutput *readerOutput = [[AVAssetReaderTrackOutput alloc] initWithTrack:track outputSettings:nil];
        readerOutput.alwaysCopiesSampleData = NO;
        AVAssetWriterInput *writerInput = [[AVAssetWriterInput alloc] initWithMediaType:AVMediaTypeVideo outputSettings:nil sourceFormatHint:description];
        writerInput.expectsMediaDataInRealTime = NO;
        writerInput.transform = track.preferredTransform;
        if (track.naturalTimeScale > 0) writerInput.mediaTimeScale = track.naturalTimeScale;
        if (description) CFRelease(description);
        if (![reader canAddOutput:readerOutput] || ![writer canAddInput:writerInput]) {
            XDFail(errorBuffer, errorCapacity, 5, @"AVFoundation rejected the passthrough video track");
            return NO;
        }
        [reader addOutput:readerOutput];
        [writer addInput:writerInput];
        [mediaPumps addObject:@{@"output": readerOutput, @"input": writerInput}];
        if (track == videoTrack) writerVideoInput = writerInput;
    }
    for (AVAssetTrack *track in [asset tracksWithMediaType:AVMediaTypeAudio]) {
        id formatValue = track.formatDescriptions.firstObject;
        CMFormatDescriptionRef description = formatValue ? (__bridge CMFormatDescriptionRef)formatValue : NULL;
        if (description) CFRetain(description);
        AVAssetReaderTrackOutput *readerOutput = [[AVAssetReaderTrackOutput alloc] initWithTrack:track outputSettings:nil];
        readerOutput.alwaysCopiesSampleData = NO;
        AVAssetWriterInput *writerInput = [[AVAssetWriterInput alloc] initWithMediaType:AVMediaTypeAudio outputSettings:nil sourceFormatHint:description];
        writerInput.expectsMediaDataInRealTime = NO;
        if (description) CFRelease(description);
        if (![reader canAddOutput:readerOutput] || ![writer canAddInput:writerInput]) {
            XDFail(errorBuffer, errorCapacity, 5, @"AVFoundation rejected the passthrough audio track");
            return NO;
        }
        [reader addOutput:readerOutput];
        [writer addInput:writerInput];
        [mediaPumps addObject:@{@"output": readerOutput, @"input": writerInput}];
    }
    if (!writerVideoInput) {
        XDFail(errorBuffer, errorCapacity, 5, @"source video track is missing from the writer");
        return NO;
    }

    NSArray *firstItems = @[
        XDMetadataItem(@"mdta/com.apple.quicktime.cinematic-video.cinematography", @"com.apple.metadata.datatype.raw-data", cinematographyPayloads.firstObject),
        XDMetadataItem(@"mdta/com.apple.quicktime.cinematic-video.rendering", @"com.apple.metadata.datatype.raw-data", renderingPayloads.firstObject),
        XDMetadataItem(@"mdta/com.apple.quicktime.aperture-float", @"com.apple.metadata.datatype.float32", @2.8f),
        XDMetadataItem(@"mdta/com.apple.quicktime.disparity-float", @"com.apple.metadata.datatype.float32", @-1.0f),
    ];
    AVTimedMetadataGroup *firstGroup = [[AVTimedMetadataGroup alloc] initWithItems:firstItems timeRange:CMTimeRangeMake(CMTimeMake(0, 600), CMTimeMake(1, 30))];
    CMMetadataFormatDescriptionRef formatDescription = [firstGroup copyFormatDescription];
    if (!formatDescription) {
        XDFail(errorBuffer, errorCapacity, 5, @"AVFoundation could not create the timed Cinematic metadata format description");
        return NO;
    }
    AVAssetWriterInput *metadataInput = [[AVAssetWriterInput alloc] initWithMediaType:AVMediaTypeMetadata outputSettings:nil sourceFormatHint:formatDescription];
    CFRelease(formatDescription);
    metadataInput.expectsMediaDataInRealTime = NO;
    // Keep metadata timestamps exactly representable for arbitrary source
    // rates (including 30000/1001); a fixed 15360 scale rounds those timestamps.
    CMTimeScale metadataTimeScale = videoTrack.naturalTimeScale > 0 ? videoTrack.naturalTimeScale : 15360;
    metadataInput.mediaTimeScale = metadataTimeScale;
    AVAssetWriterInputMetadataAdaptor *metadataAdaptor = [[AVAssetWriterInputMetadataAdaptor alloc] initWithAssetWriterInput:metadataInput];
    if (![writer canAddInput:metadataInput]) {
        XDFail(errorBuffer, errorCapacity, 5, @"AVFoundation rejected the timed Cinematic metadata track");
        return NO;
    }
    [writer addInput:metadataInput];

    // RGB does not recover the camera's physical illuminance sensor reading.
    // Keep the required timed field explicit as a synthetic placeholder.
    AVMutableMetadataItem *firstIlluminanceItem = XDMetadataItem(
        @"mdta/com.apple.quicktime.scene-illuminance", @"com.apple.quicktime.milli-lux", @0);
    AVTimedMetadataGroup *firstIlluminanceGroup = [[AVTimedMetadataGroup alloc]
        initWithItems:@[firstIlluminanceItem]
        timeRange:CMTimeRangeMake(CMTimeMake(0, 600), CMTimeMake(1, 30))];
    CMMetadataFormatDescriptionRef illuminanceFormatDescription = [firstIlluminanceGroup copyFormatDescription];
    if (!illuminanceFormatDescription) {
        XDFail(errorBuffer, errorCapacity, 5, @"AVFoundation could not create the scene-illuminance metadata format description");
        return NO;
    }
    AVAssetWriterInput *illuminanceInput = [[AVAssetWriterInput alloc]
        initWithMediaType:AVMediaTypeMetadata outputSettings:nil sourceFormatHint:illuminanceFormatDescription];
    CFRelease(illuminanceFormatDescription);
    illuminanceInput.expectsMediaDataInRealTime = NO;
    illuminanceInput.mediaTimeScale = metadataTimeScale;
    AVAssetWriterInputMetadataAdaptor *illuminanceAdaptor = [[AVAssetWriterInputMetadataAdaptor alloc]
        initWithAssetWriterInput:illuminanceInput];
    if (![writer canAddInput:illuminanceInput]) {
        XDFail(errorBuffer, errorCapacity, 5, @"AVFoundation rejected the scene-illuminance metadata track");
        return NO;
    }
    [writer addInput:illuminanceInput];

    for (NSString *association in @[@"cdsc", @"cdep"]) {
        if ([metadataInput canAddTrackAssociationWithTrackOfInput:writerVideoInput type:association]) {
            [metadataInput addTrackAssociationWithTrackOfInput:writerVideoInput type:association];
        }
    }
    if ([illuminanceInput canAddTrackAssociationWithTrackOfInput:writerVideoInput type:@"rndr"]) {
        [illuminanceInput addTrackAssociationWithTrackOfInput:writerVideoInput type:@"rndr"];
    } else {
        XDFail(errorBuffer, errorCapacity, 5, @"AVFoundation rejected the scene-illuminance rendering association");
        return NO;
    }

    if (![reader startReading] || ![writer startWriting]) {
        XDFail(errorBuffer, errorCapacity, 5, [NSString stringWithFormat:@"cannot start AVFoundation copy: %@ %@", reader.error.localizedDescription ?: @"", writer.error.localizedDescription ?: @""]);
        return NO;
    }
    [writer startSessionAtSourceTime:kCMTimeZero];
    dispatch_group_t group = dispatch_group_create();
    dispatch_queue_t queue = dispatch_queue_create("com.xdremux.cinematic.writer", DISPATCH_QUEUE_SERIAL);
    __block NSString *workerError = nil;
    for (NSDictionary *pump in mediaPumps) {
        AVAssetReaderTrackOutput *readerOutput = pump[@"output"];
        AVAssetWriterInput *writerInput = pump[@"input"];
        __block NSUInteger appendedSampleCount = 0;
        dispatch_group_enter(group);
        [writerInput requestMediaDataWhenReadyOnQueue:queue usingBlock:^{
            while (writerInput.readyForMoreMediaData) {
                CMSampleBufferRef sample = [readerOutput copyNextSampleBuffer];
                if (!sample) {
                    fprintf(stderr, "cinematic-passthrough media=%s appended_samples=%lu reader_status=%ld\n",
                            writerInput.mediaType.UTF8String, (unsigned long)appendedSampleCount, (long)reader.status);
                    [writerInput markAsFinished];
                    dispatch_group_leave(group);
                    return;
                }
                BOOL appended = [writerInput appendSampleBuffer:sample];
                CFRelease(sample);
                if (!appended) {
                    @synchronized (writer) { workerError = writer.error.localizedDescription ?: @"passthrough sample append failed"; }
                    [reader cancelReading];
                    [writerInput markAsFinished];
                    dispatch_group_leave(group);
                    return;
                }
                appendedSampleCount++;
            }
        }];
    }
    dispatch_group_enter(group);
    __block NSUInteger nextGroup = 0;
    [metadataInput requestMediaDataWhenReadyOnQueue:queue usingBlock:^{
        while (metadataInput.readyForMoreMediaData && nextGroup < frameRows.count) {
            NSDictionary *row = frameRows[nextGroup];
            CMTime pts = CMTimeMake([row[@"pts"] longLongValue], [row[@"timescale"] intValue]);
            CMTime duration = CMTimeMake([row[@"duration"] longLongValue], [row[@"durationScale"] intValue]);
            if (!CMTIME_IS_VALID(duration) || CMTimeCompare(duration, kCMTimeZero) <= 0) duration = CMTimeMake(1, 30);
            CMTimeRange range = CMTimeRangeMake(pts, duration);
            float focus = [row[@"focus"] floatValue];
            NSArray *items = @[
                XDMetadataItem(@"mdta/com.apple.quicktime.cinematic-video.cinematography", @"com.apple.metadata.datatype.raw-data", cinematographyPayloads[nextGroup]),
                XDMetadataItem(@"mdta/com.apple.quicktime.cinematic-video.rendering", @"com.apple.metadata.datatype.raw-data", renderingPayloads[nextGroup]),
                XDMetadataItem(@"mdta/com.apple.quicktime.aperture-float", @"com.apple.metadata.datatype.float32", @([row[@"aperture"] floatValue])),
                // A needsPreprocessing capture has not materialized disparity. Keep the native
                // pending sentinel; refined focus distance remains in the Cinematography payload.
                XDMetadataItem(@"mdta/com.apple.quicktime.disparity-float", @"com.apple.metadata.datatype.float32", @-1.0f),
            ];
            AVTimedMetadataGroup *timedGroup = [[AVTimedMetadataGroup alloc] initWithItems:items timeRange:range];
            if (![metadataAdaptor appendTimedMetadataGroup:timedGroup]) {
                @synchronized (writer) { workerError = writer.error.localizedDescription ?: @"timed metadata append failed"; }
                [writer cancelWriting];
                [metadataInput markAsFinished];
                dispatch_group_leave(group);
                return;
            }
            nextGroup++;
        }
        if (nextGroup == frameRows.count) {
            [metadataInput markAsFinished];
            dispatch_group_leave(group);
        }
    }];

    dispatch_group_enter(group);
    __block NSUInteger nextIlluminanceGroup = 0;
    [illuminanceInput requestMediaDataWhenReadyOnQueue:queue usingBlock:^{
        while (illuminanceInput.readyForMoreMediaData && nextIlluminanceGroup < frameRows.count) {
            NSDictionary *row = frameRows[nextIlluminanceGroup];
            CMTime pts = CMTimeMake([row[@"pts"] longLongValue], [row[@"timescale"] intValue]);
            CMTime duration = CMTimeMake([row[@"duration"] longLongValue], [row[@"durationScale"] intValue]);
            if (!CMTIME_IS_VALID(duration) || CMTimeCompare(duration, kCMTimeZero) <= 0) duration = CMTimeMake(1, 30);
            AVMutableMetadataItem *illuminanceItem = XDMetadataItem(
                @"mdta/com.apple.quicktime.scene-illuminance", @"com.apple.quicktime.milli-lux",
                @([row[@"sceneIlluminanceMilliLux"] intValue]));
            AVTimedMetadataGroup *timedGroup = [[AVTimedMetadataGroup alloc]
                initWithItems:@[illuminanceItem] timeRange:CMTimeRangeMake(pts, duration)];
            if (![illuminanceAdaptor appendTimedMetadataGroup:timedGroup]) {
                @synchronized (writer) { workerError = writer.error.localizedDescription ?: @"scene-illuminance metadata append failed"; }
                [writer cancelWriting];
                [illuminanceInput markAsFinished];
                dispatch_group_leave(group);
                return;
            }
            nextIlluminanceGroup++;
        }
        if (nextIlluminanceGroup == frameRows.count) {
            [illuminanceInput markAsFinished];
            dispatch_group_leave(group);
        }
    }];
    if (dispatch_group_wait(group, dispatch_time(DISPATCH_TIME_NOW, 3600LL * NSEC_PER_SEC)) != 0) {
        [reader cancelReading];
        [writer cancelWriting];
        XDFail(errorBuffer, errorCapacity, 5, @"timed out while writing the Cinematic MOV");
        return NO;
    }
    if (workerError) {
        [writer cancelWriting];
        XDFail(errorBuffer, errorCapacity, 5, workerError);
        return NO;
    }
    dispatch_semaphore_t finished = dispatch_semaphore_create(0);
    [writer finishWritingWithCompletionHandler:^{ dispatch_semaphore_signal(finished); }];
    if (dispatch_semaphore_wait(finished, dispatch_time(DISPATCH_TIME_NOW, 3600LL * NSEC_PER_SEC)) != 0 || writer.status != AVAssetWriterStatusCompleted) {
        XDFail(errorBuffer, errorCapacity, 5, [NSString stringWithFormat:@"AVAssetWriter did not complete: %@", writer.error.localizedDescription ?: @"unknown error"]);
        return NO;
    }
    return YES;
}

int xdremux_cinematic_generate(const char *input_path, const char *output_path,
                               float focal_length_35mm, float aperture,
                               uint64_t *processed_frames,
                               char *error_buffer, size_t error_buffer_size) {
    if (error_buffer && error_buffer_size) error_buffer[0] = '\0';
    if (processed_frames) *processed_frames = 0;
    if (!input_path || !output_path || !*input_path || !*output_path ||
        !isfinite(focal_length_35mm) || focal_length_35mm <= 0 ||
        !isfinite(aperture) || aperture < 1.0f || aperture > 64.0f) {
        return XDFail(error_buffer, error_buffer_size, 64, @"invalid Cinematic input, output, focal length, or aperture");
    }
    @try {
      @autoreleasepool {
        NSError *error = nil;
        for (NSString *frameworkPath in @[@"/System/Library/PrivateFrameworks/PhotoImaging.framework",
                                           @"/System/Library/PrivateFrameworks/CinematicUtil.framework"]) {
            NSBundle *bundle = [NSBundle bundleWithPath:frameworkPath];
            if (![bundle loadAndReturnError:&error]) {
                return XDFail(error_buffer, error_buffer_size, 3,
                              [NSString stringWithFormat:@"cannot load %@: %@", frameworkPath.lastPathComponent, error.localizedDescription]);
            }
        }
        AVURLAsset *asset = [AVURLAsset URLAssetWithURL:[NSURL fileURLWithPath:[NSString stringWithUTF8String:input_path]] options:nil];
        AVAssetTrack *videoTrack = [asset tracksWithMediaType:AVMediaTypeVideo].firstObject;
        if (!videoTrack) return XDFail(error_buffer, error_buffer_size, 65, @"input has no video track");

        NSBundle *portrait = [NSBundle bundleWithPath:@"/System/Library/PrivateFrameworks/Portrait.framework"];
        if (![portrait loadAndReturnError:&error]) {
            return XDFail(error_buffer, error_buffer_size, 3, [NSString stringWithFormat:@"cannot load Portrait.framework: %@", error.localizedDescription]);
        }
        Class providerClass = NSClassFromString(@"PTMonocularDisparityProvider");
        Class scriptClass = NSClassFromString(@"PTCinematographyScript");
        Class frameClass = NSClassFromString(@"PTCinematographyFrame");
        Class detectionClass = NSClassFromString(@"PTCinematographyDetection");
        Class decisionClass = NSClassFromString(@"PTCinematographyDecision");
        Class refinementClass = NSClassFromString(@"PTCinematographyPostcaptureRefinement");
        if (!providerClass || !scriptClass || !frameClass || !detectionClass || !decisionClass || !refinementClass) {
            return XDFail(error_buffer, error_buffer_size, 3, @"required Apple Cinematic SPI classes are unavailable");
        }
        Method supportedMethod = XDRequireMethod(providerClass, "isSupported", YES, "B16@0:8", error_buffer, error_buffer_size);
        Method providerInit = XDRequireMethod(providerClass, "initWithQuality:inputSize:", NO,
                                             "@36@0:8i16{CGSize=dd}20", error_buffer, error_buffer_size);
        Method disparitySizeMethod = XDRequireMethod(providerClass, "disparitySize", NO, "{?=ii}16@0:8", error_buffer, error_buffer_size);
        Method disparityFormatMethod = XDRequireMethod(providerClass, "disparityPixelFormat", NO, "I16@0:8", error_buffer, error_buffer_size);
        Method inferMethod = XDRequireMethod(providerClass, "disparityForColorBuffer:focalLenIn35mmFilm:", NO,
                                             "^{__CVBuffer=}28@0:8^{__CVBuffer=}16f24", error_buffer, error_buffer_size);
        if (!supportedMethod || !providerInit || !disparitySizeMethod || !disparityFormatMethod || !inferMethod ||
            !((PTProviderSupported)method_getImplementation(supportedMethod))(providerClass, sel_registerName("isSupported"))) {
            return XDFail(error_buffer, error_buffer_size, 3, @"PTMonocularDisparityProvider is unsupported on this system");
        }
        CGSize inputSize = videoTrack.naturalSize;
        id provider = ((PTProviderInit)method_getImplementation(providerInit))([providerClass alloc],
                           sel_registerName("initWithQuality:inputSize:"), 1, inputSize);
        if (!provider) return XDFail(error_buffer, error_buffer_size, 3, @"PTMonocularDisparityProvider initialization failed");
        PTDisparitySize32 disparitySize = ((PTDisparitySize)method_getImplementation(disparitySizeMethod))(provider, sel_registerName("disparitySize"));
        OSType disparityFormat = ((PTDisparityFormat)method_getImplementation(disparityFormatMethod))(provider, sel_registerName("disparityPixelFormat"));
        if (disparitySize.width <= 0 || disparitySize.height <= 0) return XDFail(error_buffer, error_buffer_size, 3, @"provider returned invalid disparity geometry");

        NSString *tempDirectory = [NSTemporaryDirectory() stringByAppendingPathComponent:[NSString stringWithFormat:@"xdremux-cinematic-%@", NSUUID.UUID.UUIDString]];
        if (![[NSFileManager defaultManager] createDirectoryAtPath:tempDirectory withIntermediateDirectories:NO attributes:nil error:&error]) {
            return XDFail(error_buffer, error_buffer_size, 73, [NSString stringWithFormat:@"cannot create transient disparity directory: %@", error.localizedDescription]);
        }
        NSMutableArray *frameRows = [NSMutableArray array];
        NSMutableArray *scriptFrames = [NSMutableArray array];
        id script = [[scriptClass alloc] init];
        Method trackAllocatorGetter = XDRequireMethod(scriptClass, "trackAllocator", NO, "@16@0:8", error_buffer, error_buffer_size);
        Method focusAllocatorGetter = XDRequireMethod(scriptClass, "trackAllocatorForFocusIdentifier", NO, "@16@0:8", error_buffer, error_buffer_size);
        Method nextTrackIdentifier = XDRequireMethod(NSClassFromString(@"PTCinematographyTrackAllocator"),
            "nextTrackIdentifier", NO, "q16@0:8", error_buffer, error_buffer_size);
        Method internalizeFocusIdentifier = XDRequireMethod(scriptClass, "_internalizeTrackNumberForFocusIdentifier:",
            NO, "@24@0:8@16", error_buffer, error_buffer_size);
        Method setFrames = XDRequireMethod(scriptClass, "setFrames:", NO, "v24@0:8@16", error_buffer, error_buffer_size);
        Method internalizeTracks = XDRequireMethod(scriptClass, "_internalizeTracksFromFrames:",
            NO, "v24@0:8@16", error_buffer, error_buffer_size);
        Method didInternalizeTracks = XDRequireMethod(scriptClass, "didInternalizeTracks", NO,
            "B16@0:8", error_buffer, error_buffer_size);
        Method setTrack = XDRequireMethod(detectionClass, "setTrackIdentifier:", NO,
            "v24@0:8q16", error_buffer, error_buffer_size);
        Method setGroup = XDRequireMethod(detectionClass, "setGroupIdentifier:", NO,
            "v24@0:8q16", error_buffer, error_buffer_size);
        Method setType = XDRequireMethod(detectionClass, "setDetectionType:", NO,
            "v24@0:8Q16", error_buffer, error_buffer_size);
        Method setFocusIdentifier = XDRequireMethod(detectionClass, "setFocusIdentifier:", NO,
            "v24@0:8@16", error_buffer, error_buffer_size);
        if (!script || !trackAllocatorGetter || !focusAllocatorGetter || !nextTrackIdentifier ||
            !internalizeFocusIdentifier || !setFrames || !internalizeTracks || !didInternalizeTracks ||
            !setTrack || !setGroup || !setType || !setFocusIdentifier) {
            [[NSFileManager defaultManager] removeItemAtPath:tempDirectory error:nil];
            return XDFail(error_buffer, error_buffer_size, 3, @"Apple Cinematography track allocator API is unavailable");
        }
        id trackAllocator = ((PTObjectGetter)method_getImplementation(trackAllocatorGetter))(
            script, sel_registerName("trackAllocator"));
        id focusAllocator = ((PTObjectGetter)method_getImplementation(focusAllocatorGetter))(
            script, sel_registerName("trackAllocatorForFocusIdentifier"));
        if (!trackAllocator || !focusAllocator) {
            [[NSFileManager defaultManager] removeItemAtPath:tempDirectory error:nil];
            return XDFail(error_buffer, error_buffer_size, 3, @"Apple Cinematography script returned no track allocator");
        }
        PTNextTrackIdentifier nextTrack = (PTNextTrackIdentifier)method_getImplementation(nextTrackIdentifier);
        long long trackIdentifier = nextTrack(trackAllocator, sel_registerName("nextTrackIdentifier"));
        long long groupIdentifier = nextTrack(trackAllocator, sel_registerName("nextTrackIdentifier"));
        long long secondTrackIdentifier = nextTrack(trackAllocator, sel_registerName("nextTrackIdentifier"));
        long long secondGroupIdentifier = nextTrack(trackAllocator, sel_registerName("nextTrackIdentifier"));
        long long subjectTrackIdentifiers[2] = {trackIdentifier, secondTrackIdentifier};
        long long subjectGroupIdentifiers[2] = {groupIdentifier, secondGroupIdentifier};
        id autofocusTrackNumber = ((PTInternalizeFocusTrack)method_getImplementation(internalizeFocusIdentifier))(
            script, sel_registerName("_internalizeTrackNumberForFocusIdentifier:"), @"BA-1");
        if (!autofocusTrackNumber || ![autofocusTrackNumber respondsToSelector:@selector(longLongValue)]) {
            [[NSFileManager defaultManager] removeItemAtPath:tempDirectory error:nil];
            return XDFail(error_buffer, error_buffer_size, 3, @"Apple Cinematography script could not allocate the autofocus track");
        }
        long long autofocusTrackIdentifier = [autofocusTrackNumber longLongValue];
        AVAssetReader *decodeReader = [[AVAssetReader alloc] initWithAsset:asset error:&error];
        NSDictionary *pixelAttributes = @{
            (id)kCVPixelBufferPixelFormatTypeKey: @(kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange),
            (id)kCVPixelBufferIOSurfacePropertiesKey: @{},
            (id)kCVPixelBufferMetalCompatibilityKey: @YES,
        };
        AVAssetReaderTrackOutput *decodeOutput = [[AVAssetReaderTrackOutput alloc] initWithTrack:videoTrack outputSettings:pixelAttributes];
        decodeOutput.alwaysCopiesSampleData = NO;
        if (![decodeReader canAddOutput:decodeOutput]) {
            [[NSFileManager defaultManager] removeItemAtPath:tempDirectory error:nil];
            return XDFail(error_buffer, error_buffer_size, 65, @"AVAssetReader cannot decode the source video");
        }
        [decodeReader addOutput:decodeOutput];
        if (![decodeReader startReading]) {
            [[NSFileManager defaultManager] removeItemAtPath:tempDirectory error:nil];
            return XDFail(error_buffer, error_buffer_size, 65, [NSString stringWithFormat:@"AVAssetReader failed: %@", decodeReader.error.localizedDescription]);
        }
        NSUInteger index = 0;
        VNSequenceRequestHandler *visionHandler = [[VNSequenceRequestHandler alloc] init];
        CGImagePropertyOrientation visionOrientation = XDVisionOrientation(videoTrack.preferredTransform);
        CGRect previousFocusBox = CGRectNull;
        id previousZoomImage = nil;
        double previousZoomTime = -INFINITY, relativeZoom = 1;
        while (YES) {
            @autoreleasepool {
                CMSampleBufferRef sample = [decodeOutput copyNextSampleBuffer];
                if (!sample) break;
                CVPixelBufferRef color = CMSampleBufferGetImageBuffer(sample);
                CMTime pts = CMSampleBufferGetPresentationTimeStamp(sample);
                CMTime duration = CMSampleBufferGetDuration(sample);
                if (!color || !CMTIME_IS_VALID(pts)) {
                    CFRelease(sample);
                    [decodeReader cancelReading];
                    [[NSFileManager defaultManager] removeItemAtPath:tempDirectory error:nil];
                    return XDFail(error_buffer, error_buffer_size, 65, @"decoded frame is missing a pixel buffer or presentation time");
                }
                uint32_t sceneIlluminanceMilliLux = XDApproximateSceneIlluminanceMilliLux(color);
                if (CMTimeGetSeconds(pts) - previousZoomTime >= 0.19) {
                    id currentZoomImage = XDRegistrationLuma(color);
                    if (currentZoomImage) {
                        relativeZoom *= XDRelativeZoom(previousZoomImage, currentZoomImage);
                        relativeZoom = MIN(300.0 / focal_length_35mm, MAX(1.0 / focal_length_35mm, relativeZoom));
                        previousZoomImage = currentZoomImage;
                    }
                    previousZoomTime = CMTimeGetSeconds(pts);
                }
                float frameFocalLength = focal_length_35mm * relativeZoom;
                CVPixelBufferRef inferred = ((PTInferDisparity)method_getImplementation(inferMethod))(
                    provider, sel_registerName("disparityForColorBuffer:focalLenIn35mmFilm:"), color, frameFocalLength);
                if (!inferred) {
                    CFRelease(sample);
                    [decodeReader cancelReading];
                    [[NSFileManager defaultManager] removeItemAtPath:tempDirectory error:nil];
                    return XDFail(error_buffer, error_buffer_size, 3, [NSString stringWithFormat:@"Apple disparity inference failed at frame %lu", (unsigned long)index]);
                }
                if (CVPixelBufferGetWidth(inferred) != (size_t)disparitySize.width ||
                    CVPixelBufferGetHeight(inferred) != (size_t)disparitySize.height ||
                    CVPixelBufferGetPixelFormatType(inferred) != disparityFormat) {
                    CVPixelBufferRelease(inferred);
                    CFRelease(sample);
                    [decodeReader cancelReading];
                    [[NSFileManager defaultManager] removeItemAtPath:tempDirectory error:nil];
                    return XDFail(error_buffer, error_buffer_size, 3, @"Apple provider returned an unexpected disparity buffer");
                }
                NSString *rawPath = [tempDirectory stringByAppendingPathComponent:[NSString stringWithFormat:@"hdis-%06lu.raw", (unsigned long)index]];
                BOOL saved = XDSavePixelBuffer(inferred, rawPath);
                if (!saved) {
                    CVPixelBufferRelease(inferred);
                    CFRelease(sample);
                    [decodeReader cancelReading];
                    [[NSFileManager defaultManager] removeItemAtPath:tempDirectory error:nil];
                    return XDFail(error_buffer, error_buffer_size, 73, @"cannot preserve transient disparity output for Apple refinement");
                }
                CVPixelBufferRelease(inferred);

                VNDetectFaceRectanglesRequest *faceRequest = [[VNDetectFaceRectanglesRequest alloc] init];
                NSError *visionError = nil;
                BOOL faceDetectionSucceeded = [visionHandler performRequests:@[faceRequest]
                                                            onCMSampleBuffer:sample orientation:visionOrientation error:&visionError];
                if (!faceDetectionSucceeded) {
                    CFRelease(sample);
                    [decodeReader cancelReading];
                    [[NSFileManager defaultManager] removeItemAtPath:tempDirectory error:nil];
                    return XDFail(error_buffer, error_buffer_size, 65,
                                  [NSString stringWithFormat:@"Vision face detection failed at frame %lu: %@",
                                   (unsigned long)index, visionError.localizedDescription ?: @"unknown error"]);
                }
                NSMutableArray<VNDetectedObjectObservation *> *subjects = [NSMutableArray array];
                for (VNFaceObservation *face in (NSArray<VNFaceObservation *> *)faceRequest.results) {
                    if (face.confidence > 0.05 && face.boundingBox.size.width > 0 &&
                        face.boundingBox.size.height > 0) [subjects addObject:face];
                }
                CNDetectionType subjectType = CNDetectionTypeHumanFace;
                if (subjects.count == 0) {
                    VNDetectHumanRectanglesRequest *bodyRequest = [[VNDetectHumanRectanglesRequest alloc] init];
                    bodyRequest.upperBodyOnly = YES;
                    if (![visionHandler performRequests:@[bodyRequest] onCMSampleBuffer:sample
                                             orientation:visionOrientation error:&visionError]) {
                        CFRelease(sample); [decodeReader cancelReading];
                        [[NSFileManager defaultManager] removeItemAtPath:tempDirectory error:nil];
                        return XDFail(error_buffer, error_buffer_size, 65,
                                      [NSString stringWithFormat:@"Vision person detection failed: %@", visionError.localizedDescription]);
                    }
                    for (VNHumanObservation *body in bodyRequest.results) {
                        if (body.confidence >= 0.3 && body.boundingBox.size.width > 0 && body.boundingBox.size.height > 0)
                            [subjects addObject:body];
                    }
                    subjectType = CNDetectionTypeHumanTorso;
                }
                [subjects sortUsingComparator:^NSComparisonResult(VNDetectedObjectObservation *left, VNDetectedObjectObservation *right) {
                    CGFloat leftArea = left.boundingBox.size.width * left.boundingBox.size.height;
                    CGFloat rightArea = right.boundingBox.size.width * right.boundingBox.size.height;
                    if (leftArea > rightArea) return NSOrderedAscending;
                    if (leftArea < rightArea) return NSOrderedDescending;
                    return NSOrderedSame;
                }];
                // Retain a detected subject while its rectangle remains near
                // the prior anchor. A substantially larger subject may replace it.
                if (subjects.count > 1 && !CGRectIsNull(previousFocusBox)) {
                    CGFloat largestArea = subjects[0].boundingBox.size.width * subjects[0].boundingBox.size.height;
                    NSUInteger bestIndex = 0; CGFloat bestOverlap = 0.15;
                    for (NSUInteger candidate = 0; candidate < subjects.count; ++candidate) {
                        VNDetectedObjectObservation *subject = subjects[candidate];
                        CGFloat area = subject.boundingBox.size.width * subject.boundingBox.size.height;
                        CGFloat overlap = XDRectOverlap(previousFocusBox, XDEncodedVisionRect(subject.boundingBox, visionOrientation));
                        if (area >= largestArea * 0.6 && overlap > bestOverlap) { bestIndex = candidate; bestOverlap = overlap; }
                    }
                    if (bestIndex != 0) [subjects exchangeObjectAtIndex:0 withObjectAtIndex:bestIndex];
                }
                if (subjects.count > 2) [subjects removeObjectsInRange:NSMakeRange(2, subjects.count - 2)];

                // Vision rectangles have a bottom-left origin. Cinematic uses
                // top-left normalized coordinates, independent of display rotation.
                CGRect fallbackBox = CGRectMake(0.40, 0.40, 0.20, 0.20);
                if (subjects.count == 0) {
                    // Native capture can focus on non-human objects. Use
                    // Vision's objectness regions rather than inventing a
                    // person at the center of an otherwise empty scene.
                    VNGenerateObjectnessBasedSaliencyImageRequest *saliency = [[VNGenerateObjectnessBasedSaliencyImageRequest alloc] init];
                    NSError *saliencyError = nil;
                    if ([visionHandler performRequests:@[saliency] onCMSampleBuffer:sample orientation:visionOrientation error:&saliencyError]) {
                        VNRectangleObservation *best = nil;
                        for (VNRectangleObservation *object in saliency.results.firstObject.salientObjects) {
                            CGFloat area = object.boundingBox.size.width * object.boundingBox.size.height;
                            if (object.confidence >= 0.3f && area >= 0.005 && area <= 0.6 && (!best || object.confidence > best.confidence)) best = object;
                        }
                        if (best) fallbackBox = XDEncodedVisionRect(best.boundingBox, visionOrientation);
                    }
                }
                CGRect focusBox = subjects.firstObject ? XDEncodedVisionRect(subjects.firstObject.boundingBox, visionOrientation) : fallbackBox;
                previousFocusBox = subjects.firstObject ? focusBox : CGRectNull;
                id focusDetection = nil;
                NSMutableArray *allDetections = [NSMutableArray arrayWithCapacity:subjects.count + 1];
                for (NSUInteger faceIndex = 0; faceIndex < subjects.count; ++faceIndex) {
                    CGRect subjectBox = XDEncodedVisionRect(subjects[faceIndex].boundingBox, visionOrientation);
                    id subjectDetection = ((PTInitDetection)objc_msgSend)([detectionClass alloc],
                        sel_registerName("initWithTime:rect:focusDistance:"), pts, subjectBox, -1.0f);
                    if (!subjectDetection) {
                        CFRelease(sample);
                        [decodeReader cancelReading];
                        [[NSFileManager defaultManager] removeItemAtPath:tempDirectory error:nil];
                        return XDFail(error_buffer, error_buffer_size, 3,
                                      @"cannot construct Apple Cinematography subject detection");
                    }
                    ((void (*)(id, SEL, long long))method_getImplementation(setTrack))(
                        subjectDetection, sel_registerName("setTrackIdentifier:"), subjectTrackIdentifiers[faceIndex]);
                    ((void (*)(id, SEL, long long))method_getImplementation(setGroup))(
                        subjectDetection, sel_registerName("setGroupIdentifier:"), subjectGroupIdentifiers[faceIndex]);
                    ((void (*)(id, SEL, unsigned long long))method_getImplementation(setType))(
                        subjectDetection, sel_registerName("setDetectionType:"), (unsigned long long)subjectType);
                    [allDetections addObject:subjectDetection];
                    if (faceIndex == 0) focusDetection = subjectDetection;
                }
                if (!focusDetection) {
                    // Preserve a serializable synthetic anchor when Vision
                    // finds neither a face nor a torso.
                    id fallbackDetection = ((PTInitDetection)objc_msgSend)([detectionClass alloc],
                        sel_registerName("initWithTime:rect:focusDistance:"), pts, fallbackBox, -1.0f);
                    if (!fallbackDetection) {
                        CFRelease(sample);
                        [decodeReader cancelReading];
                        [[NSFileManager defaultManager] removeItemAtPath:tempDirectory error:nil];
                        return XDFail(error_buffer, error_buffer_size, 3,
                                      @"cannot construct fallback Apple Cinematography detection");
                    }
                    ((void (*)(id, SEL, long long))method_getImplementation(setTrack))(
                        fallbackDetection, sel_registerName("setTrackIdentifier:"), trackIdentifier);
                    ((void (*)(id, SEL, long long))method_getImplementation(setGroup))(
                        fallbackDetection, sel_registerName("setGroupIdentifier:"), groupIdentifier);
                    ((void (*)(id, SEL, unsigned long long))method_getImplementation(setType))(
                        fallbackDetection, sel_registerName("setDetectionType:"), 3ULL);
                    [allDetections addObject:fallbackDetection];
                    focusDetection = fallbackDetection;
                }

                CGFloat focusMarginX = CGRectGetWidth(focusBox) * 0.25;
                CGFloat focusMarginY = CGRectGetHeight(focusBox) * 0.12;
                CGRect expandedFocusBox = CGRectInset(focusBox, -focusMarginX, -focusMarginY);
                CGFloat focusMinX = MAX(0.0, CGRectGetMinX(expandedFocusBox));
                CGFloat focusMinY = MAX(0.0, CGRectGetMinY(expandedFocusBox));
                CGFloat focusMaxX = MIN(1.0, CGRectGetMaxX(expandedFocusBox));
                CGFloat focusMaxY = MIN(1.0, CGRectGetMaxY(expandedFocusBox));
                expandedFocusBox = CGRectMake(focusMinX, focusMinY,
                    MAX(0.001, focusMaxX - focusMinX), MAX(0.001, focusMaxY - focusMinY));
                id autofocusDetection = ((PTInitDetection)objc_msgSend)([detectionClass alloc],
                    sel_registerName("initWithTime:rect:focusDistance:"), pts, expandedFocusBox, -1.0f);
                if (!autofocusDetection) {
                    CFRelease(sample); [decodeReader cancelReading];
                    [[NSFileManager defaultManager] removeItemAtPath:tempDirectory error:nil];
                    return XDFail(error_buffer, error_buffer_size, 3, @"cannot construct Apple autofocus detection");
                }
                ((void (*)(id, SEL, long long))method_getImplementation(setTrack))(
                    autofocusDetection, sel_registerName("setTrackIdentifier:"), autofocusTrackIdentifier);
                ((void (*)(id, SEL, long long))method_getImplementation(setGroup))(
                    autofocusDetection, sel_registerName("setGroupIdentifier:"), -1LL);
                ((void (*)(id, SEL, unsigned long long))method_getImplementation(setType))(
                    autofocusDetection, sel_registerName("setDetectionType:"), 100ULL);
                ((void (*)(id, SEL, id))method_getImplementation(setFocusIdentifier))(
                    autofocusDetection, sel_registerName("setFocusIdentifier:"), @"BA-1");
                [allDetections addObject:autofocusDetection];
                id frame = [[frameClass alloc] init];
                Method setFrameTime = XDRequireMethod(frameClass, "setTime:", NO, "v40@0:8{?=qiIq}16", error_buffer, error_buffer_size);
                Method setDetections = XDRequireMethod(frameClass, "setAllDetections:", NO, "v24@0:8@16", error_buffer, error_buffer_size);
                Method setFocus = XDRequireMethod(frameClass, "setFocusDetection:", NO, "v24@0:8@16", error_buffer, error_buffer_size);
                Method detectorRan = XDRequireMethod(frameClass, "_setDetectorDidRun:", NO, "v24@0:8@16", error_buffer, error_buffer_size);
                if (!frame || !setFrameTime || !setDetections || !setFocus || !detectorRan) {
                    CFRelease(sample); [decodeReader cancelReading];
                    [[NSFileManager defaultManager] removeItemAtPath:tempDirectory error:nil]; return 3;
                }
                ((void (*)(id, SEL, CMTime))method_getImplementation(setFrameTime))(frame, sel_registerName("setTime:"), pts);
                ((void (*)(id, SEL, id))method_getImplementation(setDetections))(
                    frame, sel_registerName("setAllDetections:"), allDetections);
                ((void (*)(id, SEL, id))method_getImplementation(setFocus))(
                    frame, sel_registerName("setFocusDetection:"), focusDetection);
                ((void (*)(id, SEL, id))method_getImplementation(detectorRan))(frame, sel_registerName("_setDetectorDidRun:"), @YES);
                [scriptFrames addObject:frame];
                [frameRows addObject:@{@"pts": @(pts.value), @"timescale": @(pts.timescale),
                                       @"duration": @(duration.value), @"durationScale": @(duration.timescale),
                                       @"raw": rawPath, @"aperture": @(aperture),
                                       @"focalLength": @(frameFocalLength),
                                       @"sceneIlluminanceMilliLux": @(sceneIlluminanceMilliLux)}];
                CFRelease(sample);
                index++;
            }
        }
        if (decodeReader.status != AVAssetReaderStatusCompleted || frameRows.count == 0) {
            [[NSFileManager defaultManager] removeItemAtPath:tempDirectory error:nil];
            return XDFail(error_buffer, error_buffer_size, 65, [NSString stringWithFormat:@"video inference did not complete: %@", decodeReader.error.localizedDescription ?: @"no decoded frames"]);
        }

        Method setBaseDecisions = XDRequireMethod(scriptClass, "setBaseDecisions:", NO, "v24@0:8@16", error_buffer, error_buffer_size);
        Method setTrackDecisions = XDRequireMethod(scriptClass, "setTrackDecisions:", NO, "v24@0:8@16", error_buffer, error_buffer_size);
        Method addBaseDecision = XDRequireMethod(scriptClass, "addBaseDecision:", NO, "v24@0:8@16", error_buffer, error_buffer_size);
        Method decisionInit = XDRequireMethod(decisionClass, "_initWithTime:trackIdentifier:groupIdentifier:options:", NO,
                                              "@64@0:8{?=qiIq}16q40q48Q56", error_buffer, error_buffer_size);
        if (!script || !setFrames || !setBaseDecisions || !setTrackDecisions || !addBaseDecision || !decisionInit) {
            [[NSFileManager defaultManager] removeItemAtPath:tempDirectory error:nil]; return 3;
        }
        ((void (*)(id, SEL, id))method_getImplementation(setFrames))(script, sel_registerName("setFrames:"), scriptFrames);
        ((PTInternalizeFrames)method_getImplementation(internalizeTracks))(
            script, sel_registerName("_internalizeTracksFromFrames:"), scriptFrames);
        if (!((PTBoolGetter)method_getImplementation(didInternalizeTracks))(
                script, sel_registerName("didInternalizeTracks"))) {
            [[NSFileManager defaultManager] removeItemAtPath:tempDirectory error:nil];
            return XDFail(error_buffer, error_buffer_size, 3,
                          @"Apple Cinematography script did not internalize its frame tracks");
        }
        ((void (*)(id, SEL, id))method_getImplementation(setBaseDecisions))(script, sel_registerName("setBaseDecisions:"), @[]);
        ((void (*)(id, SEL, id))method_getImplementation(setTrackDecisions))(script, sel_registerName("setTrackDecisions:"), @[]);
        NSDictionary *firstRow = frameRows.firstObject;
        CMTime decisionTime = CMTimeMake([firstRow[@"pts"] longLongValue], [firstRow[@"timescale"] intValue]);
        id decision = ((PTInitDecision)method_getImplementation(decisionInit))([decisionClass alloc],
                          sel_registerName("_initWithTime:trackIdentifier:groupIdentifier:options:"),
                          decisionTime, trackIdentifier, groupIdentifier, 0ULL);
        if (!decision) {
            [[NSFileManager defaultManager] removeItemAtPath:tempDirectory error:nil];
            return XDFail(error_buffer, error_buffer_size, 3, @"cannot construct the initial Apple focus decision");
        }
        ((void (*)(id, SEL, id))method_getImplementation(addBaseDecision))(script, sel_registerName("addBaseDecision:"), decision);
        Method calculate = XDRequireMethod(scriptClass, "_calculateTrackDecisions", NO, "@16@0:8", error_buffer, error_buffer_size);
        id calculated = calculate ? ((PTObjectGetter)method_getImplementation(calculate))(script, sel_registerName("_calculateTrackDecisions")) : nil;
        if ([calculated isKindOfClass:NSArray.class] && [calculated count] > 0) {
            ((void (*)(id, SEL, id))method_getImplementation(setTrackDecisions))(script, sel_registerName("setTrackDecisions:"), calculated);
        } else {
            ((void (*)(id, SEL, id))method_getImplementation(setTrackDecisions))(script, sel_registerName("setTrackDecisions:"), @[decision]);
        }
        Method refinementInit = XDRequireMethod(refinementClass, "initWithScript:samplesAllDetections:", NO,
                                               "@28@0:8@16B24", error_buffer, error_buffer_size);
        Method processDisparity = XDRequireMethod(refinementClass, "processNextDisparityBuffer:", NO,
                                                  "{?=qiIq}24@0:8^{__CVBuffer=}16", error_buffer, error_buffer_size);
        Method endInputs = XDRequireMethod(refinementClass, "endInputs", NO, "v16@0:8", error_buffer, error_buffer_size);
        Method hasNext = XDRequireMethod(refinementClass, "isNextFrameAvailable", NO, "B16@0:8", error_buffer, error_buffer_size);
        Method nextFrame = XDRequireMethod(refinementClass, "nextFrame", NO, "@16@0:8", error_buffer, error_buffer_size);
        Method frameTimeGetter = XDRequireMethod(frameClass, "time", NO, "{?=qiIq}16@0:8", error_buffer, error_buffer_size);
        Method focusGetter = XDRequireMethod(frameClass, "focusDistance", NO, "f16@0:8", error_buffer, error_buffer_size);
        Method setCaptureFocus = XDRequireMethod(frameClass, "setFocusDistance:", NO, "v20@0:8f16", error_buffer, error_buffer_size);
        Method setCaptureRawFocus = XDRequireMethod(frameClass, "setRawFocusDistance:", NO, "v20@0:8f16", error_buffer, error_buffer_size);
        Method setCaptureAperture = XDRequireMethod(frameClass, "setAperture:", NO, "v20@0:8f16", error_buffer, error_buffer_size);
        Method setFocusTrackNumber = XDRequireMethod(frameClass, "setFocusTrackNumber:", NO, "v24@0:8@16", error_buffer, error_buffer_size);
        Method setBaseFocusTrackNumber = XDRequireMethod(frameClass, "setBaseFocusTrackNumber:", NO, "v24@0:8@16", error_buffer, error_buffer_size);
        if (!refinementInit || !processDisparity || !endInputs || !hasNext || !nextFrame || !frameTimeGetter || !focusGetter ||
            !setCaptureFocus || !setCaptureRawFocus || !setCaptureAperture || !setFocusTrackNumber || !setBaseFocusTrackNumber) {
            [[NSFileManager defaultManager] removeItemAtPath:tempDirectory error:nil]; return 3;
        }
        id refinement = ((PTInitRefinement)method_getImplementation(refinementInit))([refinementClass alloc],
                            sel_registerName("initWithScript:samplesAllDetections:"), script, YES);
        if (!refinement) {
            [[NSFileManager defaultManager] removeItemAtPath:tempDirectory error:nil];
            return XDFail(error_buffer, error_buffer_size, 3, @"Apple postcapture refiner initialization failed");
        }
        for (NSDictionary *row in frameRows) {
            CVPixelBufferRef buffer = XDReadDisparity(row[@"raw"], disparitySize.width, disparitySize.height,
                                                      disparityFormat, error_buffer, error_buffer_size);
            if (!buffer) {
                [[NSFileManager defaultManager] removeItemAtPath:tempDirectory error:nil]; return 4;
            }
            @try {
                ((PTProcessDisparity)method_getImplementation(processDisparity))(refinement, sel_registerName("processNextDisparityBuffer:"), buffer);
            } @catch (NSException *exception) {
                CVPixelBufferRelease(buffer);
                [[NSFileManager defaultManager] removeItemAtPath:tempDirectory error:nil];
                return XDFail(error_buffer, error_buffer_size, 3, [NSString stringWithFormat:@"Apple refiner exception: %@", exception.reason ?: exception.name]);
            }
            CVPixelBufferRelease(buffer);
        }
        ((void (*)(id, SEL))method_getImplementation(endInputs))(refinement, sel_registerName("endInputs"));
        NSMutableArray<NSData *> *cinematographyPayloads = [NSMutableArray arrayWithCapacity:frameRows.count];
        NSMutableArray<NSData *> *renderingPayloads = [NSMutableArray arrayWithCapacity:frameRows.count];
        for (NSUInteger frameIndex = 0; frameIndex < frameRows.count; ++frameIndex) {
            if (!((PTBoolGetter)method_getImplementation(hasNext))(refinement, sel_registerName("isNextFrameAvailable"))) break;
            id frame = ((PTObjectGetter)method_getImplementation(nextFrame))(refinement, sel_registerName("nextFrame"));
            if (!frame) break;
            CMTime time = ((PTTimeGetter)method_getImplementation(frameTimeGetter))(frame, sel_registerName("time"));
            NSDictionary *row = frameRows[frameIndex];
            CMTime expected = CMTimeMake([row[@"pts"] longLongValue], [row[@"timescale"] intValue]);
            if (CMTimeCompare(time, expected) != 0) {
                [[NSFileManager defaultManager] removeItemAtPath:tempDirectory error:nil];
                return XDFail(error_buffer, error_buffer_size, 3, [NSString stringWithFormat:@"refiner PTS mismatch at frame %lu", (unsigned long)frameIndex]);
            }
            float focus = ((PTFloatGetter)method_getImplementation(focusGetter))(frame, sel_registerName("focusDistance"));
            // The refiner returns a focus track but does not copy the source
            // script's base-focus selection into its output frame header.
            // Photos needs that field to resolve an autofocus detection.
            NSNumber *selectedTrack = @(trackIdentifier);
            ((void (*)(id, SEL, id))method_getImplementation(setFocusTrackNumber))(
                frame, sel_registerName("setFocusTrackNumber:"), selectedTrack);
            ((void (*)(id, SEL, id))method_getImplementation(setBaseFocusTrackNumber))(
                frame, sel_registerName("setBaseFocusTrackNumber:"), selectedTrack);
            // This MOV is still a capture-time .needsPreprocessing asset. Its
            // image-derived focus estimate is not an observed capture focus;
            // match genuine capture metadata and let Photos resolve focus from
            // the materialized disparity resource.
            ((void (*)(id, SEL, float))method_getImplementation(setCaptureFocus))(
                frame, sel_registerName("setFocusDistance:"), -1.0f);
            ((void (*)(id, SEL, float))method_getImplementation(setCaptureRawFocus))(
                frame, sel_registerName("setRawFocusDistance:"), -1.0f);
            ((PTSetFloat)method_getImplementation(setCaptureAperture))(
                frame, sel_registerName("setAperture:"), aperture);
            NSData *frameData = XDSerializeCinematographyFrame(frame, error_buffer, error_buffer_size);
            if (frameData) {
                frameData = XDResetPreprocessingDetectionFocusDistances(frameData, error_buffer, error_buffer_size);
            }
            NSData *renderingData = XDSerializeTimedRendering(-1.0f, aperture,
                                                              [row[@"focalLength"] floatValue], inputSize, (uint32_t)frameIndex,
                                                              error_buffer, error_buffer_size);
            if (!frameData || !renderingData) {
                [[NSFileManager defaultManager] removeItemAtPath:tempDirectory error:nil]; return 3;
            }
            [cinematographyPayloads addObject:frameData];
            [renderingPayloads addObject:renderingData];
            NSMutableDictionary *updatedRow = [row mutableCopy];
            updatedRow[@"focus"] = @(focus);
            frameRows[frameIndex] = updatedRow;
        }
        if (cinematographyPayloads.count != frameRows.count || renderingPayloads.count != frameRows.count) {
            [[NSFileManager defaultManager] removeItemAtPath:tempDirectory error:nil];
            return XDFail(error_buffer, error_buffer_size, 3, @"Apple postcapture refiner returned an incomplete frame sequence");
        }
        NSData *globalCinematicData = XDGlobalCinematicVideoData(error_buffer, error_buffer_size);
        if (!globalCinematicData) {
            [[NSFileManager defaultManager] removeItemAtPath:tempDirectory error:nil]; return 3;
        }
        BOOL written = XDWriteMediaAndMetadata(asset, videoTrack, frameRows, cinematographyPayloads,
                                               renderingPayloads, globalCinematicData,
                                               [NSString stringWithUTF8String:output_path], error_buffer, error_buffer_size);
        [[NSFileManager defaultManager] removeItemAtPath:tempDirectory error:nil];
        if (!written || !XDValidatePendingAsset([NSString stringWithUTF8String:output_path], error_buffer, error_buffer_size)) {
            [[NSFileManager defaultManager] removeItemAtPath:[NSString stringWithUTF8String:output_path] error:nil];
            return 5;
        }
        if (processed_frames) *processed_frames = frameRows.count;
        return 0;
      }
    } @catch (NSException *exception) {
        return XDFail(error_buffer, error_buffer_size, 3,
                      [NSString stringWithFormat:@"Apple metadata framework exception %@: %@",
                       exception.name, exception.reason ?: @"unknown reason"]);
    }
}
