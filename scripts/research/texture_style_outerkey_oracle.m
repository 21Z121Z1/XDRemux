#import <Foundation/Foundation.h>
#import <CoreGraphics/CoreGraphics.h>
#import <ImageIO/ImageIO.h>
#import <dlfcn.h>
#import <objc/runtime.h>

typedef id (*PITextureStyleSettingsParser)(id properties);

@interface TSOuterKeySpy : NSObject
@property(nonatomic, strong) id capturedKey;
@property(nonatomic, strong) NSDictionary *returnedInner;
@end

@implementation TSOuterKeySpy
- (id)objectForKey:(id)key {
    if (self.capturedKey == nil && key != nil) {
        self.capturedKey = key;
    }
    fprintf(stdout,
            "oracle_objectForKey class=%s description=%s\n",
            key ? object_getClassName(key) : "(nil)",
            key ? [[key description] UTF8String] : "(nil)");
    fflush(stdout);
    return self.returnedInner;
}

- (id)objectForKeyedSubscript:(id)key {
    return [self objectForKey:key];
}
@end

static PITextureStyleSettingsParser load_parser(void) {
    const char *frameworks[] = {
        "/System/Library/PrivateFrameworks/PhotoImaging.framework/PhotoImaging",
        "/System/Library/PrivateFrameworks/PhotoImaging.framework/Versions/A/PhotoImaging",
    };
    for (size_t i = 0; i < sizeof(frameworks) / sizeof(frameworks[0]); ++i) {
        void *handle = dlopen(frameworks[i], RTLD_NOW | RTLD_LOCAL);
        if (handle == NULL) {
            fprintf(stdout, "dlopen_failed path=%s error=%s\n", frameworks[i], dlerror());
            continue;
        }
        void *symbol = dlsym(handle, "PITextureStyleSettingsFromMakerNoteProperties");
        if (symbol != NULL) {
            fprintf(stdout, "parser_symbol_found path=%s address=%p\n", frameworks[i], symbol);
            return (PITextureStyleSettingsParser)symbol;
        }
        fprintf(stdout, "parser_symbol_missing path=%s\n", frameworks[i]);
    }
    return NULL;
}

static NSDictionary *texture_inner(NSString *preset, double intensity, double grain) {
    return @{
        @"TextureStylePreset": preset,
        @"TextureStyleIntensity": @(intensity),
        @"TextureStyleGrain": @(grain),
    };
}

static void describe_settings(const char *label, id settings) {
    fprintf(stdout, "%s_nonnull=%d\n", label, settings != nil ? 1 : 0);
    if (settings == nil) {
        return;
    }
    fprintf(stdout, "%s_class=%s\n", label, object_getClassName(settings));
    fprintf(stdout, "%s_description=%s\n", label, [[settings description] UTF8String]);
    for (NSString *key in @[@"presetName", @"preset", @"intensity", @"grain"]) {
        @try {
            id value = [settings valueForKey:key];
            fprintf(stdout, "%s_kvc_%s=%s\n", label, [key UTF8String], value ? [[value description] UTF8String] : "(nil)");
        } @catch (NSException *exception) {
            fprintf(stdout, "%s_kvc_%s_exception=%s\n", label, [key UTF8String], [[exception name] UTF8String]);
        }
    }
}

static CGImageRef create_test_image(void) {
    const size_t width = 8;
    const size_t height = 8;
    const size_t bytesPerRow = width * 4;
    uint8_t *pixels = calloc(height, bytesPerRow);
    if (pixels == NULL) {
        return NULL;
    }
    for (size_t y = 0; y < height; ++y) {
        for (size_t x = 0; x < width; ++x) {
            size_t o = y * bytesPerRow + x * 4;
            pixels[o + 0] = (uint8_t)(32 + x * 16);
            pixels[o + 1] = (uint8_t)(64 + y * 16);
            pixels[o + 2] = 128;
            pixels[o + 3] = 255;
        }
    }
    CGColorSpaceRef colorSpace = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
    CGContextRef context = CGBitmapContextCreate(pixels,
                                                  width,
                                                  height,
                                                  8,
                                                  bytesPerRow,
                                                  colorSpace,
                                                  kCGImageAlphaPremultipliedLast | kCGBitmapByteOrder32Big);
    CGColorSpaceRelease(colorSpace);
    free(pixels);
    if (context == NULL) {
        return NULL;
    }
    CGImageRef image = CGBitmapContextCreateImage(context);
    CGContextRelease(context);
    return image;
}

