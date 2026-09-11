#import <Foundation/Foundation.h>
#import <ImageIO/ImageIO.h>
#import <CoreGraphics/CoreGraphics.h>
#import <objc/runtime.h>
#import <objc/message.h>
#import <dlfcn.h>

static IMP gOriginalTextureParser = NULL;
static NSString *gCurrentFile = @"unknown";
static NSMutableArray *gEvents;

typedef id (*ObjMsg0)(id, SEL);
typedef id (*ObjMsg1)(id, SEL, id);
typedef id (*ObjMsg2)(id, SEL, id, id);
typedef id (*ObjMsg3)(id, SEL, id, id, id);
typedef BOOL (*BoolMsg1)(id, SEL, id);
typedef NSInteger (*IntMsg0)(id, SEL);

static id Send0(id obj, NSString *name) {
    SEL sel = NSSelectorFromString(name);
    if (!obj || ![obj respondsToSelector:sel]) return nil;
    return ((ObjMsg0)objc_msgSend)(obj, sel);
}

static id JSONSafe(id value) {
    if (!value || value == [NSNull null]) return [NSNull null];
    if ([value isKindOfClass:[NSString class]] || [value isKindOfClass:[NSNumber class]]) return value;
    if ([value isKindOfClass:[NSData class]]) {
        return @{ @"class": NSStringFromClass([value class]), @"length": @([(NSData *)value length]), @"base64": [(NSData *)value base64EncodedStringWithOptions:0] };
    }
    if ([value isKindOfClass:[NSDictionary class]]) {
        NSMutableDictionary *out = [NSMutableDictionary dictionary];
        [(NSDictionary *)value enumerateKeysAndObjectsUsingBlock:^(id k, id v, BOOL *stop) {
            out[[k description]] = JSONSafe(v);
        }];
        return out;
    }
    if ([value isKindOfClass:[NSArray class]]) {
        NSMutableArray *out = [NSMutableArray array];
        for (id v in (NSArray *)value) [out addObject:JSONSafe(v)];
        return out;
    }
    return @{ @"class": NSStringFromClass([value class]), @"description": [value description] ?: @"" };
}

static id DescribeMetadataObject(id obj) {
    if (!obj) return [NSNull null];
    NSMutableDictionary *d = [NSMutableDictionary dictionary];
    d[@"class"] = NSStringFromClass([obj class]);
    d[@"description"] = [obj description] ?: @"";
    id ident = Send0(obj, @"identifier");
    if (ident) d[@"identifier"] = JSONSafe(ident);
    id props = Send0(obj, @"cgImageProperties");
    if (props) d[@"cgImageProperties"] = JSONSafe(props);
    SEL typeSel = NSSelectorFromString(@"type");
    if ([obj respondsToSelector:typeSel]) {
        NSInteger t = ((IntMsg0)objc_msgSend)(obj, typeSel);
        d[@"type"] = @(t);
    }
    id cgMetadataObj = Send0(obj, @"cgImageMetadata");
    if (cgMetadataObj) d[@"cgImageMetadataObject"] = JSONSafe(cgMetadataObj);
    return d;
}

static void Record(NSDictionary *event) {
    @synchronized (gEvents) { [gEvents addObject:event]; }
    NSData *data = [NSJSONSerialization dataWithJSONObject:event options:NSJSONWritingSortedKeys error:nil];
    if (data) {
        NSString *s = [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding];
        fprintf(stderr, "TRACE_JSON %s\n", s.UTF8String);
    }
}

static id HookTextureParser(id self, SEL _cmd, id imageMetadata, id auxImageMetadata, NSError **error) {
    NSMutableDictionary *before = [@{
        @"event": @"texture-parser-enter",
        @"file": gCurrentFile ?: @"unknown",
        @"imageMetadata": DescribeMetadataObject(imageMetadata),
        @"auxImageMetadata": DescribeMetadataObject(auxImageMetadata)
    } mutableCopy];
    Record(before);

    NSError *localError = nil;
    NSError **targetError = error ?: &localError;
    typedef id (*ParserFn)(id, SEL, id, id, NSError **);
    id result = ((ParserFn)gOriginalTextureParser)(self, _cmd, imageMetadata, auxImageMetadata, targetError);
    NSError *observed = error ? *error : localError;
    NSMutableDictionary *after = [@{
        @"event": @"texture-parser-exit",
        @"file": gCurrentFile ?: @"unknown",
        @"resultClass": result ? NSStringFromClass([result class]) : [NSNull null],
        @"resultDescription": result ? ([result description] ?: @"") : [NSNull null]
    } mutableCopy];
    if (observed) {
        after[@"error"] = @{
            @"domain": observed.domain ?: @"",
            @"code": @(observed.code),
            @"description": observed.localizedDescription ?: @"",
            @"reason": observed.localizedFailureReason ?: [NSNull null],
            @"userInfo": JSONSafe(observed.userInfo ?: @{})
        };
    } else {
        after[@"error"] = [NSNull null];
    }
    Record(after);
    return result;
}

