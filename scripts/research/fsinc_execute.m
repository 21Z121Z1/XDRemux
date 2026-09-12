#import <Foundation/Foundation.h>
#import <CoreImage/CoreImage.h>
#import <CoreVideo/CoreVideo.h>
#import <CoreGraphics/CoreGraphics.h>
#import <ImageIO/ImageIO.h>
#import <objc/runtime.h>
#import <objc/message.h>
#import <dlfcn.h>

typedef NSString *(*NameFn)(uint64_t);

static const char *descstr(id obj) { return obj ? [[obj description] UTF8String] : "nil"; }

static CVPixelBufferRef makeBGRA(NSString *path, size_t w, size_t h) {
    CIImage *src = [CIImage imageWithContentsOfURL:[NSURL fileURLWithPath:path] options:@{}];
    if (!src) return NULL;
    CGRect e = src.extent;
    CGFloat s = MAX((CGFloat)w/e.size.width, (CGFloat)h/e.size.height);
    CIImage *scaled = [src imageByApplyingTransform:CGAffineTransformMakeScale(s,s)];
    CGRect se = scaled.extent;
    CIImage *placed = [scaled imageByApplyingTransform:CGAffineTransformMakeTranslation(((CGFloat)w-se.size.width)*.5-se.origin.x, ((CGFloat)h-se.size.height)*.5-se.origin.y)];
    NSDictionary *attrs=@{(id)kCVPixelBufferIOSurfacePropertiesKey:@{},(id)kCVPixelBufferCGImageCompatibilityKey:@YES,(id)kCVPixelBufferCGBitmapContextCompatibilityKey:@YES};
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
    BOOL ok=[ctx writePNGRepresentationOfImage:[CIImage imageWithCVPixelBuffer:pb] toURL:[NSURL fileURLWithPath:path] format:kCIFormatL8 colorSpace:gray options:@{} error:&err];
    CGColorSpaceRelease(gray);
    if(!ok) fprintf(stderr,"PNG_ERROR %s %s\n",path.UTF8String,descstr(err));
    return ok;
}

static NSString *safeName(NameFn fn,uint64_t v){ if(!fn)return nil; @try{return fn(v);}@catch(NSException *e){return nil;} }
static BOOL validName(NSString *s){ if(!s.length)return NO; NSString*l=s.lowercaseString; return ![l containsString:@"unexpected"]&&![l containsString:@"unknown"]&&![l containsString:@"invalid"]; }
static Class concrete(Class base,uint64_t v){ @try{return ((Class(*)(id,SEL,uint64_t))objc_msgSend)((id)base,sel_registerName("_concreteClassOfVersion:"),v);}@catch(NSException*e){return Nil;} }

static void dumpClassMethods(Class cls){
    if(!cls)return;
    unsigned n=0; Method*m=class_copyMethodList(cls,&n);
    printf("CLASS_METHODS %s n=%u\n",class_getName(cls),n);
    for(unsigned i=0;i<n;i++) printf("  - %s %s\n",sel_getName(method_getName(m[i])),method_getTypeEncoding(m[i]));
    free(m);
}

static id tryRun(Class cls, Class cfgCls, uint64_t v, NSUInteger r, NSString *input) {
    @try {
        id ca=((id(*)(id,SEL))objc_msgSend)((id)cfgCls,sel_registerName("alloc"));
        id cfg=((id(*)(id,SEL,uint64_t,NSUInteger))objc_msgSend)(ca,sel_registerName("initWithVersion:resolution:"),v,r);
        id aa=((id(*)(id,SEL))objc_msgSend)((id)cls,sel_registerName("alloc"));
        id alg=((id(*)(id,SEL,id))objc_msgSend)(aa,sel_registerName("initWithConfiguration:"),cfg);
        printf("RUN class=%s v=0x%llx r=%lu cfg=%s instanceClass=%s\n",class_getName(cls),(unsigned long long)v,(unsigned long)r,descstr(cfg),alg?object_getClassName(alg):"nil");
        if(!alg)return nil;
        SEL prepSel=sel_registerName("prepareWithError:");
        SEL bindSel=sel_registerName("bindNetworkInputPixelBuffer:error:");
        SEL execSel=sel_registerName("executeInferenceWithError:");
        if(![alg respondsToSelector:prepSel]||![alg respondsToSelector:bindSel]||![alg respondsToSelector:execSel]) { printf("  missing prepare/bind/execute\n"); return nil; }
        NSError*err=nil;
        BOOL prepared=((BOOL(*)(id,SEL,NSError**))objc_msgSend)(alg,prepSel,&err);
        printf("  prepare=%d err=%s\n",prepared,descstr(err));
        if(!prepared)return nil;
        size_t w=r==1?576:(r==2?256:768), h=r==1?768:(r==2?256:576);
        CVPixelBufferRef pb=makeBGRA(input,w,h); if(!pb)return nil;
        err=nil;
        BOOL bound=((BOOL(*)(id,SEL,CVPixelBufferRef,NSError**))objc_msgSend)(alg,bindSel,pb,&err);
        printf("  bind=%d err=%s\n",bound,descstr(err));
        if(!bound){CFRelease(pb);return nil;}
        err=nil;
        id result=((id(*)(id,SEL,NSError**))objc_msgSend)(alg,execSel,&err);
        printf("  execute=%s class=%s err=%s\n",descstr(result),result?object_getClassName(result):"nil",descstr(err));
        CFRelease(pb);
        return result;
    } @catch(NSException*e){ printf("  EXCEPTION %s: %s\n",e.name.UTF8String,e.reason.UTF8String); return nil; }
}

