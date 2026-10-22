#import "XDStylesNative.h"
#import <CoreImage/CoreImage.h>
#import <CoreMedia/CoreMedia.h>
#import <ImageIO/ImageIO.h>
#import <Metal/Metal.h>
#import <objc/runtime.h>
#import <dlfcn.h>

typedef struct { int64_t width, height; } XDNUSize;
@interface NSObject (XDStylesSPI)
+ (id)semanticStylePropertiesFromData:(NSData *)data fromVideo:(BOOL)video keyTime:(CMTime)time error:(NSError **)error;
+ (id)reverseSettingsForSemanticStyleProperties:(id)properties size:(XDNUSize)size;
+ (id)displayP3ColorSpace;
+ (CIImage *)generateThumbnailForImage:(CIImage *)image targetSize:(XDNUSize)size colorSpace:(id)space configuration:(id)config tuningParameters:(id)tuning error:(NSError **)error;
+ (CIImage *)applyStyle:(CIImage *)style toImage:(CIImage *)image thumbnail:(CIImage *)thumbnail target:(CIImage *)target deltaMap:(CIImage *)delta displacement:(CIImage *)displacement colorSpace:(id)space configuration:(id)config tuningParameters:(id)tuning noiseModel:(id)noise error:(NSError **)error;
- (id)initWithConfigurationDictionary:(id)dictionary;
- (id)styleEngineConfiguration;
+ (CGSize)coefficientPixelBufferSizeForConfiguration:(id)configuration float16:(BOOL)half;
@end

static void Require(BOOL condition, NSString *message) {
    if (!condition) @throw [NSException exceptionWithName:@"XDStylesUnavailable" reason:message userInfo:nil];
}

// Compare logical Objective-C ABI types, independently of argument stack offsets.
// Struct names are compiler labels; their exact field layout is checked.
static BOOL TypeMatches(const char *actual, const char *expected) {
    while (*actual && strchr("rnNoORV",*actual)) actual++;
    if (*expected == '{') {
        const char *a = strchr(actual, '='), *e = strchr(expected, '=');
        return *actual == '{' && a && e && strcmp(a,e) == 0;
    }
    return strcmp(actual, expected) == 0;
}
static void Check(Class cls, BOOL classMethod, SEL selector, const char *result, NSArray<NSString *> *args) {
    Method method = classMethod ? class_getClassMethod(cls,selector) : class_getInstanceMethod(cls,selector);
    Require(method != NULL, [NSString stringWithFormat:@"Missing Apple selector %@.%@",cls,NSStringFromSelector(selector)]);
    NSMethodSignature *s = [NSMethodSignature signatureWithObjCTypes:method_getTypeEncoding(method)];
    Require(s.numberOfArguments == args.count+2 && TypeMatches(s.methodReturnType,result), @"Unsupported Apple method ABI");
    for (NSUInteger i=0; i<args.count; i++) {
        Require(TypeMatches([s getArgumentTypeAtIndex:i+2],args[i].UTF8String),
                [NSString stringWithFormat:@"Unsupported argument ABI for %@: %s",NSStringFromSelector(selector),[s getArgumentTypeAtIndex:i+2]]);
    }
}

