#import <Foundation/Foundation.h>
#import <objc/runtime.h>
#import <objc/message.h>
#import <dlfcn.h>

static IMP gOriginalSetInputPersonData = NULL;
static IMP gOriginalSetInputSkinSmoothingFaceDetections = NULL;
static IMP gOriginalPersonInputDataFromStillProperties = NULL;
static IMP gOriginalPersonInputDataArrayFromDetectedFaces = NULL;
static NSLock *gWriteLock = nil;

static id DescribeObject(id value, NSUInteger depth) {
    if (value == nil) return [NSNull null];
    if (depth > 4) return @{ @"class": NSStringFromClass([value class]) ?: @"?", @"description": [value description] ?: @"" };
    if ([value isKindOfClass:[NSString class]] || [value isKindOfClass:[NSNumber class]]) return value;
    if ([value isKindOfClass:[NSArray class]]) {
        NSArray *array = value;
        NSMutableArray *items = [NSMutableArray array];
        NSUInteger limit = MIN(array.count, 8);
        for (NSUInteger i = 0; i < limit; ++i) [items addObject:DescribeObject(array[i], depth + 1)];
        return @{ @"class": NSStringFromClass([value class]) ?: @"NSArray",
                  @"count": @(array.count), @"items": items,
                  @"truncated": @(array.count > limit) };
    }
    if ([value isKindOfClass:[NSDictionary class]]) {
        NSDictionary *dict = value;
        NSMutableDictionary *items = [NSMutableDictionary dictionary];
        NSArray *keys = [[dict allKeys] sortedArrayUsingComparator:^NSComparisonResult(id a, id b) {
            return [[a description] compare:[b description]];
        }];
        NSUInteger limit = MIN(keys.count, 32);
        for (NSUInteger i = 0; i < limit; ++i) {
            id key = keys[i];
            items[[key description]] = DescribeObject(dict[key], depth + 1);
        }
        return @{ @"class": NSStringFromClass([value class]) ?: @"NSDictionary",
                  @"count": @(dict.count), @"items": items,
                  @"truncated": @(dict.count > limit) };
    }

    NSMutableDictionary *summary = [NSMutableDictionary dictionary];
    summary[@"class"] = NSStringFromClass([value class]) ?: @"?";
    NSArray<NSString *> *personKeys = @[
        @"faceID", @"faceROI", @"faceSkinROI", @"faceROIAndLandmarksROIRelativeScalingROI",
        @"faceLandmarkType", @"faceLandmarks", @"faceYaw", @"facePitch", @"faceRoll",
        @"faceUnitOfAngle", @"instanceROI", @"instanceMaskReferenceKey"
    ];
    for (NSString *key in personKeys) {
        @try {
            id field = [value valueForKey:key];
            if (field != nil) summary[key] = DescribeObject(field, depth + 1);
        } @catch (__unused NSException *exception) {
        }
    }
    if (summary.count == 1) summary[@"description"] = [value description] ?: @"";
    return summary;
}

static void WriteEvent(NSString *eventName, id receiver, id payload, id result) {
    if (gWriteLock == nil) gWriteLock = [[NSLock alloc] init];
    NSMutableDictionary *event = [NSMutableDictionary dictionary];
    event[@"event"] = eventName;
    event[@"pid"] = @([[NSProcessInfo processInfo] processIdentifier]);
    event[@"process"] = [NSProcessInfo processInfo].processName ?: @"?";
    event[@"timestamp"] = @([[NSDate date] timeIntervalSince1970]);
    event[@"receiverClass"] = receiver ? (NSStringFromClass([receiver class]) ?: @"?") : @"<nil>";
    if (payload != nil) event[@"payload"] = DescribeObject(payload, 0);
    if (result != nil) event[@"result"] = DescribeObject(result, 0);

    NSError *error = nil;
    NSData *data = [NSJSONSerialization dataWithJSONObject:event options:0 error:&error];
    if (data == nil) return;
    NSMutableData *line = [data mutableCopy];
    [line appendBytes:"\n" length:1];

    const char *path = getenv("XDREMUX_TEXTURE_TRACE_FILE");
    [gWriteLock lock];
    if (path != NULL && path[0] != '\0') {
        NSString *filePath = [NSString stringWithUTF8String:path];
        if (![[NSFileManager defaultManager] fileExistsAtPath:filePath]) {
            [[NSFileManager defaultManager] createFileAtPath:filePath contents:nil attributes:nil];
        }
        NSFileHandle *handle = [NSFileHandle fileHandleForWritingAtPath:filePath];
        [handle seekToEndOfFile];
        [handle writeData:line];
        [handle closeFile];
    } else {
        fwrite(line.bytes, 1, line.length, stderr);
        fflush(stderr);
    }
    [gWriteLock unlock];
}

static void TracedSetInputPersonData(id self, SEL _cmd, id payload) {
    WriteEvent(@"CMITextureStylesProcessor.setInputPersonData", self, payload, nil);
    ((void (*)(id, SEL, id))gOriginalSetInputPersonData)(self, _cmd, payload);
}

