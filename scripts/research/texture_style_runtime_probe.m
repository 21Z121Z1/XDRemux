#import <Foundation/Foundation.h>
#import <objc/message.h>
#import <objc/runtime.h>
#import <dlfcn.h>

static id Send0(id target, NSString *selector) {
    SEL sel = NSSelectorFromString(selector);
    if (!target || ![target respondsToSelector:sel]) return nil;
    return ((id (*)(id, SEL))objc_msgSend)(target, sel);
}

static id Send1(id target, NSString *selector, id arg) {
    SEL sel = NSSelectorFromString(selector);
    if (!target || ![target respondsToSelector:sel]) return nil;
    return ((id (*)(id, SEL, id))objc_msgSend)(target, sel, arg);
}

static NSString *ResolvedCFString(const char *symbol) {
    void *address = dlsym(RTLD_DEFAULT, symbol);
    if (!address) return nil;
    CFTypeRef value = *(CFTypeRef *)address;
    if (!value || CFGetTypeID(value) != CFStringGetTypeID()) return nil;
    return (__bridge NSString *)value;
}

static id JSONSafe(id value) {
    if (!value || value == [NSNull null]) return [NSNull null];
    if ([value isKindOfClass:[NSString class]] ||
        [value isKindOfClass:[NSNumber class]]) return value;
    if ([value isKindOfClass:[NSData class]]) {
        return @{ @"type": @"data", @"length": @([(NSData *)value length]) };
    }
    if ([value isKindOfClass:[NSArray class]]) {
        NSMutableArray *out = [NSMutableArray array];
        for (id item in (NSArray *)value) [out addObject:JSONSafe(item)];
        return out;
    }
    if ([value isKindOfClass:[NSDictionary class]]) {
        NSMutableDictionary *out = [NSMutableDictionary dictionary];
        for (id key in (NSDictionary *)value) {
            out[[key description]] = JSONSafe(((NSDictionary *)value)[key]);
        }
        return out;
    }
    return @{
        @"class": NSStringFromClass([value class]),
        @"description": [value description] ?: @""
    };
}

static NSDictionary *TextureStyleRecord(id style) {
    if (!style) return @{};
    NSMutableDictionary *out = [NSMutableDictionary dictionary];
    out[@"description"] = [style description] ?: @"";
    for (NSString *key in @[ @"preset", @"intensity", @"grain" ]) {
        @try {
            id value = [style valueForKey:key];
            if (value) out[key] = JSONSafe(value);
        } @catch (__unused NSException *exception) {}
    }
    id dict = Send1(style, @"dictionaryRepresentationUsingReferenceStyle:", nil);
    if (dict) out[@"dictionaryRepresentation"] = JSONSafe(dict);
    return out;
}

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        if (argc != 2) return 64;

        NSArray<NSString *> *frameworks = @[
            @"/System/Library/PrivateFrameworks/PhotoImaging.framework/PhotoImaging",
            @"/System/Library/PrivateFrameworks/CameraEditKit.framework/CameraEditKit",
            @"/System/Library/PrivateFrameworks/CMImaging.framework/CMImaging",
            @"/System/Library/PrivateFrameworks/AVFCapture.framework/AVFCapture",
            @"/System/Library/PrivateFrameworks/CMCaptureCore.framework/CMCaptureCore"
        ];
        NSMutableDictionary *loaded = [NSMutableDictionary dictionary];
        for (NSString *path in frameworks) {
            void *handle = dlopen(path.fileSystemRepresentation, RTLD_LAZY | RTLD_GLOBAL);
            loaded[path] = @(handle != NULL);
        }

        NSMutableDictionary *symbols = [NSMutableDictionary dictionary];
        for (NSString *name in @[
            @"PITextureStyleAdjustmentKey",
            @"AVAppleMakerNote_TextureStyleKey_Grain",
            @"AVAppleMakerNote_TextureStyleKey_Intensity",
            @"AVAppleMakerNote_TextureStyleKey_OriginalInsteadOfReversibility",
            @"AVAppleMakerNote_TextureStyleKey_Preset",
            @"AVAppleMakerNote_TextureStyleKey_RenderingVersion",
            @"kFigAppleMakerNote_TextureStyleKey_Grain",
            @"kFigAppleMakerNote_TextureStyleKey_Intensity",
            @"kFigAppleMakerNote_TextureStyleKey_OriginalInsteadOfReversibility",
            @"kFigAppleMakerNote_TextureStyleKey_Preset",
            @"kFigAppleMakerNote_TextureStyleKey_RenderingVersion"
        ]) {
            NSString *value = ResolvedCFString(name.UTF8String);
            if (value) symbols[name] = value;
        }

        NSMutableArray *defaultStyles = [NSMutableArray array];
        Class cek = NSClassFromString(@"CEKTextureStyle");
        id styles = Send0(cek, @"defaultStyles");
        if ([styles isKindOfClass:[NSArray class]]) {
            for (id style in styles) [defaultStyles addObject:TextureStyleRecord(style)];
        }

        Class controller = NSClassFromString(@"PITextureStyleAdjustmentController");
        Class piTexture = NSClassFromString(@"PITextureStyle");
        Class schema = NSClassFromString(@"PISchema");
        Class processor = NSClassFromString(@"CMITextureStylesProcessor");

        NSMutableDictionary *objects = [NSMutableDictionary dictionary];
        id allPresets = Send0(controller, @"allPresets");
        if (allPresets) {
            objects[@"PITextureStyleAdjustmentController.allPresets"] = JSONSafe(allPresets);
        }
        for (NSString *selector in @[ @"identifier", @"adjustmentFormat", @"availableOptions" ]) {
            id value = Send0(piTexture, selector);
            if (value) {
                objects[[NSString stringWithFormat:@"PITextureStyle.%@", selector]] = JSONSafe(value);
            }
        }
        id textureSchema = Send0(schema, @"textureStyleSchema");
        if (textureSchema) objects[@"PISchema.textureStyleSchema"] = JSONSafe(textureSchema);

        NSDictionary *result = @{
            @"frameworks": loaded,
            @"symbols": symbols,
            @"classes": @{
                @"CEKTextureStyle": @(cek != Nil),
                @"PITextureStyleAdjustmentController": @(controller != Nil),
                @"PITextureStyle": @(piTexture != Nil),
                @"PISchema": @(schema != Nil),
                @"CMITextureStylesProcessor": @(processor != Nil)
            },
            @"defaultStyles": defaultStyles,
            @"objects": objects
        };
        NSError *error = nil;
        NSData *json = [NSJSONSerialization dataWithJSONObject:result
                                                       options:NSJSONWritingPrettyPrinted
                                                         error:&error];
        if (!json || error) {
            fprintf(stderr, "JSON serialization failed: %s\n", error.localizedDescription.UTF8String);
            return 1;
        }
        if (![json writeToFile:[NSString stringWithUTF8String:argv[1]] atomically:YES]) return 1;
    }
    return 0;
}
