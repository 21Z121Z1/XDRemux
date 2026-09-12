#import <Foundation/Foundation.h>
#import <ImageIO/ImageIO.h>
#import <objc/message.h>
#import <objc/runtime.h>
#import <dlfcn.h>

static id InvokeZeroArg(id obj, NSString *name) {
    SEL sel = NSSelectorFromString(name);
    if (!obj || ![obj respondsToSelector:sel]) return @{ @"available": @NO };
    NSMethodSignature *sig = [obj methodSignatureForSelector:sel];
    if (!sig) return @{ @"available": @YES, @"error": @"no-signature" };
    NSInvocation *inv = [NSInvocation invocationWithMethodSignature:sig];
    inv.target = obj;
    inv.selector = sel;
    @try { [inv invoke]; }
    @catch (NSException *e) {
        return @{ @"available": @YES, @"exception": e.reason ?: e.name };
    }
    const char *t = sig.methodReturnType;
    while (*t=='r'||*t=='n'||*t=='N'||*t=='o'||*t=='O'||*t=='R'||*t=='V') t++;
    if (*t=='B'||*t=='c') {
        BOOL v = NO; [inv getReturnValue:&v];
        return @{ @"available": @YES, @"value": @(v) };
    }
    if (*t=='i'||*t=='s'||*t=='l'||*t=='q') {
        long long v = 0; [inv getReturnValue:&v];
        return @{ @"available": @YES, @"value": @(v) };
    }
    if (*t=='I'||*t=='S'||*t=='L'||*t=='Q') {
        unsigned long long v = 0; [inv getReturnValue:&v];
        return @{ @"available": @YES, @"value": @(v) };
    }
    if (*t=='@') {
        __unsafe_unretained id v = nil; [inv getReturnValue:&v];
        return @{ @"available": @YES, @"value": v ? [v description] : @"<nil>" };
    }
    return @{ @"available": @YES,
              @"encoding": [NSString stringWithUTF8String:sig.methodReturnType] };
}

static id OpenMetadata(NSURL *url) {
    Class c = NSClassFromString(@"PFMetadataImage");
    if (!c) return nil;
    Class UT = NSClassFromString(@"UTType");
    id type = nil;
    SEL ts = NSSelectorFromString(@"typeWithIdentifier:");
    if (UT && [UT respondsToSelector:ts])
        type = ((id(*)(id,SEL,id))objc_msgSend)(UT, ts, @"public.heic");

    id o = [c alloc];
    SEL s = NSSelectorFromString(@"initWithImageURL:contentType:timeZoneLookup:");
    if ([o respondsToSelector:s]) {
        @try {
            id v = ((id(*)(id,SEL,id,id,id))objc_msgSend)(o, s, url, type, nil);
            if (v) return v;
        } @catch (__unused NSException *e) {}
    }

    o = [c alloc];
    s = NSSelectorFromString(@"initWithImageURL:contentType:options:timeZoneLookup:cacheImageSource:cacheImageData:");
    if ([o respondsToSelector:s]) {
        @try {
            return ((id(*)(id,SEL,id,id,unsigned long long,id,BOOL,BOOL))objc_msgSend)(
                o, s, url, type, 0, nil, YES, YES);
        } @catch (__unused NSException *e) {}
    }
    return nil;
}

int main(void) { @autoreleasepool {
    void *pf = dlopen("/System/Library/PrivateFrameworks/PhotosFormats.framework/PhotosFormats",
                      RTLD_NOW | RTLD_GLOBAL);
    void *pi = dlopen("/System/Library/PrivateFrameworks/PhotoImaging.framework/PhotoImaging",
                      RTLD_NOW | RTLD_GLOBAL);
    Class autoCalc = NSClassFromString(@"PISemanticStyleAutoCalculator");
    SEL canRender = NSSelectorFromString(@"canRenderStylesWithImageProperties:");

    NSMutableArray *rows = [NSMutableArray array];
    NSArray<NSString *> *files = [[[NSFileManager defaultManager]
        contentsOfDirectoryAtPath:[[NSBundle mainBundle] resourcePath] error:nil]
        filteredArrayUsingPredicate:[NSPredicate predicateWithBlock:^BOOL(NSString *s, NSDictionary *_) {
            return [s.pathExtension.lowercaseString isEqualToString:@"heic"];
        }]];
    files = [files sortedArrayUsingSelector:@selector(compare:)];

    for (NSString *file in files) {
        NSString *path = [[[NSBundle mainBundle] resourcePath] stringByAppendingPathComponent:file];
        NSURL *url = [NSURL fileURLWithPath:path];
        id m = OpenMetadata(url);
        NSMutableDictionary *d = [@{
            @"file": file,
            @"photosFormatsLoaded": pf ? @YES : @NO,
            @"photoImagingLoaded": pi ? @YES : @NO,
            @"metadataOpened": m ? @YES : @NO,
            @"autoCalculatorClass": autoCalc ? @YES : @NO
        } mutableCopy];
        if (m) {
            d[@"hasSmartStyle"] = InvokeZeroArg(m, @"hasSmartStyle");
            d[@"hasTextureStyle"] = InvokeZeroArg(m, @"hasTextureStyle");
        }

        CGImageSourceRef src = CGImageSourceCreateWithURL((__bridge CFURLRef)url, NULL);
        if (src) {
            NSDictionary *props = CFBridgingRelease(CGImageSourceCopyPropertiesAtIndex(src, 0, NULL));
            CFRelease(src);
            d[@"imagePropertiesAvailable"] = @(props != nil);
            if (autoCalc && [autoCalc respondsToSelector:canRender] && props) {
                @try {
                    BOOL ok = ((BOOL(*)(id,SEL,id))objc_msgSend)(autoCalc, canRender, props);
                    d[@"semanticStyleCanRender"] = @(ok);
                } @catch (NSException *e) {
                    d[@"semanticStyleCanRenderException"] = e.reason ?: e.name;
                }
            }
        }
        [rows addObject:d];
    }

    NSData *json = [NSJSONSerialization dataWithJSONObject:rows
                                                    options:NSJSONWritingPrettyPrinted|NSJSONWritingSortedKeys
                                                      error:nil];
    NSString *out = [NSTemporaryDirectory() stringByAppendingPathComponent:@"ps3-v8-results.json"];
    BOOL ok = [json writeToFile:out atomically:YES];
    return ok ? 0 : 2;
} }
