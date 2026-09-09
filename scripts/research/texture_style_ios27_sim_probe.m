#import <UIKit/UIKit.h>
#import <Foundation/Foundation.h>
#import <ImageIO/ImageIO.h>
#import <objc/message.h>
#import <objc/runtime.h>
#import <dlfcn.h>

static NSURL *DocumentsURL(void) {
    return [[[NSFileManager defaultManager] URLsForDirectory:NSDocumentDirectory
                                                   inDomains:NSUserDomainMask] firstObject];
}

static void WriteCheckpoint(NSString *stage, id value) {
    NSMutableDictionary *record = [NSMutableDictionary dictionary];
    record[@"stage"] = stage ?: @"<nil>";
    record[@"value"] = value ?: [NSNull null];
    record[@"os"] = NSProcessInfo.processInfo.operatingSystemVersionString ?: @"";
    NSData *data = [NSJSONSerialization dataWithJSONObject:record
                                                   options:NSJSONWritingPrettyPrinted | NSJSONWritingSortedKeys
                                                     error:nil];
    if (data) {
        NSString *safe = [[stage ?: @"stage" componentsSeparatedByCharactersInSet:
            [[NSCharacterSet alphanumericCharacterSet] invertedSet]] componentsJoinedByString:@"-"];
        NSURL *url = [DocumentsURL() URLByAppendingPathComponent:
            [NSString stringWithFormat:@"checkpoint-%@.json", safe]];
        [data writeToURL:url atomically:YES];
    }
}

static void *LookupSymbol(NSString *name) {
    if (!name.length) return NULL;
    void *symbol = dlsym(RTLD_DEFAULT, name.UTF8String);
    if (!symbol && [name hasPrefix:@"_"] && name.length > 1) {
        symbol = dlsym(RTLD_DEFAULT, [[name substringFromIndex:1] UTF8String]);
    }
    if (!symbol && ![name hasPrefix:@"_"]) {
        NSString *underscored = [@"_" stringByAppendingString:name];
        symbol = dlsym(RTLD_DEFAULT, underscored.UTF8String);
    }
    return symbol;
}

static id SymbolRawInfo(NSString *name) {
    void *symbol = LookupSymbol(name);
    if (!symbol) return [NSNull null];
    unsigned char bytes[16] = {0};
    memcpy(bytes, symbol, sizeof(bytes));
    NSMutableString *hex = [NSMutableString stringWithCapacity:32];
    for (NSUInteger i = 0; i < sizeof(bytes); ++i) [hex appendFormat:@"%02x", bytes[i]];
    return @{
        @"address": [NSString stringWithFormat:@"%p", symbol],
        @"first16BytesHex": hex,
    };
}

static id SafeObject(id obj) {
    if (!obj) return [NSNull null];
    if ([obj isKindOfClass:[NSString class]] ||
        [obj isKindOfClass:[NSNumber class]] ||
        [obj isKindOfClass:[NSNull class]]) return obj;
    if ([obj isKindOfClass:[NSData class]]) {
        NSData *data = obj;
        return @{ @"class": NSStringFromClass([obj class]) ?: @"NSData",
                  @"bytes": @(data.length),
                  @"base64": [data base64EncodedStringWithOptions:0] };
    }
    if ([obj isKindOfClass:[NSArray class]]) {
        NSMutableArray *array = [NSMutableArray array];
        for (id value in (NSArray *)obj) [array addObject:SafeObject(value)];
        return array;
    }
    if ([obj isKindOfClass:[NSDictionary class]]) {
        NSMutableDictionary *dictionary = [NSMutableDictionary dictionary];
        for (id key in (NSDictionary *)obj) {
            dictionary[[key description]] = SafeObject([(NSDictionary *)obj objectForKey:key]);
        }
        return dictionary;
    }
    return @{ @"class": NSStringFromClass([obj class]) ?: @"<unknown>",
              @"description": [obj description] ?: @"<nil>" };
}

