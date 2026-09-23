#import <Foundation/Foundation.h>
#import <CoreGraphics/CoreGraphics.h>
#import <objc/runtime.h>
#import <objc/message.h>
#import <dlfcn.h>
#import <math.h>

static void *gCMImaging = NULL;
static void *gCMCapture = NULL;
static NSMutableDictionary *gResolvedKeys = nil;

static void *OpenFramework(const char *const *paths) {
    for (size_t i = 0; paths[i] != NULL; ++i) {
        void *handle = dlopen(paths[i], RTLD_NOW | RTLD_GLOBAL);
        if (handle != NULL) {
            fprintf(stderr, "[probe] dlopen %s\n", paths[i]);
            return handle;
        }
    }
    return NULL;
}

static NSString *ResolveStringConstant(const char *symbol, NSString *fallback) {
    void *handles[] = {RTLD_DEFAULT, gCMCapture, gCMImaging};
    char underscored[256];
    snprintf(underscored, sizeof(underscored), "_%s", symbol);
    const char *names[] = {symbol, underscored};

    for (size_t h = 0; h < sizeof(handles) / sizeof(handles[0]); ++h) {
        if (handles[h] == NULL) continue;
        for (size_t n = 0; n < sizeof(names) / sizeof(names[0]); ++n) {
            void *slot = dlsym(handles[h], names[n]);
            if (slot == NULL) continue;
            @try {
                id value = *(__unsafe_unretained id *)slot;
                if ([value isKindOfClass:[NSString class]]) {
                    fprintf(stderr, "[probe] resolved %s => %s\n", symbol, [(NSString *)value UTF8String]);
                    gResolvedKeys[[NSString stringWithUTF8String:symbol]] = @{ @"value": value, @"resolved": @YES };
                    return value;
                }
            } @catch (__unused NSException *exception) {
            }
        }
    }
    fprintf(stderr, "[probe] unresolved %s; fallback=%s\n", symbol, fallback.UTF8String);
    gResolvedKeys[[NSString stringWithUTF8String:symbol]] = @{ @"value": fallback, @"resolved": @NO };
    return fallback;
}

static id JSONSafe(id value, NSUInteger depth) {
    if (value == nil || value == [NSNull null]) return [NSNull null];
    if (depth > 5) return [value description] ?: @"<description unavailable>";
    if ([value isKindOfClass:[NSString class]] ||
        [value isKindOfClass:[NSNumber class]]) return value;
    if ([value isKindOfClass:[NSArray class]]) {
        NSMutableArray *out = [NSMutableArray array];
        for (id item in (NSArray *)value) [out addObject:JSONSafe(item, depth + 1) ?: [NSNull null]];
        return out;
    }
    if ([value isKindOfClass:[NSDictionary class]]) {
        NSMutableDictionary *out = [NSMutableDictionary dictionary];
        [(NSDictionary *)value enumerateKeysAndObjectsUsingBlock:^(id key, id obj, BOOL *stop) {
            (void)stop;
            out[[key description]] = JSONSafe(obj, depth + 1) ?: [NSNull null];
        }];
        return out;
    }
    if ([value isKindOfClass:[NSValue class]]) {
        const char *type = [(NSValue *)value objCType];
        if (strcmp(type, @encode(CGRect)) == 0) {
            CGRect rect = CGRectZero;
            [(NSValue *)value getValue:&rect size:sizeof(rect)];
            return @{ @"type": @"CGRect",
                      @"x": @(rect.origin.x), @"y": @(rect.origin.y),
                      @"width": @(rect.size.width), @"height": @(rect.size.height) };
        }
        if (strcmp(type, @encode(CGPoint)) == 0) {
            CGPoint point = CGPointZero;
            [(NSValue *)value getValue:&point size:sizeof(point)];
            return @{ @"type": @"CGPoint", @"x": @(point.x), @"y": @(point.y) };
        }
        return @{ @"type": [NSString stringWithUTF8String:type ?: "?"],
                  @"description": [value description] ?: @"" };
    }
    return @{ @"class": NSStringFromClass([value class]) ?: @"?",
              @"description": [value description] ?: @"" };
}

