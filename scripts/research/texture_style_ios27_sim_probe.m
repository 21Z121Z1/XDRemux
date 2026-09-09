#import <UIKit/UIKit.h>
#import <Foundation/Foundation.h>
#import <ImageIO/ImageIO.h>
#import <objc/message.h>
#import <objc/runtime.h>
#import <dlfcn.h>

static id SafeObject(id obj) {
    if (!obj) return [NSNull null];
    if ([obj isKindOfClass:[NSString class]] ||
        [obj isKindOfClass:[NSNumber class]] ||
        [obj isKindOfClass:[NSNull class]]) return obj;
    if ([obj isKindOfClass:[NSData class]]) {
        NSData *data = obj;
        return @{ @"class": NSStringFromClass([obj class]), @"bytes": @(data.length), @"base64": [data base64EncodedStringWithOptions:0] };
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
    return @{ @"class": NSStringFromClass([obj class]) ?: @"<unknown>", @"description": [obj description] ?: @"<nil>" };
}

static id BoxInvocationReturn(NSInvocation *invocation, NSMethodSignature *signature) {
    const char *type = signature.methodReturnType;
    while (*type == 'r' || *type == 'n' || *type == 'N' || *type == 'o' || *type == 'O' || *type == 'R' || *type == 'V') type++;
    if (*type == 'v') return [NSNull null];
    if (*type == '@' || *type == '#') {
        __unsafe_unretained id value = nil;
        [invocation getReturnValue:&value];
        return value ?: [NSNull null];
    }
    if (*type == 'B' || *type == 'c') {
        BOOL value = NO;
        [invocation getReturnValue:&value];
        return @(value);
    }
    if (*type == 'f') {
        float value = 0;
        [invocation getReturnValue:&value];
        return @(value);
    }
    if (*type == 'd') {
        double value = 0;
        [invocation getReturnValue:&value];
        return @(value);
    }
    if (strchr("islqISLQ", *type)) {
        long long value = 0;
        NSUInteger length = MIN(sizeof(value), signature.methodReturnLength);
        unsigned char buffer[sizeof(value)] = {0};
        [invocation getReturnValue:buffer];
        memcpy(&value, buffer, length);
        return @(value);
    }
    NSUInteger length = signature.methodReturnLength;
    NSMutableData *data = [NSMutableData dataWithLength:length];
    [invocation getReturnValue:data.mutableBytes];
    return @{ @"type": [NSString stringWithUTF8String:type] ?: @"?", @"bytes": [data base64EncodedStringWithOptions:0] };
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

static id CallClass0(NSString *className, NSString *selectorName) {
    return Invoke0(NSClassFromString(className), selectorName);
}

static id CallClassObject1(NSString *className, NSString *selectorName, id arg) {
    return InvokeObject1(NSClassFromString(className), selectorName, arg);
}

static id DataSymbol(NSString *name) {
    void *symbol = dlsym(RTLD_DEFAULT, name.UTF8String);
    if (!symbol) return nil;
    @try {
        __unsafe_unretained id value = *(__unsafe_unretained id *)symbol;
        if (!value) return [NSNull null];
        return value;
    } @catch (NSException *exception) {
        return @{ @"address": [NSString stringWithFormat:@"%p", symbol], @"exception": exception.reason ?: exception.name };
    }
}

static id RawSymbolBytes(NSString *name) {
    void *symbol = dlsym(RTLD_DEFAULT, name.UTF8String);
    if (!symbol) return [NSNull null];
    unsigned char bytes[16] = {0};
    memcpy(bytes, symbol, sizeof(bytes));
    NSMutableString *hex = [NSMutableString stringWithCapacity:32];
    for (NSUInteger i = 0; i < sizeof(bytes); ++i) [hex appendFormat:@"%02x", bytes[i]];
    return @{ @"address": [NSString stringWithFormat:@"%p", symbol], @"first16BytesHex": hex };
}

static NSDictionary *TexturePropertiesTrial(NSDictionary *imageMetadata, NSDictionary *auxMetadata) {
    Class cls = NSClassFromString(@"_NUTextureStyleProperties");
    SEL sel = NSSelectorFromString(@"textureStylePropertiesFromImageMetadata:auxImageMetadata:error:");
    if (!cls || ![cls respondsToSelector:sel]) return @{ @"available": @NO };
    NSError *error = nil;
    id result = nil;
    @try {
        result = ((id (*)(id, SEL, id, id, NSError **))objc_msgSend)(cls, sel, imageMetadata, auxMetadata, &error);
    } @catch (NSException *exception) {
        return @{ @"available": @YES, @"exception": exception.reason ?: exception.name };
    }
    NSMutableDictionary *out = [NSMutableDictionary dictionaryWithObject:@YES forKey:@"available"];
    out[@"result"] = SafeObject(result);
    if (error) {
        out[@"errorDomain"] = error.domain ?: @"";
        out[@"errorCode"] = @(error.code);
        out[@"errorDescription"] = error.localizedDescription ?: error.description;
        out[@"errorUserInfo"] = SafeObject(error.userInfo);
    }
    return out;
}

static NSDictionary *ClassSurface(NSString *name) {
    Class cls = NSClassFromString(name);
    if (!cls) return @{ @"available": @NO };
    NSMutableArray *classMethods = [NSMutableArray array];
    unsigned int classCount = 0;
    Method *cm = class_copyMethodList(object_getClass(cls), &classCount);
    for (unsigned int i = 0; i < classCount; ++i) {
        SEL sel = method_getName(cm[i]);
        [classMethods addObject:@{ @"selector": NSStringFromSelector(sel), @"types": [NSString stringWithUTF8String:method_getTypeEncoding(cm[i])] ?: @"" }];
    }
    free(cm);
    NSMutableArray *instanceMethods = [NSMutableArray array];
    unsigned int instanceCount = 0;
    Method *im = class_copyMethodList(cls, &instanceCount);
    for (unsigned int i = 0; i < instanceCount; ++i) {
        SEL sel = method_getName(im[i]);
        [instanceMethods addObject:@{ @"selector": NSStringFromSelector(sel), @"types": [NSString stringWithUTF8String:method_getTypeEncoding(im[i])] ?: @"" }];
    }
    free(im);
    return @{ @"available": @YES, @"classMethods": classMethods, @"instanceMethods": instanceMethods };
}

static NSDictionary *CEKSerializationProbe(void) {
    Class cls = NSClassFromString(@"CEKTextureStyle");
    if (!cls) return @{ @"available": @NO };
    id defaults = Invoke0(cls, @"defaultStyles");
    id identity = Invoke0(cls, @"identityStyle");
    NSMutableArray *styles = [NSMutableArray array];
    if ([defaults isKindOfClass:[NSArray class]]) {
        for (id style in defaults) {
            NSMutableDictionary *entry = [NSMutableDictionary dictionary];
            entry[@"description"] = [style description] ?: @"";
            for (NSString *selector in @[ @"preset", @"intensity", @"grain" ]) {
                id value = Invoke0(style, selector);
                entry[selector] = value ? SafeObject(value) : [NSNull null];
            }
            id dictionary = InvokeObject1(style, @"dictionaryRepresentationUsingReferenceStyle:", identity == [NSNull null] ? nil : identity);
            entry[@"dictionaryRepresentationUsingIdentity"] = dictionary ? SafeObject(dictionary) : [NSNull null];
            [styles addObject:entry];
        }
    }
    return @{
        @"available": @YES,
        @"identity": identity ? SafeObject(identity) : [NSNull null],
        @"defaultStyles": defaults ? SafeObject(defaults) : [NSNull null],
        @"serializedDefaultStyles": styles,
    };
}

static NSDictionary *BuildProbe(void) {
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
    for (NSString *path in frameworks) loaded[path] = @(dlopen(path.UTF8String, RTLD_NOW | RTLD_GLOBAL) != NULL);

    NSArray<NSString *> *symbols = @[
        @"_AVAppleMakerNote_TextureStyleKey_Preset",
        @"_AVAppleMakerNote_TextureStyleKey_Intensity",
        @"_AVAppleMakerNote_TextureStyleKey_Grain",
        @"_AVAppleMakerNote_TextureStyleKey_RenderingVersion",
        @"_AVAppleMakerNote_TextureStyleKey_OriginalInsteadOfReversibility",
        @"_kFigAppleMakerNote_TextureStyleKey_Preset",
        @"_kFigAppleMakerNote_TextureStyleKey_Intensity",
        @"_kFigAppleMakerNote_TextureStyleKey_Grain",
        @"_kFigAppleMakerNote_TextureStyleKey_RenderingVersion",
        @"_kFigAppleMakerNote_TextureStyleKey_OriginalInsteadOfReversibility",
        @"_AVCaptureTextureStylePresetStandard",
        @"_AVCaptureTextureStylePresetStudio",
        @"_AVCaptureTextureStylePresetSoft",
        @"_AVCaptureTextureStylePresetFilmic",
        @"_AVCaptureTextureStylePresetGlowy",
        @"_AVCaptureTextureStylePresetPreview",
        @"_kFigCaptureSampleBufferMetadata_TextureStylePreset",
        @"_kFigCaptureSampleBufferMetadata_TextureStylePresetTunings",
        @"_kFigCaptureSampleBufferMetadata_TextureStylePeopleDataVersion",
        @"_kFigCaptureSampleBufferMetadata_TextureStylePostProcessedPeopleData",
        @"_kFigCaptureSampleBufferAttachedMediaKey_TextureStyleFaceAttitudeMetadata",
        @"_PITextureStyleAdjustmentKey",
        @"_kMetadataIdentifier_TextureStyleInfo",
        @"_NUTextureStyleMetadataKey_FaceAttitude"
    ];
    NSMutableDictionary *symbolValues = [NSMutableDictionary dictionary];
    for (NSString *name in symbols) {
        id value = DataSymbol(name);
        symbolValues[name] = value ? SafeObject(value) : [NSNull null];
    }

    NSMutableDictionary *rawSymbols = [NSMutableDictionary dictionary];
    for (NSString *name in @[ @"_PITextureStyleCurrentMetadataVersion" ]) {
        rawSymbols[name] = RawSymbolBytes(name);
    }

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
        @"PFMetadataImage"
    ];
    NSMutableDictionary *classSurfaces = [NSMutableDictionary dictionary];
    for (NSString *name in classes) classSurfaces[name] = ClassSurface(name);

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
        @[ @"CEKTextureStyle", @"identityStyle" ]
    ];
    for (NSArray<NSString *> *call in calls) {
        id value = CallClass0(call[0], call[1]);
        api[[NSString stringWithFormat:@"%@.%@", call[0], call[1]]] = value ? SafeObject(value) : [NSNull null];
    }

    id presets = CallClass0(@"PITextureStyleAdjustmentController", @"allPresets");
    NSMutableDictionary *presetDictionaries = [NSMutableDictionary dictionary];
    if ([presets isKindOfClass:[NSArray class]]) {
        for (id preset in (NSArray *)presets) {
            id value = CallClassObject1(@"PITextureStylePipelineProcessor", @"defaultTextureStyleDictionaryForPreset:", preset);
            presetDictionaries[[preset description]] = value ? SafeObject(value) : [NSNull null];
        }
    }

    NSMutableDictionary *trials = [NSMutableDictionary dictionary];
    trials[@"empty"] = TexturePropertiesTrial(@{}, @{});

    NSMutableDictionary *maker = [NSMutableDictionary dictionary];
    id presetKey = DataSymbol(@"_AVAppleMakerNote_TextureStyleKey_Preset");
    id intensityKey = DataSymbol(@"_AVAppleMakerNote_TextureStyleKey_Intensity");
    id grainKey = DataSymbol(@"_AVAppleMakerNote_TextureStyleKey_Grain");
    id renderingKey = DataSymbol(@"_AVAppleMakerNote_TextureStyleKey_RenderingVersion");
    id reversibleKey = DataSymbol(@"_AVAppleMakerNote_TextureStyleKey_OriginalInsteadOfReversibility");
    if (presetKey && presetKey != [NSNull null]) maker[presetKey] = @"Standard";
    if (intensityKey && intensityKey != [NSNull null]) maker[intensityKey] = @1.0;
    if (grainKey && grainKey != [NSNull null]) maker[grainKey] = @0.0;
    if (renderingKey && renderingKey != [NSNull null]) maker[renderingKey] = @1;
    if (reversibleKey && reversibleKey != [NSNull null]) maker[reversibleKey] = @NO;
    trials[@"makerAppleTextureOnly"] = TexturePropertiesTrial(@{ (__bridge NSString *)kCGImagePropertyMakerAppleDictionary: maker }, @{});
    trials[@"directTextureOnly"] = TexturePropertiesTrial(maker, @{});

    NSDictionary *nuProperties = @{
        @"Version": @1,
        @"HardwareModel": @"iPhone18,1",
        @"PortType": @"BackWide",
        @"CaptureMode": @"Photo",
        @"CaptureType": @"Photo",
        @"FilmGrainSeed": @12345,
        @"TextureStylePeopleDataVersion": @1,
        @"TextureStylePostProcessedPeopleData": @[],
    };
    NSError *plistError = nil;
    NSData *nuPropertiesBinary = [NSPropertyListSerialization dataWithPropertyList:nuProperties format:NSPropertyListBinaryFormat_v1_0 options:0 error:&plistError];
    id metadataIdentifier = DataSymbol(@"_kMetadataIdentifier_TextureStyleInfo");
    trials[@"nuDirectProperties"] = TexturePropertiesTrial(nuProperties, @{});
    if (metadataIdentifier && metadataIdentifier != [NSNull null]) {
        trials[@"nuIdentifierDictionary"] = TexturePropertiesTrial(@{ metadataIdentifier: nuProperties }, @{});
        if (nuPropertiesBinary) {
            trials[@"nuIdentifierBinaryPlist"] = TexturePropertiesTrial(@{ metadataIdentifier: nuPropertiesBinary }, @{});
            trials[@"nuIdentifierBinaryPlistAux"] = TexturePropertiesTrial(@{}, @{ metadataIdentifier: nuPropertiesBinary });
            trials[@"makerPlusNuIdentifierBinaryPlist"] = TexturePropertiesTrial(@{
                (__bridge NSString *)kCGImagePropertyMakerAppleDictionary: maker,
                metadataIdentifier: nuPropertiesBinary,
            }, @{});
        }
    }
    if (plistError) trials[@"nuPropertyListSerializationError"] = @{ @"description": plistError.localizedDescription ?: plistError.description };

    return @{
        @"os": NSProcessInfo.processInfo.operatingSystemVersionString,
        @"frameworks": loaded,
        @"symbols": symbolValues,
        @"rawSymbols": rawSymbols,
        @"classes": classSurfaces,
        @"api": api,
        @"cekSerialization": CEKSerializationProbe(),
        @"defaultPresetDictionaries": presetDictionaries,
        @"texturePropertiesTrials": trials
    };
}

