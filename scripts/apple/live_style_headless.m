#import <AVFoundation/AVFoundation.h>
#import <CoreGraphics/CoreGraphics.h>
#import <CoreVideo/CoreVideo.h>
#import <Foundation/Foundation.h>
#import <ImageIO/ImageIO.h>
#import <dlfcn.h>
#import <zlib.h>

// These are Photos editor interfaces. Only the integration test loads them.
@interface NSObject (LiveStyleHeadless)
+ (void)configureNeutrinoCacheDirectoryIfNeeded;
+ (id)livePhotoSourceWithPhotoURL:(NSURL *)photo videoComplementURL:(NSURL *)movie;
+ (id)importPropertiesWithEXIFOrientation:(NSInteger)orientation;
+ (NSArray *)requiredVideoCompositionOutputTracksForAsset:(AVAsset *)asset;
+ (BOOL)canRenderStylesOnComposition:(id)composition;
+ (BOOL)canRenderTextureStylesOnComposition:(id)composition;
- (id)initWithEditSource:(id)source;
- (id)loadPhotoEditData:(NSData *)data formatIdentifier:(NSString *)identifier formatVersion:(NSString *)version
       importProperties:(id)properties error:(NSError **)error;
- (id)composition;
- (void)setSource:(id)source mediaType:(NSInteger)mediaType;
- (void)setCompositionController:(id)controller;
- (void)renderVideoWithTargetSize:(CGSize)size contentMode:(NSInteger)mode name:(NSString *)name
        completion:(void (^)(AVAsset *, AVVideoComposition *, AVAudioMix *, NSError *))completion;
- (id)exportVideoToURL:(NSURL *)url preset:(NSString *)preset livePhotoPairingIdentifier:(NSString *)pair
           completion:(void (^)(BOOL, CGSize, NSError *))completion;
@end

static NSData *Deflate(NSData *input) {
    if (!input) return nil;
    z_stream stream = {0};
    if (deflateInit2(&stream, 1, Z_DEFLATED, -MAX_WBITS, 8, Z_DEFAULT_STRATEGY) != Z_OK) return nil;
    NSMutableData *output = [NSMutableData dataWithLength:compressBound(input.length)];
    stream.next_in = (Bytef *)input.bytes;
    stream.avail_in = (uInt)input.length;
    stream.next_out = output.mutableBytes;
    stream.avail_out = (uInt)output.length;
    int status = deflate(&stream, Z_FINISH);
    output.length = stream.total_out;
    deflateEnd(&stream);
    return status == Z_STREAM_END ? output : nil;
}

static NSDictionary *Adjustment(NSString *identifier, NSDictionary *settings, BOOL enabled) {
    return @{@"identifier": identifier, @"settings": settings, @"enabled": @(enabled),
             @"formatVersion": @1, @"formatIdentifier": @"com.apple.photo"};
}

static id Renderer(NSURL *photo, NSURL *video, double tone, double color, BOOL enabled,
                   NSString *texture, NSError **error) {
    CGImageSourceRef image = CGImageSourceCreateWithURL((__bridge CFURLRef)photo, NULL);
    if (!image) return nil;
    NSDictionary *properties = CFBridgingRelease(CGImageSourceCopyPropertiesAtIndex(image, 0, NULL));
    CFRelease(image);
    NSNumber *orientation = properties[(NSString *)kCGImagePropertyOrientation] ?: @1;
    NSNumber *width = properties[(NSString *)kCGImagePropertyPixelWidth];
    NSNumber *height = properties[(NSString *)kCGImagePropertyPixelHeight];
    if (!width || !height) return nil;
    NSMutableArray *adjustments = [NSMutableArray arrayWithObject:Adjustment(@"SemanticStyle", @{
        @"version": @1, @"tone": @(tone), @"color": @(color), @"enabled": @(enabled),
        @"cast": @"Standard", @"intensity": @1
    }, enabled)];
    if (![texture isEqualToString:@"-"]) {
        [adjustments addObject:Adjustment(@"TextureStyle", @{
            @"version": @1, @"preset": texture, @"enabled": @YES, @"intensity": @1, @"grainIntensity": @1
        }, YES)];
    }
    NSDictionary *document = @{
        @"metadata": @{@"orientation": orientation, @"masterWidth": width, @"masterHeight": height},
        @"formatVersion": @1,
        @"versionInfo": @{@"appVersion": @"XDRemux", @"schemaRevision": @0, @"platform": @"macOS"},
        @"adjustments": adjustments
    };
    NSData *recipe = Deflate([NSJSONSerialization dataWithJSONObject:document options:0 error:error]);
    if (!recipe) return nil;
    id source = [NSClassFromString(@"PLLivePhotoEditSource") livePhotoSourceWithPhotoURL:photo videoComplementURL:video];
    id imports = [NSClassFromString(@"PLPhotoEditImportProperties") importPropertiesWithEXIFOrientation:orientation.integerValue];
    id controller = [[NSClassFromString(@"PLPhotoEditPersistenceManager") new] loadPhotoEditData:recipe
        formatIdentifier:@"com.apple.photo" formatVersion:@"1.12" importProperties:imports error:error];
    if (!source || !controller) return nil;
    [controller setSource:[source valueForKey:@"source"] mediaType:3]; // Live Photo.
    Class cls = NSClassFromString(@"PLPhotoEditRenderer");
    [cls configureNeutrinoCacheDirectoryIfNeeded];
    id renderer = [[cls alloc] initWithEditSource:source];
    [renderer setCompositionController:controller];
    return renderer;
}

