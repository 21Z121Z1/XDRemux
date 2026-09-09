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

static id CallClassObject(NSString *className, NSString *selectorName) {
    Class cls = NSClassFromString(className);
    SEL sel = NSSelectorFromString(selectorName);
    if (!cls || ![cls respondsToSelector:sel]) return nil;
    @try {
        return ((id (*)(id, SEL))objc_msgSend)(cls, sel);
    } @catch (NSException *exception) {
        return @{ @"exception": exception.reason ?: exception.name };
    }
}

static id CallClassObject1(NSString *className, NSString *selectorName, id arg) {
    Class cls = NSClassFromString(className);
    SEL sel = NSSelectorFromString(selectorName);
    if (!cls || ![cls respondsToSelector:sel]) return nil;
    @try {
        return ((id (*)(id, SEL, id))objc_msgSend)(cls, sel, arg);
    } @catch (NSException *exception) {
        return @{ @"exception": exception.reason ?: exception.name };
    }
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
    for (unsigned int i = 0; i < classCount; ++i) [classMethods addObject:NSStringFromSelector(method_getName(cm[i]))];
    free(cm);
    NSMutableArray *instanceMethods = [NSMutableArray array];
    unsigned int instanceCount = 0;
    Method *im = class_copyMethodList(cls, &instanceCount);
    for (unsigned int i = 0; i < instanceCount; ++i) [instanceMethods addObject:NSStringFromSelector(method_getName(im[i]))];
    free(im);
    [classMethods sortUsingSelector:@selector(compare:)];
    [instanceMethods sortUsingSelector:@selector(compare:)];
    return @{ @"available": @YES, @"classMethods": classMethods, @"instanceMethods": instanceMethods };
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
        @"_kFigCaptureSampleBufferAttachedMediaKey_TextureStyleFaceAttitudeMetadata"
    ];
    NSMutableDictionary *symbolValues = [NSMutableDictionary dictionary];
    for (NSString *name in symbols) {
        id value = DataSymbol(name);
        symbolValues[name] = value ? SafeObject(value) : [NSNull null];
    }

    NSArray<NSString *> *classes = @[
        @"AVCaptureTextureStyle",
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
        @[ @"AVCaptureTextureStyle", @"identityStyle" ]
    ];
    for (NSArray<NSString *> *call in calls) {
        id value = CallClassObject(call[0], call[1]);
        api[[NSString stringWithFormat:@"%@.%@", call[0], call[1]]] = value ? SafeObject(value) : [NSNull null];
    }

    id presets = CallClassObject(@"PITextureStyleAdjustmentController", @"allPresets");
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

    return @{
        @"os": NSProcessInfo.processInfo.operatingSystemVersionString,
        @"frameworks": loaded,
        @"symbols": symbolValues,
        @"classes": classSurfaces,
        @"api": api,
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
    NSData *json = [NSJSONSerialization dataWithJSONObject:probe options:NSJSONWritingPrettyPrinted | NSJSONWritingSortedKeys error:nil];
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