static CIContext *Context(CGColorSpaceRef space) {
    id<MTLDevice> device = MTLCreateSystemDefaultDevice();
    Require(device != nil,@"Metal is unavailable; no rendered result can be produced");
    return [CIContext contextWithMTLDevice:device options:@{
        kCIContextWorkingColorSpace:space ? (__bridge id)space : NSNull.null,
        kCIContextOutputColorSpace:space ? (__bridge id)space : NSNull.null,
        kCIContextWorkingFormat:@(kCIFormatRGBAf),kCIContextCacheIntermediates:@NO}];
}
static CIImage *Image(NSString *path) {
    CIImage *image = [CIImage imageWithContentsOfURL:[NSURL fileURLWithPath:path]
        options:@{kCIImageApplyOrientationProperty:@NO,kCIImageExpandToHDR:@NO}];
    Require(image != nil, [@"Cannot decode image: " stringByAppendingString:path]);
    return image;
}
static CIImage *RawImage(NSString *path) {
    CIImage *image=[CIImage imageWithContentsOfURL:[NSURL fileURLWithPath:path]
        options:@{kCIImageApplyOrientationProperty:@NO,kCIImageExpandToHDR:@NO,kCIImageColorSpace:NSNull.null}];
    Require(image!=nil,@"Cannot decode Styles auxiliary image");
    return image;
}
static CIImage *Canvas(CIImage *image, CGSize size) {
    CGRect r=image.extent;
    Require(!CGRectIsInfinite(r) && !CGRectIsEmpty(r),@"Invalid decoded extent");
    image=[image imageByApplyingTransform:CGAffineTransformMakeTranslation(-r.origin.x,-r.origin.y)];
    return [image imageByApplyingTransform:CGAffineTransformMakeScale(size.width/r.size.width,size.height/r.size.height)];
}
static NSDictionary *Render(CIImage *image, CIContext *context, CGColorSpaceRef space, NSString *path) {
    CGRect r=image.extent;
    size_t w=llround(r.size.width),h=llround(r.size.height);
    Require(w>0 && h>0 && w<=32768 && h<=32768 && w*h<=100000000,@"Image geometry exceeds the supported render limit");
    image=Canvas(image,CGSizeMake(w,h));
    NSMutableData *data=[NSMutableData dataWithLength:w*h*16];
    [context render:image toBitmap:data.mutableBytes rowBytes:w*16 bounds:CGRectMake(0,0,w,h) format:kCIFormatRGBAf colorSpace:space];
    const float *p=data.bytes;
    size_t nonzero=0,black=0;
    for(size_t i=0;i<w*h;i++) {
        BOOL zero=YES;
        for(size_t c=0;c<4;c++) { Require(isfinite(p[i*4+c]),@"Apple render returned non-finite pixels"); if(c<3 && p[i*4+c]!=0) { nonzero++;zero=NO; } }
        if(zero) black++;
    }
    Require(nonzero>0,@"Apple render returned an empty image");
    NSError *error=nil;
    Require([data writeToFile:path options:NSDataWritingAtomic error:&error],error.description ?: @"Cannot write rendered pixels");
    return @{@"width":@(w),@"height":@(h),@"black_fraction":@((double)black/(w*h))};
}

