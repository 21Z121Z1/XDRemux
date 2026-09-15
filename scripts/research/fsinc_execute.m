#import <Foundation/Foundation.h>
#import <CoreImage/CoreImage.h>
#import <CoreVideo/CoreVideo.h>
#import <CoreGraphics/CoreGraphics.h>
#import <ImageIO/ImageIO.h>
#import <objc/runtime.h>
#import <objc/message.h>
#import <dlfcn.h>

typedef NSString *(*NameFn)(uint64_t);

static const uint64_t kFSINCV23 = 0x2012c; // recovered from ANSTFsincAlgorithmVersionToNSString
static const uint64_t kFSINCV24 = 0x20190; // recovered from ANSTFsincAlgorithmVersionToNSString

static const char *descstr(id obj) { return obj ? [[obj description] UTF8String] : "nil"; }

static CVPixelBufferRef makeBGRA(NSString *path, size_t w, size_t h) {
    CIImage *src = [CIImage imageWithContentsOfURL:[NSURL fileURLWithPath:path] options:@{}];
    if (!src) { fprintf(stderr, "CIImage load failed\n"); return NULL; }
    CGRect e = src.extent;
    CGFloat s = MAX((CGFloat)w/e.size.width, (CGFloat)h/e.size.height);
    CIImage *scaled = [src imageByApplyingTransform:CGAffineTransformMakeScale(s,s)];
    CGRect se = scaled.extent;
    CIImage *placed = [scaled imageByApplyingTransform:CGAffineTransformMakeTranslation(((CGFloat)w-se.size.width)*.5-se.origin.x, ((CGFloat)h-se.size.height)*.5-se.origin.y)];
    NSDictionary *attrs=@{(id)kCVPixelBufferIOSurfacePropertiesKey:@{},
                          (id)kCVPixelBufferCGImageCompatibilityKey:@YES,
                          (id)kCVPixelBufferCGBitmapContextCompatibilityKey:@YES};
    CVPixelBufferRef pb=NULL;
    if (CVPixelBufferCreate(kCFAllocatorDefault,w,h,kCVPixelFormatType_32BGRA,(__bridge CFDictionaryRef)attrs,&pb)!=kCVReturnSuccess) return NULL;
    CIContext *ctx=[CIContext contextWithOptions:@{kCIContextUseSoftwareRenderer:@NO}];
    CGColorSpaceRef cs=CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
    [ctx render:placed toCVPixelBuffer:pb bounds:CGRectMake(0,0,w,h) colorSpace:cs];
    CGColorSpaceRelease(cs);
    return pb;
}

static BOOL writeMask(CVPixelBufferRef pb, NSString *path) {
    if (!pb) return NO;
    printf("MASK %s %zux%zu fmt=0x%08x\n",path.UTF8String,CVPixelBufferGetWidth(pb),CVPixelBufferGetHeight(pb),(unsigned)CVPixelBufferGetPixelFormatType(pb));
    CIContext *ctx=[CIContext contextWithOptions:@{kCIContextUseSoftwareRenderer:@NO}];
    CGColorSpaceRef gray=CGColorSpaceCreateDeviceGray();
    NSError *err=nil;
    BOOL ok=[ctx writePNGRepresentationOfImage:[CIImage imageWithCVPixelBuffer:pb]
                                         toURL:[NSURL fileURLWithPath:path]
                                        format:kCIFormatL8
                                    colorSpace:gray
                                       options:@{}
                                         error:&err];
    CGColorSpaceRelease(gray);
    if(!ok) fprintf(stderr,"PNG_ERROR %s %s\n",path.UTF8String,descstr(err));
    return ok;
}

static Class concreteClass(Class base,uint64_t version) {
    @try {
        return ((Class(*)(id,SEL,uint64_t))objc_msgSend)((id)base,sel_registerName("_concreteClassOfVersion:"),version);
    } @catch(NSException *e) {
        printf("CONCRETE_EXCEPTION version=0x%llx %s: %s\n",(unsigned long long)version,e.name.UTF8String,e.reason.UTF8String);
        return Nil;
    }
}

static id runFSINC(Class base, Class cfgCls, uint64_t version, NSUInteger resolution, NSString *input) {
    @try {
        Class cls=concreteClass(base,version);
        printf("CONCRETE version=0x%llx => %s\n",(unsigned long long)version,cls?class_getName(cls):"nil");
        if(!cls) return nil;

        id cfgAlloc=((id(*)(id,SEL))objc_msgSend)((id)cfgCls,sel_registerName("alloc"));
        id cfg=((id(*)(id,SEL,uint64_t,NSUInteger))objc_msgSend)(cfgAlloc,sel_registerName("initWithVersion:resolution:"),version,resolution);
        id algAlloc=((id(*)(id,SEL))objc_msgSend)((id)cls,sel_registerName("alloc"));
        id alg=((id(*)(id,SEL,id))objc_msgSend)(algAlloc,sel_registerName("initWithConfiguration:"),cfg);
        printf("RUN version=0x%llx resolution=%lu cfg=%s class=%s\n",
               (unsigned long long)version,(unsigned long)resolution,descstr(cfg),alg?object_getClassName(alg):"nil");
        if(!alg) return nil;

        NSError *err=nil;
        BOOL prepared=((BOOL(*)(id,SEL,NSError**))objc_msgSend)(alg,sel_registerName("prepareWithError:"),&err);
        printf("  prepare=%d err=%s\n",prepared,descstr(err));
        if(!prepared) return nil;

        if([alg respondsToSelector:sel_registerName("networkInputImageDescriptor")]) {
            id d=((id(*)(id,SEL))objc_msgSend)(alg,sel_registerName("networkInputImageDescriptor"));
            printf("  inputDescriptor=%s\n",descstr(d));
        }

        size_t w=768,h=576;
        if(resolution==1){w=576;h=768;}
        else if(resolution==2){w=256;h=256;}
        CVPixelBufferRef pb=makeBGRA(input,w,h);
        if(!pb) return nil;

        err=nil;
        BOOL bound=((BOOL(*)(id,SEL,CVPixelBufferRef,NSError**))objc_msgSend)(alg,sel_registerName("bindNetworkInputPixelBuffer:error:"),pb,&err);
        printf("  bind=%d err=%s\n",bound,descstr(err));
        if(!bound){CFRelease(pb);return nil;}

        err=nil;
        id result=((id(*)(id,SEL,NSError**))objc_msgSend)(alg,sel_registerName("executeInferenceWithError:"),&err);
        printf("  execute=%s class=%s err=%s\n",descstr(result),result?object_getClassName(result):"nil",descstr(err));
        CFRelease(pb);
        return result;
    } @catch(NSException *e) {
        printf("RUN_EXCEPTION version=0x%llx resolution=%lu %s: %s\n",
               (unsigned long long)version,(unsigned long)resolution,e.name.UTF8String,e.reason.UTF8String);
        return nil;
    }
}