static id BoxInvocationReturn(NSInvocation *invocation, NSMethodSignature *signature) {
    const char *type = signature.methodReturnType;
    while (*type && strchr("rnNoORV", *type)) type++;
    if (*type == 'v') return [NSNull null];
    if (*type == '@' || *type == '#') {
        __unsafe_unretained id value = nil;
        [invocation getReturnValue:&value];
        return value ?: [NSNull null];
    }
    if (*type == 'B' || *type == 'c') {
        BOOL value = NO; [invocation getReturnValue:&value]; return @(value);
    }
    if (*type == 'f') {
        float value = 0; [invocation getReturnValue:&value]; return @(value);
    }
    if (*type == 'd') {
        double value = 0; [invocation getReturnValue:&value]; return @(value);
    }
    if (strchr("islqISLQ", *type)) {
        unsigned char buffer[sizeof(unsigned long long)] = {0};
        [invocation getReturnValue:buffer];
        unsigned long long value = 0;
        memcpy(&value, buffer, MIN(sizeof(value), signature.methodReturnLength));
        return @(value);
    }
    NSMutableData *data = [NSMutableData dataWithLength:signature.methodReturnLength];
    [invocation getReturnValue:data.mutableBytes];
    return @{ @"type": [NSString stringWithUTF8String:type] ?: @"?",
              @"base64": [data base64EncodedStringWithOptions:0] };
}

static id Invoke0(id target, NSString *selectorName) {
    SEL selector = NSSelectorFromString(selectorName);
    if (!target || ![target respondsToSelector:selector]) return nil;
    @try {
        NSMethodSignature *signature = [target methodSignatureForSelector:selector];
        if (!signature) return nil;
        NSInvocation *invocation = [NSInvocation invocationWithMethodSignature:signature];
        invocation.target = target;
        invocation.selector = selector;
        [invocation invoke];
        return BoxInvocationReturn(invocation, signature);
    } @catch (NSException *exception) {
        return @{ @"exception": exception.reason ?: exception.name };
    }
}

static id InvokeObject1(id target, NSString *selectorName, id argument) {
    SEL selector = NSSelectorFromString(selectorName);
    if (!target || ![target respondsToSelector:selector]) return nil;
    @try {
        NSMethodSignature *signature = [target methodSignatureForSelector:selector];
        if (!signature || signature.numberOfArguments < 3) return nil;
        NSInvocation *invocation = [NSInvocation invocationWithMethodSignature:signature];
        invocation.target = target;
        invocation.selector = selector;
        __unsafe_unretained id arg = argument;
        [invocation setArgument:&arg atIndex:2];
        [invocation invoke];
        return BoxInvocationReturn(invocation, signature);
    } @catch (NSException *exception) {
        return @{ @"exception": exception.reason ?: exception.name };
    }
}

static NSDictionary *ClassSurface(NSString *name) {
    Class cls = NSClassFromString(name);
    if (!cls) return @{ @"available": @NO };
    NSMutableArray *classMethods = [NSMutableArray array];
    unsigned int classCount = 0;
    Method *cm = class_copyMethodList(object_getClass(cls), &classCount);
    for (unsigned int i = 0; i < classCount; ++i) {
        [classMethods addObject:@{
            @"selector": NSStringFromSelector(method_getName(cm[i])),
            @"types": [NSString stringWithUTF8String:method_getTypeEncoding(cm[i])] ?: @"",
        }];
    }
    free(cm);
    NSMutableArray *instanceMethods = [NSMutableArray array];
    unsigned int instanceCount = 0;
    Method *im = class_copyMethodList(cls, &instanceCount);
    for (unsigned int i = 0; i < instanceCount; ++i) {
        [instanceMethods addObject:@{
            @"selector": NSStringFromSelector(method_getName(im[i])),
            @"types": [NSString stringWithUTF8String:method_getTypeEncoding(im[i])] ?: @"",
        }];
    }
    free(im);
    return @{ @"available": @YES,
              @"classMethods": classMethods,
              @"instanceMethods": instanceMethods };
}

typedef id (*DictionaryTransformFunction)(id);

static id CallDictionaryTransform(NSString *symbolName, NSDictionary *input) {
    void *address = LookupSymbol(symbolName);
    if (!address) return @{ @"available": @NO };
    @try {
        DictionaryTransformFunction fn = (DictionaryTransformFunction)address;
        id result = fn(input ?: @{});
        return @{ @"available": @YES, @"result": SafeObject(result) };
    } @catch (NSException *exception) {
        return @{ @"available": @YES, @"exception": exception.reason ?: exception.name };
    }
}