static NSDictionary *Decode(NSDictionary *request) {
    NSString *inputPath=request[@"input_path"],*dir=request[@"output_path"];
    NSData *plist=[NSData dataWithContentsOfFile:[dir stringByAppendingPathComponent:@"styles.plist"]];
    NSError *error=nil;
    NSDictionary *stored=[NSPropertyListSerialization propertyListWithData:plist options:NSPropertyListImmutable format:nil error:&error];
    Require([stored isKindOfClass:NSDictionary.class],error.description ?: @"Invalid Styles property list");
    Require([request[@"supported_metadata_versions"] containsObject:stored[@"0"]],
        [NSString stringWithFormat:@"Unsupported Styles metadata version 0.%@",stored[@"0"]]);
    NSData *coeff=stored[@"1"];
    Require([coeff isKindOfClass:NSData.class] && coeff.length>0,@"Missing Styles coefficients");
    Require(dlopen("/System/Library/PrivateFrameworks/NeutrinoCore.framework/NeutrinoCore",RTLD_NOW|RTLD_LOCAL)!=NULL,@"NeutrinoCore is unavailable");
    Class propertiesClass=NSClassFromString(@"_NUSemanticStyleProperties"),reverse=NSClassFromString(@"NUStyleTransferReverseNode"),
        thumb=NSClassFromString(@"_NUStyleTransferThumbnailProcessor"),apply=NSClassFromString(@"_NUStyleTransferApplyProcessor"),
        configClass=NSClassFromString(@"_NUStyleEngineConfiguration"),processor=NSClassFromString(@"CMIStyleEngineProcessor"),spaceClass=NSClassFromString(@"NUColorSpace");
    Check(propertiesClass,YES,@selector(semanticStylePropertiesFromData:fromVideo:keyTime:error:),"@",@[@"@",@"B",@"{CMTime=qiIq}",@"^@"]);
    Check(reverse,YES,@selector(reverseSettingsForSemanticStyleProperties:size:),"@",@[@"@",@"{XDNUSize=qq}"]);
    Check(spaceClass,YES,@selector(displayP3ColorSpace),"@",@[]);
    Check(configClass,NO,@selector(initWithConfigurationDictionary:),"@",@[@"@"]);
    Check(configClass,NO,@selector(styleEngineConfiguration),"@",@[]);
    Check(processor,YES,@selector(coefficientPixelBufferSizeForConfiguration:float16:),"{CGSize=dd}",@[@"@",@"B"]);
    Check(thumb,YES,@selector(generateThumbnailForImage:targetSize:colorSpace:configuration:tuningParameters:error:),"@",@[@"@",@"{XDNUSize=qq}",@"@",@"@",@"@",@"^@"]);
    Check(apply,YES,@selector(applyStyle:toImage:thumbnail:target:deltaMap:displacement:colorSpace:configuration:tuningParameters:noiseModel:error:),"@",@[@"@",@"@",@"@",@"@",@"@",@"@",@"@",@"@",@"@",@"@",@"^@"]);
    id properties=[propertiesClass semanticStylePropertiesFromData:plist fromVideo:NO keyTime:kCMTimeInvalid error:&error];
    Require(properties!=nil,error.description ?: @"Apple rejected Styles metadata");
    CIImage *input=Image(inputPath);
    NSDictionary *settings=[reverse reverseSettingsForSemanticStyleProperties:properties size:(XDNUSize){llround(input.extent.size.width),llround(input.extent.size.height)}];
    NSDictionary *configuration=nil,*tuning=nil;
    for(id value in settings.allValues) if([value isKindOfClass:NSDictionary.class]) {
        if(value[@"spotlightCountX"]) configuration=value;
        if(value[@"WeightPlaneStandardDeviation"]) tuning=value;
    }
    Require(configuration && tuning,@"Apple did not supply a supported reverse configuration");
    id cfg=[[configClass alloc]initWithConfigurationDictionary:configuration];
    BOOL half=[stored[@"2"] boolValue];
    CGSize coeffSize=[processor coefficientPixelBufferSizeForConfiguration:[cfg styleEngineConfiguration] float16:half];
    Require(coeffSize.width>0 && coeffSize.height>0 && coeffSize.width*coeffSize.height*(half?2:4)==coeff.length,@"Styles coefficient length disagrees with native configuration");
    CIImage *style=[CIImage imageWithBitmapData:coeff bytesPerRow:llround(coeffSize.width)*(half?2:4) size:coeffSize format:half?kCIFormatRh:kCIFormatRf colorSpace:nil];
    CGImageSourceRef source=CGImageSourceCreateWithURL((__bridge CFURLRef)[NSURL fileURLWithPath:inputPath],NULL);
    Require(source!=NULL,@"ImageIO cannot open source");
    NSDictionary *imageProperties=CFBridgingRelease(CGImageSourceCopyPropertiesAtIndex(source,0,NULL)); CFRelease(source);
    int orientation=[imageProperties[(__bridge id)kCGImagePropertyOrientation] intValue] ?: 1;
    Require(orientation>=1 && orientation<=8,@"Missing or unsupported image orientation");
    NSUInteger edge=[request[@"max_edge"] unsignedIntegerValue];
    double scale=edge?fmin(1.,(double)edge/fmax(input.extent.size.width,input.extent.size.height)):1.;
    CGSize canvas=CGSizeMake(floor(input.extent.size.width*scale),floor(input.extent.size.height*scale));
    input=Canvas(input,canvas);
    // Auxiliary dimensions may differ from the primary. Map each independently.
    CIImage *delta=Canvas(RawImage([dir stringByAppendingPathComponent:@"delta.heic"]),canvas);
    id space=[spaceClass displayP3ColorSpace];
    XDNUSize thumbSize={[configuration[@"thumbnailWidth"] longLongValue],[configuration[@"thumbnailHeight"] longLongValue]};
    Require(thumbSize.width>0 && thumbSize.height>0,@"Invalid native reverse thumbnail size");
    CIImage *thumbnail=[thumb generateThumbnailForImage:input targetSize:thumbSize colorSpace:space configuration:configuration tuningParameters:tuning error:&error];
    Require(thumbnail!=nil,error.description ?: @"Native reverse thumbnail failed");
    CIImage *unstyled=[apply applyStyle:style toImage:input thumbnail:thumbnail target:nil deltaMap:delta displacement:nil colorSpace:space configuration:configuration tuningParameters:tuning noiseModel:nil error:&error];
    Require(unstyled!=nil,error.description ?: @"Native reverse Apply failed");
    CGColorSpaceRef linear=CGColorSpaceCreateWithName(kCGColorSpaceExtendedLinearDisplayP3);
    CIContext *context=Context(linear),*unmanaged=Context(NULL);
    NSDictionary *u=Render(unstyled,context,linear,[dir stringByAppendingPathComponent:@"unstyled.f32"]);
    Render(input,context,linear,[dir stringByAppendingPathComponent:@"original_sdr.f32"]);
    // Store the original HDR as a baseline and a detail source, not as a scene reference.
    CIImage *hdr=[CIImage imageWithContentsOfURL:[NSURL fileURLWithPath:inputPath] options:@{kCIImageApplyOrientationProperty:@NO,kCIImageExpandToHDR:@YES}];
    NSDictionary *b=Render(Canvas(hdr,canvas),context,linear,[dir stringByAppendingPathComponent:@"original_hdr.f32"]);
    CIImage *lt=RawImage([dir stringByAppendingPathComponent:@"linear.heic"]);
    // Unmanaged rendering preserves the stored Log-like values despite its ICC tag.
    NSDictionary *l=Render(lt,unmanaged,NULL,[dir stringByAppendingPathComponent:@"thumbnail_codes.f32"]);
    CGColorSpaceRelease(linear);
    NSMutableDictionary *facts=[@{@"schema_version":@2,@"metadata_version":stored[@"0"] ?: @(-1),@"orientation":@(orientation),
        @"unstyled":u,@"original_hdr":b,@"thumbnail":l,@"coefficient_bytes":@(coeff.length),@"coefficient_float16":@(half),
        @"configuration":configuration,@"tuning":tuning,@"os":NSProcessInfo.processInfo.operatingSystemVersionString,
        @"native_version":[[properties valueForKey:@"version"] description]} mutableCopy];
    // Decode only documented-by-measurement buffers; no claim to invert GTC here.
    for(NSString *key in @[@"globalToneCurveData",@"lightMapData",@"linearLightMapData"]) {
        if([properties respondsToSelector:NSSelectorFromString(key)]) {
            id data=[properties valueForKey:key];
            if([data isKindOfClass:NSData.class]) {
                Require([data writeToFile:[dir stringByAppendingPathComponent:[key stringByAppendingString:@".bin"]] options:NSDataWritingAtomic error:&error],@"Cannot write native metadata buffer");
                facts[key]=@([data length]);
            }
        }
    }
    return facts;
}

