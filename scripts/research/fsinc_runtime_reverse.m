#import <Foundation/Foundation.h>
#import <objc/runtime.h>
#import <objc/message.h>
#import <dlfcn.h>

static void dump_symbol(void *h, const char *name, NSString *dir) {
    void *p = dlsym(h, name);
    Dl_info info = {0};
    dladdr(p, &info);
    printf("SYMBOL %s ptr=%p image=%s base=%p sym=%s\n", name, p,
           info.dli_fname ?: "?", info.dli_fbase, info.dli_sname ?: "?");
    if (!p) return;
    NSData *d = [NSData dataWithBytes:p length:1024];
    NSString *path = [dir stringByAppendingPathComponent:[NSString stringWithFormat:@"%s.bin", name]];
    [d writeToFile:path atomically:YES];
}

static void dump_methods(NSString *name) {
    Class cls = NSClassFromString(name);
    printf("CLASS %s => %p\n", name.UTF8String, cls);
    if (!cls) return;
    for (int meta=0; meta<2; meta++) {
        Class target = meta ? object_getClass(cls) : cls;
        unsigned n=0; Method *m=class_copyMethodList(target,&n);
        printf("  %c methods=%u\n", meta?'+':'-', n);
        for(unsigned i=0;i<n;i++) printf("    %s %s\n", sel_getName(method_getName(m[i])), method_getTypeEncoding(m[i]));
        free(m);
    }
    unsigned n=0; Ivar *iv=class_copyIvarList(cls,&n);
    printf("  ivars=%u\n",n);
    for(unsigned i=0;i<n;i++) printf("    %s %s offset=%td\n",ivar_getName(iv[i]),ivar_getTypeEncoding(iv[i]),ivar_getOffset(iv[i]));
    free(iv);
}

int main(int argc,const char **argv){ @autoreleasepool {
    NSString *out = argc>1 ? [NSString stringWithUTF8String:argv[1]] : @".";
    [[NSFileManager defaultManager] createDirectoryAtPath:out withIntermediateDirectories:YES attributes:nil error:nil];
    void *h=dlopen("/System/Library/PrivateFrameworks/ANSTKit.framework/ANSTKit",RTLD_NOW|RTLD_GLOBAL);
    printf("DLOPEN=%p err=%s\n",h,h?"none":dlerror());
    if(!h) return 2;
    const char *syms[]={
      "ANSTFsincAlgorithmVersionToNSString",
      "ANSTFsincInferenceVersionToNSString",
      "ANSTFsincAlgorithmResolutionToNSString",
      "ANSTFsincInferenceResolutionToNSString",
      NULL};
    for(int i=0;syms[i];i++) dump_symbol(h,syms[i],out);
    NSArray *classes=@[
      @"ANSTFsincAlgorithm",
      @"ANSTFsincAlgorithmConfiguration",
      @"ANSTFsincAlgorithmV2Dot4",
      @"ANSTFsincInferenceConfiguration",
      @"ANSTFsincInferenceDescriptor",
      @"ANSTFsincInferenceDescriptorV2Dot4",
      @"ANSTFsincInferencePostprocessorV2Dot4",
      @"ANSTE5MLNetwork"
    ];
    for(NSString *n in classes) dump_methods(n);
    return 0;
}}
