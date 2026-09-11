#import <Foundation/Foundation.h>
#import <CoreFoundation/CoreFoundation.h>
#import <dlfcn.h>

typedef int32_t OSStatus;
typedef OSStatus (*CMPhotoCompressionSessionCreateFn)(CFAllocatorRef, CFDictionaryRef, CFTypeRef *);
typedef OSStatus (*CMPhotoCompressionSessionOpenExistingContainerForModificationFFn)(CFTypeRef, CFDictionaryRef, CFTypeRef);
typedef OSStatus (*CMPhotoCompressionSessionAddCustomMetadataFn)(CFTypeRef, int64_t, int64_t, CFDictionaryRef);
typedef OSStatus (*CMPhotoCompressionSessionCloseContainerAndCopyBackingFn)(CFTypeRef, int64_t, int64_t, CFTypeRef *);
typedef void (*CMPhotoCompressionSessionInvalidateFn)(CFTypeRef);

static void Stage(const char *name) {
    fprintf(stderr, "CMPhotoTextureStage:%s\n", name);
    fflush(stderr);
}

static id LoadConstant(void *handle, const char *name) {
    fprintf(stderr, "CMPhotoTextureConstant:resolve:%s\n", name); fflush(stderr);
    void *sym = dlsym(handle, name);
    if (!sym) {
        fprintf(stderr, "CMPhotoTextureConstant:missing:%s\n", name); fflush(stderr);
        return nil;
    }
    fprintf(stderr, "CMPhotoTextureConstant:address:%s:%p\n", name, sym); fflush(stderr);
    CFTypeRef value = *(CFTypeRef *)sym;
    fprintf(stderr, "CMPhotoTextureConstant:value:%s:%p\n", name, value); fflush(stderr);
    return (__bridge id)value;
}

static void WriteJSON(NSString *path, NSDictionary *obj) {
    NSData *d = [NSJSONSerialization dataWithJSONObject:obj options:(NSJSONWritingPrettyPrinted|NSJSONWritingSortedKeys) error:nil];
    [d writeToFile:path atomically:YES];
}

