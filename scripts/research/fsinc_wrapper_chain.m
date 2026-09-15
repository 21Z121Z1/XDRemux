#import <Foundation/Foundation.h>
#import <objc/runtime.h>
#import <objc/message.h>
#import <dlfcn.h>

static const uint64_t kFSINCV24 = 0x20190;

static const char *sdesc(id x) { return x ? [[x description] UTF8String] : "nil"; }

static BOOL interestingName(const char *n) {
    if (!n) return NO;
    NSString *s=[NSString stringWithUTF8String:n];
    NSArray *need=@[@"ANST",@"E5",@"Fsinc",@"FSINC",@"V5Dot4",@"V5D4"];
    for (NSString *q in need) if ([s rangeOfString:q options:NSCaseInsensitiveSearch].location!=NSNotFound) return YES;
    return NO;
}

static void dumpMethods(Class c, BOOL meta) {
    Class t=meta?object_getClass(c):c;
    unsigned n=0; Method *ms=class_copyMethodList(t,&n);
    printf("METHODS %s %s count=%u\n",meta?"+":"-",class_getName(c),n);
    for(unsigned i=0;i<n;i++) {
        SEL sel=method_getName(ms[i]);
        printf("  %s %s types=%s imp=%p\n",meta?"+":"-",sel_getName(sel),method_getTypeEncoding(ms[i]),method_getImplementation(ms[i]));
    }
    free(ms);
}

static void dumpIvarsForClass(Class c) {
    unsigned n=0; Ivar *iv=class_copyIvarList(c,&n);
    printf("IVARS %s count=%u\n",class_getName(c),n);
    for(unsigned i=0;i<n;i++) printf("  %s type=%s off=%td\n",ivar_getName(iv[i]),ivar_getTypeEncoding(iv[i]),ivar_getOffset(iv[i]));
    free(iv);
}

static void dumpRuntimeSurface(void) {
    int n=objc_getClassList(NULL,0);
    Class *classes=calloc((size_t)n,sizeof(Class)); objc_getClassList(classes,n);
    for(int i=0;i<n;i++) {
        Class c=classes[i]; const char *name=class_getName(c); const char *image=class_getImageName(c);
        if(!interestingName(name)) continue;
        if(image && strstr(image,"ANSTKit")==NULL && strstr(name,"E5")==NULL) continue;
        printf("\nCLASS %s image=%s super=%s\n",name,image?image:"?",class_getSuperclass(c)?class_getName(class_getSuperclass(c)):"nil");
        dumpIvarsForClass(c); dumpMethods(c,NO); dumpMethods(c,YES);
    }
    free(classes);
}

static void dumpObject(id obj, const char *tag) {
    if(!obj){printf("OBJECT %s nil\n",tag);return;}
    printf("\nOBJECT %s ptr=%p class=%s desc=%s\n",tag,obj,object_getClassName(obj),sdesc(obj));
    for(Class c=object_getClass(obj); c; c=class_getSuperclass(c)) {
        unsigned n=0; Ivar *iv=class_copyIvarList(c,&n);
        for(unsigned i=0;i<n;i++) {
            const char *name=ivar_getName(iv[i]), *ty=ivar_getTypeEncoding(iv[i]); ptrdiff_t off=ivar_getOffset(iv[i]);
            const uint8_t *base=(const uint8_t *)(__bridge const void *)obj + off;
            if(ty && ty[0]=='@') {
                id v=object_getIvar(obj,iv[i]);
                printf("  IVAR %s::%s @ ptr=%p class=%s desc=%s\n",class_getName(c),name,v,v?object_getClassName(v):"nil",sdesc(v));
            } else if(ty && (ty[0]=='Q'||ty[0]=='q')) {
                unsigned long long v=0; memcpy(&v,base,MIN(sizeof(v),(size_t)8));
                printf("  IVAR %s::%s %s 0x%llx (%llu)\n",class_getName(c),name,ty,v,v);
            } else if(ty && (ty[0]=='I'||ty[0]=='i'||ty[0]=='B'||ty[0]=='c'||ty[0]=='C')) {
                unsigned v=0; memcpy(&v,base,MIN(sizeof(v),(size_t)4));
                printf("  IVAR %s::%s %s 0x%x (%u)\n",class_getName(c),name,ty,v,v);
            } else {
                uintptr_t v=0; memcpy(&v,base,MIN(sizeof(v),(size_t)sizeof(v)));
                printf("  IVAR %s::%s type=%s raw=0x%llx\n",class_getName(c),name,ty?ty:"?",(unsigned long long)v);
            }
        }
        free(iv);
    }
}

