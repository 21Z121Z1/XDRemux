#import <Foundation/Foundation.h>
#import <CoreFoundation/CoreFoundation.h>
#import <dlfcn.h>

typedef int32_t OSStatus;
typedef OSStatus (*CMPhotoCompressionSessionCreateFn)(CFAllocatorRef, CFDictionaryRef, CFTypeRef *);
typedef OSStatus (*CMPhotoCompressionSessionOpenExistingContainerForModificationFFn)(CFTypeRef, CFDictionaryRef, CFTypeRef);
typedef OSStatus (*CMPhotoCompressionSessionAddCustomMetadataFn)(CFTypeRef, int64_t, int64_t, CFDictionaryRef);
typedef OSStatus (*CMPhotoCompressionSessionCloseContainerAndCopyBackingFn)(CFTypeRef, int64_t, int64_t, CFTypeRef *);
typedef void (*CMPhotoCompressionSessionInvalidateFn)(CFTypeRef);

static id LoadConstant(void *handle, const char *name) {
    void *sym = dlsym(handle, name);
    if (!sym) return nil;
    return (__bridge id)(*(CFTypeRef *)sym);
}

static void WriteJSON(NSString *path, NSDictionary *obj) {
    NSData *d = [NSJSONSerialization dataWithJSONObject:obj options:(NSJSONWritingPrettyPrinted|NSJSONWritingSortedKeys) error:nil];
    [d writeToFile:path atomically:YES];
}

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        if (argc != 4) {
            fprintf(stderr, "usage: cmphoto_texture_style_writer INPUT OUTPUT REPORT_JSON\n");
            return 64;
        }
        NSString *inputPath = [NSString stringWithUTF8String:argv[1]];
        NSString *outputPath = [NSString stringWithUTF8String:argv[2]];
        NSString *reportPath = [NSString stringWithUTF8String:argv[3]];
        NSMutableDictionary *report = [NSMutableDictionary dictionary];
        report[@"schema"] = @"xdremux-cmphoto-texture-style-writer-v2";
        report[@"input"] = [inputPath lastPathComponent];
        report[@"output"] = [outputPath lastPathComponent];

        void *cm = dlopen("/System/Library/PrivateFrameworks/CMPhoto.framework/CMPhoto", RTLD_NOW|RTLD_LOCAL);
        if (!cm) {
            report[@"dlerror"] = @(dlerror() ?: "unknown");
            WriteJSON(reportPath, report);
            return 2;
        }

        CMPhotoCompressionSessionCreateFn create = (CMPhotoCompressionSessionCreateFn)dlsym(cm, "CMPhotoCompressionSessionCreate");
        CMPhotoCompressionSessionOpenExistingContainerForModificationFFn openF = (CMPhotoCompressionSessionOpenExistingContainerForModificationFFn)dlsym(cm, "CMPhotoCompressionSessionOpenExistingContainerForModificationF");
        CMPhotoCompressionSessionAddCustomMetadataFn add = (CMPhotoCompressionSessionAddCustomMetadataFn)dlsym(cm, "CMPhotoCompressionSessionAddCustomMetadata");
        CMPhotoCompressionSessionCloseContainerAndCopyBackingFn closeCopy = (CMPhotoCompressionSessionCloseContainerAndCopyBackingFn)dlsym(cm, "CMPhotoCompressionSessionCloseContainerAndCopyBacking");
        CMPhotoCompressionSessionInvalidateFn invalidate = (CMPhotoCompressionSessionInvalidateFn)dlsym(cm, "CMPhotoCompressionSessionInvalidate");
        report[@"symbols"] = @{
            @"create": @(create != NULL), @"openF": @(openF != NULL), @"add": @(add != NULL),
            @"closeCopy": @(closeCopy != NULL), @"invalidate": @(invalidate != NULL)
        };
        if (!create || !openF || !add || !closeCopy) {
            WriteJSON(reportPath, report);
            return 3;
        }

        id keyData = LoadConstant(cm, "kCMPhotoCustomMetadata_Data");
        id keyURI = LoadConstant(cm, "kCMPhotoCustomMetadata_URI");
        id keyName = LoadConstant(cm, "kCMPhotoCustomMetadata_Name");
        id exportedTextureURN = LoadConstant(cm, "kCMPhotoCustomMetadataTypeURN_TextureStyles");
        // iOS 27 firmware exports kCMPhotoCustomMetadataTypeURN_TextureStyles with this exact value.
        // macOS 27 CMPhoto on the hosted runner exposes the writer APIs but not that iOS-only symbol.
        id textureURN = exportedTextureURN ?: @"tag:apple.com,2026:photo:metadata:texture_styles";
        report[@"constants"] = @{
            @"Data": keyData ?: [NSNull null], @"URI": keyURI ?: [NSNull null],
            @"Name": keyName ?: [NSNull null], @"TextureURN": textureURN,
            @"TextureURNFromRuntimeSymbol": @(exportedTextureURN != nil)
        };
        if (!keyData || !keyURI || !keyName) {
            WriteJSON(reportPath, report);
            return 4;
        }

        NSError *err = nil;
        NSData *source = [NSData dataWithContentsOfFile:inputPath options:NSDataReadingMappedIfSafe error:&err];
        if (!source) {
            report[@"sourceError"] = err.description ?: @"unknown";
            WriteJSON(reportPath, report);
            return 5;
        }

        NSDictionary *texture = @{
            @"Version": @1,
            @"HardwareModel": @"V63AP",
            @"PortType": @"PortTypeBack",
            @"CaptureMode": @"Photo",
            @"CaptureType": @"Photo",
            @"FilmGrainSeed": @1480872525,
            @"TextureStylePeopleDataVersion": @1,
            @"TextureStylePostProcessedPeopleData": @[]
        };
        NSData *plist = [NSPropertyListSerialization dataWithPropertyList:texture
                                                                   format:NSPropertyListBinaryFormat_v1_0
                                                                  options:0
                                                                    error:&err];
        if (!plist) {
            report[@"plistError"] = err.description ?: @"unknown";
            WriteJSON(reportPath, report);
            return 6;
        }
        report[@"plistLength"] = @(plist.length);

        NSDictionary *custom = @{ keyData: plist, keyURI: textureURN, keyName: @"textureStyleMetadata" };
        CFTypeRef session = NULL;
        OSStatus sCreate = create(kCFAllocatorDefault, NULL, &session);
        report[@"createStatus"] = @(sCreate);
        report[@"sessionCreated"] = @(session != NULL);
        if (sCreate || !session) {
            WriteJSON(reportPath, report);
            return 10;
        }

        OSStatus sOpen = openF(session, NULL, (__bridge CFTypeRef)source);
        report[@"openStatus"] = @(sOpen);
        if (sOpen) {
            if (invalidate) invalidate(session);
            CFRelease(session);
            WriteJSON(reportPath, report);
            return 11;
        }

        OSStatus sAdd = add(session, 0, 0, (__bridge CFDictionaryRef)custom);
        report[@"addStatus"] = @(sAdd);
        if (sAdd) {
            if (invalidate) invalidate(session);
            CFRelease(session);
            WriteJSON(reportPath, report);
            return 12;
        }

        CFTypeRef backing = NULL;
        OSStatus sClose = closeCopy(session, 0, 0, &backing);
        report[@"closeStatus"] = @(sClose);
        report[@"backingClass"] = backing ? NSStringFromClass([(__bridge id)backing class]) : [NSNull null];
        if (invalidate) invalidate(session);
        CFRelease(session);
        if (sClose || !backing) {
            if (backing) CFRelease(backing);
            WriteJSON(reportPath, report);
            return 13;
        }

        id backingObj = (__bridge id)backing;
        NSData *output = [backingObj isKindOfClass:[NSData class]] ? backingObj : nil;
        if (!output) {
            report[@"backingDescription"] = [backingObj description] ?: @"";
            CFRelease(backing);
            WriteJSON(reportPath, report);
            return 14;
        }
        BOOL wrote = [output writeToFile:outputPath options:NSDataWritingAtomic error:&err];
        report[@"outputLength"] = @(output.length);
        report[@"writeOK"] = @(wrote);
        if (!wrote) report[@"writeError"] = err.description ?: @"unknown";
        CFRelease(backing);
        WriteJSON(reportPath, report);
        return wrote ? 0 : 15;
    }
}
