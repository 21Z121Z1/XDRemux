#import <CoreImage/CoreImage.h>
#import <Foundation/Foundation.h>
#import <ImageIO/ImageIO.h>
#import <Metal/Metal.h>
#import <dlfcn.h>
#import <objc/message.h>
#import <objc/runtime.h>

// Private ABI mirror used by NUStyleTransferNode on this macOS build.
typedef struct {
    int64_t first;
    int64_t second;
} NUIntegerPair;

static IMP gOriginalLearnProcess;
static IMP gOriginalCMIProcess;
static IMP gOriginalSemanticProcess;
static IMP gOriginalUsingSharedRenderer;
static IMP gOriginalGuidedFilterEncode;
static IMP gOriginalAllocatorNewTexture;
static NSMutableDictionary<NSString *, NSValue *> *gOriginalSmartStyleMethods;
static NSMutableArray<NSDictionary *> *gEvents;

static id SendId(id object, SEL selector) {
    return ((id (*)(id, SEL))objc_msgSend)(object, selector);
}

static double SendDouble(id object, SEL selector) {
    return ((double (*)(id, SEL))objc_msgSend)(object, selector);
}

static NSInteger SendInteger(id object, SEL selector) {
    return ((NSInteger (*)(id, SEL))objc_msgSend)(object, selector);
}

static void SendObject(id object, SEL selector, id value) {
    ((void (*)(id, SEL, id))objc_msgSend)(object, selector, value);
}

static void SendDoubleValue(id object, SEL selector, double value) {
    ((void (*)(id, SEL, double))objc_msgSend)(object, selector, value);
}

static void SendBoolValue(id object, SEL selector, BOOL value) {
    ((void (*)(id, SEL, BOOL))objc_msgSend)(object, selector, value);
}

static id SendClassObjectError(Class cls, SEL selector, id value, NSError **error) {
    return ((id (*)(id, SEL, id, NSError **))objc_msgSend)((id)cls, selector, value, error);
}

static id SendClassPairPair(Class cls, SEL selector, NUIntegerPair first, NUIntegerPair second) {
    return ((id (*)(id, SEL, NUIntegerPair, NUIntegerPair))objc_msgSend)(
        (id)cls, selector, first, second
    );
}

static id SendClassCastFloats(
    Class cls,
    SEL selector,
    id cast,
    float tone,
    float color,
    float intensity,
    float priorStrength
) {
    return ((id (*)(id, SEL, id, float, float, float, float))objc_msgSend)(
        (id)cls, selector, cast, tone, color, intensity, priorStrength
    );
}

static id SendClassLearn(
    Class cls,
    SEL selector,
    id input,
    id target,
    id colorSpace,
    id configuration,
    id tuning,
    NSError **error
) {
    return ((id (*)(id, SEL, id, id, id, id, id, NSError **))objc_msgSend)(
        (id)cls,
        selector,
        input,
        target,
        colorSpace,
        configuration,
        tuning,
        error
    );
}

static id SendClassThumbnail(
    Class cls,
    SEL selector,
    id image,
    NUIntegerPair targetSize,
    id colorSpace,
    id configuration,
    id tuning,
    NSError **error
) {
    return ((id (*)(id, SEL, id, NUIntegerPair, id, id, id, NSError **))objc_msgSend)(
        (id)cls,
        selector,
        image,
        targetSize,
        colorSpace,
        configuration,
        tuning,
        error
    );
}

static id SendClassApply(
    Class cls,
    SEL selector,
    id style,
    id image,
    id thumbnail,
    id target,
    id deltaMap,
    id colorSpace,
    id configuration,
    id tuning,
    id noiseModel,
    NSError **error
) {
    return ((id (*)(id, SEL, id, id, id, id, id, id, id, id, id, NSError **))objc_msgSend)(
        (id)cls,
        selector,
        style,
        image,
        thumbnail,
        target,
        deltaMap,
        colorSpace,
        configuration,
        tuning,
        noiseModel,
        error
    );
}

static CGSize SendClassDictionarySize(Class cls, SEL selector, id dictionary) {
    return ((CGSize (*)(id, SEL, id))objc_msgSend)((id)cls, selector, dictionary);
}

static id JSONSafe(id value);

static NSDictionary *ObjectIvarSummary(id object) {
    if (!object) {
        return @{};
    }
    NSMutableDictionary *result = [NSMutableDictionary dictionary];
    for (Class cls = [object class]; cls && cls != [NSObject class]; cls = class_getSuperclass(cls)) {
        unsigned int count = 0;
        Ivar *ivars = class_copyIvarList(cls, &count);
        for (unsigned int index = 0; index < count; index++) {
            const char *name = ivar_getName(ivars[index]);
            const char *type = ivar_getTypeEncoding(ivars[index]);
            NSString *key = [NSString stringWithFormat:@"%@.%s", NSStringFromClass(cls), name ?: ""];
            if (type && type[0] == '@') {
                id child = object_getIvar(object, ivars[index]);
                result[key] = child ? @{
                    @"class": NSStringFromClass([child class]),
                    @"description": [child description] ?: @"",
                } : [NSNull null];
            } else {
                result[key] = @{
                    @"encoding": type ? [NSString stringWithUTF8String:type] : @"",
                    @"offset": @(ivar_getOffset(ivars[index])),
                };
            }
        }
        free(ivars);
    }
    return result;
}

static NSDictionary *TextureSummary(id<MTLTexture> texture) {
    if (!texture) {
        return @{};
    }
    return @{
        @"class": NSStringFromClass([(id)texture class]),
        @"width": @(texture.width),
        @"height": @(texture.height),
        @"depth": @(texture.depth),
        @"arrayLength": @(texture.arrayLength),
        @"mipmapLevelCount": @(texture.mipmapLevelCount),
        @"pixelFormat": @((NSUInteger)texture.pixelFormat),
        @"textureType": @((NSUInteger)texture.textureType),
        @"storageMode": @((NSUInteger)texture.storageMode),
        @"cpuCacheMode": @((NSUInteger)texture.cpuCacheMode),
        @"hazardTrackingMode": @((NSUInteger)texture.hazardTrackingMode),
        @"usage": @((NSUInteger)texture.usage),
        @"framebufferOnly": @(texture.framebufferOnly),
        @"allocatedSize": @(texture.allocatedSize),
        @"iosurfacePlane": @(texture.iosurfacePlane),
        @"hasIOSurface": @(texture.iosurface != NULL),
        @"label": texture.label ?: [NSNull null],
    };
}

static NSDictionary *BufferSummary(id<MTLBuffer> buffer) {
    if (!buffer) {
        return @{};
    }
    return @{
        @"class": NSStringFromClass([(id)buffer class]),
        @"length": @(buffer.length),
        @"storageMode": @((NSUInteger)buffer.storageMode),
        @"cpuCacheMode": @((NSUInteger)buffer.cpuCacheMode),
        @"hazardTrackingMode": @((NSUInteger)buffer.hazardTrackingMode),
        @"allocatedSize": @(buffer.allocatedSize),
        @"hasContents": @(buffer.contents != NULL),
        @"label": buffer.label ?: [NSNull null],
    };
}

static NSDictionary *ProcessorSummary(id processor) {
    if (!processor) {
        return @{};
    }
    NSMutableDictionary *result = [@{
        @"class": NSStringFromClass([processor class]),
        @"description": [processor description] ?: @"",
    } mutableCopy];
    for (NSString *selectorName in @[
        @"outputLinearSystemCoefficients",
        @"outputLinearSystemCoefficientsBuffer",
        @"outputLinearSystemCoefficientsTexture",
        @"inputThumbnailTexture",
        @"targetThumbnailTexture",
        @"configuration",
        @"tuningParameters",
        @"metalCommandQueue",
    ]) {
        SEL selector = NSSelectorFromString(selectorName);
        if (![processor respondsToSelector:selector]) {
            continue;
        }
        id child = SendId(processor, selector);
        if ([child conformsToProtocol:@protocol(MTLTexture)]) {
            result[selectorName] = TextureSummary(child);
        } else if ([child conformsToProtocol:@protocol(MTLBuffer)]) {
            result[selectorName] = BufferSummary(child);
        } else {
            result[selectorName] = JSONSafe(child);
        }
    }
    result[@"ivars"] = ObjectIvarSummary(processor);
    return result;
}

