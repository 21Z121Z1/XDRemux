#import <Foundation/Foundation.h>
#import <ImageIO/ImageIO.h>
#import <dlfcn.h>

static id Safe(id obj) {
    if (!obj) return [NSNull null];
    if ([obj isKindOfClass:[NSString class]] || [obj isKindOfClass:[NSNumber class]] || [obj isKindOfClass:[NSNull class]]) return obj;
    if ([obj isKindOfClass:[NSData class]]) return @{ @"class":NSStringFromClass([obj class]), @"bytes":@([(NSData *)obj length]) };
    if ([obj isKindOfClass:[NSArray class]]) { NSMutableArray *a=[NSMutableArray array]; for(id v in obj)[a addObject:Safe(v)]; return a; }
    if ([obj isKindOfClass:[NSDictionary class]]) { NSMutableDictionary *d=[NSMutableDictionary dictionary]; for(id k in obj)d[[k description]]=Safe([obj objectForKey:k]); return d; }
    return @{ @"class":NSStringFromClass([obj class])?:@"?", @"description":[obj description]?:@"" };
}

static id ValueForDescribedKey(NSDictionary *d, NSString *wanted) {
    for (id key in d) if ([[key description] isEqualToString:wanted]) return d[key];
    return nil;
}

static void LoadPrivateFrameworks(void) {
    for (NSString *p in @[
        @"/System/Library/PrivateFrameworks/PhotoImaging.framework/PhotoImaging",
        @"/System/Library/PrivateFrameworks/PhotosEditing.framework/PhotosEditing",
        @"/System/Library/PrivateFrameworks/PhotosUIEdit.framework/PhotosUIEdit",
        @"/System/Library/PrivateFrameworks/PhotosFormats.framework/PhotosFormats",
        @"/System/Library/PrivateFrameworks/CameraEditKit.framework/CameraEditKit",
        @"/System/Library/PrivateFrameworks/AVFCapture.framework/AVFCapture",
        @"/System/Library/PrivateFrameworks/CMCaptureCore.framework/CMCaptureCore",
        @"/System/Library/PrivateFrameworks/CMImaging.framework/CMImaging",
        @"/System/Library/PrivateFrameworks/NeutrinoCore.framework/NeutrinoCore"
    ]) dlopen(p.UTF8String, RTLD_NOW|RTLD_GLOBAL);
}

static void *Sym(NSString *name) {
    void *p=dlsym(RTLD_DEFAULT,name.UTF8String); if(p)return p;
    if([name hasPrefix:@"_"]) return dlsym(RTLD_DEFAULT,[[name substringFromIndex:1] UTF8String]);
    NSString *u=[@"_" stringByAppendingString:name]; return dlsym(RTLD_DEFAULT,u.UTF8String);
}

static id ObjectSymbol(NSString *name) {
    void *p=Sym(name); if(!p)return nil;
    @try { __unsafe_unretained id value=*(__unsafe_unretained id *)p; return value; }
    @catch(__unused NSException *e){ return nil; }
}

typedef id (*SettingsFn)(NSDictionary *);
static id RunSettings(SettingsFn fn, NSDictionary *d) {
    if(!fn || !d)return [NSNull null];
    @try { id value=fn(d); return value?Safe(value):[NSNull null]; }
    @catch(NSException *e){ return @{ @"exception":e.reason?:e.name }; }
}