static NSString *MethodEncoding(Class cls, SEL selector, BOOL classMethod) {
    Method method = classMethod ? class_getClassMethod(cls, selector) : class_getInstanceMethod(cls, selector);
    const char *encoding = method ? method_getTypeEncoding(method) : NULL;
    return encoding ? [NSString stringWithUTF8String:encoding] : @"<missing>";
}

// Private selectors are hypotheses. Never invoke a newly changed ABI with
// storage sized for the old one. NSInvocation includes self and _cmd.
static const char *Unqualified(const char *type) {
    while (*type && strchr("rnNoORV", *type)) ++type;
    return type;
}

static NSMethodSignature *CheckedSignature(Class cls, SEL selector, BOOL rectArgument) {
    NSMethodSignature *signature = [(id)cls methodSignatureForSelector:selector];
    const NSUInteger count = rectArgument ? 4 : 3;
    BOOL valid = signature && signature.numberOfArguments == count;
    if (valid) valid = strcmp(Unqualified([signature getArgumentTypeAtIndex:2]), "@") == 0;
    if (valid && rectArgument) {
        valid = strcmp(Unqualified([signature getArgumentTypeAtIndex:3]), @encode(CGRect)) == 0;
    }
    if (valid) {
        const char *type = Unqualified(signature.methodReturnType);
        valid = strcmp(type, "@") == 0 || (rectArgument && strcmp(type, "v") == 0);
    }
    if (!valid) {
        [NSException raise:NSInvalidArgumentException
                    format:@"Unsupported ABI for %@", NSStringFromSelector(selector)];
    }
    return signature;
}

static id InvokeClassWithOneObject(Class cls, SEL selector, id arg) {
    NSInvocation *invocation = [NSInvocation invocationWithMethodSignature:CheckedSignature(cls, selector, NO)];
    invocation.target = cls;
    invocation.selector = selector;
    __unsafe_unretained id local = arg;
    [invocation setArgument:&local atIndex:2];
    [invocation invoke];
    __unsafe_unretained id result = nil;
    [invocation getReturnValue:&result];
    return result;
}

static id InvokeClassWithObjectAndRect(Class cls, SEL selector, id arg, CGRect rect, BOOL *returnedObject) {
    NSMethodSignature *signature = CheckedSignature(cls, selector, YES);
    NSInvocation *invocation = [NSInvocation invocationWithMethodSignature:signature];
    invocation.target = cls;
    invocation.selector = selector;
    __unsafe_unretained id local = arg;
    [invocation setArgument:&local atIndex:2];
    [invocation setArgument:&rect atIndex:3];
    [invocation invoke];
    *returnedObject = strcmp(Unqualified(signature.methodReturnType), "@") == 0;
    if (!*returnedObject) return nil;
    __unsafe_unretained id result = nil;
    [invocation getReturnValue:&result];
    return result;
}

@interface SignatureTest : NSObject
+ (id)echo:(id)value;
+ (id)normalize:(id)value crop:(CGRect)rect;
+ (id)bad:(double)value;
+ (double)badReturn:(id)value;
+ (id)badRect:(id)value crop:(double)rect;
@end
@implementation SignatureTest
+ (id)echo:(id)value { return value; }
+ (id)normalize:(id)value crop:(CGRect)rect { (void)rect; return value; }
+ (id)bad:(double)value { (void)value; abort(); }
+ (double)badReturn:(id)value { (void)value; abort(); }
+ (id)badRect:(id)value crop:(double)rect { (void)value; (void)rect; abort(); }
@end

static int SignatureSelfTest(void) {
    Class cls = [SignatureTest class];
    id value = @[@17];
    if (InvokeClassWithOneObject(cls, @selector(echo:), value) != value) return 1;
    BOOL object = NO;
    if (InvokeClassWithObjectAndRect(cls, @selector(normalize:crop:), value,
                                    CGRectMake(0, 0, 1, 1), &object) != value || !object) return 2;
    NSArray *wrong = @[@"bad:", @"badReturn:", @"missing:", @"normalize:crop:", @"badRect:crop:"];
    NSUInteger rejected = 0;
    for (NSString *name in wrong) {
        @try {
            CheckedSignature(cls, NSSelectorFromString(name), [name isEqualToString:@"badRect:crop:"]);
        } @catch (NSException *exception) {
            if ([exception.name isEqualToString:NSInvalidArgumentException]) ++rejected;
        }
    }
    if (rejected != wrong.count) return 3;
    puts("{\"schema\":1,\"abiSelfTest\":true,\"privateSelectorsInvoked\":false}");
    return 0;
}

