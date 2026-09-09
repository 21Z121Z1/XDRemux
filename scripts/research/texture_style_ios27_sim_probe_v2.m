#import <UIKit/UIKit.h>
#import <Foundation/Foundation.h>
#import <ImageIO/ImageIO.h>
#import <CoreGraphics/CoreGraphics.h>
#import <objc/message.h>
#import <objc/runtime.h>
#import <dlfcn.h>

static id SafeObject(id obj) {
    if (!obj) return [NSNull null];
    if ([obj isKindOfClass:[NSString class]] || [obj isKindOfClass:[NSNumber class]] || [obj isKindOfClass:[NSNull class]]) return obj;
    if ([obj isKindOfClass:[NSData class]]) return @{ @"class": NSStringFromClass([obj class]), @"bytes": @([(NSData *)obj length]) };
    if ([obj isKindOfClass:[NSArray class]]) {
        NSMutableArray *out=[NSMutableArray array];
        for (id value in (NSArray *)obj) [out addObject:SafeObject(value)];
        return out;
    }
    if ([obj isKindOfClass:[NSDictionary class]]) {
        NSMutableDictionary *out=[NSMutableDictionary dictionary];
        for (id key in (NSDictionary *)obj) out[[key description]]=SafeObject([(NSDictionary *)obj objectForKey:key]);
        return out;
    }
    NSMutableDictionary *out=[NSMutableDictionary dictionaryWithDictionary:@{
        @"class": NSStringFromClass([obj class]) ?: @"<unknown>",
        @"description": [obj description] ?: @"<nil>"
    }];
    for (NSString *key in @[ @"preset", @"intensity", @"grain", @"version", @"captureMode", @"captureType", @"hardwareModel", @"portType", @"filmGrainSeed", @"numberOfPersons" ]) {
        @try { id value=[obj valueForKey:key]; if (value) out[key]=SafeObject(value); } @catch (__unused NSException *e) {}
    }
    return out;
}

static void *LookupSymbol(NSString *name) {
    void *p=dlsym(RTLD_DEFAULT, name.UTF8String);
    if (p) return p;
    if ([name hasPrefix:@"_"] && name.length > 1) p=dlsym(RTLD_DEFAULT, [[name substringFromIndex:1] UTF8String]);
    if (p) return p;
    if (![name hasPrefix:@"_"]) {
        NSString *underscored=[@"_" stringByAppendingString:name];
        p=dlsym(RTLD_DEFAULT, underscored.UTF8String);
    }
    return p;
}

static id DataSymbol(NSString *name) {
    void *symbol=LookupSymbol(name);
    if (!symbol) return nil;
    @try {
        __unsafe_unretained id value=*(__unsafe_unretained id *)symbol;
        if (!value) return [NSNull null];
        return value;
    } @catch (NSException *e) {
        return @{ @"address":[NSString stringWithFormat:@"%p",symbol], @"exception":e.reason ?: e.name };
    }
}

static id CallClass0(NSString *cn, NSString *sn) {
    Class cls=NSClassFromString(cn); SEL sel=NSSelectorFromString(sn);
    if (!cls || ![cls respondsToSelector:sel]) return nil;
    @try { return ((id(*)(id,SEL))objc_msgSend)(cls,sel); }
    @catch (NSException *e) { return @{ @"exception":e.reason ?: e.name }; }
}
static id CallClass1(NSString *cn, NSString *sn, id a) {
    Class cls=NSClassFromString(cn); SEL sel=NSSelectorFromString(sn);
    if (!cls || ![cls respondsToSelector:sel]) return nil;
    @try { return ((id(*)(id,SEL,id))objc_msgSend)(cls,sel,a); }
    @catch (NSException *e) { return @{ @"exception":e.reason ?: e.name }; }
}
static id CallInstance0(NSString *cn, NSString *sn) {
    Class cls=NSClassFromString(cn); if (!cls) return nil;
    id obj=nil; @try { obj=[[cls alloc] init]; } @catch (__unused NSException *e) { return nil; }
    SEL sel=NSSelectorFromString(sn); if (!obj || ![obj respondsToSelector:sel]) return nil;
    @try { return ((id(*)(id,SEL))objc_msgSend)(obj,sel); }
    @catch (NSException *e) { return @{ @"exception":e.reason ?: e.name }; }
}

static NSDictionary *ClassSurface(NSString *name) {
    Class cls=NSClassFromString(name); if (!cls) return @{ @"available":@NO };
    NSMutableArray *cm=[NSMutableArray array], *im=[NSMutableArray array];
    unsigned int n=0; Method *m=class_copyMethodList(object_getClass(cls),&n);
    for (unsigned int i=0;i<n;i++) [cm addObject:NSStringFromSelector(method_getName(m[i]))]; free(m);
    n=0; m=class_copyMethodList(cls,&n); for (unsigned int i=0;i<n;i++) [im addObject:NSStringFromSelector(method_getName(m[i]))]; free(m);
    [cm sortUsingSelector:@selector(compare:)]; [im sortUsingSelector:@selector(compare:)];
    return @{ @"available":@YES, @"classMethods":cm, @"instanceMethods":im };
}

