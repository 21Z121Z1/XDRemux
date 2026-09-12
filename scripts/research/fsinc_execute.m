#import <Foundation/Foundation.h>
#import <CoreImage/CoreImage.h>
#import <CoreVideo/CoreVideo.h>
#import <CoreGraphics/CoreGraphics.h>
#import <ImageIO/ImageIO.h>
#import <objc/runtime.h>
#import <objc/message.h>
#import <dlfcn.h>

typedef NSString *(*NameFn)(NSUInteger);

static id send_id(id obj, SEL sel) {
    return ((id(*)(id,SEL))objc_msgSend)(obj, sel);
}
static NSUInteger send_u(id obj, SEL sel) {
    return ((NSUInteger(*)(id,SEL))objc_msgSend)(obj, sel);
}

static CVPixelBufferRef makeBGRA(NSString *path, size_t w, size_t h) {
    NSURL *url = [NSURL fileURLWithPath:path];
    CIImage *src = [CIImage imageWithContentsOfURL:url options:@{}];
    if (!src) { fprintf(stderr, "CIImage load failed\n"); return NULL; }
    CGRect e = src.extent;
    CGFloat sx = (CGFloat)w / e.size.width;
    CGFloat sy = (CGFloat)h / e.size.height;
    CGFloat s = MAX(sx, sy);
    CIImage *scaled = [src imageByApplyingTransform:CGAffineTransformMakeScale(s, s)];
    CGRect se = scaled.extent;
    CGFloat tx = ((CGFloat)w - se.size.width) * 0.5 - se.origin.x;
    CGFloat ty = ((CGFloat)h - se.size.height) * 0.5 - se.origin.y;
    CIImage *placed = [scaled imageByApplyingTransform:CGAffineTransformMakeTranslation(tx, ty)];

    NSDictionary *attrs = @{(id)kCVPixelBufferIOSurfacePropertiesKey:@{},
                            (id)kCVPixelBufferCGImageCompatibilityKey:@YES,
                            (id)kCVPixelBufferCGBitmapContextCompatibilityKey:@YES};
    CVPixelBufferRef pb = NULL;
    CVReturn r = CVPixelBufferCreate(kCFAllocatorDefault, w, h, kCVPixelFormatType_32BGRA,
                                     (__bridge CFDictionaryRef)attrs, &pb);
    if (r != kCVReturnSuccess || !pb) { fprintf(stderr, "CVPixelBufferCreate=%d\n", r); return NULL; }
    CIContext *ctx = [CIContext contextWithOptions:@{kCIContextUseSoftwareRenderer:@NO}];
    CGColorSpaceRef cs = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
    [ctx render:placed toCVPixelBuffer:pb bounds:CGRectMake(0,0,w,h) colorSpace:cs];
    CGColorSpaceRelease(cs);
    return pb;
}

static BOOL writeMask(CVPixelBufferRef pb, NSString *path) {
    if (!pb) return NO;
    size_t w = CVPixelBufferGetWidth(pb), h = CVPixelBufferGetHeight(pb);
    OSType fmt = CVPixelBufferGetPixelFormatType(pb);
    printf("MASK %s %zux%zu fmt=0x%08x\n", path.UTF8String, w, h, (unsigned)fmt);
    CIImage *im = [CIImage imageWithCVPixelBuffer:pb];
    CIContext *ctx = [CIContext contextWithOptions:@{kCIContextUseSoftwareRenderer:@NO}];
    NSURL *url = [NSURL fileURLWithPath:path];
    CGColorSpaceRef gray = CGColorSpaceCreateDeviceGray();
    NSError *error = nil;
    BOOL ok = [ctx writePNGRepresentationOfImage:im toURL:url format:kCIFormatL8 colorSpace:gray options:@{} error:&error];
    CGColorSpaceRelease(gray);
    if (!ok) fprintf(stderr, "write PNG failed %s: %s\n", path.UTF8String, error.description.UTF8String);
    return ok;
}

static NSString *safeName(NameFn fn, NSUInteger x) {
    if (!fn) return nil;
    @try { return fn(x); } @catch (NSException *e) { return nil; }
}