static void TracedSetInputSkinSmoothingFaceDetections(id self, SEL _cmd, id payload) {
    WriteEvent(@"CMITextureStylesProcessor.setInputSkinSmoothingFaceDetections", self, payload, nil);
    ((void (*)(id, SEL, id))gOriginalSetInputSkinSmoothingFaceDetections)(self, _cmd, payload);
}

static id TracedPersonInputDataFromStillProperties(id self, SEL _cmd, id properties) {
    WriteEvent(@"PITextureStyleProcessorKernel.personInputDataFromStillProperties.input", self, properties, nil);
    id result = ((id (*)(id, SEL, id))gOriginalPersonInputDataFromStillProperties)(self, _cmd, properties);
    WriteEvent(@"PITextureStyleProcessorKernel.personInputDataFromStillProperties.output", self, nil, result);
    return result;
}

static id TracedPersonInputDataArrayFromDetectedFaces(id self, SEL _cmd, id faces) {
    WriteEvent(@"CMITextureStylesPersonInputDataUtilities.personInputDataArrayFromDetectedFaces.input", self, faces, nil);
    id result = ((id (*)(id, SEL, id))gOriginalPersonInputDataArrayFromDetectedFaces)(self, _cmd, faces);
    WriteEvent(@"CMITextureStylesPersonInputDataUtilities.personInputDataArrayFromDetectedFaces.output", self, nil, result);
    return result;
}

static BOOL ReplaceInstanceMethod(Class cls, SEL selector, IMP replacement, IMP *originalOut) {
    Method method = cls ? class_getInstanceMethod(cls, selector) : NULL;
    if (method == NULL) return NO;
    *originalOut = method_getImplementation(method);
    method_setImplementation(method, replacement);
    fprintf(stderr, "[texture-trace] hooked -[%s %s]\n", class_getName(cls), sel_getName(selector));
    return YES;
}

static BOOL ReplaceClassMethod(Class cls, SEL selector, IMP replacement, IMP *originalOut) {
    Method method = cls ? class_getClassMethod(cls, selector) : NULL;
    if (method == NULL) return NO;
    *originalOut = method_getImplementation(method);
    method_setImplementation(method, replacement);
    fprintf(stderr, "[texture-trace] hooked +[%s %s]\n", class_getName(cls), sel_getName(selector));
    return YES;
}

__attribute__((constructor))
static void XDRemuxTextureTraceInstall(void) {
    @autoreleasepool {
        const char *enabled = getenv("XDREMUX_TEXTURE_TRACE");
        if (enabled == NULL || strcmp(enabled, "1") != 0) return;

        const char *cmImagingPaths[] = {
            "/System/Library/PrivateFrameworks/CMImaging.framework/Versions/A/CMImaging",
            "/System/Library/PrivateFrameworks/CMImaging.framework/CMImaging",
            NULL
        };
        const char *photoImagingPaths[] = {
            "/System/Library/PrivateFrameworks/PhotoImaging.framework/Versions/A/PhotoImaging",
            "/System/Library/PrivateFrameworks/PhotoImaging.framework/PhotoImaging",
            NULL
        };
        for (size_t i = 0; cmImagingPaths[i] != NULL; ++i) {
            if (dlopen(cmImagingPaths[i], RTLD_NOW | RTLD_GLOBAL) != NULL) break;
        }
        for (size_t i = 0; photoImagingPaths[i] != NULL; ++i) {
            if (dlopen(photoImagingPaths[i], RTLD_NOW | RTLD_GLOBAL) != NULL) break;
        }

        Class processor = NSClassFromString(@"CMITextureStylesProcessor");
        ReplaceInstanceMethod(processor, NSSelectorFromString(@"setInputPersonData:"),
                              (IMP)TracedSetInputPersonData, &gOriginalSetInputPersonData);
        ReplaceInstanceMethod(processor, NSSelectorFromString(@"setInputSkinSmoothingFaceDetections:"),
                              (IMP)TracedSetInputSkinSmoothingFaceDetections,
                              &gOriginalSetInputSkinSmoothingFaceDetections);

        Class kernel = NSClassFromString(@"PITextureStyleProcessorKernel");
        ReplaceInstanceMethod(kernel, NSSelectorFromString(@"personInputDataFromStillProperties:"),
                              (IMP)TracedPersonInputDataFromStillProperties,
                              &gOriginalPersonInputDataFromStillProperties);

        Class utilities = NSClassFromString(@"CMITextureStylesPersonInputDataUtilities");
        ReplaceClassMethod(utilities, NSSelectorFromString(@"personInputDataArrayFromDetectedFaces:"),
                           (IMP)TracedPersonInputDataArrayFromDetectedFaces,
                           &gOriginalPersonInputDataArrayFromDetectedFaces);

        WriteEvent(@"trace-installed", nil,
                   @{ @"processor": @(processor != Nil),
                      @"photoKernel": @(kernel != Nil),
                      @"utilities": @(utilities != Nil) }, nil);
    }
}