static NSDictionary *ProcessorOutputSummary(id output) {
    if (!output) {
        return @{};
    }
    NSMutableDictionary *result = [@{
        @"class": NSStringFromClass([output class]),
        @"description": [output description] ?: @"",
        @"ivars": ObjectIvarSummary(output),
    } mutableCopy];
    SEL regionSelector = NSSelectorFromString(@"region");
    if ([output respondsToSelector:regionSelector]) {
        CGRect region = ((CGRect (*)(id, SEL))objc_msgSend)(output, regionSelector);
        result[@"region"] = @{
            @"x": @(region.origin.x),
            @"y": @(region.origin.y),
            @"width": @(region.size.width),
            @"height": @(region.size.height),
        };
    }
    SEL formatSelector = NSSelectorFromString(@"format");
    if ([output respondsToSelector:formatSelector]) {
        result[@"format"] = @(((int (*)(id, SEL))objc_msgSend)(output, formatSelector));
    }
    SEL bytesPerRowSelector = NSSelectorFromString(@"bytesPerRow");
    if ([output respondsToSelector:bytesPerRowSelector]) {
        result[@"bytesPerRow"] = @(((size_t (*)(id, SEL))objc_msgSend)(
            output, bytesPerRowSelector
        ));
    }
    SEL baseAddressSelector = NSSelectorFromString(@"baseAddress");
    if ([output respondsToSelector:baseAddressSelector]) {
        result[@"hasBaseAddress"] = @(
            ((void *(*)(id, SEL))objc_msgSend)(output, baseAddressSelector) != NULL
        );
    }
    SEL metalTextureSelector = NSSelectorFromString(@"metalTexture");
    if ([output respondsToSelector:metalTextureSelector]) {
        result[@"metalTexture"] = TextureSummary(SendId(output, metalTextureSelector));
    }
    return result;
}

static NSDictionary *CompactObjectSummary(id object) {
    if (!object) {
        return @{};
    }
    if ([object conformsToProtocol:@protocol(MTLTexture)]) {
        return TextureSummary(object);
    }
    if ([object conformsToProtocol:@protocol(MTLBuffer)]) {
        return BufferSummary(object);
    }
    if ([object isKindOfClass:[NSData class]]) {
        return @{
            @"class": NSStringFromClass([object class]),
            @"length": @([(NSData *)object length]),
        };
    }
    if ([object isKindOfClass:[NSDictionary class]] ||
        [object isKindOfClass:[NSArray class]] ||
        [object isKindOfClass:[NSSet class]]) {
        return @{
            @"class": NSStringFromClass([object class]),
            @"count": @([(id)object count]),
        };
    }
    return @{
        @"class": NSStringFromClass([object class]),
        @"description": [object description] ?: @"",
    };
}

static NSDictionary *TextureDescriptorSummary(id descriptorWrapper) {
    if (!descriptorWrapper) {
        return @{};
    }
    id descriptor = descriptorWrapper;
    SEL descSelector = NSSelectorFromString(@"desc");
    if ([descriptor respondsToSelector:descSelector]) {
        descriptor = SendId(descriptor, descSelector) ?: descriptor;
    }
    if (![descriptor isKindOfClass:[MTLTextureDescriptor class]]) {
        return CompactObjectSummary(descriptorWrapper);
    }
    MTLTextureDescriptor *textureDescriptor = descriptor;
    return @{
        @"wrapperClass": NSStringFromClass([descriptorWrapper class]),
        @"class": NSStringFromClass([textureDescriptor class]),
        @"width": @(textureDescriptor.width),
        @"height": @(textureDescriptor.height),
        @"depth": @(textureDescriptor.depth),
        @"arrayLength": @(textureDescriptor.arrayLength),
        @"mipmapLevelCount": @(textureDescriptor.mipmapLevelCount),
        @"pixelFormat": @((NSUInteger)textureDescriptor.pixelFormat),
        @"textureType": @((NSUInteger)textureDescriptor.textureType),
        @"storageMode": @((NSUInteger)textureDescriptor.storageMode),
        @"cpuCacheMode": @((NSUInteger)textureDescriptor.cpuCacheMode),
        @"hazardTrackingMode": @((NSUInteger)textureDescriptor.hazardTrackingMode),
        @"usage": @((NSUInteger)textureDescriptor.usage),
        @"resourceOptions": @((NSUInteger)textureDescriptor.resourceOptions),
    };
}

static id IvarObjectValue(id object, NSString *name) {
    for (Class cls = [object class]; cls && cls != [NSObject class]; cls = class_getSuperclass(cls)) {
        Ivar ivar = class_getInstanceVariable(cls, name.UTF8String);
        if (ivar && ivar_getTypeEncoding(ivar)[0] == '@') {
            return object_getIvar(object, ivar);
        }
    }
    return nil;
}

static NSDictionary *SmartStyleRendererCompactSummary(id renderer) {
    if (!renderer) {
        return @{};
    }
    NSMutableDictionary *result = [@{
        @"class": NSStringFromClass([renderer class]),
    } mutableCopy];
    for (NSString *selectorName in @[
        @"inputImageTexture",
        @"inputImageThumbnailTexture",
        @"inputLinearImageTexture",
        @"inputLinearImageLumaTexture",
        @"inputLinearImageChromaTexture",
        @"inputPersonMaskTexture",
        @"inputSkinMaskTexture",
        @"inputSkyMaskTexture",
        @"inputGainMapTexture",
        @"inputGlobalToneCurveTexture",
        @"inputLightMapTexture",
        @"inputLinearLightMapTexture",
        @"inputSmallLightMapTexture",
        @"inputSmallLinearLightMapTexture",
        @"inputStyle",
        @"inputStatisticsByStatsKey",
        @"inputStatisticsByStatsType",
        @"tuningParameterVariant",
        @"tuningParameters",
        @"outputImageTexture",
        @"outputGainMapTexture",
        @"outputSmallLightMapTexture",
        @"outputSmallLinearLightMapTexture",
    ]) {
        SEL selector = NSSelectorFromString(selectorName);
        if ([renderer respondsToSelector:selector]) {
            result[selectorName] = CompactObjectSummary(SendId(renderer, selector));
        }
    }
    for (NSString *selectorName in @[
        @"baselineExposure",
        @"castIntensity",
        @"colorBias",
        @"faceBasedGlobalExposureBoostRatio",
        @"inputLinearBaseGain",
        @"inputLinearEncodingGain",
        @"inputLinearImageGainDownRatio",
        @"inputSRLCurveParameter",
        @"personMasksValidHint",
        @"toneBias",
    ]) {
        SEL selector = NSSelectorFromString(selectorName);
        if ([renderer respondsToSelector:selector]) {
            result[selectorName] = @(((float (*)(id, SEL))objc_msgSend)(renderer, selector));
        }
    }
    SEL sceneTypeSelector = NSSelectorFromString(@"semanticStyleSceneType");
    if ([renderer respondsToSelector:sceneTypeSelector]) {
        result[@"semanticStyleSceneType"] = @(((int (*)(id, SEL))objc_msgSend)(
            renderer, sceneTypeSelector
        ));
    }
    result[@"internalTuningParameters"] = CompactObjectSummary(
        IvarObjectValue(renderer, @"_internalTuningParams")
    );
    return result;
}

static id JSONSafe(id value) {
    if (!value) {
        return [NSNull null];
    }
    if ([value isKindOfClass:[NSString class]] ||
        [value isKindOfClass:[NSNumber class]] ||
        [value isKindOfClass:[NSNull class]]) {
        return value;
    }
    if ([value isKindOfClass:[NSData class]]) {
        return @{
            @"class": NSStringFromClass([value class]),
            @"length": @([(NSData *)value length]),
        };
    }
    if ([value isKindOfClass:[NSDictionary class]]) {
        NSMutableDictionary *result = [NSMutableDictionary dictionary];
        for (id key in value) {
            result[[key description] ?: @"<nil>"] = JSONSafe(value[key]);
        }
        return result;
    }
    if ([value isKindOfClass:[NSArray class]]) {
        NSMutableArray *result = [NSMutableArray array];
        for (id child in value) {
            [result addObject:JSONSafe(child)];
        }
        return result;
    }
    if ([value conformsToProtocol:@protocol(MTLTexture)]) {
        return TextureSummary(value);
    }
    if ([value conformsToProtocol:@protocol(MTLBuffer)]) {
        return BufferSummary(value);
    }
    return @{
        @"class": NSStringFromClass([value class]),
        @"description": [value description] ?: @"",
    };
}

static void AppendEvent(NSDictionary *event) {
    @synchronized (gEvents) {
        [gEvents addObject:event];
    }
}