static id ReadKVC(id object, NSString *key) {
    @try {
        return [object valueForKey:key];
    } @catch (__unused NSException *exception) {
        return nil;
    }
}

static NSDictionary *SummarizePerson(id person) {
    NSArray<NSString *> *keys = @[
        @"faceID", @"faceROI", @"faceSkinROI", @"faceROIAndLandmarksROIRelativeScalingROI",
        @"faceLandmarkType", @"faceLandmarks", @"faceYaw", @"facePitch", @"faceRoll",
        @"faceUnitOfAngle", @"instanceROI", @"instanceMaskReferenceKey"
    ];
    NSMutableDictionary *out = [NSMutableDictionary dictionary];
    out[@"class"] = NSStringFromClass([person class]) ?: @"?";
    for (NSString *key in keys) {
        id value = ReadKVC(person, key);
        if (value != nil) out[key] = JSONSafe(value, 0);
    }
    SEL dictionarySelector = NSSelectorFromString(@"dictionaryRepresentation");
    if ([person respondsToSelector:dictionarySelector]) {
        id (*send)(id, SEL) = (void *)objc_msgSend;
        @try {
            id dictionary = send(person, dictionarySelector);
            if (dictionary != nil) out[@"dictionaryRepresentation"] = JSONSafe(dictionary, 0);
        } @catch (NSException *exception) {
            out[@"dictionaryRepresentationException"] = exception.reason ?: exception.name;
        }
    }
    return out;
}

static NSArray *SummarizePeople(id people) {
    if (![people isKindOfClass:[NSArray class]]) return @[];
    NSMutableArray *out = [NSMutableArray array];
    for (id person in (NSArray *)people) [out addObject:SummarizePerson(person)];
    return out;
}

static NSArray *SyntheticLandmarks(BOOL dictionaryPoints) {
    NSMutableArray *landmarks = [NSMutableArray arrayWithCapacity:76];
    const CGFloat cx = 0.40, cy = 0.45, rx = 0.14, ry = 0.17;
    for (NSUInteger i = 0; i < 76; ++i) {
        CGFloat t = ((CGFloat)i / 76.0) * (CGFloat)(M_PI * 2.0);
        CGPoint point = CGPointMake(cx + cos(t) * rx, cy + sin(t) * ry);
        if (dictionaryPoints) {
            NSDictionary *pointDict = CFBridgingRelease(CGPointCreateDictionaryRepresentation(point));
            [landmarks addObject:@{ @"point": pointDict, @"error": @0.0 }];
        } else {
            NSValue *boxed = [NSValue value:&point withObjCType:@encode(CGPoint)];
            [landmarks addObject:boxed];
        }
    }
    return landmarks;
}