static Class concreteClass(Class base,uint64_t version) {
    @try { return ((Class(*)(id,SEL,uint64_t))objc_msgSend)((id)base,sel_registerName("_concreteClassOfVersion:"),version); }
    @catch(NSException *e){ printf("CONCRETE_EXCEPTION %s %s\n",e.name.UTF8String,e.reason.UTF8String); return Nil; }
}

int main(void) { @autoreleasepool {
    void *h=dlopen("/System/Library/PrivateFrameworks/ANSTKit.framework/Versions/A/ANSTKit",RTLD_NOW|RTLD_GLOBAL);
    if(!h){fprintf(stderr,"DLOPEN %s\n",dlerror());return 2;}
    printf("ANSTKIT_LOADED\n");
    dumpRuntimeSurface();

    Class base=NSClassFromString(@"ANSTFsincAlgorithm");
    Class cfgCls=NSClassFromString(@"ANSTFsincAlgorithmConfiguration");
    if(!base||!cfgCls){fprintf(stderr,"FSINC_CLASSES_MISSING\n");return 3;}
    Class cls=concreteClass(base,kFSINCV24);
    printf("FSINC_CONCRETE=%s\n",cls?class_getName(cls):"nil");
    if(!cls)return 4;

    // Use the smallest square entry point so this probe isolates preparation/compiler setup.
    NSUInteger resolution=2;
    id cfgAlloc=((id(*)(id,SEL))objc_msgSend)((id)cfgCls,sel_registerName("alloc"));
    id cfg=((id(*)(id,SEL,uint64_t,NSUInteger))objc_msgSend)(cfgAlloc,sel_registerName("initWithVersion:resolution:"),kFSINCV24,resolution);
    id algAlloc=((id(*)(id,SEL))objc_msgSend)((id)cls,sel_registerName("alloc"));
    id alg=((id(*)(id,SEL,id))objc_msgSend)(algAlloc,sel_registerName("initWithConfiguration:"),cfg);
    printf("CONFIG=%s\n",sdesc(cfg));
    dumpObject(cfg,"configuration-before-prepare");
    dumpObject(alg,"algorithm-before-prepare");

    NSError *err=nil; BOOL ok=NO;
    @try { ok=((BOOL(*)(id,SEL,NSError**))objc_msgSend)(alg,sel_registerName("prepareWithError:"),&err); }
    @catch(NSException *e) { printf("PREPARE_EXCEPTION %s %s\n",e.name.UTF8String,e.reason.UTF8String); }
    printf("PREPARE_RESULT ok=%d error=%s\n",ok,sdesc(err));
    dumpObject(cfg,"configuration-after-prepare");
    dumpObject(alg,"algorithm-after-prepare");

    // Print any object ivars one level deeper after prepare, especially E5/network/execution-stream objects.
    for(Class c=object_getClass(alg); c; c=class_getSuperclass(c)) {
        unsigned n=0; Ivar *iv=class_copyIvarList(c,&n);
        for(unsigned i=0;i<n;i++) if(ivar_getTypeEncoding(iv[i]) && ivar_getTypeEncoding(iv[i])[0]=='@') {
            id v=object_getIvar(alg,iv[i]);
            if(v && interestingName(object_getClassName(v))) dumpObject(v,ivar_getName(iv[i]));
        }
        free(iv);
    }
    return ok?0:10;
}}