static BOOL RecordingLearnProcess(
    id cls,
    SEL selector,
    NSArray *inputs,
    NSDictionary *arguments,
    id output,
    NSError **error
) {
    NSMutableArray *inputRows = [NSMutableArray array];
    for (id input in inputs) {
        [inputRows addObject:@{
            @"class": NSStringFromClass([input class]),
            @"description": [input description] ?: @"",
            @"ivars": ObjectIvarSummary(input),
        }];
    }
    NSDictionary *before = @{
        @"event": @"_NUStyleTransferLearnProcessor.process.before",
        @"inputs": inputRows,
        @"arguments": JSONSafe(arguments),
        @"output": ProcessorOutputSummary(output),
    };
    AppendEvent(before);
    BOOL result = ((BOOL (*)(id, SEL, NSArray *, NSDictionary *, id, NSError **))
        gOriginalLearnProcess)(cls, selector, inputs, arguments, output, error);
    AppendEvent(@{
        @"event": @"_NUStyleTransferLearnProcessor.process.after",
        @"result": @(result),
        @"error": error && *error ? JSONSafe(*error) : [NSNull null],
        @"output": ProcessorOutputSummary(output),
    });
    return result;
}

static BOOL RecordingSemanticProcess(
    id cls,
    SEL selector,
    NSArray *inputs,
    NSDictionary *arguments,
    id output,
    NSError **error
) {
    NSMutableArray *inputRows = [NSMutableArray array];
    for (id input in inputs) {
        [inputRows addObject:@{
            @"class": NSStringFromClass([input class]),
            @"description": [input description] ?: @"",
            @"ivars": ObjectIvarSummary(input),
        }];
    }
    AppendEvent(@{
        @"event": @"PISemanticStyleProcessor.process.before",
        @"inputs": inputRows,
        @"arguments": JSONSafe(arguments),
        @"output": ProcessorOutputSummary(output),
    });
    BOOL result = ((BOOL (*)(id, SEL, NSArray *, NSDictionary *, id, NSError **))
        gOriginalSemanticProcess)(cls, selector, inputs, arguments, output, error);
    AppendEvent(@{
        @"event": @"PISemanticStyleProcessor.process.after",
        @"result": @(result),
        @"error": error && *error ? JSONSafe(*error) : [NSNull null],
        @"output": ProcessorOutputSummary(output),
    });
    return result;
}

static NSDictionary *SemanticRendererSummary(id renderer) {
    if (!renderer) {
        return @{};
    }
    NSMutableDictionary *summary = [@{
        @"class": NSStringFromClass([renderer class]),
        @"description": [renderer description] ?: @"",
        @"ivars": ObjectIvarSummary(renderer),
    } mutableCopy];
    for (NSString *selectorName in @[
        @"processingType", @"useStyleEngine", @"processor", @"metalCommandQueue"
    ]) {
        SEL selector = NSSelectorFromString(selectorName);
        if (![renderer respondsToSelector:selector]) {
            continue;
        }
        if ([selectorName isEqualToString:@"processingType"]) {
            summary[selectorName] = @(SendInteger(renderer, selector));
        } else if ([selectorName isEqualToString:@"useStyleEngine"]) {
            summary[selectorName] = @(((BOOL (*)(id, SEL))objc_msgSend)(renderer, selector));
        } else {
            id child = SendId(renderer, selector);
            summary[selectorName] = child ? @{
                @"class": NSStringFromClass([child class]),
                @"description": [child description] ?: @"",
                @"ivars": ObjectIvarSummary(child),
            } : (id)[NSNull null];
        }
    }
    return summary;
}

static BOOL RecordingUsingSharedRenderer(
    id cls,
    SEL selector,
    id commandQueue,
    int processingType,
    BOOL useStyleEngine,
    id blockObject
) {
    BOOL (^originalBlock)(id) = blockObject;
    BOOL (^recordingBlock)(id) = ^BOOL(id renderer) {
        AppendEvent(@{
            @"event": @"PISemanticStyleRenderer.callback.before",
            @"renderer": SemanticRendererSummary(renderer),
        });
        BOOL result = originalBlock(renderer);
        AppendEvent(@{
            @"event": @"PISemanticStyleRenderer.callback.after",
            @"result": @(result),
            @"renderer": SemanticRendererSummary(renderer),
        });
        return result;
    };
    BOOL result = ((BOOL (*)(id, SEL, id, int, BOOL, id))gOriginalUsingSharedRenderer)(
        cls,
        selector,
        commandQueue,
        processingType,
        useStyleEngine,
        recordingBlock
    );
    AppendEvent(@{
        @"event": @"PISemanticStyleRenderer.usingShared.after",
        @"result": @(result),
        @"processingType": @(processingType),
        @"useStyleEngine": @(useStyleEngine),
    });
    return result;
}

static int RecordingSmartStyleStatus(id renderer, SEL selector) {
    NSString *selectorName = NSStringFromSelector(selector);
    AppendEvent(@{
        @"event": [NSString stringWithFormat:@"CMISmartStyleMetalRendererV1.%@.before",
                                            selectorName],
        @"renderer": SmartStyleRendererCompactSummary(renderer),
    });
    IMP original = [gOriginalSmartStyleMethods[selectorName] pointerValue];
    int result = ((int (*)(id, SEL))original)(renderer, selector);
    AppendEvent(@{
        @"event": [NSString stringWithFormat:@"CMISmartStyleMetalRendererV1.%@.after",
                                            selectorName],
        @"result": @(result),
        @"renderer": SmartStyleRendererCompactSummary(renderer),
    });
    return result;
}

static int RecordingSmartStylePrepare(id renderer, SEL selector, unsigned int processingType) {
    NSString *selectorName = NSStringFromSelector(selector);
    AppendEvent(@{
        @"event": @"CMISmartStyleMetalRendererV1.prepareToProcess:.before",
        @"processingType": @(processingType),
        @"renderer": SmartStyleRendererCompactSummary(renderer),
    });
    IMP original = [gOriginalSmartStyleMethods[selectorName] pointerValue];
    int result = ((int (*)(id, SEL, unsigned int))original)(
        renderer, selector, processingType
    );
    AppendEvent(@{
        @"event": @"CMISmartStyleMetalRendererV1.prepareToProcess:.after",
        @"processingType": @(processingType),
        @"result": @(result),
        @"renderer": SmartStyleRendererCompactSummary(renderer),
    });
    return result;
}

static id RecordingAllocatorNewTexture(id allocator, SEL selector, id descriptor) {
    id result = ((id (*)(id, SEL, id))gOriginalAllocatorNewTexture)(
        allocator, selector, descriptor
    );
    AppendEvent(@{
        @"event": @"CMIGuidedFilter.allocator.newTextureWithDescriptor:",
        @"allocatorClass": NSStringFromClass([allocator class]),
        @"descriptor": TextureDescriptorSummary(descriptor),
        @"result": CompactObjectSummary(result),
        @"returnedNil": @(result == nil),
    });
    return result;
}

static int RecordingGuidedFilterEncode(
    id filter,
    SEL selector,
    id commandBuffer,
    id inputTexture,
    id guideTexture,
    id outputTexture,
    NSUInteger kernelRadius,
    float epsilon
) {
    id metal = IvarObjectValue(filter, @"_metal");
    SEL allocatorSelector = NSSelectorFromString(@"allocator");
    id allocator = [metal respondsToSelector:allocatorSelector]
        ? SendId(metal, allocatorSelector)
        : nil;
    Method allocatorMethod = allocator ? class_getInstanceMethod(
        [allocator class], NSSelectorFromString(@"newTextureWithDescriptor:")
    ) : NULL;
    BOOL allocatorHookInstalled = NO;
    if (allocatorMethod) {
        gOriginalAllocatorNewTexture = method_getImplementation(allocatorMethod);
        method_setImplementation(allocatorMethod, (IMP)RecordingAllocatorNewTexture);
        allocatorHookInstalled = YES;
    }
    AppendEvent(@{
        @"event": @"CMIGuidedFilter.encode.before",
        @"filterClass": NSStringFromClass([filter class]),
        @"commandBuffer": CompactObjectSummary(commandBuffer),
        @"inputTexture": CompactObjectSummary(inputTexture),
        @"guideTexture": CompactObjectSummary(guideTexture),
        @"outputTexture": CompactObjectSummary(outputTexture),
        @"kernelRadius": @(kernelRadius),
        @"epsilon": @(epsilon),
        @"metal": CompactObjectSummary(metal),
        @"allocator": CompactObjectSummary(allocator),
        @"allocatorHookInstalled": @(allocatorHookInstalled),
    });
    int result = 0;
    @try {
        result = ((int (*)(id, SEL, id, id, id, id, NSUInteger, float))
            gOriginalGuidedFilterEncode)(
                filter,
                selector,
                commandBuffer,
                inputTexture,
                guideTexture,
                outputTexture,
                kernelRadius,
                epsilon
            );
    } @finally {
        if (allocatorHookInstalled) {
            method_setImplementation(allocatorMethod, gOriginalAllocatorNewTexture);
        }
    }
    AppendEvent(@{
        @"event": @"CMIGuidedFilter.encode.after",
        @"result": @(result),
        @"allocatorHookRestored": @(!allocatorHookInstalled ||
            method_getImplementation(allocatorMethod) == gOriginalAllocatorNewTexture),
    });
    return result;
}