static NSDictionary *TexturePropertiesTrial(NSDictionary *image, NSDictionary *aux) {
    Class cls=NSClassFromString(@"_NUTextureStyleProperties");
    SEL sel=NSSelectorFromString(@"textureStylePropertiesFromImageMetadata:auxImageMetadata:error:");
    if (!cls || ![cls respondsToSelector:sel]) return @{ @"available":@NO };
    NSError *error=nil; id result=nil;
    @try { result=((id(*)(id,SEL,id,id,NSError**))objc_msgSend)(cls,sel,image,aux,&error); }
    @catch (NSException *e) { return @{ @"available":@YES, @"exception":e.reason ?: e.name }; }
    NSMutableDictionary *out=[NSMutableDictionary dictionaryWithObject:@YES forKey:@"available"];
    out[@"result"]=SafeObject(result);
    if (error) {
        out[@"errorDomain"]=error.domain ?: @"";
        out[@"errorCode"]=@(error.code);
        out[@"errorDescription"]=error.localizedDescription ?: error.description;
        out[@"errorUserInfo"]=SafeObject(error.userInfo);
    }
    return out;
}

static CGImageRef TinyImage(void) {
    const size_t w=8,h=8,bpr=w*4; uint8_t *bytes=calloc(h,bpr);
    for (size_t y=0;y<h;y++) for (size_t x=0;x<w;x++) {
        size_t i=y*bpr+x*4; bytes[i+0]=(uint8_t)(x*32); bytes[i+1]=(uint8_t)(y*32); bytes[i+2]=128; bytes[i+3]=255;
    }
    CGDataProviderRef provider=CGDataProviderCreateWithData(NULL,bytes,h*bpr,^(void *info,const void *data,size_t size){ free((void *)data); });
    CGColorSpaceRef cs=CGColorSpaceCreateDeviceRGB();
    CGImageRef image=CGImageCreate(w,h,8,32,bpr,cs,kCGBitmapByteOrderDefault|kCGImageAlphaLast,provider,NULL,false,kCGRenderingIntentDefault);
    CGColorSpaceRelease(cs); CGDataProviderRelease(provider); return image;
}

static NSDictionary *WriteAndReadImageIOProbe(NSURL *documents, NSDictionary *maker) {
    NSURL *url=[documents URLByAppendingPathComponent:@"texture-style-imageio-probe.jpg"];
    CGImageRef image=TinyImage();
    CGImageDestinationRef dest=CGImageDestinationCreateWithURL((__bridge CFURLRef)url,CFSTR("public.jpeg"),1,NULL);
    NSDictionary *props=@{ (__bridge NSString *)kCGImagePropertyMakerAppleDictionary: maker,
                            (__bridge NSString *)kCGImagePropertyTIFFDictionary: @{ (__bridge NSString *)kCGImagePropertyTIFFMake:@"Apple", (__bridge NSString *)kCGImagePropertyTIFFModel:@"iPhone17,1" } };
    BOOL ok=NO;
    if (dest) { CGImageDestinationAddImage(dest,image,(__bridge CFDictionaryRef)props); ok=CGImageDestinationFinalize(dest); CFRelease(dest); }
    CGImageRelease(image);
    NSMutableDictionary *out=[NSMutableDictionary dictionaryWithObject:@(ok) forKey:@"finalized"];
    if (!ok) return out;
    CGImageSourceRef src=CGImageSourceCreateWithURL((__bridge CFURLRef)url,NULL);
    if (src) {
        CFDictionaryRef read=CGImageSourceCopyPropertiesAtIndex(src,0,NULL);
        if (read) { out[@"readback"]=SafeObject((__bridge NSDictionary *)read); CFRelease(read); }
        CFRelease(src);
    }
    NSDictionary *attrs=[[NSFileManager defaultManager] attributesOfItemAtPath:url.path error:nil];
    out[@"bytes"]=attrs[NSFileSize] ?: @0;
    return out;
}