static NSDictionary *MethodInfo(Class cls, BOOL classMethod, NSString *name) {
    if (!cls) return @{};
    SEL sel = NSSelectorFromString(name);
    Method m = classMethod ? class_getClassMethod(cls, sel) : class_getInstanceMethod(cls, sel);
    if (!m) return @{ @"selector": name, @"available": @NO };
    return @{ @"selector": name, @"available": @YES, @"types": [NSString stringWithUTF8String:method_getTypeEncoding(m) ?: ""] };
}

static NSDictionary *PFMetadataSummary(NSURL *url) {
    Class cls = NSClassFromString(@"PFMetadataImage");
    if (!cls) return @{ @"classAvailable": @NO };
    id obj = nil;
    SEL shortSel = NSSelectorFromString(@"initWithImageURL:contentType:timeZoneLookup:");
    if ([cls instancesRespondToSelector:shortSel]) {
        obj = ((ObjMsg3)objc_msgSend)([cls alloc], shortSel, url, @"public.heic", nil);
    }
    if (!obj) return @{ @"classAvailable": @YES, @"initSucceeded": @NO };
    NSMutableDictionary *d = [@{ @"classAvailable": @YES, @"initSucceeded": @YES } mutableCopy];
    for (NSString *key in @[@"hasSmartStyle", @"hasTextureStyle", @"textureStylePreset", @"textureStyleIntensity", @"textureStyleGrainIntensity", @"textureStyleIsReversible"]) {
        id v = Send0(obj, key);
        if (v) d[key] = JSONSafe(v);
        else if ([key hasPrefix:@"has"]) {
            SEL sel = NSSelectorFromString(key);
            if ([obj respondsToSelector:sel]) d[key] = @(((BOOL (*)(id, SEL))objc_msgSend)(obj, sel));
            else d[key] = [NSNull null];
        } else d[key] = [NSNull null];
    }
    return d;
}