static int RecordingCMIProcess(id processor, SEL selector) {
    AppendEvent(@{
        @"event": @"CMIStyleEngineProcessor.process.before",
        @"processor": ProcessorSummary(processor),
    });
    int result = ((int (*)(id, SEL))gOriginalCMIProcess)(processor, selector);
    AppendEvent(@{
        @"event": @"CMIStyleEngineProcessor.process.after",
        @"result": @(result),
        @"processor": ProcessorSummary(processor),
    });
    return result;
}

static id ValueForDescribedKey(NSDictionary *dictionary, NSString *description) {
    for (id key in dictionary) {
        if ([[key description] isEqualToString:description]) {
            return dictionary[key];
        }
    }
    return nil;
}

static CIImage *LoadImage(NSString *path, BOOL applyOrientation) {
    if (!path || [path isEqualToString:@"-"]) {
        return nil;
    }
    return [CIImage imageWithContentsOfURL:[NSURL fileURLWithPath:path]
                                  options:@{
        kCIImageApplyOrientationProperty: @(applyOrientation),
    }];
}

static CIImage *BlackImage(CGRect extent) {
    return [[CIImage imageWithColor:[CIColor colorWithRed:0 green:0 blue:0 alpha:1]]
        imageByCroppingToRect:extent];
}

static CGSize SemanticStyleRenderSize(CGSize inputSize) {
    return inputSize.width >= inputSize.height
        ? CGSizeMake(256, 192)
        : CGSizeMake(192, 256);
}

static CIImage *ScaleImageToSize(CIImage *image, CGSize targetSize) {
    CGRect extent = image.extent;
    CIImage *originNormalized = [image imageByApplyingTransform:
        CGAffineTransformMakeTranslation(-extent.origin.x, -extent.origin.y)];
    CGFloat scaleY = targetSize.height / extent.size.height;
    CGFloat scaleX = targetSize.width / extent.size.width;
    CIImage *scaled = [originNormalized imageByApplyingFilter:@"CILanczosScaleTransform"
                                         withInputParameters:@{
        kCIInputScaleKey: @(scaleY),
        kCIInputAspectRatioKey: @(scaleX / scaleY),
    }];
    return [scaled imageByCroppingToRect:CGRectMake(
        0, 0, targetSize.width, targetSize.height
    )];
}

static CIImage *NormalizeLinearThumbnail(
    CIImage *image,
    BOOL applyInverseCurve,
    double linearGain,
    double linearRangeMin,
    double linearRangeMax
) {
    CIImage *result = image;
    if (applyInverseCurve) {
        result = [result imageByApplyingFilter:@"CIAppleLogToLinear"];
    }
    double scale = linearGain != 0.0 ? 1.0 / linearGain : 1.0;
    result = [result imageByApplyingFilter:@"CIColorMatrix" withInputParameters:@{
        @"inputRVector": [CIVector vectorWithX:scale Y:0 Z:0 W:0],
        @"inputGVector": [CIVector vectorWithX:0 Y:scale Z:0 W:0],
        @"inputBVector": [CIVector vectorWithX:0 Y:0 Z:scale W:0],
        @"inputAVector": [CIVector vectorWithX:0 Y:0 Z:0 W:1],
        @"inputBiasVector": [CIVector vectorWithX:0 Y:0 Z:0 W:0],
    }];
    return [result imageByApplyingFilter:@"CIColorClamp" withInputParameters:@{
        @"inputMinComponents": [CIVector vectorWithX:linearRangeMin
                                                   Y:linearRangeMin
                                                   Z:linearRangeMin
                                                   W:0],
        @"inputMaxComponents": [CIVector vectorWithX:linearRangeMax
                                                   Y:linearRangeMax
                                                   Z:linearRangeMax
                                                   W:1],
    }];
}

static BOOL RenderHalfRGBA(
    CIContext *context,
    CIImage *image,
    NSString *path,
    NSError **error
) {
    size_t width = (size_t)llround(image.extent.size.width);
    size_t height = (size_t)llround(image.extent.size.height);
    size_t rowBytes = width * 4 * sizeof(uint16_t);
    NSMutableData *data = [NSMutableData dataWithLength:rowBytes * height];
    @try {
        [context render:image
               toBitmap:data.mutableBytes
               rowBytes:rowBytes
                 bounds:image.extent
                 format:kCIFormatRGBAh
             colorSpace:NULL];
    } @catch (NSException *exception) {
        if (error) {
            *error = [NSError errorWithDomain:@"LearnNodeCoefficientProbe"
                                         code:3
                                     userInfo:@{
                NSLocalizedDescriptionKey: exception.reason ?: exception.name,
            }];
        }
        return NO;
    }
    return [data writeToFile:path options:NSDataWritingAtomic error:error];
}