int main(int argc,const char *argv[]) {
    @autoreleasepool {
        if(argc!=3){fprintf(stderr,"usage: probe IMAGE OUTPUT_JSON\n");return 64;}
        LoadPrivateFrameworks();
        NSURL *url=[NSURL fileURLWithPath:[NSString stringWithUTF8String:argv[1]]];
        CGImageSourceRef source=CGImageSourceCreateWithURL((__bridge CFURLRef)url,NULL);
        if(!source){fprintf(stderr,"cannot open image\n");return 2;}
        CFDictionaryRef raw=CGImageSourceCopyPropertiesAtIndex(source,0,NULL); CFRelease(source);
        NSDictionary *props=CFBridgingRelease(raw);
        NSDictionary *maker=props[(__bridge NSString *)kCGImagePropertyMakerAppleDictionary];
        if(![maker isKindOfClass:[NSDictionary class]]) maker=@{};
        id maker84=ValueForDescribedKey(maker,@"84");
        NSDictionary *base84=[maker84 isKindOfClass:[NSDictionary class]] ? maker84 : @{};

        SettingsFn texture=(SettingsFn)Sym(@"PITextureStyleSettingsFromMakerNoteProperties");
        SettingsFn semantic=(SettingsFn)Sym(@"PISemanticStyleSettingsFromMakerNoteProperties");

        NSArray *keyNames=@[@"AVAppleMakerNote_TextureStyleKey_Preset",@"AVAppleMakerNote_TextureStyleKey_Intensity",@"AVAppleMakerNote_TextureStyleKey_Grain",@"AVAppleMakerNote_TextureStyleKey_RenderingVersion",@"AVAppleMakerNote_TextureStyleKey_OriginalInsteadOfReversibility",@"kFigAppleMakerNote_TextureStyleKey_Preset",@"kFigAppleMakerNote_TextureStyleKey_Intensity",@"kFigAppleMakerNote_TextureStyleKey_Grain",@"kFigAppleMakerNote_TextureStyleKey_RenderingVersion",@"kFigAppleMakerNote_TextureStyleKey_OriginalInsteadOfReversibility",@"PITextureStyleAdjustmentKey",@"PITextureStylePresetStandard",@"PITextureStylePresetSoft",@"PITextureStylePresetFilmic",@"PITextureStylePresetGlowy",@"CMITextureStylePresetNameStandard",@"CMITextureStylePresetNameStudio",@"CMITextureStylePresetNameSoft",@"CMITextureStylePresetNameFilmic",@"CMITextureStylePresetNameGlowy"];
        NSMutableDictionary *symbols=[NSMutableDictionary dictionary];
        for(NSString *name in keyNames){id v=ObjectSymbol(name);symbols[name]=v?Safe(v):[NSNull null];}

        NSString *presetKey=ObjectSymbol(@"AVAppleMakerNote_TextureStyleKey_Preset") ?: @"Preset";
        NSString *intensityKey=ObjectSymbol(@"AVAppleMakerNote_TextureStyleKey_Intensity") ?: @"Intensity";
        NSString *grainKey=ObjectSymbol(@"AVAppleMakerNote_TextureStyleKey_Grain") ?: @"Grain";
        NSString *renderKey=ObjectSymbol(@"AVAppleMakerNote_TextureStyleKey_RenderingVersion") ?: @"RenderingVersion";
        NSString *reversibleKey=ObjectSymbol(@"AVAppleMakerNote_TextureStyleKey_OriginalInsteadOfReversibility") ?: @"OriginalInsteadOfReversibility";
        id standard=ObjectSymbol(@"PITextureStylePresetStandard") ?: ObjectSymbol(@"CMITextureStylePresetNameStandard") ?: @"Standard";

        NSMutableArray *variants=[NSMutableArray array];
        void (^AddVariant)(NSString *,NSDictionary *) = ^(NSString *name,NSDictionary *dict){ [variants addObject:@{ @"name":name, @"input":Safe(dict), @"textureResult":RunSettings(texture,dict), @"semanticControl":RunSettings(semantic,dict) }]; };
        AddVariant(@"maker84-original",base84);
        AddVariant(@"makerApple-entire",maker);

        for(NSNumber *ver in @[@1,@2,@3]) {
            NSMutableDictionary *d=[base84 mutableCopy];
            d[presetKey]=standard; d[intensityKey]=@1.0; d[grainKey]=@0.0; d[renderKey]=ver; d[reversibleKey]=@NO;
            AddVariant([NSString stringWithFormat:@"maker84-runtime-keys-v%@",ver],d);
        }
        for(NSNumber *ver in @[@1,@2,@3]) {
            NSMutableDictionary *d=[base84 mutableCopy];
            [d addEntriesFromDictionary:@{ @"Preset":@"Standard",@"Intensity":@1.0,@"Grain":@0.0,@"RenderingVersion":ver,@"OriginalInsteadOfReversibility":@NO }];
            AddVariant([NSString stringWithFormat:@"maker84-literal-short-v%@",ver],d);
        }
        for(NSNumber *ver in @[@1,@2,@3]) {
            NSMutableDictionary *d=[base84 mutableCopy];
            [d addEntriesFromDictionary:@{ @"TextureStylePreset":@"Standard",@"TextureStyleIntensity":@1.0,@"TextureStyleGrain":@0.0,@"TextureStyleRenderingVersion":ver,@"TextureStyleOriginalInsteadOfReversibility":@NO }];
            AddVariant([NSString stringWithFormat:@"maker84-literal-prefixed-v%@",ver],d);
        }
        NSMutableDictionary *makerExtended=[maker mutableCopy];
        makerExtended[presetKey]=standard;makerExtended[intensityKey]=@1.0;makerExtended[grainKey]=@0.0;makerExtended[renderKey]=@1;makerExtended[reversibleKey]=@NO;
        AddVariant(@"makerApple-runtime-keys",makerExtended);

        NSDictionary *out=@{
            @"os":NSProcessInfo.processInfo.operatingSystemVersionString,
            @"textureParserFound":@(texture!=NULL),
            @"semanticParserFound":@(semantic!=NULL),
            @"makerApple":Safe(maker),
            @"maker84":Safe(maker84),
            @"symbols":symbols,
            @"variants":variants
        };
        NSData *json=[NSJSONSerialization dataWithJSONObject:out options:NSJSONWritingPrettyPrinted|NSJSONWritingSortedKeys error:nil];
        [json writeToFile:[NSString stringWithUTF8String:argv[2]] atomically:YES];
        fwrite(json.bytes,1,json.length,stdout);fputc('\n',stdout);
        return 0;
    }
}