static NSDictionary *Encode(NSDictionary *request) {
    NSString *path=request[@"input_path"],*out=request[@"output_path"];
    size_t w=[request[@"width"] unsignedLongValue],h=[request[@"height"] unsignedLongValue];
    NSData *data=[NSData dataWithContentsOfFile:path];
    Require(w>0 && h>0 && w<=32768 && h<=32768 && w*h<=100000000 && data.length==w*h*16,@"Invalid output pixel buffer");
    CGColorSpaceRef linear=CGColorSpaceCreateWithName(kCGColorSpaceExtendedLinearDisplayP3);
    CIContext *context=Context(linear);
    CIImage *image=[CIImage imageWithBitmapData:data bytesPerRow:w*16 size:CGSizeMake(w,h) format:kCIFormatRGBAf colorSpace:linear];
    int orientation=[request[@"orientation"] intValue] ?: 1;
    image=[image imageByApplyingOrientation:orientation];
    NSError *error=nil; NSURL *url=[NSURL fileURLWithPath:out];
    Require([out.pathExtension.lowercaseString isEqualToString:@"exr"],@"Linear diagnostic output requires an EXR destination");
    Require([context writeOpenEXRRepresentationOfImage:image toURL:url options:@{} error:&error],error.description ?: @"EXR encoding failed");
    CGColorSpaceRelease(linear);
    // Reopen and render the published representation to prove consumer decoding.
    CIImage *readback=[CIImage imageWithContentsOfURL:url options:@{kCIImageApplyOrientationProperty:@YES,kCIImageExpandToHDR:@YES}];
    const int inverse[]={0,1,2,3,4,5,8,7,6};
    readback=[readback imageByApplyingOrientation:inverse[orientation]];
    linear=CGColorSpaceCreateWithName(kCGColorSpaceExtendedLinearDisplayP3);
    NSDictionary *facts=Render(readback,Context(linear),linear,[request[@"readback_path"] stringByAppendingString:@".f32"]);
    CGColorSpaceRelease(linear);
    return @{@"schema_version":@2,@"readback":facts};
}