int main(int argc,const char*argv[]){@autoreleasepool{
    if(argc<3)return 64;
    NSString*input=[NSString stringWithUTF8String:argv[1]],*out=[NSString stringWithUTF8String:argv[2]];
    void*h=dlopen("/System/Library/PrivateFrameworks/ANSTKit.framework/Versions/A/ANSTKit",RTLD_NOW|RTLD_GLOBAL); if(!h)return 2;
    NameFn algName=(NameFn)dlsym(h,"ANSTFsincAlgorithmVersionToNSString");
    NameFn infName=(NameFn)dlsym(h,"ANSTFsincInferenceVersionToNSString");
    Class base=NSClassFromString(@"ANSTFsincAlgorithm"),cfgCls=NSClassFromString(@"ANSTFsincAlgorithmConfiguration"),direct=NSClassFromString(@"ANSTFsincAlgorithmV2Dot4");
    printf("BASE=%s DIRECT_V2D4=%s\n",base?class_getName(base):"nil",direct?class_getName(direct):"nil");
    dumpClassMethods(direct);

    uint64_t candidates[]={
      0,1,2,3,4,5,6,7,8,9,10,11,
      23,24,25,26,203,204,205,206,230,240,250,260,
      0x203,0x204,0x205,0x206,
      0x20003,0x20004,0x20005,0x20006,
      0x20300,0x20400,0x20500,0x20600,
      0x2000300,0x2000400,0x2000500,0x2000600,
      0x02030000,0x02040000,0x02050000,0x02060000,
      ((uint64_t)2<<16)|3,((uint64_t)2<<16)|4,((uint64_t)2<<16)|5,((uint64_t)2<<16)|6,
      ((uint64_t)2<<32)|3,((uint64_t)2<<32)|4,((uint64_t)2<<32)|5,((uint64_t)2<<32)|6,
      UINT32_MAX,UINT64_MAX
    };
    uint64_t discovered[128]; size_t discoveredN=0;
    for(size_t i=0;i<sizeof(candidates)/sizeof(candidates[0]);i++){
      uint64_t v=candidates[i]; NSString*a=safeName(algName,v),*b=safeName(infName,v); Class c=concrete(base,v);
      printf("CAND 0x%llx alg=%s inf=%s concrete=%s\n",(unsigned long long)v,a.UTF8String?:"nil",b.UTF8String?:"nil",c?class_getName(c):"nil");
      if((a&&validName(a))||(b&&validName(b))||(c&&c!=base)){ discovered[discoveredN++]=v; }
    }
    if(discoveredN==0){
      for(uint64_t v=0;v<=0xffff;v++){ Class c=concrete(base,v); if(c&&c!=base){printf("SCAN_HIT 0x%llx class=%s\n",(unsigned long long)v,class_getName(c));discovered[discoveredN++]=v;if(discoveredN==128)break;} }
    }
    printf("DISCOVERED_N=%zu\n",discoveredN);

    id result=nil; uint64_t usedV=0;
    for(size_t i=0;i<discoveredN&&!result;i++){
      uint64_t v=discovered[i]; Class c=concrete(base,v); if(!c)c=direct; if(c){result=tryRun(c,cfgCls,v,1,input);if(result)usedV=v;}
    }
    if(!result&&direct){
      for(size_t i=0;i<sizeof(candidates)/sizeof(candidates[0])&&!result;i++){result=tryRun(direct,cfgCls,candidates[i],1,input);if(result)usedV=candidates[i];}
    }
    if(!result){fprintf(stderr,"NO_SUCCESSFUL_FSINC_RESULT\n");return 10;}
    printf("SUCCESS_VERSION=0x%llx RESULT_CLASS=%s\n",(unsigned long long)usedV,object_getClassName(result));

    SEL cntSel=sel_registerName("outputMaskCount"); NSUInteger count=[result respondsToSelector:cntSel]?((NSUInteger(*)(id,SEL))objc_msgSend)(result,cntSel):0;
    printf("INSTANCE_MASK_COUNT=%lu\n",(unsigned long)count);
    for(NSUInteger i=0;i<count;i++){
      NSError*err=nil; CVPixelBufferRef pb=((CVPixelBufferRef(*)(id,SEL,NSUInteger,NSError**))objc_msgSend)(result,sel_registerName("outputMaskAtIndex:error:"),i,&err);
      id score=((id(*)(id,SEL,NSInteger,NSError**))objc_msgSend)(result,sel_registerName("outputMaskConfidenceScoreAtIndex:error:"),(NSInteger)i,&err);
      printf("INSTANCE %lu score=%s err=%s\n",(unsigned long)i,descstr(score),descstr(err));
      if(pb)writeMask(pb,[out stringByAppendingPathComponent:[NSString stringWithFormat:@"instances/instance-%02lu.png",(unsigned long)i]]);
    }
    NSUInteger sem=0; SEL semSel=sel_registerName("outputVisegMaskForCategory:error:");
    if([result respondsToSelector:semSel])for(NSUInteger cat=0;cat<64;cat++){@autoreleasepool{NSError*err=nil;CVPixelBufferRef pb=((CVPixelBufferRef(*)(id,SEL,NSUInteger,NSError**))objc_msgSend)(result,semSel,cat,&err);if(pb){sem++;printf("SEMANTIC %lu\n",(unsigned long)cat);writeMask(pb,[out stringByAppendingPathComponent:[NSString stringWithFormat:@"semantic/category-%02lu.png",(unsigned long)cat]]);}}}
    printf("SEMANTIC_MASK_COUNT=%lu\n",(unsigned long)sem);
    return 0;
}}