static CIImage *SemanticTarget(
    CIImage *input,
    NSString *imagePath,
    NSString *metadataPath,
    NSString *linearThumbnailPath,
    NSString *subjectMattePath,
    NSString *skinMattePath,
    NSString *skyMattePath,
    NSDictionary **capture,
    NSData **nativeStyleData,
    NSError **error
) {
    NSData *metadata = [NSData dataWithContentsOfFile:metadataPath options:0 error:error];
    if (!metadata) {
        return nil;
    }
    Class propertiesClass = NSClassFromString(@"_NUSemanticStyleProperties");
    id properties = SendClassObjectError(
        propertiesClass,
        NSSelectorFromString(@"semanticStylePropertiesFromImageMetadata:error:"),
        metadata,
        error
    );
    if (!properties) {
        return nil;
    }
    if (nativeStyleData) {
        *nativeStyleData = SendId(properties, NSSelectorFromString(@"styleData"));
    }

    CGImageSourceRef source = CGImageSourceCreateWithURL(
        (__bridge CFURLRef)[NSURL fileURLWithPath:imagePath],
        NULL
    );
    NSDictionary *imageProperties = source
        ? CFBridgingRelease(CGImageSourceCopyPropertiesAtIndex(source, 0, NULL))
        : nil;
    if (source) {
        CFRelease(source);
    }
    NSDictionary *makerApple = imageProperties[(NSString *)kCGImagePropertyMakerAppleDictionary];
    id maker84 = ValueForDescribedKey(makerApple, @"84");
    typedef id (*SettingsFromMakerNoteFunction)(id);
    SettingsFromMakerNoteFunction settingsFromMakerNote =
        (SettingsFromMakerNoteFunction)dlsym(
            RTLD_DEFAULT,
            "PISemanticStyleSettingsFromMakerNoteProperties"
        );
    NSDictionary *styleSettings = settingsFromMakerNote && maker84
        ? settingsFromMakerNote(maker84)
        : nil;
    if (![styleSettings isKindOfClass:[NSDictionary class]]) {
        if (error) {
            *error = [NSError errorWithDomain:@"LearnNodeCoefficientProbe"
                                         code:4
                                     userInfo:@{
                NSLocalizedDescriptionKey: @"failed to derive style settings from MakerApple 84",
            }];
        }
        return nil;
    }

    CIImage *linearThumbnail = LoadImage(linearThumbnailPath, NO);
    if (!linearThumbnail) {
        if (error) {
            *error = [NSError errorWithDomain:@"LearnNodeCoefficientProbe"
                                         code:5
                                     userInfo:@{
                NSLocalizedDescriptionKey: @"failed to load linear thumbnail",
            }];
        }
        return nil;
    }
    id version = SendId(properties, NSSelectorFromString(@"version"));
    NSInteger versionMinor = version && [version respondsToSelector:NSSelectorFromString(@"minor")]
        ? SendInteger(version, NSSelectorFromString(@"minor"))
        : 0;
    BOOL applyInverseCurve = versionMinor >= 10;
    NSDictionary *environment = [NSProcessInfo processInfo].environment;
    NSString *inverseOverride = environment[@"LEARNNODE_APPLY_INVERSE_CURVE"];
    if (inverseOverride) {
        applyInverseCurve = inverseOverride.boolValue;
    }
    BOOL useStyleEngine = [environment[@"LEARNNODE_USE_STYLE_ENGINE"] boolValue];
    double linearGain = [SendId(properties, NSSelectorFromString(@"linearGain")) doubleValue];
    double linearRangeMin = [SendId(
        properties,
        NSSelectorFromString(@"linearRangeMin")
    ) doubleValue];
    double linearRangeMax = [SendId(
        properties,
        NSSelectorFromString(@"linearRangeMax")
    ) doubleValue];
    linearThumbnail = NormalizeLinearThumbnail(
        linearThumbnail,
        applyInverseCurve,
        linearGain,
        linearRangeMin,
        linearRangeMax
    );

    CGSize semanticRenderSize = SemanticStyleRenderSize(input.extent.size);
    CIImage *semanticInput = ScaleImageToSize(input, semanticRenderSize);
    CIImage *black = BlackImage(semanticInput.extent);
    CIImage *subjectSource = LoadImage(subjectMattePath, NO);
    CIImage *skinSource = LoadImage(skinMattePath, NO);
    CIImage *skySource = LoadImage(skyMattePath, NO);
    CIImage *subject = subjectSource
        ? ScaleImageToSize(subjectSource, semanticRenderSize)
        : black;
    CIImage *skin = skinSource
        ? ScaleImageToSize(skinSource, semanticRenderSize)
        : black;
    CIImage *sky = skySource
        ? ScaleImageToSize(skySource, semanticRenderSize)
        : black;
    id filter = SendId(SendId((id)NSClassFromString(@"PISemanticStyleFilter"),
                              sel_registerName("alloc")),
                       sel_registerName("init"));
    SendObject(filter, NSSelectorFromString(@"setInputImage:"), semanticInput);
    SendObject(filter, NSSelectorFromString(@"setInputSubjectMatteImage:"), subject);
    SendObject(filter, NSSelectorFromString(@"setInputSkinMatteImage:"), skin);
    SendObject(filter, NSSelectorFromString(@"setInputSkyMatteImage:"), sky);
    SendObject(filter, NSSelectorFromString(@"setInputLinearThumbnailImage:"), linearThumbnail);
    SendObject(filter, NSSelectorFromString(@"setInputGainMapImage:"), black);
    SendDoubleValue(filter, NSSelectorFromString(@"setInputToneBias:"),
                    [styleSettings[@"tone"] doubleValue]);
    SendDoubleValue(filter, NSSelectorFromString(@"setInputColorBias:"),
                    [styleSettings[@"color"] doubleValue]);
    SendObject(filter, NSSelectorFromString(@"setInputCast:"),
               styleSettings[@"cast"] ?: @"Standard");
    SendDoubleValue(filter, NSSelectorFromString(@"setInputIntensity:"),
                    [styleSettings[@"intensity"] doubleValue]);
    SendObject(filter, NSSelectorFromString(@"setInputSceneType:"),
               SendId(properties, NSSelectorFromString(@"sceneType")));
    SendObject(filter, NSSelectorFromString(@"setInputTRCData:"),
               SendId(properties, NSSelectorFromString(@"globalToneCurveData")));
    SendDoubleValue(filter, NSSelectorFromString(@"setInputBaselineExposure:"),
                    SendDouble(properties, NSSelectorFromString(@"baselineExposure")));
    SendObject(filter, NSSelectorFromString(@"setInputSRLCurveParameter:"),
               SendId(properties, NSSelectorFromString(@"subjectRelightingValue")));
    SendObject(filter, NSSelectorFromString(@"setInputStatistics:"),
               SendId(properties, NSSelectorFromString(@"stats")));
    SendObject(filter, NSSelectorFromString(@"setInputExtendedStatistics:"),
               SendId(properties, NSSelectorFromString(@"extendedStats")));
    SendObject(filter, NSSelectorFromString(@"setInputLightMapData:"),
               SendId(properties, NSSelectorFromString(@"lightMapData")));
    SendObject(filter, NSSelectorFromString(@"setInputLinearLightMapData:"),
               SendId(properties, NSSelectorFromString(@"linearLightMapData")));
    SendObject(filter, NSSelectorFromString(@"setInputLightMapWidth:"),
               SendId(properties, NSSelectorFromString(@"lightMapWidth")));
    SendObject(filter, NSSelectorFromString(@"setInputLightMapHeight:"),
               SendId(properties, NSSelectorFromString(@"lightMapHeight")));
    SendObject(filter, NSSelectorFromString(@"setBrightnessValue:"),
               SendId(properties, NSSelectorFromString(@"brightness")));
    SendObject(filter, NSSelectorFromString(@"setTuningType:"),
               SendId(properties, NSSelectorFromString(@"tuningType")));
    SendObject(filter, NSSelectorFromString(@"setBaseGain:"),
               SendId(properties, NSSelectorFromString(@"baseGain")));
    SendObject(filter, NSSelectorFromString(@"setFaceBasedGlobalExposureBoostRatio:"),
               SendId(properties, NSSelectorFromString(@"faceBasedGlobalExposureBoostRatio")));
    SendBoolValue(filter, NSSelectorFromString(@"setUseStyleEngine:"), useStyleEngine);

    CIImage *target = SendId(filter, NSSelectorFromString(@"outputImage"));
    if (capture) {
        *capture = @{
            @"metadataPath": metadataPath,
            @"metadataLength": @(metadata.length),
            @"propertiesClass": NSStringFromClass([properties class]),
            @"version": version ? [version description] : (id)[NSNull null],
            @"versionMinor": @(versionMinor),
            @"styleSettings": JSONSafe(styleSettings),
            @"applyInverseCurveToLinearThumbnail": @(applyInverseCurve),
            @"applyInverseCurveSource": inverseOverride
                ? @"LEARNNODE_APPLY_INVERSE_CURVE"
                : @"static still-image version-minor gate",
            @"useStyleEngine": @(useStyleEngine),
            @"useStyleEngineSource": environment[@"LEARNNODE_USE_STYLE_ENGINE"]
                ? @"LEARNNODE_USE_STYLE_ENGINE"
                : @"absent adjustment setting defaults false",
            @"linearGain": @(linearGain),
            @"linearRangeMin": @(linearRangeMin),
            @"linearRangeMax": @(linearRangeMax),
            @"inputPolicy": @{
                @"main": imagePath,
                @"mainAndMatteRenderSize": @{
                    @"width": @(semanticRenderSize.width),
                    @"height": @(semanticRenderSize.height),
                },
                @"mainAndMatteScaleSource": @"PISemanticStyleRenderNode replay geometry",
                @"linearThumbnail": linearThumbnailPath,
                @"subjectMatte": subjectMattePath,
                @"skinMatte": skinMattePath,
                @"skyMatte": skyMattePath,
                @"gainMap": @"cropped black CIImage; no HEIC gain auxiliary",
            },
            @"filterClass": NSStringFromClass([filter class]),
            @"filterIvars": ObjectIvarSummary(filter),
            @"target": target ? @{
                @"class": NSStringFromClass([target class]),
                @"extent": NSStringFromRect(NSRectFromCGRect(target.extent)),
                @"description": [target description] ?: @"",
            } : (id)[NSNull null],
        };
    }
    return target;
}

static NSDictionary *FindNestedDictionary(
    NSDictionary *settings,
    NSString *preferredKey,
    NSString *requiredKey
) {
    id preferred = settings[preferredKey];
    if ([preferred isKindOfClass:[NSDictionary class]]) {
        return preferred;
    }
    for (id key in settings) {
        id candidate = settings[key];
        if ([candidate isKindOfClass:[NSDictionary class]] && candidate[requiredKey] != nil) {
            return candidate;
        }
    }
    return nil;
}