@interface TextureProbeDelegate : UIResponder <UIApplicationDelegate>
@property(nonatomic,strong) UIWindow *window;
@end

@implementation TextureProbeDelegate
- (BOOL)application:(UIApplication *)application didFinishLaunchingWithOptions:(NSDictionary *)launchOptions {
    (void)application; (void)launchOptions;
    NSDictionary *probe = BuildProbe();
    NSError *error = nil;
    NSData *json = [NSJSONSerialization dataWithJSONObject:probe options:NSJSONWritingPrettyPrinted | NSJSONWritingSortedKeys error:&error];
    if (!json) {
        NSLog(@"TextureStyle probe JSON serialization failed: %@", error);
        exit(2);
    }
    NSURL *documents = [[[NSFileManager defaultManager] URLsForDirectory:NSDocumentDirectory inDomains:NSUserDomainMask] firstObject];
    NSURL *output = [documents URLByAppendingPathComponent:@"texture-style-ios27-probe.json"];
    [json writeToURL:output atomically:YES];
    NSString *text = [[NSString alloc] initWithData:json encoding:NSUTF8StringEncoding];
    NSLog(@"TEXTURE_STYLE_PROBE_JSON_BEGIN\n%@\nTEXTURE_STYLE_PROBE_JSON_END", text);
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)(0.25 * NSEC_PER_SEC)), dispatch_get_main_queue(), ^{ exit(0); });
    return YES;
}
@end

int main(int argc, char *argv[]) {
    @autoreleasepool { return UIApplicationMain(argc, argv, nil, NSStringFromClass([TextureProbeDelegate class])); }
}