static NSDictionary *write_and_read_imageio(NSString *path, id outerKey, NSDictionary *inner) {
    CGImageRef image = create_test_image();
    if (image == NULL) {
        fprintf(stdout, "imageio_create_image_failed=1\n");
        return nil;
    }

    NSURL *url = [NSURL fileURLWithPath:path];
    CGImageDestinationRef destination = CGImageDestinationCreateWithURL((__bridge CFURLRef)url,
                                                                         CFSTR("public.heic"),
                                                                         1,
                                                                         NULL);
    if (destination == NULL) {
        fprintf(stdout, "imageio_destination_create_failed=1\n");
        CGImageRelease(image);
        return nil;
    }

    NSMutableDictionary *maker = [NSMutableDictionary dictionary];
    @try {
        [maker setObject:inner forKey:outerKey];
    } @catch (NSException *exception) {
        fprintf(stdout, "imageio_maker_set_exception=%s\n", [[exception name] UTF8String]);
        CFRelease(destination);
        CGImageRelease(image);
        return nil;
    }
    NSDictionary *properties = @{
        (id)kCGImagePropertyMakerAppleDictionary: maker,
    };
    CGImageDestinationAddImage(destination, image, (__bridge CFDictionaryRef)properties);
    BOOL finalized = CGImageDestinationFinalize(destination);
    fprintf(stdout, "imageio_finalize=%d path=%s\n", finalized ? 1 : 0, [path UTF8String]);
    CFRelease(destination);
    CGImageRelease(image);
    if (!finalized) {
        return nil;
    }

    CGImageSourceRef source = CGImageSourceCreateWithURL((__bridge CFURLRef)url, NULL);
    if (source == NULL) {
        fprintf(stdout, "imageio_source_create_failed=1\n");
        return nil;
    }
    NSDictionary *roundtrip = CFBridgingRelease(CGImageSourceCopyPropertiesAtIndex(source, 0, NULL));
    CFRelease(source);
    NSDictionary *roundtripMaker = roundtrip[(id)kCGImagePropertyMakerAppleDictionary];
    fprintf(stdout,
            "imageio_roundtrip_maker_class=%s\n",
            roundtripMaker ? object_getClassName(roundtripMaker) : "(nil)");
    fprintf(stdout,
            "imageio_roundtrip_maker=%s\n",
            roundtripMaker ? [[roundtripMaker description] UTF8String] : "(nil)");
    return roundtripMaker;
}

static void run_candidate(PITextureStyleSettingsParser parser, NSString *outer, NSDictionary *inner) {
    NSDictionary *envelope = @{outer: inner};
    id result = parser(envelope);
    NSString *labelString = [NSString stringWithFormat:@"candidate_%@", outer];
    const char *label = [labelString UTF8String];
    fprintf(stdout, "candidate_outer=%s\n", [outer UTF8String]);
    describe_settings(label, result);
}

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        NSString *outputDirectory = argc > 1
            ? [NSString stringWithUTF8String:argv[1]]
            : NSTemporaryDirectory();
        [[NSFileManager defaultManager] createDirectoryAtPath:outputDirectory
                                  withIntermediateDirectories:YES
                                                   attributes:nil
                                                        error:nil];

        PITextureStyleSettingsParser parser = load_parser();
        if (parser == NULL) {
            fprintf(stdout, "oracle_status=parser_unavailable\n");
            return 2;
        }

        NSDictionary *inner = texture_inner(@"Standard", 0.0, 0.0);
        fprintf(stdout, "inner=%s\n", [[inner description] UTF8String]);

        TSOuterKeySpy *spy = [TSOuterKeySpy new];
        spy.returnedInner = inner;
        id spyResult = parser((id)spy);
        describe_settings("spy", spyResult);

        id capturedKey = spy.capturedKey;
        fprintf(stdout,
                "captured_outer_key_class=%s\n",
                capturedKey ? object_getClassName(capturedKey) : "(nil)");
        fprintf(stdout,
                "captured_outer_key=%s\n",
                capturedKey ? [[capturedKey description] UTF8String] : "(nil)");

        for (NSString *candidate in @[@"TextureStyle", @"TextureStyle~1.0", @"84", @"SmartStyle", @"SemanticStyle"]) {
            run_candidate(parser, candidate, inner);
        }

        if (capturedKey != nil) {
            @try {
                NSDictionary *exactEnvelope = @{capturedKey: inner};
                id exactResult = parser(exactEnvelope);
                describe_settings("captured_envelope", exactResult);
            } @catch (NSException *exception) {
                fprintf(stdout, "captured_envelope_exception=%s\n", [[exception name] UTF8String]);
            }

            NSString *path = [outputDirectory stringByAppendingPathComponent:@"texture-style-imageio-oracle.heic"];
            NSDictionary *roundtripMaker = write_and_read_imageio(path, capturedKey, inner);
            if (roundtripMaker != nil) {
                id roundtripResult = parser(roundtripMaker);
                describe_settings("imageio_roundtrip", roundtripResult);
            } else {
                fprintf(stdout, "imageio_roundtrip_nonnull=0\n");
            }
        }

        fprintf(stdout, "oracle_status=completed\n");
        return 0;
    }
}