static NSDictionary *BuildFace(NSString *caseName) {
    CGRect rect = CGRectMake(0.20, 0.25, 0.40, 0.40);
    NSString *rectKey = ResolveStringConstant("kFigCaptureStreamMetadata_Rect", @"Rect");
    NSString *faceIDKey = ResolveStringConstant("kFigCaptureStreamMetadata_FaceID", @"FaceID");
    NSString *yawKey = ResolveStringConstant("kFigCaptureStreamMetadata_AngleInfoYaw", @"AngleInfoYaw");
    NSString *pitchKey = ResolveStringConstant("kFigCaptureStreamMetadata_AngleInfoPitch", @"AngleInfoPitch");
    NSString *rollKey = ResolveStringConstant("kFigCaptureStreamMetadata_AngleInfoRoll", @"AngleInfoRoll");

    NSMutableDictionary *face = [NSMutableDictionary dictionary];
    BOOL rectAsValue = [caseName containsString:@"rect-value"];
    face[rectKey] = rectAsValue
        ? [NSValue value:&rect withObjCType:@encode(CGRect)]
        : CFBridgingRelease(CGRectCreateDictionaryRepresentation(rect));
    face[faceIDKey] = @17;

    if ([caseName containsString:@"pose"] || [caseName containsString:@"landmarks"]) {
        face[yawKey] = @12.5;
        face[pitchKey] = @(-4.25);
        face[rollKey] = @2.75;
    }

    if ([caseName isEqualToString:@"rect-pose-landmarks-faceLandmarks"]) {
        face[@"faceLandmarks"] = SyntheticLandmarks(YES);
    } else if ([caseName isEqualToString:@"rect-pose-landmarks-features"]) {
        face[@"Features"] = SyntheticLandmarks(YES);
    } else if ([caseName isEqualToString:@"rect-pose-landmarks-landmarks"]) {
        face[@"Landmarks"] = SyntheticLandmarks(YES);
    } else if ([caseName isEqualToString:@"rect-pose-landmarks-values"]) {
        face[@"faceLandmarks"] = SyntheticLandmarks(NO);
    } else if ([caseName isEqualToString:@"rect-pose-landmarks-all-aliases"]) {
        NSArray *landmarks = SyntheticLandmarks(YES);
        face[@"faceLandmarks"] = landmarks;
        face[@"FaceLandmarks"] = landmarks;
        face[@"Landmarks"] = landmarks;
        face[@"Features"] = landmarks;
        face[@"faceLandmarkType"] = @1;
    }
    return face;
}

static NSArray<NSString *> *Cases(void) {
    return @[
        @"rect-dict",
        @"rect-value",
        @"rect-pose",
        @"rect-value-pose",
        @"rect-pose-landmarks-faceLandmarks",
        @"rect-pose-landmarks-features",
        @"rect-pose-landmarks-landmarks",
        @"rect-pose-landmarks-values",
        @"rect-pose-landmarks-all-aliases"
    ];
}

static void PrintUsage(const char *argv0) {
    fprintf(stderr, "usage: %s --list | --case NAME [--normalize x y w h]\n", argv0);
}