static NSDictionary *BuildProbe(NSURL *documents) {
    NSArray *frameworks=@[
        @"/System/Library/PrivateFrameworks/CMImaging.framework/CMImaging",
        @"/System/Library/PrivateFrameworks/CMCaptureCore.framework/CMCaptureCore",
        @"/System/Library/PrivateFrameworks/AVFCapture.framework/AVFCapture",
        @"/System/Library/PrivateFrameworks/NeutrinoCore.framework/NeutrinoCore",
        @"/System/Library/PrivateFrameworks/PhotoImaging.framework/PhotoImaging",
        @"/System/Library/PrivateFrameworks/PhotosFormats.framework/PhotosFormats",
        @"/System/Library/PrivateFrameworks/CameraEditKit.framework/CameraEditKit"
    ];
    NSMutableDictionary *loaded=[NSMutableDictionary dictionary];
    for (NSString *path in frameworks) loaded[path]=@(dlopen(path.UTF8String,RTLD_NOW|RTLD_GLOBAL)!=NULL);

    NSArray *symbolNames=@[
        @"AVAppleMakerNote_TextureStyleKey_Preset", @"AVAppleMakerNote_TextureStyleKey_Intensity", @"AVAppleMakerNote_TextureStyleKey_Grain", @"AVAppleMakerNote_TextureStyleKey_RenderingVersion", @"AVAppleMakerNote_TextureStyleKey_OriginalInsteadOfReversibility",
        @"kFigAppleMakerNote_TextureStyleKey_Preset", @"kFigAppleMakerNote_TextureStyleKey_Intensity", @"kFigAppleMakerNote_TextureStyleKey_Grain", @"kFigAppleMakerNote_TextureStyleKey_RenderingVersion", @"kFigAppleMakerNote_TextureStyleKey_OriginalInsteadOfReversibility",
        @"PITextureStyleAdjustmentKey", @"PITextureStyleCurrentMetadataVersion",
        @"AVCaptureTextureStylePresetStandard", @"AVCaptureTextureStylePresetStudio", @"AVCaptureTextureStylePresetSoft", @"AVCaptureTextureStylePresetFilmic", @"AVCaptureTextureStylePresetGlowy", @"AVCaptureTextureStylePresetPreview",
        @"kFigCaptureSampleBufferMetadata_TextureStylePreset", @"kFigCaptureSampleBufferMetadata_TextureStylePresetTunings", @"kFigCaptureSampleBufferMetadata_TextureStylePeopleDataVersion", @"kFigCaptureSampleBufferMetadata_TextureStylePostProcessedPeopleData", @"kFigCaptureSampleBufferAttachedMediaKey_TextureStyleFaceAttitudeMetadata"
    ];
    NSMutableDictionary *symbols=[NSMutableDictionary dictionary];
    for (NSString *name in symbolNames) { id v=DataSymbol(name); symbols[name]=v?SafeObject(v):[NSNull null]; }

    NSArray *classes=@[ @"AVCaptureTextureStyle", @"CMITextureStyle", @"CMITextureStyleTuningLookup", @"_NUTextureStyleProperties", @"_NUTextureStylePersonInstanceProperties", @"PIAdjustmentConstants", @"PISchema", @"PITextureStyle", @"PITextureStyleAdjustmentController", @"PITextureStyleAutoCalculator", @"PITextureStylePipelineProcessor", @"PFMetadata", @"PFMetadataImage" ];
    NSMutableDictionary *surfaces=[NSMutableDictionary dictionary]; for (NSString *name in classes) surfaces[name]=ClassSurface(name);

    NSMutableDictionary *api=[NSMutableDictionary dictionary];
    NSArray *calls=@[
        @[ @"PISchema", @"textureStyleSchema" ], @[ @"PITextureStyle", @"identifier" ], @[ @"PITextureStyle", @"adjustmentFormat" ], @[ @"PITextureStyle", @"adjustmentDescriptor" ], @[ @"PITextureStyle", @"availableOptions" ],
        @[ @"PITextureStyleAdjustmentController", @"allPresets" ], @[ @"PITextureStyleAdjustmentController", @"presetKey" ], @[ @"PITextureStyleAdjustmentController", @"intensityKey" ], @[ @"PITextureStyleAdjustmentController", @"grainIntensityKey" ], @[ @"AVCaptureTextureStyle", @"identityStyle" ], @[ @"CMITextureStyle", @"initStandardTextureStyle" ]
    ];
    for (NSArray *call in calls) { id v=CallClass0(call[0],call[1]); api[[NSString stringWithFormat:@"%@.%@",call[0],call[1]]]=v?SafeObject(v):[NSNull null]; }
    id adjustmentInstance=CallInstance0(@"PIAdjustmentConstants",@"PITextureStyleAdjustmentKey");
    api[@"PIAdjustmentConstants(instance).PITextureStyleAdjustmentKey"]=adjustmentInstance?SafeObject(adjustmentInstance):[NSNull null];

    for (NSString *preset in @[ @"Standard", @"Studio", @"Soft", @"Filmic", @"Glowy", @"Preview" ]) {
        id v=CallClass1(@"CMITextureStyleTuningLookup",@"defaultTextureStyleForPresetName:",preset);
        api[[NSString stringWithFormat:@"CMITextureStyleTuningLookup.default.%@",preset]]=v?SafeObject(v):[NSNull null];
    }

    NSMutableDictionary *maker=[NSMutableDictionary dictionary];
    NSDictionary *fallback=@{ @"Preset":@"Standard", @"Intensity":@1.0, @"Grain":@0.0, @"RenderingVersion":@1, @"OriginalInsteadOfReversibility":@NO };
    NSDictionary *mapping=@{ @"Preset":@"AVAppleMakerNote_TextureStyleKey_Preset", @"Intensity":@"AVAppleMakerNote_TextureStyleKey_Intensity", @"Grain":@"AVAppleMakerNote_TextureStyleKey_Grain", @"RenderingVersion":@"AVAppleMakerNote_TextureStyleKey_RenderingVersion", @"OriginalInsteadOfReversibility":@"AVAppleMakerNote_TextureStyleKey_OriginalInsteadOfReversibility" };
    for (NSString *logical in fallback) {
        id key=DataSymbol(mapping[logical]); if (!key || key==[NSNull null]) key=logical; maker[key]=fallback[logical];
    }

    NSDictionary *full=@{ @"TextureStyleVersion":@1, @"Version":@1, @"CaptureMode":@"Photo", @"CaptureType":@"Photo", @"HardwareModel":@"iPhone17,1", @"PortType":@"BackWide", @"FilmGrainSeed":@1, @"Preset":@"Standard", @"Intensity":@1.0, @"Grain":@0.0, @"RenderingVersion":@1, @"OriginalInsteadOfReversibility":@NO };
    NSDictionary *makerContainer=@{ (__bridge NSString *)kCGImagePropertyMakerAppleDictionary: maker };
    NSDictionary *fullMakerContainer=@{ (__bridge NSString *)kCGImagePropertyMakerAppleDictionary: full };
    NSDictionary *imageIO=WriteAndReadImageIOProbe(documents,maker);
    NSDictionary *readback=[imageIO[@"readback"] isKindOfClass:[NSDictionary class]] ? imageIO[@"readback"] : @{};

    NSDictionary *trials=@{
        @"empty":TexturePropertiesTrial(@{},@{}),
        @"makerStyleOnly":TexturePropertiesTrial(makerContainer,@{}),
        @"directStyleOnly":TexturePropertiesTrial(maker,@{}),
        @"makerFullGuessed":TexturePropertiesTrial(fullMakerContainer,@{}),
        @"directFullGuessed":TexturePropertiesTrial(full,@{}),
        @"auxFullGuessed":TexturePropertiesTrial(@{},full),
        @"makerStyleAuxFull":TexturePropertiesTrial(makerContainer,full),
        @"imageIOReadback":TexturePropertiesTrial(readback,@{})
    };
    id tuningMaker=CallClass1(@"CMITextureStyleTuningLookup",@"tuningDictionaryForMetadata:",maker);
    id tuningFull=CallClass1(@"CMITextureStyleTuningLookup",@"tuningDictionaryForMetadata:",full);

    return @{ @"os":NSProcessInfo.processInfo.operatingSystemVersionString, @"frameworks":loaded, @"symbols":symbols, @"classes":surfaces, @"api":api, @"imageIOProbe":imageIO, @"makerDictionary":SafeObject(maker), @"texturePropertiesTrials":trials, @"tuningFromMaker":tuningMaker?SafeObject(tuningMaker):[NSNull null], @"tuningFromFullGuessed":tuningFull?SafeObject(tuningFull):[NSNull null] };
}