static BOOL Wait(dispatch_semaphore_t done, NSTimeInterval seconds) {
    NSDate *deadline = [NSDate dateWithTimeIntervalSinceNow:seconds];
    while (dispatch_semaphore_wait(done, DISPATCH_TIME_NOW) != 0) {
        if (deadline.timeIntervalSinceNow <= 0) return NO;
        [[NSRunLoop currentRunLoop] runMode:NSDefaultRunLoopMode
                                beforeDate:[NSDate dateWithTimeIntervalSinceNow:0.05]];
    }
    return YES;
}

static NSArray *Grid(CVPixelBufferRef buffer, size_t plane, size_t channels) {
    const size_t edge = 16;
    NSMutableArray *result = [NSMutableArray arrayWithCapacity:edge * edge * channels];
    const uint8_t *base = CVPixelBufferGetBaseAddressOfPlane(buffer, plane);
    size_t height = CVPixelBufferGetHeightOfPlane(buffer, plane);
    size_t width = CVPixelBufferGetWidthOfPlane(buffer, plane);
    size_t stride = CVPixelBufferGetBytesPerRowOfPlane(buffer, plane);
    for (size_t y = 0; y < edge; ++y) {
        const uint16_t *row = (const uint16_t *)(base + stride * ((2 * y + 1) * height / (2 * edge)));
        for (size_t x = 0; x < edge; ++x) {
            size_t index = channels * ((2 * x + 1) * width / (2 * edge));
            for (size_t c = 0; c < channels; ++c) [result addObject:@(row[index + c] >> 6)];
        }
    }
    return result;
}

static NSDictionary *Frame(CMSampleBufferRef sample, NSUInteger index) {
    CVPixelBufferRef buffer = CMSampleBufferGetImageBuffer(sample);
    if (!buffer || CVPixelBufferGetPixelFormatType(buffer) != kCVPixelFormatType_420YpCbCr10BiPlanarVideoRange ||
        CVPixelBufferLockBaseAddress(buffer, kCVPixelBufferLock_ReadOnly) != kCVReturnSuccess) return nil;
    size_t width = CVPixelBufferGetWidthOfPlane(buffer, 0);
    size_t height = CVPixelBufferGetHeightOfPlane(buffer, 0);
    size_t stride = CVPixelBufferGetBytesPerRowOfPlane(buffer, 0);
    const uint8_t *plane = CVPixelBufferGetBaseAddressOfPlane(buffer, 0);
    uint64_t shadow = 0, highlights = 0;
    unsigned maxCode = 0;
    for (size_t y = 0; y < height; ++y) {
        const uint16_t *row = (const uint16_t *)(plane + y * stride);
        for (size_t x = 0; x < width; ++x) {
            unsigned code = row[x] >> 6;
            if (code > maxCode) maxCode = code;
            // 75% of the nominal ten-bit video code range.
            highlights += code > 721;
            if (x >= width / 16 && x < width / 4 && y >= height / 3 && y < height / 2) shadow += code;
        }
    }
    double shadowMean = (double)shadow / ((width / 4 - width / 16) * (height / 2 - height / 3));
    CMTime stamp = CMSampleBufferGetPresentationTimeStamp(sample);
    NSString *transfer = CFBridgingRelease(CVBufferCopyAttachment(buffer, kCVImageBufferTransferFunctionKey, NULL));
    NSString *primaries = CFBridgingRelease(CVBufferCopyAttachment(buffer, kCVImageBufferColorPrimariesKey, NULL));
    NSMutableDictionary *result = [@{
        @"pts": @[@(stamp.value), @(stamp.timescale)], @"shadow": @(shadowMean), @"max": @(maxCode),
        @"above721": @((double)highlights / (width * height)),
        @"transfer": transfer ?: @"", @"primaries": primaries ?: @""
    } mutableCopy];
    if (index % 7 == 0) {
        result[@"y"] = Grid(buffer, 0, 1);
        result[@"uv"] = Grid(buffer, 1, 2);
    }
    CVPixelBufferUnlockBaseAddress(buffer, kCVPixelBufferLock_ReadOnly);
    return result;
}