static BOOL usefulName(NSString *s) {
    if (!s.length) return NO;
    NSString *l = s.lowercaseString;
    return ![l containsString:@"unknown"] && ![l containsString:@"unsupported"] && ![l containsString:@"invalid"];
}

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        if (argc < 3) return 64;
        NSString *input = [NSString stringWithUTF8String:argv[1]];
        NSString *outDir = [NSString stringWithUTF8String:argv[2]];
        void *h = dlopen("/System/Library/PrivateFrameworks/ANSTKit.framework/Versions/A/ANSTKit", RTLD_NOW|RTLD_GLOBAL);
        if (!h) { fprintf(stderr, "dlopen failed: %s\n", dlerror()); return 2; }

        NameFn versionName = (NameFn)dlsym(h, "ANSTFsincAlgorithmVersionToNSString");
        NameFn resolutionName = (NameFn)dlsym(h, "ANSTFsincAlgorithmResolutionToNSString");
        printf("=== enum discovery ===\n");
        for (NSUInteger v=0; v<12; v++) printf("VERSION %lu = %s\n", (unsigned long)v, safeName(versionName,v).UTF8String ?: "(nil)");
        for (NSUInteger r=0; r<12; r++) printf("RESOLUTION %lu = %s\n", (unsigned long)r, safeName(resolutionName,r).UTF8String ?: "(nil)");

        Class cfgCls = NSClassFromString(@"ANSTFsincAlgorithmConfiguration");
        Class algCls = NSClassFromString(@"ANSTFsincAlgorithm");
        if (!cfgCls || !algCls) { fprintf(stderr, "FSINC classes missing\n"); return 3; }

        id successfulResult = nil;
        NSUInteger successfulV = NSNotFound, successfulR = NSNotFound;
        for (NSUInteger v=0; v<12 && !successfulResult; v++) {
            NSString *vn = safeName(versionName,v);
            if (!usefulName(vn)) continue;
            for (NSUInteger r=0; r<12 && !successfulResult; r++) {
                NSString *rn = safeName(resolutionName,r);
                if (!usefulName(rn)) continue;
                size_t w=0,hgt=0;
                NSString *low = rn.lowercaseString;
                if ([low containsString:@"768"] && [low containsString:@"576"]) {
                    NSRange a=[low rangeOfString:@"768"], b=[low rangeOfString:@"576"];
                    if (a.location < b.location) { w=768; hgt=576; } else { w=576; hgt=768; }
                } else if ([low containsString:@"256"]) { w=256; hgt=256; }
                else continue;
                @try {
                    id cfgAlloc = ((id(*)(id,SEL))objc_msgSend)((id)cfgCls, sel_registerName("alloc"));
                    id cfg = ((id(*)(id,SEL,NSUInteger,NSUInteger))objc_msgSend)(cfgAlloc, sel_registerName("initWithVersion:resolution:"), v, r);
                    printf("TRY v=%lu(%s) r=%lu(%s) cfg=%s\n", (unsigned long)v, vn.UTF8String, (unsigned long)r, rn.UTF8String, [[cfg description] UTF8String]);
                    id algAlloc = ((id(*)(id,SEL))objc_msgSend)((id)algCls, sel_registerName("alloc"));
                    id alg = ((id(*)(id,SEL,id))objc_msgSend)(algAlloc, sel_registerName("initWithConfiguration:"), cfg);
                    if (!alg) { printf("  alg=nil\n"); continue; }
                    id inputDesc = send_id(alg, sel_registerName("networkInputImageDescriptor"));
                    printf("  alg=%s inputDesc=%s\n", [[alg description] UTF8String], [[inputDesc description] UTF8String]);
                    CVPixelBufferRef inputPB = makeBGRA(input,w,hgt);
                    if (!inputPB) continue;
                    NSError *err=nil;
                    BOOL bound = ((BOOL(*)(id,SEL,CVPixelBufferRef,NSError**))objc_msgSend)(alg, sel_registerName("bindNetworkInputPixelBuffer:error:"), inputPB, &err);
                    printf("  bind=%d error=%s\n", bound, err.description.UTF8String ?: "none");
                    if (!bound) { CFRelease(inputPB); continue; }
                    err=nil;
                    id result = ((id(*)(id,SEL,NSError**))objc_msgSend)(alg, sel_registerName("executeInferenceWithError:"), &err);
                    printf("  execute result=%s class=%s error=%s\n", [[result description] UTF8String], result ? object_getClassName(result) : "nil", err.description.UTF8String ?: "none");
                    CFRelease(inputPB);
                    if (result) { successfulResult=result; successfulV=v; successfulR=r; }
                } @catch (NSException *e) {
                    printf("  EXCEPTION %s: %s\n", e.name.UTF8String, e.reason.UTF8String);
                }
            }
        }

        if (!successfulResult) { fprintf(stderr, "NO_SUCCESSFUL_FSINC_RESULT\n"); return 10; }
        printf("SUCCESS v=%lu r=%lu\n", (unsigned long)successfulV, (unsigned long)successfulR);
        NSUInteger count = send_u(successfulResult, sel_registerName("outputMaskCount"));
        printf("INSTANCE_MASK_COUNT=%lu\n", (unsigned long)count);
        for (NSUInteger i=0; i<count; i++) {
            NSError *err=nil;
            CVPixelBufferRef pb = ((CVPixelBufferRef(*)(id,SEL,NSUInteger,NSError**))objc_msgSend)(successfulResult, sel_registerName("outputMaskAtIndex:error:"), i, &err);
            id score = ((id(*)(id,SEL,NSInteger,NSError**))objc_msgSend)(successfulResult, sel_registerName("outputMaskConfidenceScoreAtIndex:error:"), (NSInteger)i, &err);
            printf("INSTANCE %lu score=%s error=%s\n", (unsigned long)i, [[score description] UTF8String], err.description.UTF8String ?: "none");
            if (pb) writeMask(pb, [outDir stringByAppendingPathComponent:[NSString stringWithFormat:@"instances/instance-%02lu.png",(unsigned long)i]]);
        }

        NSUInteger semanticOK=0;
        for (NSUInteger cat=0; cat<64; cat++) {
            NSError *err=nil;
            CVPixelBufferRef pb = ((CVPixelBufferRef(*)(id,SEL,NSUInteger,NSError**))objc_msgSend)(successfulResult, sel_registerName("outputVisegMaskForCategory:error:"), cat, &err);
            if (!pb) continue;
            semanticOK++;
            printf("SEMANTIC category=%lu error=%s\n", (unsigned long)cat, err.description.UTF8String ?: "none");
            writeMask(pb, [outDir stringByAppendingPathComponent:[NSString stringWithFormat:@"semantic/category-%02lu.png",(unsigned long)cat]]);
        }
        printf("SEMANTIC_MASK_COUNT=%lu\n", (unsigned long)semanticOK);
        return semanticOK ? 0 : 11;
    }
}