@interface ProbeDelegate:UIResponder<UIApplicationDelegate> @end
@implementation ProbeDelegate
- (BOOL)application:(UIApplication *)app didFinishLaunchingWithOptions:(NSDictionary *)opts {
    (void)app;(void)opts;
    NSURL *docs=[[[NSFileManager defaultManager] URLsForDirectory:NSDocumentDirectory inDomains:NSUserDomainMask] firstObject];
    NSDictionary *probe=BuildProbe(docs);
    NSData *json=[NSJSONSerialization dataWithJSONObject:probe options:NSJSONWritingPrettyPrinted|NSJSONWritingSortedKeys error:nil];
    [json writeToURL:[docs URLByAppendingPathComponent:@"texture-style-ios27-probe-v2.json"] atomically:YES];
    NSLog(@"TEXTURE_STYLE_PROBE_V2_BEGIN\n%@\nTEXTURE_STYLE_PROBE_V2_END",[[NSString alloc] initWithData:json encoding:NSUTF8StringEncoding]);
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW,(int64_t)(0.2*NSEC_PER_SEC)),dispatch_get_main_queue(),^{ exit(0); });
    return YES;
}
@end
int main(int argc,char *argv[]){ @autoreleasepool { return UIApplicationMain(argc,argv,nil,NSStringFromClass([ProbeDelegate class])); } }