static int Export(id renderer, NSURL *destination) {
    dispatch_semaphore_t done = dispatch_semaphore_create(0);
    __block BOOL finished = NO;
    [renderer exportVideoToURL:destination preset:AVAssetExportPresetHEVCHighestQuality
          livePhotoPairingIdentifier:NSUUID.UUID.UUIDString completion:^(BOOL ok, CGSize size, NSError *error) {
        finished = ok;
        if (error) fprintf(stderr, "Export failed: %s\n", error.description.UTF8String);
        dispatch_semaphore_signal(done);
    }];
    return Wait(done, 120) && finished ? 0 : 4;
}

static int Preview(id renderer) {
    id native = [renderer composition];
    BOOL photographic = [NSClassFromString(@"PISemanticStyleAutoCalculator") canRenderStylesOnComposition:native];
    BOOL texture = [NSClassFromString(@"PITextureStyleAutoCalculator") canRenderTextureStylesOnComposition:native];
    dispatch_semaphore_t done = dispatch_semaphore_create(0);
    __block AVAsset *asset;
    __block AVVideoComposition *composition;
    [renderer renderVideoWithTargetSize:CGSizeMake(512, 512) contentMode:0 name:@"XDRemux Live Styles test"
                             completion:^(AVAsset *a, AVVideoComposition *c, AVAudioMix *mix, NSError *error) {
        asset = a;
        composition = c;
        if (error) fprintf(stderr, "Preview failed: %s\n", error.description.UTF8String);
        dispatch_semaphore_signal(done);
    }];
    if (!Wait(done, 30) || !asset || !composition) return 5;
    NSError *error = nil;
    AVAssetReader *reader = [AVAssetReader assetReaderWithAsset:asset error:&error];
    NSArray *tracks = [NSClassFromString(@"NUVideoUtilities") requiredVideoCompositionOutputTracksForAsset:asset];
    if (!reader || !tracks.count) return 6;
    AVAssetReaderVideoCompositionOutput *output = [AVAssetReaderVideoCompositionOutput
        assetReaderVideoCompositionOutputWithVideoTracks:tracks
        videoSettings:@{(NSString *)kCVPixelBufferPixelFormatTypeKey: @(kCVPixelFormatType_420YpCbCr10BiPlanarVideoRange)}];
    output.videoComposition = composition;
    if (![reader canAddOutput:output]) return 6;
    [reader addOutput:output];
    if (![reader startReading]) {
        fprintf(stderr, "Reader failed: %s\n", reader.error.description.UTF8String);
        return 6;
    }
    NSMutableArray *frames = [NSMutableArray array];
    for (;;) @autoreleasepool {
        CMSampleBufferRef sample = [output copyNextSampleBuffer];
        if (!sample) break;
        NSDictionary *frame = Frame(sample, frames.count);
        CFRelease(sample);
        if (!frame) return 7;
        [frames addObject:frame];
    }
    if (reader.status != AVAssetReaderStatusCompleted || frames.count == 0) return 8;
    NSDictionary *manifest = @{
        @"photographicStyles": @(photographic), @"textureStyles": @(texture), @"frames": frames,
        @"compositor": NSStringFromClass(composition.customVideoCompositorClass) ?: @""
    };
    NSData *json = [NSJSONSerialization dataWithJSONObject:manifest options:0 error:&error];
    if (!json) return 9;
    fwrite(json.bytes, 1, json.length, stdout);
    puts("");
    return 0;
}

int main(int argc, char **argv) {
    @autoreleasepool {
        if (argc != 7 && argc != 8) {
            fputs("usage: live-style-headless COVER MOV TONE COLOR ENABLED TEXTURE [EXPORT]\n", stderr);
            return 2;
        }
        for (NSString *framework in @[@"PhotoImaging", @"NeutrinoCore", @"PhotosFormats", @"PhotoLibraryServices", @"PhotosUICore"]) {
            NSString *path = [NSString stringWithFormat:@"/System/Library/PrivateFrameworks/%@.framework/%@", framework, framework];
            dlopen(path.UTF8String, RTLD_NOW | RTLD_GLOBAL);
        }
        NSError *error = nil;
        id renderer = Renderer([NSURL fileURLWithPath:@(argv[1])], [NSURL fileURLWithPath:@(argv[2])],
                               atof(argv[3]), atof(argv[4]), atoi(argv[5]) != 0, @(argv[6]), &error);
        if (!renderer) {
            fprintf(stderr, "Recipe or renderer error: %s\n", error.description.UTF8String ?: "no result");
            return 3;
        }
        return argc == 8 ? Export(renderer, [NSURL fileURLWithPath:@(argv[7])]) : Preview(renderer);
    }
}
