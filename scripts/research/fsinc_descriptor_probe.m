#import <Foundation/Foundation.h>
#import <CoreVideo/CoreVideo.h>
#import <objc/runtime.h>
#import <objc/message.h>
#import <dlfcn.h>

static const uint64_t kFSINCV24 = 0x20190;

static const char *ds(id obj) { return obj ? [[obj description] UTF8String] : "nil"; }

static void dump_methods(Class cls) {
    if (!cls) return;
    for (int meta = 0; meta < 2; meta++) {
        Class target = meta ? object_getClass(cls) : cls;
        unsigned n = 0; Method *m = class_copyMethodList(target, &n);
        printf("METHODS %c%s count=%u\n", meta?'+':'-', class_getName(cls), n);
        for (unsigned i=0; i<n; i++)
            printf("  %s types=%s\n", sel_getName(method_getName(m[i])), method_getTypeEncoding(m[i]));
        free(m);
    }
}

static id call_class_obj_q_err(Class cls, const char *selName, uint64_t q, NSError **err) {
    SEL s = sel_registerName(selName);
    if (![cls respondsToSelector:s]) return nil;
    return ((id(*)(id,SEL,uint64_t,NSError**))objc_msgSend)((id)cls,s,q,err);
}

static id call_class_obj_obj_err(Class cls, const char *selName, id arg, NSError **err) {
    SEL s = sel_registerName(selName);
    if (![cls respondsToSelector:s]) return nil;
    return ((id(*)(id,SEL,id,NSError**))objc_msgSend)((id)cls,s,arg,err);
}

static id call_obj_noarg(id obj, const char *name) {
    SEL s=sel_registerName(name);
    if (![obj respondsToSelector:s]) return nil;
    return ((id(*)(id,SEL))objc_msgSend)(obj,s);
}

static BOOL call_bool_err(id obj, const char *name, NSError **err) {
    SEL s=sel_registerName(name);
    if (![obj respondsToSelector:s]) return NO;
    return ((BOOL(*)(id,SEL,NSError**))objc_msgSend)(obj,s,err);
}

int main(void) { @autoreleasepool {
    void *h = dlopen("/System/Library/PrivateFrameworks/ANSTKit.framework/Versions/A/ANSTKit", RTLD_NOW|RTLD_GLOBAL);
    printf("DLOPEN=%p err=%s\n", h, h?"none":dlerror());
    if (!h) return 2;

    Class cfgCls = NSClassFromString(@"ANSTFsincInferenceConfiguration");
    Class descCls = NSClassFromString(@"ANSTFsincInferenceDescriptor");
    Class netCls = NSClassFromString(@"ANSTE5MLNetwork");
    printf("CLASSES cfg=%s desc=%s net=%s\n", cfgCls?class_getName(cfgCls):"nil", descCls?class_getName(descCls):"nil", netCls?class_getName(netCls):"nil");
    dump_methods(cfgCls); dump_methods(descCls); dump_methods(netCls);

    NSError *err=nil;
    id available = nil;
    SEL avSel=sel_registerName("availableInferenceResolutionForVersion:");
    if ([cfgCls respondsToSelector:avSel])
        available=((id(*)(id,SEL,uint64_t))objc_msgSend)((id)cfgCls,avSel,kFSINCV24);
    printf("availableInferenceResolutionForVersion(0x%llx)=%s\n",(unsigned long long)kFSINCV24,ds(available));

    err=nil;
    id cfg = call_class_obj_q_err(cfgCls,"defaultConfigurationForVersion:withError:",kFSINCV24,&err);
    printf("defaultConfiguration=%s class=%s err=%s\n",ds(cfg),cfg?object_getClassName(cfg):"nil",ds(err));
    if (!cfg) return 3;

    const char *cfgProps[]={"version","resolution","computeDevice","qualityOfService",NULL};
    for(int i=0;cfgProps[i];i++) {
        SEL s=sel_registerName(cfgProps[i]);
        if ([cfg respondsToSelector:s]) {
            const char *types=NULL; Method m=class_getInstanceMethod([cfg class],s); if(m)types=method_getTypeEncoding(m);
            if(types && (types[0]=='Q'||types[0]=='q'||types[0]=='I'||types[0]=='i')) {
                unsigned long long v=((unsigned long long(*)(id,SEL))objc_msgSend)(cfg,s);
                printf("cfg.%s=%llu (0x%llx)\n",cfgProps[i],v,v);
            } else {
                id v=((id(*)(id,SEL))objc_msgSend)(cfg,s); printf("cfg.%s=%s\n",cfgProps[i],ds(v));
            }
        }
    }

    const char *descFactories[]={"e5DescriptorWithConfiguration:error:","descriptorWithConfiguration:error:","_descriptorWithConfiguration:error:",NULL};
    for (int f=0; descFactories[f]; f++) {
        err=nil;
        id desc=call_class_obj_obj_err(descCls,descFactories[f],cfg,&err);
        printf("FACTORY %s => %s class=%s err=%s\n",descFactories[f],ds(desc),desc?object_getClassName(desc):"nil",ds(err));
        if(!desc) continue;

        const char *props[]={"name","assetURL","assetType","e5FunctionName","inputDescriptors","outputDescriptors","configuration","requiresPostprocessing",NULL};
        for(int i=0;props[i];i++) {
            SEL s=sel_registerName(props[i]);
            if (![desc respondsToSelector:s]) continue;
            Method m=class_getInstanceMethod([desc class],s); const char *t=m?method_getTypeEncoding(m):NULL;
            if(t && (t[0]=='B'||t[0]=='Q'||t[0]=='q'||t[0]=='I'||t[0]=='i')) {
                unsigned long long v=((unsigned long long(*)(id,SEL))objc_msgSend)(desc,s);
                printf("  desc.%s=%llu\n",props[i],v);
            } else {
                id v=((id(*)(id,SEL))objc_msgSend)(desc,s); printf("  desc.%s=%s\n",props[i],ds(v));
            }
        }

        // Try every plausible E5 compute-device enum through the high-level network wrapper.
        for (uint64_t device=0; device<8; device++) {
            @try {
                err=nil;
                id alloc=((id(*)(id,SEL))objc_msgSend)((id)netCls,sel_registerName("alloc"));
                id net=((id(*)(id,SEL,id,uint64_t,NSError**))objc_msgSend)(alloc,sel_registerName("initWithInferenceDescriptor:computeDevice:error:"),desc,device,&err);
                printf("  NET device=%llu init=%s class=%s err=%s\n",(unsigned long long)device,ds(net),net?object_getClassName(net):"nil",ds(err));
                if(!net) continue;
                err=nil;
                BOOL loaded=call_bool_err(net,"loadNetworkWithError:",&err);
                printf("    loadNetwork=%d err=%s\n",loaded,ds(err));
                if(loaded) {
                    printf("HIGH_LEVEL_LOAD_SUCCESS factory=%s device=%llu\n",descFactories[f],(unsigned long long)device);
                    return 0;
                }
            } @catch(NSException *e) {
                printf("  NET device=%llu EXCEPTION %s: %s\n",(unsigned long long)device,e.name.UTF8String,e.reason.UTF8String);
            }
        }
    }
    printf("NO_HIGH_LEVEL_LOAD_SUCCESS\n");
    return 10;
}}