static NSDictionary *DecodeMatte(NSDictionary *request) {
    NSString *path=request[@"input_path"],*output=request[@"output_path"];
    size_t w=[request[@"width"]unsignedIntegerValue],h=[request[@"height"]unsignedIntegerValue];
    Require(w>0&&h>0&&w<=32768&&h<=32768&&w*h<=100000000,@"Invalid matte output geometry");
    CIImage *image=[CIImage imageWithContentsOfURL:[NSURL fileURLWithPath:path] options:@{kCIImageApplyOrientationProperty:@NO,kCIImageColorSpace:NSNull.null}];
    Require(image!=nil,@"Cannot decode linked matte");
    CIContext *context=[CIContext contextWithOptions:@{kCIContextWorkingColorSpace:NSNull.null,kCIContextOutputColorSpace:NSNull.null}];
    NSMutableData *data=[NSMutableData dataWithLength:w*h*4];
    [context render:Canvas(image,CGSizeMake(w,h)) toBitmap:data.mutableBytes rowBytes:w*4 bounds:CGRectMake(0,0,w,h) format:kCIFormatRf colorSpace:NULL];
    float *pixels=data.mutableBytes;size_t nonzero=0;for(size_t i=0;i<w*h;i++){Require(isfinite(pixels[i]),@"Nonfinite matte");pixels[i]=fminf(fmaxf(pixels[i],0),1);nonzero+=pixels[i]>0;}
    NSError *error=nil;Require([data writeToFile:output options:NSDataWritingAtomic error:&error],error.description?:@"Matte pixel write failed");
    return @{@"schema_version":@2,@"width":@(w),@"height":@(h),@"positive_pixels":@(nonzero),@"orientation_applied":@NO,@"color_managed":@NO};
}

static NSDictionary *Readback(NSDictionary *request) {
    NSString *dir=request[@"output_path"],*path=request[@"input_path"];
    CGSize size=CGSizeMake([request[@"width"] doubleValue],[request[@"height"] doubleValue]);
    CGColorSpaceRef linear=CGColorSpaceCreateWithName(kCGColorSpaceExtendedLinearDisplayP3);
    CIContext *context=Context(linear);
    NSDictionary *sdr=Render(Canvas(Image(path),size),context,linear,[dir stringByAppendingPathComponent:@"published_sdr.f32"]);
    CIImage *hdr=[CIImage imageWithContentsOfURL:[NSURL fileURLWithPath:path]
        options:@{kCIImageApplyOrientationProperty:@NO,kCIImageExpandToHDR:@YES}];
    NSDictionary *expanded=Render(Canvas(hdr,size),context,linear,[dir stringByAppendingPathComponent:@"published_hdr.f32"]);
    CGColorSpaceRelease(linear);
    return @{@"schema_version":@2,@"sdr":sdr,@"hdr":expanded,@"hdr_content_headroom":@(hdr.contentHeadroom)};
}

NSDictionary *XDStylesPerform(NSDictionary *request, NSError **error) {
    NSDictionary *result=nil;
    NSError *failure=nil;
    @autoreleasepool {
    @try {
        if([request[@"operation"] isEqualToString:@"styles-decode-reverse"]) result=Decode(request);
        else if([request[@"operation"] isEqualToString:@"styles-readback"]) result=Readback(request);
        else if([request[@"operation"] isEqualToString:@"styles-decode-matte"]) result=DecodeMatte(request);
        else result=Encode(request);
    } @catch(NSException *exception) {
        failure=[NSError errorWithDomain:@"XDStylesNative" code:1 userInfo:@{NSLocalizedDescriptionKey:exception.reason ?: exception.name}];
    }
    }
    // Both objects must survive the operation's pool. NSError** is an
    // autoreleasing out parameter; assign it only after the pool has drained.
    if(error) *error=failure;
    return result;
}