static NSDictionary *TexturePropertiesTrial(NSDictionary *imageMetadata, NSDictionary *auxMetadata) {
    Class cls = NSClassFromString(@"_NUTextureStyleProperties");
    SEL sel = NSSelectorFromString(@"textureStylePropertiesFromImageMetadata:auxImageMetadata:error:");
    if (!cls || ![cls respondsToSelector:sel]) return @{ @"available": @NO };
    NSError *error = nil;
    @try {
        id result = ((id (*)(id, SEL, id, id, NSError **))objc_msgSend)(
            cls, sel, imageMetadata ?: @{}, auxMetadata ?: @{}, &error
        );
        NSMutableDictionary *out = [NSMutableDictionary dictionaryWithObject:@YES forKey:@"available"];
        out[@"result"] = SafeObject(result);
        if (error) {
            out[@"errorDomain"] = error.domain ?: @"";
            out[@"errorCode"] = @(error.code);
            out[@"errorDescription"] = error.localizedDescription ?: error.description;
            out[@"errorUserInfo"] = SafeObject(error.userInfo);
        }
        return out;
    } @catch (NSException *exception) {
        return @{ @"available": @YES, @"exception": exception.reason ?: exception.name };
    }
}

static NSDictionary *BuildProbe(void) {
    WriteCheckpoint(@"00-start", @{ @"pid": @(NSProcessInfo.processInfo.processIdentifier) });

    NSArray<NSString *> *frameworks = @[
        @"/System/Library/PrivateFrameworks/CMImaging.framework/CMImaging",
        @"/System/Library/PrivateFrameworks/CMCaptureCore.framework/CMCaptureCore",
        @"/System/Library/PrivateFrameworks/AVFCapture.framework/AVFCapture",
        @"/System/Library/PrivateFrameworks/NeutrinoCore.framework/NeutrinoCore",
        @"/System/Library/PrivateFrameworks/PhotoImaging.framework/PhotoImaging",
        @"/System/Library/PrivateFrameworks/PhotosFormats.framework/PhotosFormats",
        @"/System/Library/PrivateFrameworks/CameraEditKit.framework/CameraEditKit"
    ];
    NSMutableDictionary *loaded = [NSMutableDictionary dictionary];
    for (NSString *path in frameworks) {
        dlerror();
        void *handle = dlopen(path.UTF8String, RTLD_NOW | RTLD_GLOBAL);
        const char *error = handle ? NULL : dlerror();
        loaded[path] = handle ? @YES : @{ @"loaded": @NO,
                                          @"error": error ? [NSString stringWithUTF8String:error] : @"unknown" };
    }
    WriteCheckpoint(@"10-frameworks", SafeObject(loaded));

    NSArray<NSString *> *symbols = @[
        @"AVAppleMakerNote_TextureStyleKey_Preset",
        @"AVAppleMakerNote_TextureStyleKey_Intensity",
        @"AVAppleMakerNote_TextureStyleKey_Grain",
        @"AVAppleMakerNote_TextureStyleKey_RenderingVersion",
        @"AVAppleMakerNote_TextureStyleKey_OriginalInsteadOfReversibility",
        @"PITextureStyleAdjustmentKey",
        @"PITextureStyleCurrentMetadataVersion",
        @"PITextureStyleSettingsFromMakerNoteProperties",
        @"PITextureStylePresetFromMakerNoteValue",
        @"PITextureStylePresetFromString",
        @"PISemanticStyleSettingsFromMakerNoteProperties",
        @"kMetadataIdentifier_TextureStyleInfo",
    ];
    NSMutableDictionary *symbolInfo = [NSMutableDictionary dictionary];
    for (NSString *name in symbols) symbolInfo[name] = SymbolRawInfo(name);
    WriteCheckpoint(@"20-symbols", symbolInfo);

    NSArray<NSString *> *classes = @[
        @"AVCaptureTextureStyle",
        @"CEKTextureStyle",
        @"CMITextureStyle",
        @"CMITextureStyleTuningLookup",
        @"_NUTextureStyleProperties",
        @"_NUTextureStylePersonInstanceProperties",
        @"PIAdjustmentConstants",
        @"PISchema",
        @"PITextureStyle",
        @"PITextureStyleAdjustmentController",
        @"PITextureStyleAutoCalculator",
        @"PITextureStylePipelineProcessor",
        @"PFMetadata",
        @"PFMetadataImage",
    ];
    NSMutableDictionary *classSurfaces = [NSMutableDictionary dictionary];
    for (NSString *name in classes) classSurfaces[name] = ClassSurface(name);
    WriteCheckpoint(@"30-classes", classSurfaces);

    NSMutableDictionary *api = [NSMutableDictionary dictionary];
    NSArray<NSArray<NSString *> *> *calls = @[
        @[ @"PIAdjustmentConstants", @"PITextureStyleAdjustmentKey" ],
        @[ @"PISchema", @"textureStyleSchema" ],
        @[ @"PITextureStyle", @"identifier" ],
        @[ @"PITextureStyle", @"adjustmentFormat" ],
        @[ @"PITextureStyle", @"adjustmentDescriptor" ],
        @[ @"PITextureStyle", @"availableOptions" ],
        @[ @"PITextureStyleAdjustmentController", @"allPresets" ],
        @[ @"PITextureStyleAdjustmentController", @"presetKey" ],
        @[ @"PITextureStyleAdjustmentController", @"intensityKey" ],
        @[ @"PITextureStyleAdjustmentController", @"grainIntensityKey" ],
        @[ @"AVCaptureTextureStyle", @"identityStyle" ],
        @[ @"CEKTextureStyle", @"defaultStyles" ],
        @[ @"CEKTextureStyle", @"identityStyle" ],
    ];
    for (NSArray<NSString *> *call in calls) {
        id value = Invoke0(NSClassFromString(call[0]), call[1]);
        api[[NSString stringWithFormat:@"%@.%@", call[0], call[1]]] = SafeObject(value);
    }
    WriteCheckpoint(@"40-api", api);

    id presets = Invoke0(NSClassFromString(@"PITextureStyleAdjustmentController"), @"allPresets");
    NSMutableDictionary *presetDictionaries = [NSMutableDictionary dictionary];
    if ([presets isKindOfClass:[NSArray class]]) {
        for (id preset in (NSArray *)presets) {
            id value = InvokeObject1(NSClassFromString(@"PITextureStylePipelineProcessor"),
                                     @"defaultTextureStyleDictionaryForPreset:", preset);
            presetDictionaries[[preset description]] = SafeObject(value);
        }
    }
    WriteCheckpoint(@"50-presets", presetDictionaries);

    // The current XDRemux baseline's Apple iOS MakerNote tag 84 decodes to this
    // compact SemanticStyle property dictionary. Probe both the semantic and the
    // new TextureStyle decoders with the exact baseline object before any guesses.
    NSDictionary *baseline84 = @{
        @"7": @0, @"3": @1, @"4": @1, @"0": @1,
        @"5": @1, @"1": @0, @"6": @4, @"2": @0,
    };

    NSMutableDictionary *makerNoteTransforms = [NSMutableDictionary dictionary];
    makerNoteTransforms[@"semantic-baseline84"] =
        CallDictionaryTransform(@"PISemanticStyleSettingsFromMakerNoteProperties", baseline84);
    makerNoteTransforms[@"texture-baseline84"] =
        CallDictionaryTransform(@"PITextureStyleSettingsFromMakerNoteProperties", baseline84);
    makerNoteTransforms[@"texture-wrapper84"] =
        CallDictionaryTransform(@"PITextureStyleSettingsFromMakerNoteProperties", @{ @"84": baseline84 });

    NSArray<NSDictionary *> *textureInputs = @[
        @{ @"Preset": @"Standard", @"Intensity": @1.0, @"Grain": @0.0 },
        @{ @"preset": @"Standard", @"intensity": @1.0, @"grainIntensity": @0.0 },
        @{ @"TextureStylePreset": @"Standard", @"TextureStyleIntensity": @1.0, @"TextureStyleGrain": @0.0 },
        @{ @"Preset": @0, @"Intensity": @1.0, @"Grain": @0.0 },
        @{ @"Preset": @1, @"Intensity": @1.0, @"Grain": @0.0 },
        @{ @"Preset": @2, @"Intensity": @1.0, @"Grain": @0.0 },
        @{ @"Preset": @3, @"Intensity": @1.0, @"Grain": @0.0 },
        @{ @"Preset": @4, @"Intensity": @1.0, @"Grain": @0.0 },
    ];
    NSInteger index = 0;
    for (NSDictionary *input in textureInputs) {
        makerNoteTransforms[[NSString stringWithFormat:@"texture-input-%02ld", (long)index++]] = @{
            @"input": input,
            @"direct": CallDictionaryTransform(@"PITextureStyleSettingsFromMakerNoteProperties", input),
            @"wrapped84": CallDictionaryTransform(@"PITextureStyleSettingsFromMakerNoteProperties", @{ @"84": input }),
        };
    }
    WriteCheckpoint(@"60-maker-note-transforms", makerNoteTransforms);

    NSMutableDictionary *nuTrials = [NSMutableDictionary dictionary];
    nuTrials[@"empty"] = TexturePropertiesTrial(@{}, @{});
    NSArray<NSString *> *hardwareModels = @[ @"V53AP", @"D93AP", @"iPhone17,1" ];
    NSArray *portTypes = @[ @"PortTypeBack", @"BackWide", @"BackWideCamera", @0, @1 ];
    NSArray *captureModes = @[ @"Photo", @"StillImage", @0, @1 ];
    NSArray *captureTypes = @[ @"Photo", @"StillImage", @0, @1 ];
    NSInteger trialIndex = 0;
    for (NSString *hardware in hardwareModels) {
        for (id port in portTypes) {
            for (id mode in captureModes) {
                for (id type in captureTypes) {
                    if (trialIndex >= 48) break;
                    NSDictionary *properties = @{
                        @"Version": @1,
                        @"HardwareModel": hardware,
                        @"PortType": port,
                        @"CaptureMode": mode,
                        @"CaptureType": type,
                        @"FilmGrainSeed": @12345,
                        @"TextureStylePeopleDataVersion": @1,
                        @"TextureStylePostProcessedPeopleData": @[],
                    };
                    NSString *key = [NSString stringWithFormat:@"trial-%02ld-%@-%@-%@-%@",
                        (long)trialIndex, hardware, [port description], [mode description], [type description]];
                    nuTrials[key] = TexturePropertiesTrial(properties, @{});
                    trialIndex++;
                }
                if (trialIndex >= 48) break;
            }
            if (trialIndex >= 48) break;
        }
        if (trialIndex >= 48) break;
    }
    WriteCheckpoint(@"70-neutrino", nuTrials);

    return @{
        @"os": NSProcessInfo.processInfo.operatingSystemVersionString,
        @"frameworks": loaded,
        @"symbols": symbolInfo,
        @"classes": classSurfaces,
        @"api": api,
        @"defaultPresetDictionaries": presetDictionaries,
        @"makerNoteTransforms": makerNoteTransforms,
        @"texturePropertiesTrials": nuTrials,
    };
}

