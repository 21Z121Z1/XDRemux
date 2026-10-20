#pragma once
#include <math.h>
#import <Foundation/Foundation.h>
#import <CoreImage/CoreImage.h>
#import <CoreVideo/CoreVideo.h>
#import <ImageIO/ImageIO.h>
#import <Vision/Vision.h>
#import <CommonCrypto/CommonDigest.h>
#import <objc/message.h>
#import <objc/runtime.h>
#import <dlfcn.h>

static NSString *SHA(NSData *data) {
    unsigned char digest[CC_SHA256_DIGEST_LENGTH];
    CC_SHA256(data.bytes, (CC_LONG)data.length, digest);
    NSMutableString *result = [NSMutableString new];
    for (unsigned i = 0; i < sizeof(digest); i++) [result appendFormat:@"%02x", digest[i]];
    return result;
}

static id JSONCompatible(id value) {
    if (!value || value == NSNull.null) return NSNull.null;
    if ([value isKindOfClass:NSString.class]) return value;
    if ([value isKindOfClass:NSNumber.class]) return isfinite([value doubleValue]) ? value : @{@"nonfinite_number": [value description]};
    if ([value isKindOfClass:NSDictionary.class]) {
        NSMutableDictionary *result = [NSMutableDictionary new];
        for (id key in value) result[[key description]] = JSONCompatible(value[key]);
        return result;
    }
    if ([value isKindOfClass:NSArray.class]) {
        NSMutableArray *result = [NSMutableArray new];
        for (id item in value) [result addObject:JSONCompatible(item)];
        return result;
    }
    if ([value isKindOfClass:NSData.class]) return @{@"bytes": @([value length]), @"sha256": SHA(value)};
    return [value description] ?: @"";
}

static void Write(NSDictionary *report, NSString *path) {
    NSError *error = nil;
    NSData *data = [NSJSONSerialization dataWithJSONObject:JSONCompatible(report) options:NSJSONWritingPrettyPrinted | NSJSONWritingSortedKeys error:&error];
    if (!data || ![data writeToFile:path options:NSDataWritingAtomic error:&error]) {
        fprintf(stderr, "JSON_ERROR %s\n", error.description.UTF8String); exit(1);
    }
}

static void Require(id target, NSString *name, NSArray<NSString *> *arguments, const char *result) {
    NSMethodSignature *signature = [target methodSignatureForSelector:NSSelectorFromString(name)];
    if (!signature || signature.numberOfArguments != arguments.count + 2 || strcmp(signature.methodReturnType, result))
        @throw [NSException exceptionWithName:@"SPIABI" reason:name userInfo:nil];
    for (NSUInteger i = 0; i < arguments.count; i++) {
        const char *actual = [signature getArgumentTypeAtIndex:i + 2];
        while (strchr("rnNoORV", *actual) && *actual) actual++;
        if (strcmp(actual, arguments[i].UTF8String))
            @throw [NSException exceptionWithName:@"SPIABI" reason:[name stringByAppendingFormat:@" argument %lu: %s", (unsigned long)i, [signature getArgumentTypeAtIndex:i + 2]] userInfo:nil];
    }
}

static NSDictionary *BufferFacts(CVPixelBufferRef buffer) {
    if (!buffer) return @{@"available": @NO};
    CVReturn status = CVPixelBufferLockBaseAddress(buffer, kCVPixelBufferLock_ReadOnly);
    if (status != kCVReturnSuccess) return @{@"available": @YES, @"lock_status": @(status)};
    size_t width = CVPixelBufferGetWidth(buffer), height = CVPixelBufferGetHeight(buffer);
    size_t rowBytes = CVPixelBufferGetBytesPerRow(buffer);
    const uint8_t *bytes = CVPixelBufferGetBaseAddress(buffer);
    // The raw hash includes row padding; its scope is explicit.
    NSString *digest = bytes ? SHA([NSData dataWithBytes:bytes length:rowBytes * height]) : @"";
    NSMutableDictionary *result = [@{
        @"available": @YES, @"width": @(width), @"height": @(height), @"bytes_per_row": @(rowBytes),
        @"pixel_format": @(CVPixelBufferGetPixelFormatType(buffer)), @"raw_with_padding_sha256": digest,
        @"planar": @(CVPixelBufferIsPlanar(buffer)),
    } mutableCopy];
    OSType format = CVPixelBufferGetPixelFormatType(buffer);
    if (bytes && !CVPixelBufferIsPlanar(buffer) && (format == kCVPixelFormatType_OneComponent8 || format == kCVPixelFormatType_OneComponent32Float)) {
        double mass = 0, sx = 0, sy = 0, maximum = 0;
        uint64_t nonzero = 0, nonfinite = 0;
        for (size_t y = 0; y < height; y++) for (size_t x = 0; x < width; x++) {
            double v = format == kCVPixelFormatType_OneComponent8 ? bytes[y * rowBytes + x] / 255.0 : ((const float *)(bytes + y * rowBytes))[x];
            if (!isfinite(v)) { nonfinite++; continue; }
            mass += v; sx += v * (x + 0.5); sy += v * (y + 0.5);
            maximum = MAX(maximum, v); nonzero += v > 0.001;
        }
        result[@"mask_values"] = @{@"mean": @(mass / (width * height)), @"maximum": @(maximum),
            @"nonzero_over_0_001": @(nonzero), @"nonfinite": @(nonfinite),
            @"centroid_row_order": mass > 0 ? @[@(sx / mass), @(sy / mass)] : @[]};
    }
    CVPixelBufferUnlockBaseAddress(buffer, kCVPixelBufferLock_ReadOnly);
    return result;
}

static NSArray *FaceFacts(NSArray<VNFaceObservation *> *observations) {
    NSMutableArray *result = [NSMutableArray new];
    for (VNFaceObservation *face in observations) {
        CGRect box = face.boundingBox;
        VNFaceLandmarkRegion2D *landmarks = face.landmarks.allPoints;
        NSMutableArray *points = [NSMutableArray new];
        for (NSUInteger i = 0; i < landmarks.pointCount; i++) {
            CGPoint point = landmarks.normalizedPoints[i];
            [points addObject:@[@(point.x), @(point.y)]];
        }
        [result addObject:@{
            @"bounding_box": @[@(box.origin.x), @(box.origin.y), @(box.size.width), @(box.size.height)],
            @"box_coordinates": @"Vision normalized image coordinates, lower-left origin",
            @"yaw_radians": face.yaw ?: NSNull.null, @"pitch_radians": face.pitch ?: NSNull.null,
            @"roll_radians": face.roll ?: NSNull.null, @"confidence": @(face.confidence),
            @"landmarks_in_face_coordinates": points,
        }];
    }
    return result;
}