static BOOL WriteJSON(NSDictionary *object, NSString *path, NSError **error) {
    NSData *data = [NSJSONSerialization dataWithJSONObject:object
                                                   options:NSJSONWritingPrettyPrinted |
                                                           NSJSONWritingSortedKeys
                                                     error:error];
    return data && [data writeToFile:path options:NSDataWritingAtomic error:error];
}

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        BOOL semanticMode = argc == 9 && strcmp(argv[1], "--semantic") == 0;
        BOOL directMode = argc == 4;
        if (!semanticMode && !directMode) {
            fprintf(stderr,
                    "usage: %s input-image target-image output-directory\n"
                    "       %s --semantic image.heic style-metadata.bplist "
                    "linear-thumbnail subject-matte skin-matte sky-matte output-directory\n",
                    argv[0], argv[0]);
            return 2;
        }
        int inputIndex = semanticMode ? 2 : 1;
        int outputIndex = semanticMode ? 8 : 3;
        NSString *inputPath = [[NSString stringWithUTF8String:argv[inputIndex]]
            stringByStandardizingPath];
        NSString *targetPath = directMode
            ? [[NSString stringWithUTF8String:argv[2]] stringByStandardizingPath]
            : @"<PISemanticStyleFilter output>";
        NSString *outputDirectory = [[NSString stringWithUTF8String:argv[outputIndex]]
            stringByStandardizingPath];
        NSError *error = nil;
        if (![[NSFileManager defaultManager] createDirectoryAtPath:outputDirectory
                                       withIntermediateDirectories:YES
                                                        attributes:nil
                                                             error:&error]) {
            fprintf(stderr, "%s\n", error.localizedDescription.UTF8String);
            return 1;
        }

        void *neutrino = dlopen(
            "/System/Library/PrivateFrameworks/NeutrinoCore.framework/NeutrinoCore",
            RTLD_NOW
        );
        void *photoImaging = dlopen(
            "/System/Library/PrivateFrameworks/PhotoImaging.framework/PhotoImaging",
            RTLD_NOW
        );
        void *cmImaging = dlopen(
            "/System/Library/PrivateFrameworks/CMImaging.framework/CMImaging",
            RTLD_NOW
        );
        if (!neutrino || !photoImaging || !cmImaging) {
            fprintf(stderr, "failed to load private frameworks\n");
            return 1;
        }

        NSDictionary *imageOptions = @{
            kCIImageApplyOrientationProperty: @YES,
        };
        CIImage *input = [CIImage imageWithContentsOfURL:[NSURL fileURLWithPath:inputPath]
                                                 options:imageOptions];
        CIImage *target = directMode
            ? [CIImage imageWithContentsOfURL:[NSURL fileURLWithPath:targetPath]
                                      options:imageOptions]
            : nil;
        NSDictionary *semanticCapture = nil;
        NSData *nativeStyleData = nil;
        NSError *semanticError = nil;
        if (semanticMode && input) {
            target = SemanticTarget(
                input,
                inputPath,
                [[NSString stringWithUTF8String:argv[3]] stringByStandardizingPath],
                [[NSString stringWithUTF8String:argv[4]] stringByStandardizingPath],
                [[NSString stringWithUTF8String:argv[5]] stringByStandardizingPath],
                [[NSString stringWithUTF8String:argv[6]] stringByStandardizingPath],
                [[NSString stringWithUTF8String:argv[7]] stringByStandardizingPath],
                &semanticCapture,
                &nativeStyleData,
                &semanticError
            );
        }
        if (!input || !target) {
            NSDictionary *failure = @{
                @"schema": @"learnnode-coefficient-probe-v2",
                @"mode": semanticMode ? @"semantic" : @"direct",
                @"input": inputPath,
                @"target": targetPath,
                @"semantic": semanticCapture ?: (id)[NSNull null],
                @"semanticError": JSONSafe(semanticError),
            };
            WriteJSON(failure, [outputDirectory stringByAppendingPathComponent:@"probe.json"], NULL);
            fprintf(stderr, "failed to load input or target CIImage\n");
            return 1;
        }

        NUIntegerPair scale = {1, 1};
        NUIntegerPair aspect = {
            (int64_t)llround(input.extent.size.width),
            (int64_t)llround(input.extent.size.height),
        };
        Class nodeClass = NSClassFromString(@"NUStyleTransferNode");
        NSDictionary *settings = SendClassPairPair(
            nodeClass,
            NSSelectorFromString(@"semanticStyleImageSettingsForScale:aspectRatio:"),
            scale,
            aspect
        );
        NSDictionary *baseConfiguration = FindNestedDictionary(
            settings,
            @"configuration",
            @"spotlightCountX"
        );
        NSDictionary *baseTuning = FindNestedDictionary(
            settings,
            @"tuningParameters",
            @"StylePriorStrength"
        );
        if (!baseConfiguration || !baseTuning) {
            NSDictionary *failure = @{
                @"schema": @"learnnode-coefficient-probe-v1",
                @"error": @"failed to locate configuration or tuning dictionary",
                @"settings": JSONSafe(settings),
            };
            WriteJSON(failure, [outputDirectory stringByAppendingPathComponent:@"probe.json"], NULL);
            fprintf(stderr, "failed to locate configuration or tuning dictionary\n");
            return 1;
        }

        NSMutableDictionary *configuration = [baseConfiguration mutableCopy];
        NSMutableDictionary *tuning = [baseTuning mutableCopy];
        if (directMode) {
            // Same-image learning is a deterministic source-derived identity baseline.
            configuration[@"applyDithering"] = @NO;
            configuration[@"applySyntheticNoise"] = @NO;
        }
        Class filterClass = NSClassFromString(@"PISemanticStyleFilter");
        NSDictionary *requestedStyle = semanticMode ? semanticCapture[@"styleSettings"] : nil;
        NSString *cast = requestedStyle[@"cast"] ?: @"Standard";
        float tone = requestedStyle ? [requestedStyle[@"tone"] floatValue] : 0.0f;
        float color = requestedStyle ? [requestedStyle[@"color"] floatValue] : 0.0f;
        float intensity = requestedStyle ? [requestedStyle[@"intensity"] floatValue] : 1.0f;
        NSDictionary *castTuning = ((id (*)(id, SEL, id))objc_msgSend)(
            (id)filterClass,
            NSSelectorFromString(@"styleTuningParametersForCast:"),
            cast
        );
        if (castTuning) {
            [tuning addEntriesFromDictionary:castTuning];
        }
        float priorStrength = [tuning[@"StylePriorStrength"] floatValue];
        NSData *prior = SendClassCastFloats(
            filterClass,
            NSSelectorFromString(@"stylePriorDataForCast:tone:color:intensity:priorStrength:"),
            cast,
            tone,
            color,
            intensity,
            priorStrength
        );
        configuration[@"useFloat16"] = @YES;
        if (prior) {
            configuration[@"priorMatrix"] = prior;
        }

        Class wrapperClass = NSClassFromString(@"_NUStyleEngineConfiguration");
        id wrapper = ((id (*)(id, SEL, id))objc_msgSend)(
            ((id (*)(id, SEL))objc_msgSend)((id)wrapperClass, sel_registerName("alloc")),
            NSSelectorFromString(@"initWithConfigurationDictionary:"),
            configuration
        );
        CGSize thumbnailSize = ((CGSize (*)(id, SEL))objc_msgSend)(
            wrapper,
            NSSelectorFromString(@"thumbnailSize")
        );
        CGSize coefficientSize = SendClassDictionarySize(
            wrapperClass,
            NSSelectorFromString(@"coefficientTextureSizeForConfigurationDictionary:"),
            configuration
        );
        id nuColorSpace = SendId(
            (id)NSClassFromString(@"NUColorSpace"),
            NSSelectorFromString(@"workingColorSpace")
        );
        NUIntegerPair thumbnailTargetSize = {
            (int64_t)llround(thumbnailSize.width),
            (int64_t)llround(thumbnailSize.height),
        };
        Class thumbnailProcessorClass = NSClassFromString(@"_NUStyleTransferThumbnailProcessor");
        NSError *inputThumbnailError = nil;
        NSError *targetThumbnailError = nil;
        CIImage *inputThumbnail = SendClassThumbnail(
            thumbnailProcessorClass,
            NSSelectorFromString(@"generateThumbnailForImage:targetSize:colorSpace:configuration:tuningParameters:error:"),
            input,
            thumbnailTargetSize,
            nuColorSpace,
            configuration,
            tuning,
            &inputThumbnailError
        );
        CIImage *targetThumbnail = SendClassThumbnail(
            thumbnailProcessorClass,
            NSSelectorFromString(@"generateThumbnailForImage:targetSize:colorSpace:configuration:tuningParameters:error:"),
            target,
            thumbnailTargetSize,
            nuColorSpace,
            configuration,
            tuning,
            &targetThumbnailError
        );
        if (!inputThumbnail || !targetThumbnail) {
            NSDictionary *failure = @{
                @"schema": @"learnnode-coefficient-probe-v2",
                @"mode": semanticMode ? @"semantic" : @"direct",
                @"inputThumbnailError": JSONSafe(inputThumbnailError),
                @"targetThumbnailError": JSONSafe(targetThumbnailError),
                @"semantic": semanticCapture ?: (id)[NSNull null],
            };
            WriteJSON(failure, [outputDirectory stringByAppendingPathComponent:@"probe.json"], NULL);
            fprintf(stderr, "failed to create exact style-transfer thumbnails\n");
            return 1;
        }

        Class learnProcessorClass = NSClassFromString(@"_NUStyleTransferLearnProcessor");
        Method learnMethod = class_getClassMethod(
            learnProcessorClass,
            NSSelectorFromString(@"processWithInputs:arguments:output:error:")
        );
        Class cmiProcessorClass = NSClassFromString(@"CMIStyleEngineProcessor");
        Method cmiMethod = class_getInstanceMethod(cmiProcessorClass, NSSelectorFromString(@"process"));
        Class semanticProcessorClass = NSClassFromString(@"PISemanticStyleProcessor");
        Method semanticMethod = class_getClassMethod(
            semanticProcessorClass,
            NSSelectorFromString(@"processWithInputs:arguments:output:error:")
        );
        Class semanticRendererClass = NSClassFromString(@"PISemanticStyleRenderer");
        Method usingSharedMethod = class_getClassMethod(
            semanticRendererClass,
            NSSelectorFromString(@"usingSharedSemanticStyleRendererWithMetalCommandQueue:processingType:useStyleEngine:perform:")
        );
        Class smartStyleRendererClass = NSClassFromString(@"CMISmartStyleMetalRendererV1");
        NSArray<NSString *> *smartStyleStatusSelectors = @[
            @"setup",
            @"prepareToProcess:",
            @"process",
            @"finishProcessing",
            @"_updateRenderPipelineConfigForInputs",
            @"_setupStatsAndRenderParamBuffer",
            @"_processSegmentationMasks",
            @"_processLTMGainMap",
            @"_calculateDynamicRenderParameters",
            @"_applyFinalRendering",
        ];
        Class guidedFilterClass = NSClassFromString(@"CMIGuidedFilter");
        Method guidedFilterMethod = class_getInstanceMethod(
            guidedFilterClass,
            NSSelectorFromString(@"encodeToCommandBuffer:inputTexture:guideTexture:outputTexture:kernelRadius:epsilon:")
        );
        gOriginalLearnProcess = method_getImplementation(learnMethod);
        gOriginalCMIProcess = method_getImplementation(cmiMethod);
        gOriginalSemanticProcess = method_getImplementation(semanticMethod);
        gOriginalUsingSharedRenderer = method_getImplementation(usingSharedMethod);
        gOriginalGuidedFilterEncode = method_getImplementation(guidedFilterMethod);
        gOriginalSmartStyleMethods = [NSMutableDictionary dictionary];
        gEvents = [NSMutableArray array];
        method_setImplementation(learnMethod, (IMP)RecordingLearnProcess);
        method_setImplementation(cmiMethod, (IMP)RecordingCMIProcess);
        method_setImplementation(semanticMethod, (IMP)RecordingSemanticProcess);
        method_setImplementation(usingSharedMethod, (IMP)RecordingUsingSharedRenderer);
        method_setImplementation(guidedFilterMethod, (IMP)RecordingGuidedFilterEncode);
        for (NSString *selectorName in smartStyleStatusSelectors) {
            Method method = class_getInstanceMethod(
                smartStyleRendererClass,
                NSSelectorFromString(selectorName)
            );
            IMP original = method_getImplementation(method);
            gOriginalSmartStyleMethods[selectorName] = [NSValue valueWithPointer:original];
            IMP recording = [selectorName isEqualToString:@"prepareToProcess:"]
                ? (IMP)RecordingSmartStylePrepare
                : (IMP)RecordingSmartStyleStatus;
            method_setImplementation(method, recording);
        }

        CIImage *learnedImage = nil;
        NSError *learnError = nil;
        NSError *renderError = nil;
        NSMutableData *raw = nil;
        BOOL hooksRestored = NO;
        BOOL swapLearnDirection = [NSProcessInfo processInfo]
            .environment[@"LEARNNODE_SWAP_INPUT_TARGET"].boolValue;
        CIImage *learnSourceThumbnail = swapLearnDirection
            ? targetThumbnail
            : inputThumbnail;
        CIImage *learnTargetThumbnail = swapLearnDirection
            ? inputThumbnail
            : targetThumbnail;
        id<MTLDevice> device = MTLCreateSystemDefaultDevice();
        CIContext *context = [CIContext contextWithMTLDevice:device options:@{
            kCIContextWorkingColorSpace: [NSNull null],
            kCIContextOutputColorSpace: [NSNull null],
        }];
        @try {
            learnedImage = SendClassLearn(
                learnProcessorClass,
                NSSelectorFromString(@"learnStyleFromInputThumbnail:targetThumbnail:colorSpace:configuration:tuningParameters:error:"),
                learnSourceThumbnail,
                learnTargetThumbnail,
                nuColorSpace,
                configuration,
                tuning,
                &learnError
            );
            if (learnedImage) {
                size_t width = (size_t)llround(learnedImage.extent.size.width);
                size_t height = (size_t)llround(learnedImage.extent.size.height);
                size_t rowBytes = width * sizeof(uint16_t);
                raw = [NSMutableData dataWithLength:rowBytes * height];
                @try {
                    [context render:learnedImage
                           toBitmap:raw.mutableBytes
                           rowBytes:rowBytes
                             bounds:learnedImage.extent
                             format:kCIFormatRh
                         colorSpace:NULL];
                } @catch (NSException *exception) {
                    renderError = [NSError errorWithDomain:@"LearnNodeCoefficientProbe"
                                                       code:2
                                                   userInfo:@{
                        NSLocalizedDescriptionKey: exception.reason ?: exception.name,
                    }];
                }
            }
        } @finally {
            method_setImplementation(learnMethod, gOriginalLearnProcess);
            method_setImplementation(cmiMethod, gOriginalCMIProcess);
            method_setImplementation(semanticMethod, gOriginalSemanticProcess);
            method_setImplementation(usingSharedMethod, gOriginalUsingSharedRenderer);
            method_setImplementation(guidedFilterMethod, gOriginalGuidedFilterEncode);
            for (NSString *selectorName in smartStyleStatusSelectors) {
                Method method = class_getInstanceMethod(
                    smartStyleRendererClass,
                    NSSelectorFromString(selectorName)
                );
                method_setImplementation(
                    method,
                    [gOriginalSmartStyleMethods[selectorName] pointerValue]
                );
            }
            hooksRestored = method_getImplementation(learnMethod) == gOriginalLearnProcess &&
                method_getImplementation(cmiMethod) == gOriginalCMIProcess &&
                method_getImplementation(semanticMethod) == gOriginalSemanticProcess &&
                method_getImplementation(usingSharedMethod) == gOriginalUsingSharedRenderer &&
                method_getImplementation(guidedFilterMethod) == gOriginalGuidedFilterEncode;
            for (NSString *selectorName in smartStyleStatusSelectors) {
                Method method = class_getInstanceMethod(
                    smartStyleRendererClass,
                    NSSelectorFromString(selectorName)
                );
                hooksRestored = hooksRestored &&
                    method_getImplementation(method) ==
                        [gOriginalSmartStyleMethods[selectorName] pointerValue];
            }
        }

        NSString *rawPath = [outputDirectory stringByAppendingPathComponent:@"learned_style.f16.bin"];
        if (raw && !renderError) {
            [raw writeToFile:rawPath options:NSDataWritingAtomic error:&renderError];
        }
        Class applyProcessorClass = NSClassFromString(@"_NUStyleTransferApplyProcessor");
        SEL applySelector = NSSelectorFromString(
            @"applyStyle:toImage:thumbnail:target:deltaMap:colorSpace:configuration:tuningParameters:noiseModel:error:"
        );
        NSError *learnedApplyError = nil;
        NSError *nativeApplyError = nil;
        CIImage *learnedAppliedImage = learnedImage ? SendClassApply(
            applyProcessorClass,
            applySelector,
            learnedImage,
            learnSourceThumbnail,
            learnSourceThumbnail,
            learnTargetThumbnail,
            nil,
            nuColorSpace,
            configuration,
            tuning,
            nil,
            &learnedApplyError
        ) : nil;
        NSUInteger coefficientRowBytes =
            (NSUInteger)llround(coefficientSize.width) * sizeof(uint16_t);
        NSUInteger expectedCoefficientBytes = coefficientRowBytes *
            (NSUInteger)llround(coefficientSize.height);
        CIImage *nativeStyleImage = nativeStyleData.length == expectedCoefficientBytes
            ? [CIImage imageWithBitmapData:nativeStyleData
                               bytesPerRow:coefficientRowBytes
                                      size:coefficientSize
                                    format:kCIFormatRh
                                colorSpace:nil]
            : nil;
        CIImage *nativeAppliedImage = nativeStyleImage ? SendClassApply(
            applyProcessorClass,
            applySelector,
            nativeStyleImage,
            learnSourceThumbnail,
            learnSourceThumbnail,
            learnTargetThumbnail,
            nil,
            nuColorSpace,
            configuration,
            tuning,
            nil,
            &nativeApplyError
        ) : nil;
        NSString *inputThumbnailPath = [outputDirectory
            stringByAppendingPathComponent:@"input_thumbnail.rgba16f.bin"];
        NSString *targetThumbnailPath = [outputDirectory
            stringByAppendingPathComponent:@"target_thumbnail.rgba16f.bin"];
        NSString *learnedAppliedPath = [outputDirectory
            stringByAppendingPathComponent:@"learned_applied_thumbnail.rgba16f.bin"];
        NSString *nativeAppliedPath = [outputDirectory
            stringByAppendingPathComponent:@"native_key1_applied_thumbnail.rgba16f.bin"];
        NSError *inputThumbnailCaptureError = nil;
        NSError *targetThumbnailCaptureError = nil;
        NSError *learnedAppliedCaptureError = nil;
        NSError *nativeAppliedCaptureError = nil;
        BOOL inputThumbnailCaptured = RenderHalfRGBA(
            context,
            inputThumbnail,
            inputThumbnailPath,
            &inputThumbnailCaptureError
        );
        BOOL targetThumbnailCaptured = RenderHalfRGBA(
            context,
            targetThumbnail,
            targetThumbnailPath,
            &targetThumbnailCaptureError
        );
        BOOL learnedAppliedCaptured = learnedAppliedImage && RenderHalfRGBA(
            context,
            learnedAppliedImage,
            learnedAppliedPath,
            &learnedAppliedCaptureError
        );
        BOOL nativeAppliedCaptured = nativeAppliedImage && RenderHalfRGBA(
            context,
            nativeAppliedImage,
            nativeAppliedPath,
            &nativeAppliedCaptureError
        );
        NSDictionary *result = @{
            @"schema": @"learnnode-coefficient-probe-v2",
            @"mode": semanticMode ? @"semantic" : @"direct",
            @"input": inputPath,
            @"target": targetPath,
            @"inputExtent": NSStringFromRect(NSRectFromCGRect(input.extent)),
            @"targetExtent": NSStringFromRect(NSRectFromCGRect(target.extent)),
            @"semantic": semanticCapture ?: (id)[NSNull null],
            @"semanticError": JSONSafe(semanticError),
            @"settings": JSONSafe(settings),
            @"configuration": JSONSafe(configuration),
            @"tuning": JSONSafe(tuning),
            @"requestedStyle": @{
                @"cast": cast,
                @"tone": @(tone),
                @"color": @(color),
                @"intensity": @(intensity),
            },
            @"learnDirection": @{
                @"swapped": @(swapLearnDirection),
                @"source": swapLearnDirection
                    ? @"semantic target thumbnail"
                    : @"input thumbnail",
                @"target": swapLearnDirection
                    ? @"input thumbnail"
                    : @"semantic target thumbnail",
                @"sourceEnvironment": [NSProcessInfo processInfo]
                    .environment[@"LEARNNODE_SWAP_INPUT_TARGET"]
                        ? @"LEARNNODE_SWAP_INPUT_TARGET"
                        : @"default",
            },
            @"priorLength": @(prior.length),
            @"priorStrength": @(priorStrength),
            @"thumbnailSize": @{
                @"width": @(thumbnailSize.width),
                @"height": @(thumbnailSize.height),
            },
            @"coefficientTextureSize": @{
                @"width": @(coefficientSize.width),
                @"height": @(coefficientSize.height),
            },
            @"inputThumbnail": @{
                @"class": NSStringFromClass([inputThumbnail class]),
                @"description": [inputThumbnail description] ?: @"",
                @"extent": NSStringFromRect(NSRectFromCGRect(inputThumbnail.extent)),
                @"creationError": JSONSafe(inputThumbnailError),
                @"capturePath": inputThumbnailPath,
                @"captureFormat": @"kCIFormatRGBAh",
                @"captureLength": inputThumbnailCaptured
                    ? @((NSUInteger)llround(inputThumbnail.extent.size.width) *
                        (NSUInteger)llround(inputThumbnail.extent.size.height) * 8)
                    : (id)[NSNull null],
                @"captureError": JSONSafe(inputThumbnailCaptureError),
            },
            @"targetThumbnail": @{
                @"class": NSStringFromClass([targetThumbnail class]),
                @"description": [targetThumbnail description] ?: @"",
                @"extent": NSStringFromRect(NSRectFromCGRect(targetThumbnail.extent)),
                @"creationError": JSONSafe(targetThumbnailError),
                @"capturePath": targetThumbnailPath,
                @"captureFormat": @"kCIFormatRGBAh",
                @"captureLength": targetThumbnailCaptured
                    ? @((NSUInteger)llround(targetThumbnail.extent.size.width) *
                        (NSUInteger)llround(targetThumbnail.extent.size.height) * 8)
                    : (id)[NSNull null],
                @"captureError": JSONSafe(targetThumbnailCaptureError),
            },
            @"behavioralApply": @{
                @"processorClass": NSStringFromClass(applyProcessorClass),
                @"selector": NSStringFromSelector(applySelector),
                @"imageArgumentPolicy": swapLearnDirection
                    ? @"semantic target used for image and thumbnail; input thumbnail supplied as target"
                    : @"input thumbnail used for image and thumbnail; semantic target thumbnail supplied as target",
                @"learned": @{
                    @"outputClass": learnedAppliedImage
                        ? NSStringFromClass([learnedAppliedImage class])
                        : (id)[NSNull null],
                    @"outputExtent": learnedAppliedImage
                        ? NSStringFromRect(NSRectFromCGRect(learnedAppliedImage.extent))
                        : (id)[NSNull null],
                    @"creationError": JSONSafe(learnedApplyError),
                    @"capturePath": learnedAppliedCaptured
                        ? learnedAppliedPath
                        : (id)[NSNull null],
                    @"captureError": JSONSafe(learnedAppliedCaptureError),
                },
                @"nativeKey1": @{
                    @"styleDataLength": @(nativeStyleData.length),
                    @"coefficientImageCreated": @(nativeStyleImage != nil),
                    @"outputClass": nativeAppliedImage
                        ? NSStringFromClass([nativeAppliedImage class])
                        : (id)[NSNull null],
                    @"outputExtent": nativeAppliedImage
                        ? NSStringFromRect(NSRectFromCGRect(nativeAppliedImage.extent))
                        : (id)[NSNull null],
                    @"creationError": JSONSafe(nativeApplyError),
                    @"capturePath": nativeAppliedCaptured
                        ? nativeAppliedPath
                        : (id)[NSNull null],
                    @"captureError": JSONSafe(nativeAppliedCaptureError),
                },
            },
            @"learnedObject": learnedImage ? @{
                @"class": NSStringFromClass([learnedImage class]),
                @"isCIImage": @([learnedImage isKindOfClass:[CIImage class]]),
                @"description": [learnedImage description] ?: @"",
                @"extent": NSStringFromRect(NSRectFromCGRect(learnedImage.extent)),
                @"ivars": ObjectIvarSummary(learnedImage),
            } : [NSNull null],
            @"learnError": JSONSafe(learnError),
            @"renderError": JSONSafe(renderError),
            @"rawOutput": raw && !renderError ? @{
                @"path": rawPath,
                @"length": @(raw.length),
                @"rowBytes": @((NSUInteger)llround(coefficientSize.width) * sizeof(uint16_t)),
                @"format": @"kCIFormatRh",
            } : [NSNull null],
            @"events": gEvents,
            @"hooksRestored": @(hooksRestored),
            @"claimBoundary": @"Local private-framework invocation and reversible in-process Objective-C hooks; no system framework was modified on disk.",
        };
        NSString *jsonPath = [outputDirectory stringByAppendingPathComponent:@"probe.json"];
        if (!WriteJSON(result, jsonPath, &error)) {
            fprintf(stderr, "%s\n", error.localizedDescription.UTF8String);
            return 1;
        }
        printf("%s\n", jsonPath.UTF8String);
        return learnedImage && raw && !renderError &&
            inputThumbnailCaptured && targetThumbnailCaptured ? 0 : 1;
    }
}