@interface TextureProbeDelegate : UIResponder <UIApplicationDelegate>
@property(nonatomic,strong) UIWindow *window;
@end

@implementation TextureProbeDelegate
- (BOOL)application:(UIApplication *)application didFinishLaunchingWithOptions:(NSDictionary *)launchOptions {
    (void)application; (void)launchOptions;
    @try {
        NSDictionary *probe = BuildProbe();
        NSError *error = nil;
        NSData *json = [NSJSONSerialization dataWithJSONObject:SafeObject(probe)
                                                       options:NSJSONWritingPrettyPrinted | NSJSONWritingSortedKeys
                                                         error:&error];
        if (!json) {
            WriteCheckpoint(@"90-json-error", @{ @"error": error.localizedDescription ?: @"unknown" });
            exit(2);
        }
        NSURL *output = [DocumentsURL() URLByAppendingPathComponent:@"texture-style-ios27-probe.json"];
        [json writeToURL:output atomically:YES];
        WriteCheckpoint(@"99-complete", @{ @"output": output.path ?: @"" });
        dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)(0.2 * NSEC_PER_SEC)),
                       dispatch_get_main_queue(), ^{ exit(0); });
    } @catch (NSException *exception) {
        WriteCheckpoint(@"98-exception", @{
            @"name": exception.name ?: @"",
            @"reason": exception.reason ?: @"",
        });
        exit(3);
    }
    return YES;
}
@end

int main(int argc, char *argv[]) {
    @autoreleasepool {
        return UIApplicationMain(argc, argv, nil, NSStringFromClass([TextureProbeDelegate class]));
    }
}