int main(int argc, const char **argv) {
    @autoreleasepool {
        if (argc == 2 && strcmp(argv[1], "--self-test") == 0) return SignatureSelfTest();
        gResolvedKeys = [NSMutableDictionary dictionary];
        NSString *caseName = nil;
        BOOL listOnly = NO;
        BOOL normalize = NO;
        CGRect crop = CGRectMake(0.03, 0.04, 0.94, 0.92);
        for (int i = 1; i < argc; ++i) {
            if (strcmp(argv[i], "--list") == 0) {
                listOnly = YES;
            } else if (strcmp(argv[i], "--case") == 0 && i + 1 < argc) {
                caseName = [NSString stringWithUTF8String:argv[++i]];
            } else if (strcmp(argv[i], "--normalize") == 0 && i + 4 < argc) {
                normalize = YES;
                double values[4];
                for (int j = 0; j < 4; ++j) {
                    const char *start = argv[++i]; char *end = NULL;
                    values[j] = strtod(start, &end);
                    if (start == end || *end || !isfinite(values[j])) return 64;
                }
                if (values[0] < 0 || values[1] < 0 || values[2] <= 0 || values[3] <= 0 ||
                    values[0] + values[2] > 1 || values[1] + values[3] > 1) return 64;
                crop = CGRectMake(values[0], values[1], values[2], values[3]);
            } else {
                PrintUsage(argv[0]);
                return 64;
            }
        }

        if (listOnly) {
            for (NSString *name in Cases()) printf("%s\n", name.UTF8String);
            return 0;
        }
        if (caseName == nil || ![Cases() containsObject:caseName]) {
            PrintUsage(argv[0]);
            return 64;
        }

        const char *cmImagingPaths[] = {
            "/System/Library/PrivateFrameworks/CMImaging.framework/Versions/A/CMImaging",
            "/System/Library/PrivateFrameworks/CMImaging.framework/CMImaging",
            NULL
        };
        const char *cmCapturePaths[] = {
            "/System/Library/PrivateFrameworks/CMCapture.framework/Versions/A/CMCapture",
            "/System/Library/PrivateFrameworks/CMCapture.framework/CMCapture",
            NULL
        };
        gCMImaging = OpenFramework(cmImagingPaths);
        gCMCapture = OpenFramework(cmCapturePaths);
        if (gCMImaging == NULL) {
            fprintf(stderr, "[probe] CMImaging unavailable: %s\n", dlerror());
            return 2;
        }


        Class utility = NSClassFromString(@"CMITextureStylesPersonInputDataUtilities");
        SEL convertSelector = NSSelectorFromString(@"personInputDataArrayFromDetectedFaces:");
        SEL normalizeSelector = NSSelectorFromString(@"normalizePersonInputDataArray:toCropRect:");
        if (utility == Nil || ![(id)utility respondsToSelector:convertSelector]) {
            fprintf(stderr, "[probe] CMITextureStylesPersonInputDataUtilities/personInputDataArrayFromDetectedFaces: unavailable\n");
            return 3;
        }

        NSDictionary *face = BuildFace(caseName);
        NSArray *faces = @[face];
        NSMutableDictionary *report = [NSMutableDictionary dictionary];
        report[@"schema"] = @1;
        report[@"case"] = caseName;
        report[@"inputKind"] = @"synthetic-geometry-not-capture-facts";
        report[@"keyResolution"] = gResolvedKeys;
        report[@"osVersion"] = [NSProcessInfo processInfo].operatingSystemVersionString;
        report[@"productAccepted"] = @NO;
        BOOL succeeded = NO;
        report[@"utilityClass"] = NSStringFromClass(utility) ?: @"?";
        report[@"convertEncoding"] = MethodEncoding(utility, convertSelector, YES);
        report[@"normalizeEncoding"] = MethodEncoding(utility, normalizeSelector, YES);
        report[@"input"] = JSONSafe(face, 0);
        report[@"inputKeys"] = [[face allKeys] sortedArrayUsingSelector:@selector(compare:)];

        @try {
            id people = InvokeClassWithOneObject(utility, convertSelector, faces);
            report[@"resultClass"] = people ? NSStringFromClass([people class]) : @"<nil>";
            report[@"beforeNormalize"] = SummarizePeople(people);
            report[@"resultCount"] = [people respondsToSelector:@selector(count)] ? @([people count]) : @0;
            if (![people isKindOfClass:[NSArray class]] || [people count] != 1) {
                [NSException raise:NSInternalInconsistencyException format:@"Expected one materialized person"];
            }
            if (normalize && ![(id)utility respondsToSelector:normalizeSelector]) {
                [NSException raise:NSInvalidArgumentException format:@"Normalization selector is unavailable"];
            }

            if (normalize && people != nil && [(id)utility respondsToSelector:normalizeSelector]) {
                BOOL returnedObject = NO;
                id normalized = InvokeClassWithObjectAndRect(utility, normalizeSelector, people, crop, &returnedObject);
                id finalPeople = returnedObject && normalized != nil ? normalized : people;
                report[@"normalizeCrop"] = @{ @"x": @(crop.origin.x), @"y": @(crop.origin.y),
                                               @"width": @(crop.size.width), @"height": @(crop.size.height) };
                report[@"normalizeReturnedObject"] = @(returnedObject);
                report[@"afterNormalize"] = SummarizePeople(finalPeople);
            }
            succeeded = YES;
        } @catch (NSException *exception) {
            report[@"exception"] = @{ @"name": exception.name ?: @"?", @"reason": exception.reason ?: @"" };
        }

        report[@"succeeded"] = @(succeeded);
        NSError *error = nil;
        NSData *json = [NSJSONSerialization dataWithJSONObject:report options:NSJSONWritingPrettyPrinted | NSJSONWritingSortedKeys error:&error];
        if (json == nil) {
            fprintf(stderr, "[probe] JSON serialization failed: %s\n", error.localizedDescription.UTF8String);
            return 4;
        }
        fwrite(json.bytes, 1, json.length, stdout);
        fputc('\n', stdout);
        return succeeded ? 0 : 5;
    }
}