static void Checkpoint(NSString *path, NSMutableDictionary *report, NSString *stage) {
    report[@"lastCompletedStage"] = stage;
    WriteJSON(path, report);
    fprintf(stderr, "CMPhotoTextureCheckpoint:%s\n", stage.UTF8String); fflush(stderr);
}

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        Stage("enter-main");
        if (argc != 4) {
            fprintf(stderr, "usage: cmphoto_texture_style_writer INPUT OUTPUT REPORT_JSON\n");
            return 64;
        }
        NSString *inputPath = [NSString stringWithUTF8String:argv[1]];
        NSString *outputPath = [NSString stringWithUTF8String:argv[2]];
        NSString *reportPath = [NSString stringWithUTF8String:argv[3]];
        NSMutableDictionary *report = [NSMutableDictionary dictionary];
        report[@"schema"] = @"xdremux-cmphoto-texture-style-writer-v3";
        report[@"input"] = [inputPath lastPathComponent];
        report[@"output"] = [outputPath lastPathComponent];
        Checkpoint(reportPath, report, @"arguments");

        Stage("dlopen");
        void *cm = dlopen("/System/Library/PrivateFrameworks/CMPhoto.framework/CMPhoto", RTLD_NOW|RTLD_LOCAL);
        if (!cm) {
            report[@"dlerror"] = @(dlerror() ?: "unknown");
            Checkpoint(reportPath, report, @"dlopen-failed");
            return 2;
        }
        Checkpoint(reportPath, report, @"dlopen");

        Stage("resolve-functions");
        CMPhotoCompressionSessionCreateFn create = (CMPhotoCompressionSessionCreateFn)dlsym(cm, "CMPhotoCompressionSessionCreate");
        CMPhotoCompressionSessionOpenExistingContainerForModificationFFn openF = (CMPhotoCompressionSessionOpenExistingContainerForModificationFFn)dlsym(cm, "CMPhotoCompressionSessionOpenExistingContainerForModificationF");
        CMPhotoCompressionSessionAddCustomMetadataFn add = (CMPhotoCompressionSessionAddCustomMetadataFn)dlsym(cm, "CMPhotoCompressionSessionAddCustomMetadata");
        CMPhotoCompressionSessionCloseContainerAndCopyBackingFn closeCopy = (CMPhotoCompressionSessionCloseContainerAndCopyBackingFn)dlsym(cm, "CMPhotoCompressionSessionCloseContainerAndCopyBacking");
        CMPhotoCompressionSessionInvalidateFn invalidate = (CMPhotoCompressionSessionInvalidateFn)dlsym(cm, "CMPhotoCompressionSessionInvalidate");
        report[@"symbols"] = @{
            @"create": @(create != NULL), @"openF": @(openF != NULL), @"add": @(add != NULL),
            @"closeCopy": @(closeCopy != NULL), @"invalidate": @(invalidate != NULL)
        };
        Checkpoint(reportPath, report, @"functions");
        if (!create || !openF || !add || !closeCopy) return 3;

        Stage("resolve-constants");
        id keyData = LoadConstant(cm, "kCMPhotoCustomMetadata_Data");
        id keyURI = LoadConstant(cm, "kCMPhotoCustomMetadata_URI");
        id keyName = LoadConstant(cm, "kCMPhotoCustomMetadata_Name");
        id exportedTextureURN = LoadConstant(cm, "kCMPhotoCustomMetadataTypeURN_TextureStyles");
        id textureURN = exportedTextureURN ?: @"tag:apple.com,2026:photo:metadata:texture_styles";
        report[@"constants"] = @{
            @"Data": keyData ?: [NSNull null], @"URI": keyURI ?: [NSNull null],
            @"Name": keyName ?: [NSNull null], @"TextureURN": textureURN,
            @"TextureURNFromRuntimeSymbol": @(exportedTextureURN != nil)
        };
        Checkpoint(reportPath, report, @"constants");
        if (!keyData || !keyURI || !keyName) return 4;

        Stage("read-source");
        NSError *err = nil;
        NSData *source = [NSData dataWithContentsOfFile:inputPath options:0 error:&err];
        if (!source) {
            report[@"sourceError"] = err.description ?: @"unknown";
            Checkpoint(reportPath, report, @"source-failed");
            return 5;
        }
        report[@"sourceLength"] = @(source.length);
        Checkpoint(reportPath, report, @"source");

        // PITextureStyleCurrentMetadataVersion() in the iOS 27 RC 24A435
        // PhotoImaging binary returns NSNumber(3) whenever TextureStyle rendering
        // is supported. Use that device-firmware value instead of the earlier v1 guess.
        NSDictionary *texture = @{
            @"Version": @3,
            @"HardwareModel": @"V63AP",
            @"PortType": @"PortTypeBack",
            @"CaptureMode": @"Photo",
            @"CaptureType": @"Photo",
            @"FilmGrainSeed": @1480872525,
            @"TextureStylePeopleDataVersion": @1,
            @"TextureStylePostProcessedPeopleData": @[]
        };
        report[@"textureStyleInfo"] = texture;
        Stage("serialize-plist");
        NSData *plist = [NSPropertyListSerialization dataWithPropertyList:texture
                                                                   format:NSPropertyListBinaryFormat_v1_0
                                                                  options:0
                                                                    error:&err];
        if (!plist) {
            report[@"plistError"] = err.description ?: @"unknown";
            Checkpoint(reportPath, report, @"plist-failed");
            return 6;
        }
        report[@"plistLength"] = @(plist.length);
        Checkpoint(reportPath, report, @"plist");

        Stage("build-custom-dictionary");
        NSDictionary *custom = @{ keyData: plist, keyURI: textureURN, keyName: @"textureStyleMetadata" };
        report[@"customMetadataEntryCount"] = @(custom.count);
        Checkpoint(reportPath, report, @"custom-dictionary");

        CFTypeRef session = NULL;
        Stage("create-before");
        OSStatus sCreate = create(kCFAllocatorDefault, NULL, &session);
        fprintf(stderr, "CMPhotoTextureCreate:status=%d session=%p\n", (int)sCreate, session); fflush(stderr);
        report[@"createStatus"] = @(sCreate);
        report[@"sessionCreated"] = @(session != NULL);
        Checkpoint(reportPath, report, @"create");
        if (sCreate || !session) return 10;

        Stage("open-before");
        OSStatus sOpen = openF(session, NULL, (__bridge CFTypeRef)source);
        fprintf(stderr, "CMPhotoTextureOpen:status=%d\n", (int)sOpen); fflush(stderr);
        report[@"openStatus"] = @(sOpen);
        Checkpoint(reportPath, report, @"open");
        if (sOpen) {
            if (invalidate) invalidate(session);
            CFRelease(session);
            return 11;
        }

        Stage("add-before");
        OSStatus sAdd = add(session, 0, 0, (__bridge CFDictionaryRef)custom);
        fprintf(stderr, "CMPhotoTextureAdd:status=%d\n", (int)sAdd); fflush(stderr);
        report[@"addStatus"] = @(sAdd);
        Checkpoint(reportPath, report, @"add");
        if (sAdd) {
            if (invalidate) invalidate(session);
            CFRelease(session);
            return 12;
        }

        CFTypeRef backing = NULL;
        Stage("close-before");
        OSStatus sClose = closeCopy(session, 0, 0, &backing);
        fprintf(stderr, "CMPhotoTextureClose:status=%d backing=%p\n", (int)sClose, backing); fflush(stderr);
        report[@"closeStatus"] = @(sClose);
        report[@"backingPresent"] = @(backing != NULL);
        if (backing) {
            CFTypeID typeID = CFGetTypeID(backing);
            report[@"backingCFTypeID"] = @(typeID);
            report[@"backingIsCFData"] = @(typeID == CFDataGetTypeID());
            fprintf(stderr, "CMPhotoTextureBacking:typeID=%lu cfDataTypeID=%lu\n", (unsigned long)typeID, (unsigned long)CFDataGetTypeID()); fflush(stderr);
        }
        Checkpoint(reportPath, report, @"close");
        if (invalidate) invalidate(session);
        CFRelease(session);
        if (sClose || !backing) {
            if (backing) CFRelease(backing);
            return 13;
        }

        if (CFGetTypeID(backing) != CFDataGetTypeID()) {
            report[@"unsupportedBackingType"] = @YES;
            Checkpoint(reportPath, report, @"unsupported-backing");
            CFRelease(backing);
            return 14;
        }

        Stage("write-output");
        CFDataRef data = (CFDataRef)backing;
        NSData *output = (__bridge NSData *)data;
        BOOL wrote = [output writeToFile:outputPath options:NSDataWritingAtomic error:&err];
        report[@"outputLength"] = @(CFDataGetLength(data));
        report[@"writeOK"] = @(wrote);
        if (!wrote) report[@"writeError"] = err.description ?: @"unknown";
        Checkpoint(reportPath, report, wrote ? @"complete" : @"write-failed");
        CFRelease(backing);
        return wrote ? 0 : 15;
    }
}