static NSDictionary *NeutrinoSummary(NSURL *url) {
    NSMutableDictionary *out = [NSMutableDictionary dictionary];
    Class assetCls = NSClassFromString(@"_NUImageAsset");
    if (assetCls && [assetCls instancesRespondToSelector:NSSelectorFromString(@"initWithImageURL:")]) {
        @try {
            id asset = ((ObjMsg1)objc_msgSend)([assetCls alloc], NSSelectorFromString(@"initWithImageURL:"), url);
            out[@"assetClass"] = asset ? NSStringFromClass([asset class]) : [NSNull null];
            id props = Send0(asset, @"imageProperties");
            if (props) {
                out[@"imageProperties"] = JSONSafe(@{
                    @"class": NSStringFromClass([props class]),
                    @"description": [props description] ?: @"",
                    @"textureStyleProperties": JSONSafe(Send0(props, @"textureStyleProperties")),
                    @"semanticStyleProperties": JSONSafe(Send0(props, @"semanticStyleProperties"))
                });
            } else out[@"imageProperties"] = [NSNull null];
        } @catch (NSException *e) {
            out[@"assetException"] = @{ @"name": e.name ?: @"", @"reason": e.reason ?: @"" };
        }
    }

    Class nodeCls = NSClassFromString(@"NUCGImageSourceNode");
    out[@"nodeLoadMethod"] = MethodInfo(nodeCls, NO, @"load:");
    out[@"nodeTextureLoadMethod"] = MethodInfo(nodeCls, NO, @"_loadTextureStylesProperties:error:");
    out[@"nodeInitMethod"] = MethodInfo(nodeCls, NO, @"initWithURL:UTI:identifier:");
    if (nodeCls && [nodeCls instancesRespondToSelector:NSSelectorFromString(@"initWithURL:UTI:identifier:")]) {
        @try {
            id node = ((ObjMsg3)objc_msgSend)([nodeCls alloc], NSSelectorFromString(@"initWithURL:UTI:identifier:"), url, @"public.heic", @"xdremux-trace");
            out[@"nodeClass"] = node ? NSStringFromClass([node class]) : [NSNull null];
            Method loadMethod = class_getInstanceMethod(nodeCls, NSSelectorFromString(@"load:"));
            if (node && loadMethod) {
                char ret[32] = {0}; method_getReturnType(loadMethod, ret, sizeof(ret));
                if (ret[0] == 'B' || ret[0] == 'c') {
                    NSError *err = nil;
                    BOOL ok = ((BOOL (*)(id, SEL, NSError **))objc_msgSend)(node, NSSelectorFromString(@"load:"), &err);
                    out[@"nodeLoadOK"] = @(ok);
                    out[@"nodeLoadError"] = err ? JSONSafe(@{ @"domain": err.domain, @"code": @(err.code), @"description": err.localizedDescription ?: @"", @"userInfo": err.userInfo ?: @{} }) : [NSNull null];
                }
            }
            id t = Send0(node, @"textureStylesProperties");
            out[@"nodeTextureStylesProperties"] = JSONSafe(t);
            id s = Send0(node, @"semanticStylesProperties");
            out[@"nodeSemanticStylesProperties"] = JSONSafe(s);
        } @catch (NSException *e) {
            out[@"nodeException"] = @{ @"name": e.name ?: @"", @"reason": e.reason ?: @"" };
        }
    }
    return out;
}

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        if (argc < 3) {
            fprintf(stderr, "usage: trace OUTPUT_JSON FILE...\n");
            return 64;
        }
        gEvents = [NSMutableArray array];
        NSMutableDictionary *root = [NSMutableDictionary dictionary];
        NSMutableArray *files = [NSMutableArray array];
        root[@"schema"] = @"xdremux-texture-style-native-loader-trace-v1";

        void *nu = dlopen("/System/Library/PrivateFrameworks/NeutrinoCore.framework/NeutrinoCore", RTLD_NOW);
        void *pf = dlopen("/System/Library/PrivateFrameworks/PhotosFormats.framework/PhotosFormats", RTLD_NOW);
        root[@"neutrinoLoaded"] = @(nu != NULL);
        root[@"photosFormatsLoaded"] = @(pf != NULL);
        if (!nu) root[@"neutrinoDLError"] = @(dlerror() ?: "");

        Class textureCls = NSClassFromString(@"_NUTextureStyleProperties");
        root[@"textureParserMethod"] = MethodInfo(textureCls, YES, @"textureStylePropertiesFromImageMetadata:auxImageMetadata:error:");
        if (textureCls) {
            Method m = class_getClassMethod(textureCls, NSSelectorFromString(@"textureStylePropertiesFromImageMetadata:auxImageMetadata:error:"));
            if (m) {
                gOriginalTextureParser = method_getImplementation(m);
                method_setImplementation(m, (IMP)HookTextureParser);
                root[@"parserHookInstalled"] = @YES;
            } else root[@"parserHookInstalled"] = @NO;
        }

        for (int i = 2; i < argc; i++) {
            NSString *path = [NSString stringWithUTF8String:argv[i]];
            gCurrentFile = [path lastPathComponent];
            NSURL *url = [NSURL fileURLWithPath:path];
            NSMutableDictionary *entry = [NSMutableDictionary dictionary];
            entry[@"file"] = gCurrentFile;
            entry[@"pfMetadata"] = PFMetadataSummary(url);
            entry[@"neutrino"] = NeutrinoSummary(url);
            [files addObject:entry];
        }
        root[@"files"] = files;
        root[@"parserEvents"] = gEvents;

        NSData *json = [NSJSONSerialization dataWithJSONObject:root options:(NSJSONWritingPrettyPrinted | NSJSONWritingSortedKeys) error:nil];
        if (!json || ![json writeToFile:[NSString stringWithUTF8String:argv[1]] atomically:YES]) return 2;
        fwrite(json.bytes, 1, json.length, stdout); fputc('\n', stdout);
        return 0;
    }
}