int main(int argc,const char*argv[]){ @autoreleasepool {
    if(argc<3)return 64;
    NSString *input=[NSString stringWithUTF8String:argv[1]];
    NSString *out=[NSString stringWithUTF8String:argv[2]];
    void *h=dlopen("/System/Library/PrivateFrameworks/ANSTKit.framework/Versions/A/ANSTKit",RTLD_NOW|RTLD_GLOBAL);
    if(!h){fprintf(stderr,"dlopen failed: %s\n",dlerror());return 2;}

    NameFn algName=(NameFn)dlsym(h,"ANSTFsincAlgorithmVersionToNSString");
    NameFn infName=(NameFn)dlsym(h,"ANSTFsincInferenceVersionToNSString");
    NameFn resName=(NameFn)dlsym(h,"ANSTFsincAlgorithmResolutionToNSString");
    uint64_t versions[]={kFSINCV24,kFSINCV23};
    for(size_t i=0;i<2;i++){
        uint64_t v=versions[i];
        printf("VERSION 0x%llx algorithm=%s inference=%s\n",(unsigned long long)v,
               algName?descstr(algName(v)):"nil",infName?descstr(infName(v)):"nil");
    }
    for(NSUInteger r=0;r<3;r++) printf("RESOLUTION %lu=%s\n",(unsigned long)r,resName?descstr(resName(r)):"nil");

    Class base=NSClassFromString(@"ANSTFsincAlgorithm");
    Class cfgCls=NSClassFromString(@"ANSTFsincAlgorithmConfiguration");
    if(!base||!cfgCls){fprintf(stderr,"FSINC base/config classes missing\n");return 3;}

    id result=nil;
    uint64_t usedVersion=0;
    NSUInteger usedResolution=NSNotFound;
    // Portrait first; then landscape and square to distinguish an entry-point-specific compiler problem.
    NSUInteger resolutions[]={1,0,2};
    for(size_t vi=0;vi<2&&!result;vi++){
        for(size_t ri=0;ri<3&&!result;ri++){
            result=runFSINC(base,cfgCls,versions[vi],resolutions[ri],input);
            if(result){usedVersion=versions[vi];usedResolution=resolutions[ri];}
        }
    }
    if(!result){fprintf(stderr,"NO_SUCCESSFUL_FSINC_RESULT\n");return 10;}
    printf("SUCCESS version=0x%llx resolution=%lu class=%s\n",(unsigned long long)usedVersion,(unsigned long)usedResolution,object_getClassName(result));

    NSUInteger count=0;
    SEL countSel=sel_registerName("outputMaskCount");
    if([result respondsToSelector:countSel]) count=((NSUInteger(*)(id,SEL))objc_msgSend)(result,countSel);
    printf("INSTANCE_MASK_COUNT=%lu\n",(unsigned long)count);
    for(NSUInteger i=0;i<count;i++){
        NSError *err=nil;
        CVPixelBufferRef pb=((CVPixelBufferRef(*)(id,SEL,NSUInteger,NSError**))objc_msgSend)(result,sel_registerName("outputMaskAtIndex:error:"),i,&err);
        printf("INSTANCE index=%lu pb=%p err=%s\n",(unsigned long)i,pb,descstr(err));
        if(pb) writeMask(pb,[out stringByAppendingPathComponent:[NSString stringWithFormat:@"instances/instance-%02lu.png",(unsigned long)i]]);
    }

    NSUInteger sem=0;
    SEL semSel=sel_registerName("outputVisegMaskForCategory:error:");
    if([result respondsToSelector:semSel]){
        for(NSUInteger cat=0;cat<64;cat++){@autoreleasepool{
            NSError *err=nil;
            CVPixelBufferRef pb=((CVPixelBufferRef(*)(id,SEL,NSUInteger,NSError**))objc_msgSend)(result,semSel,cat,&err);
            if(pb){
                sem++;
                printf("SEMANTIC category=%lu pb=%p\n",(unsigned long)cat,pb);
                writeMask(pb,[out stringByAppendingPathComponent:[NSString stringWithFormat:@"semantic/category-%02lu.png",(unsigned long)cat]]);
            }
        }}
    }
    printf("SEMANTIC_MASK_COUNT=%lu\n",(unsigned long)sem);
    return 0;
}}
