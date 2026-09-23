// Runtime declarations only: no native model execution, memory dumps, or system changes.
#import <Foundation/Foundation.h>
#import <objc/runtime.h>
#import <dlfcn.h>

static NSArray *Methods(Class target) {
    NSMutableArray *result = [NSMutableArray array];
    unsigned count = 0;
    Method *methods = target ? class_copyMethodList(target, &count) : NULL;
    for (unsigned i = 0; i < count; ++i) {
        const char *encoding = method_getTypeEncoding(methods[i]);
        [result addObject:@{
            @"selector": NSStringFromSelector(method_getName(methods[i])),
            @"encoding": encoding ? [NSString stringWithUTF8String:encoding] : @"?"
        }];
    }
    free(methods);
    return [result sortedArrayUsingComparator:^NSComparisonResult(NSDictionary *a, NSDictionary *b) {
        return [a[@"selector"] compare:b[@"selector"]];
    }];
}

int main(int argc, const char **argv) {
    @autoreleasepool {
        if (argc != 1) { (void)argv; return 64; }
        NSMutableDictionary *frameworks = [NSMutableDictionary dictionary];
        NSMutableArray *handles = [NSMutableArray array];
        for (NSString *name in @[@"ANSTKit", @"CMImaging", @"CMCapture", @"PhotoImaging"]) {
            void *handle = NULL;
            for (NSString *suffix in @[@"Versions/A/", @""]) {
                NSString *path = [NSString stringWithFormat:@"/System/Library/PrivateFrameworks/%@.framework/%@%@", name, suffix, name];
                handle = dlopen(path.UTF8String, RTLD_NOW | RTLD_LOCAL);
                if (handle) break;
            }
            frameworks[name] = @(handle != NULL);
            if (handle) [handles addObject:[NSValue valueWithPointer:handle]];
        }
        NSMutableDictionary *classes = [NSMutableDictionary dictionary];
        for (NSString *name in @[
            @"ANSTFsincAlgorithm", @"ANSTFsincAlgorithmConfiguration", @"ANSTFsincAlgorithmV2Dot4",
            @"ANSTFsincInferenceConfiguration", @"ANSTFsincInferenceDescriptor",
            @"ANSTFsincInferenceDescriptorV2Dot4", @"ANSTFsincInferencePostprocessorV2Dot4",
            @"ANSTE5MLNetwork", @"CMITextureStylesPersonInputDataUtilities",
            @"CMITextureStylesPersonInputData", @"CMITextureStylesProcessor", @"PITextureStyleProcessorKernel"
        ]) {
            Class cls = NSClassFromString(name);
            if (!cls) { classes[name] = [NSNull null]; continue; }
            const char *image = class_getImageName(cls);
            classes[name] = @{
                @"image": image ? [NSString stringWithUTF8String:image] : @"?",
                @"instanceMethods": Methods(cls), @"classMethods": Methods(object_getClass(cls))
            };
        }
        NSMutableDictionary *symbols = [NSMutableDictionary dictionary];
        for (NSString *name in @[@"ANSTFsincAlgorithmVersionToNSString", @"ANSTFsincInferenceVersionToNSString",
                                 @"ANSTFsincAlgorithmResolutionToNSString", @"ANSTFsincInferenceResolutionToNSString"]) {
            BOOL found = NO;
            for (NSValue *value in handles) {
                if (dlsym(value.pointerValue, name.UTF8String)) { found = YES; break; }
            }
            symbols[name] = @(found);
        }
        NSDictionary *report = @{
            @"schema": @1, @"evidenceLevel": @"runtime-declarations-only", @"productAccepted": @NO,
            @"osVersion": [NSProcessInfo processInfo].operatingSystemVersionString,
            @"frameworks": frameworks, @"classes": classes, @"symbols": symbols
        };
        NSError *error = nil;
        NSData *data = [NSJSONSerialization dataWithJSONObject:report options:NSJSONWritingSortedKeys error:&error];
        if (!data) { fprintf(stderr, "%s\n", error.localizedDescription.UTF8String); return 2; }
        fwrite(data.bytes, 1, data.length, stdout); fputc('\n', stdout);
        // Keep frameworks loaded while Objective-C objects can still be alive.
        return 0;
    }
}
