#import "spi_call.h"
#import "include/xdremux_texture.h"
#import <MetalKit/MetalKit.h>


static id<MTLTexture> Texture(id<MTLDevice> device, MTLPixelFormat format, size_t width, size_t height) {
    MTLTextureDescriptor *descriptor = [MTLTextureDescriptor texture2DDescriptorWithPixelFormat:format width:width height:height mipmapped:NO];
    descriptor.usage = MTLTextureUsageShaderRead | MTLTextureUsageShaderWrite;
    descriptor.storageMode = MTLStorageModeShared;
    return [device newTextureWithDescriptor:descriptor];
}

static id Wrap(id<MTLTexture> texture, CGRect region) {
    id wrapper = Make(@"CMITSTextureAndFullImageRegion");
    Call(wrapper, @"setTexture:", @[texture]);
    Call(wrapper, @"setFullImageRegion:", @[[NSValue valueWithRect:region]]);
    return wrapper;
}

static NSDictionary *ScalarTextureFacts(id<MTLTexture> texture) {
    if (!texture) return @{@"available": @NO};
    size_t size = texture.pixelFormat == MTLPixelFormatR8Unorm ? 1 : texture.pixelFormat == MTLPixelFormatR16Float ? 2 : texture.pixelFormat == MTLPixelFormatR32Float ? 4 : 0;
    NSMutableDictionary *result = [@{@"available": @YES, @"width": @(texture.width), @"height": @(texture.height), @"format": @(texture.pixelFormat), @"type": @(texture.textureType)} mutableCopy];
    if (!size || texture.textureType != MTLTextureType2D || texture.storageMode == MTLStorageModePrivate) return result;
    NSMutableData *pixels = [NSMutableData dataWithLength:texture.width * texture.height * size];
    [texture getBytes:pixels.mutableBytes bytesPerRow:texture.width * size fromRegion:MTLRegionMake2D(0, 0, texture.width, texture.height) mipmapLevel:0];
    double sum = 0, maxValue = 0; uint64_t nonzero = 0;
    for (size_t i = 0; i < texture.width * texture.height; i++) {
        double v = size == 1 ? ((const uint8_t *)pixels.bytes)[i] / 255.0 : size == 2 ? ((const __fp16 *)pixels.bytes)[i] : ((const float *)pixels.bytes)[i];
        if (isfinite(v)) { sum += v; maxValue = MAX(maxValue, v); nonzero += v > 0.001; }
    }
    result[@"mean"] = @(sum / (texture.width * texture.height)); result[@"maximum"] = @(maxValue);
    result[@"nonzero_over_0_001"] = @(nonzero); result[@"sha256"] = SHA(pixels);
    return result;
}

static void RecordGPUExecution(NSMutableDictionary *report, id executionStatus, NSMutableArray *failures) {
    for (id<MTLCommandBuffer> failed in Call(executionStatus, @"failedCommandBuffers", @[]))
        [failures addObject:@{@"status": @(failed.status), @"error": failed.error.description ?: @"", @"label": failed.label ?: @""}];
    report[@"native_GPU_execution"] = @{@"diagnostic_counters_are_not_execution_proof": @YES, @"status": Call(executionStatus, @"status", @[]), @"completed_command_buffers": Call(executionStatus, @"completedCommandBuffersCount", @[]), @"failures": [failures copy]};
}

int xdremux_texture_statistics(const char *inputPath, const char *outputPath, const char *directoryPath, uint32_t selectedPerson, const char *effectName, const char *tuningJSON) {
    @autoreleasepool {
        NSString *input = @(inputPath), *out = @(outputPath);
        uint64_t landmarkType = 1;
        NSString *effectKind = @(effectName);
        NSDictionary *profile = [NSJSONSerialization JSONObjectWithData:[@(tuningJSON) dataUsingEncoding:NSUTF8StringEncoding] options:0 error:NULL];
        NSString *parameterSource = profile[@"preset"];
        double exposure = 0;
        NSMutableDictionary *report = [@{
            @"schema": @"texture-image-statistics-v1", @"stage": @"begin", @"input_sha256": SHA([NSData dataWithContentsOfFile:input]),
            @"apple_implementation_patched": @NO, @"provided_capture_image_stats": @NO,
            @"landmark_type_control": @(landmarkType), @"color_domain": @"newly derived extended linear Display P3 from upright SDR input",
            @"parameter_source_control": parameterSource, @"exposure_EV_control": @(exposure),
            @"producer_effect": effectKind,
        } mutableCopy];
        Write(report, out);
        @try {
            dlopen("/System/Library/PrivateFrameworks/CMImaging.framework/CMImaging", RTLD_NOW | RTLD_LOCAL);
            dlopen("/System/Library/PrivateFrameworks/PhotoImaging.framework/PhotoImaging", RTLD_NOW | RTLD_LOCAL);
            id<MTLDevice> device = MTLCreateSystemDefaultDevice();
            CGImageSourceRef source = CGImageSourceCreateWithURL((__bridge CFURLRef)[NSURL fileURLWithPath:input], NULL);
            CGImageRef image = CGImageSourceCreateImageAtIndex(source, 0, NULL);
            if (!image) @throw [NSException exceptionWithName:@"Decode" reason:@"image unavailable" userInfo:nil];
            size_t width = CGImageGetWidth(image), height = CGImageGetHeight(image);
            CGRect full = CGRectMake(0, 0, width, height);
            NSError *error = nil;
            NSString *replayRoot = nil;
            NSString *rawRoot = @(directoryPath);
            NSString *contextRoot = replayRoot ?: rawRoot;
            NSUInteger personIndex = selectedPerson;
            BOOL linearOnly = NO;
            if(!contextRoot) @throw [NSException exceptionWithName:@"SharedContext" reason:@"an inferred or final-file context is required" userInfo:nil];
            NSDictionary *rawContext = rawRoot ? [NSJSONSerialization JSONObjectWithData:[NSData dataWithContentsOfFile:[rawRoot stringByAppendingPathComponent:@"carrier-production.json"]] options:0 error:NULL] : nil;
            NSDictionary *selected = nil, *storedPerson = nil;
            if(replayRoot) {
                NSString *personPath=[replayRoot stringByAppendingPathComponent:[NSString stringWithFormat:@"stored-person-%lu-pixels.json",(unsigned long)personIndex]];
                if(![[NSFileManager defaultManager] fileExistsAtPath:personPath] && personIndex==0)personPath=[replayRoot stringByAppendingPathComponent:@"stored-person-pixels.json"];
                storedPerson=[NSJSONSerialization JSONObjectWithData:[NSData dataWithContentsOfFile:personPath] options:0 error:NULL];
                report[@"provided_precomputed_image_stats_from_final_HEIC"]=@YES;
            } else {
                NSArray *people=rawContext[@"people"];
                if(people && personIndex<people.count) selected=people[personIndex];
                else if(!people && rawContext[@"native_person_without_stats"])selected=@{@"native_person_without_stats":rawContext[@"native_person_without_stats"]};
                report[@"shared_context_geometry_used"]=@YES;
            }
            NSDictionary *personDictionary = storedPerson ?: selected[@"native_person_without_stats"];
            if(!personDictionary && !linearOnly) @throw [NSException exceptionWithName:@"PersonSelection" reason:@"no measured person at requested index" userInfo:nil];
            id person=personDictionary?Call(NSClassFromString(@"CMITextureStylesPersonInputData"),@"personDataFromDictionary:",@[personDictionary]):nil;
            report[@"person_index"]=@(personIndex); report[@"fresh_Vision_observations_used_as_replay_inputs"]=@NO;
            if(person){report[@"person_before_stats"]=PersonDictionary(person);report[@"unadjusted_native_person_dictionary_before_stats"]=Call(person,@"dictionaryRepresentation",@[]);}
            NSDictionary *roleFiles=@{@"skin":@"semanticskinmattev2",@"face":@"semanticfaceskinmatte",@"instance":selected[@"instance_role"] ?: @"semanticpersonmatte",@"person":@"semanticpersonmatte",@"hair":@"semantichairmatte",@"glasses":@"semanticglassesmattev2",@"teeth":@"semanticteethmattev2",@"nose":@"semanticnosematte",@"lips":@"semanticlipsmatte",@"eyebrows":@"semanticeyebrowsmatte",@"body":@"semanticnonfaceskinmatte",@"hands":@"semantichandsmatte",@"ears":@"semanticearsmatte",@"tattoo":@"semantictattoomatte"};
            NSMutableDictionary *contextTextures=[NSMutableDictionary new],*inputFacts=[NSMutableDictionary new];
            for(NSString *role in roleFiles) {
                NSString *file=replayRoot?[@"stored-" stringByAppendingFormat:@"%@.l8",role]:[roleFiles[role] stringByAppendingString:@".l8"];
                if(replayRoot && [role isEqual:@"instance"]) {
                    NSString *specific=[NSString stringWithFormat:@"stored-instance-%lu.l8",(unsigned long)personIndex];
                    if([[NSFileManager defaultManager] fileExistsAtPath:[replayRoot stringByAppendingPathComponent:specific]])file=specific;
                }
                NSData *pixels=[NSData dataWithContentsOfFile:[contextRoot stringByAppendingPathComponent:file]];
                if(pixels.length!=width*height)@throw [NSException exceptionWithName:@"ContextMaskDimensions" reason:file userInfo:nil];
                id<MTLTexture> texture=Texture(device,MTLPixelFormatR8Unorm,width,height);
                [texture replaceRegion:MTLRegionMake2D(0,0,width,height) mipmapLevel:0 withBytes:pixels.bytes bytesPerRow:width];
                contextTextures[role]=texture;inputFacts[role]=ScalarTextureFacts(texture);
            }
            id<MTLTexture> skinTexture=contextTextures[@"skin"],faceTexture=contextTextures[@"face"],instanceTexture=contextTextures[@"instance"];
            report[@"actual_input_mask_textures"]=inputFacts;
            report[@"face_skin_sha256"]=inputFacts[@"face"][@"sha256"];report[@"face_skin_mean"]=inputFacts[@"face"][@"mean"];
            report[@"replay_mask_source"]=replayRoot?@"native decoded final-file masks":@"shared measured image-derived masks";
            MTKTextureLoader *loader = [[MTKTextureLoader alloc] initWithDevice:device];
            id<MTLTexture> encoded = [loader newTextureWithCGImage:image options:@{MTKTextureLoaderOptionOrigin: MTKTextureLoaderOriginTopLeft, MTKTextureLoaderOptionSRGB: @NO} error:&error];
            if (!encoded) @throw [NSException exceptionWithName:@"ImageTexture" reason:error.description userInfo:nil];
            CGColorSpaceRef srgb = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
            CGColorSpaceRef working = CGColorSpaceCreateWithName(kCGColorSpaceExtendedLinearDisplayP3);
            CIImage *ciImage = [CIImage imageWithMTLTexture:encoded options:@{kCIImageColorSpace: (__bridge id)srgb}];
            if (exposure != 0) ciImage = [ciImage imageByApplyingFilter:@"CIExposureAdjust" withInputParameters:@{kCIInputEVKey: @(exposure)}];
            id<MTLTexture> inputTexture = Texture(device, MTLPixelFormatRGBA16Float, width, height);
            id<MTLTexture> outputTexture = Texture(device, MTLPixelFormatRGBA16Float, width, height);
            id<MTLCommandQueue> queue = [device newCommandQueue];
            id<MTLCommandBuffer> command = [queue commandBuffer];
            CIContext *context = [CIContext contextWithMTLDevice:device options:@{}];
            [context render:ciImage toMTLTexture:inputTexture commandBuffer:command bounds:full colorSpace:working];
            // The original _calculateStats reads IO.outputImage. The main CMI
            // processor initializes that working image before calculating stats.
            id<MTLBlitCommandEncoder> initialCopy = [command blitCommandEncoder];
            [initialCopy copyFromTexture:inputTexture toTexture:outputTexture];
            [initialCopy endEncoding];
            [command commit]; [command waitUntilCompleted];
            report[@"image_upload_status"] = @(command.status);
            report[@"statistics_samples_initialized_output_image"] = @YES;
            NSMutableData *imagePixels = [NSMutableData dataWithLength:width * height * 8];
            [inputTexture getBytes:imagePixels.mutableBytes bytesPerRow:width * 8 fromRegion:MTLRegionMake2D(0, 0, width, height) mipmapLevel:0];
            const __fp16 *imageValues = imagePixels.bytes;
            double mean[3] = {0}, maxValue = 0; uint64_t nonfinite = 0;
            for (size_t i = 0; i < width * height; i++) for (size_t c = 0; c < 3; c++) {
                float v = imageValues[i * 4 + c];
                if (!isfinite(v)) { nonfinite++; continue; }
                mean[c] += v / (width * height); maxValue = MAX(maxValue, v);
            }
            report[@"image_pixels"] = @{@"sha256": SHA(imagePixels), @"mean_rgb": @[@(mean[0]), @(mean[1]), @(mean[2])], @"maximum": @(maxValue), @"nonfinite": @(nonfinite)};
            if(linearOnly) {
                report[@"linearization_only"]=@YES;report[@"stage"]=@"finished";Write(report,out);
                CGImageRelease(image);CFRelease(source);CGColorSpaceRelease(srgb);CGColorSpaceRelease(working);return 0;
            }
            NSMutableDictionary *nativeStats = [NSMutableDictionary new];
            if (storedPerson) [nativeStats addEntriesFromDictionary:storedPerson[@"imageStats"][@"Mattify"]];
            NSString *ioName = [effectKind isEqual:@"SkinSmoothStandalone"] ? @"CMITextureStylesSkinSmoothIO" : [[@"CMITextureStyles" stringByAppendingString:effectKind] stringByAppendingString:@"IO"];
            id io = Make(ioName);
            Call(io, @"setInputImage:", @[Wrap(inputTexture, full)]);
            Call(io, @"setOutputImage:", @[Wrap(outputTexture, full)]);
            if ([io respondsToSelector:NSSelectorFromString(@"setInputSkinMask:")]) Call(io, @"setInputSkinMask:", @[Wrap(skinTexture, full)]);
            if ([io respondsToSelector:NSSelectorFromString(@"setInputFaceMask:")]) Call(io, @"setInputFaceMask:", @[Wrap(faceTexture, full)]);
            if ([effectKind isEqual:@"SkinSmoothStandalone"]) {
                Call(io, @"setInputSkinMaskAndFullImageRegion:", @[Wrap(skinTexture, full)]);
                Call(io, @"setFaceSkinMaskTextureAndFullImageRegion:", @[Wrap(faceTexture, full)]);
                Call(io, @"setInstanceMaskTextureAndFullImageRegion:", @[Wrap(instanceTexture, full)]);
                Call(io, @"setPersonMaskTextureAndFullImageRegion:", @[Wrap(instanceTexture, full)]);
                Call(io, @"setInputFaceRect:", @[Call(person,@"faceROI",@[])]);
                Call(io, @"setAllPersonDataForBlending:", @[@[person]]);
            } else Call(io, @"setInputInstanceMask:", @[Wrap(instanceTexture, full)]);
            if (contextTextures.count) {
                NSDictionary *selectors = [effectKind isEqual:@"SkinSmoothStandalone"] ? @{@"hair":@"setHairMaskTextureAndFullImageRegion:",@"glasses":@"setGlassesMaskTextureAndFullImageRegion:",@"teeth":@"setTeethMaskTextureAndFullImageRegion:",@"lips":@"setLipsMaskTextureAndFullImageRegion:",@"eyebrows":@"setEyebrowsMaskTextureAndFullImageRegion:",@"body":@"setBodyMaskTextureAndFullImageRegion:",@"hands":@"setHandsMaskTextureAndFullImageRegion:",@"ears":@"setEarsMaskTextureAndFullImageRegion:",@"tattoo":@"setTattoosMaskTextureAndFullImageRegion:"} : @{@"hair":@"setInputHairMask:",@"glasses":@"setInputGlassesMask:",@"nose":@"setInputNoseMask:",@"lips":@"setInputLipMask:",@"ears":@"setInputEarMask:",@"tattoo":@"setInputTattooMask:"};
                for (NSString *role in selectors) if ([io respondsToSelector:NSSelectorFromString(selectors[role])]) Call(io,selectors[role],@[Wrap(contextTextures[role],full)]);
                if ([effectKind isEqual:@"SkinSmoothStandalone"]) Call(io,@"setAllPersonDataForBlending:",@[@[person]]);
            }
            Call(io, @"setOutputPersonStats:", @[nativeStats]);
            id effect = Call([NSClassFromString([@"CMITextureStyles" stringByAppendingString:effectKind]) alloc], @"initWithOptionalMetalContext:", @[NSNull.null]);
            id parameters = nil;
            if ([parameterSource isEqual:@"defaults"]) parameters = Make([ioName stringByReplacingOccurrencesOfString:@"IO" withString:@"Parameters"]);
            else {
                id tuning = Call(NSClassFromString(@"CMITextureStyleTuningLookup"), @"tuningDictionaryForHardwareModel:portType:captureMode:preset:captureType:", @[profile[@"hardware_model"], profile[@"port_type"], profile[@"capture_mode"], parameterSource, profile[@"capture_type"]]);
                if (!tuning) @throw [NSException exceptionWithName:@"TuningUnavailable" reason:parameterSource userInfo:nil];
                NSString *descriptorSelector = [effectKind isEqual:@"Mattify"] ? @"descriptorForMattifyWithTuningDictionary:" : [effectKind isEqual:@"UnderEyeBrighten"] ? @"descriptorForUnderEyeBrighteningWithTuningDictionary:" : @"descriptorForSkinSmoothingWithTuningDictionary:";
                id effectDescriptor = Call(NSClassFromString(@"PITextureStyleProcessorKernel"), descriptorSelector, @[tuning]);
                parameters = Call(effectDescriptor, @"parameters", @[]);
                if (!parameters) @throw [NSException exceptionWithName:@"TuningUnavailable" reason:@"native Mattify parameters unavailable" userInfo:nil];
                report[@"tuning_target_is_source_camera_provenance"] = @NO;
                report[@"native_effect_type"] = Call(effectDescriptor, @"type", @[]);
            }
            NSMutableDictionary *parameterValues = [NSMutableDictionary new];
            for (NSString *key in @[@"editingStrength", @"smallBlurRadiusFaceDiagonalFactor", @"largeBlurRadiusFaceDiagonalFactor", @"fracMaskHighlightsHeadroom", @"fracMaskHighlightsNormFactor"])
                if ([parameters respondsToSelector:NSSelectorFromString(key)]) parameterValues[key] = Call(parameters, key, @[]);
            report[@"native_parameters"] = parameterValues;
            Call(effect, @"setParameters:", @[parameters]);
            Call(effect, @"setInputOutput:", @[io]);
            Call(effect, @"setPersonData:", @[person]);
            Call(effect, @"setInstanceID:", @[Call(person,@"faceID",@[])]);
            Call(effect, @"setFullImageSize:", @[[NSValue valueWithSize:full.size]]);
            Call(effect, @"setRegionToRender:", @[[NSValue valueWithRect:full]]);
            if ([effect respondsToSelector:NSSelectorFromString(@"setSkipRendering:")]) Call(effect, @"setSkipRendering:", @[@YES]);
            report[@"parameter_validation"] = Call(parameters, @"validate", @[]);
            id allocator = Call(effect, @"allocator", @[]);
            if (!allocator) {
                // Type 2 is the native CMI allocator path recorded in the prior
                // CMIStyleEngineProcessor investigation; no cache implementation is changed.
                allocator = Call([NSClassFromString(@"FigMetalAllocator") alloc], @"initWithDevice:allocatorType:", @[device, @2]);
                id descriptor = Make(@"FigMetalAllocatorDescriptor");
                uint64_t allocatorMiB=128;
                if(allocatorMiB<64||allocatorMiB>2048)@throw [NSException exceptionWithName:@"AllocatorBudget" reason:@"outside diagnostic budget bounds" userInfo:nil];
                report[@"allocator_budget_MiB"]=@(allocatorMiB);
                Call(descriptor, @"setMemSize:", @[@(allocatorMiB * 1024 * 1024)]);
                Call(descriptor, @"setAllowFallback:", @[@YES]);
                report[@"allocator_setup"] = Call(allocator, @"setupWithDescriptor:", @[descriptor]);
                Call(effect, @"setAllocator:", @[allocator]);
            }
            report[@"allocator_class"] = allocator ? NSStringFromClass([allocator class]) : @"nil";
            id nativeContext = [effect valueForKey:@"_context"];
            id originalQueue = Call(effect, @"metalCommandQueue", @[]);
            report[@"native_queue_before_binding"] = originalQueue ? [originalQueue description] : @"nil";
            Call(effect, @"setMetalCommandQueue:", @[queue]);
            report[@"native_queue_explicitly_bound"] = @(Call(effect, @"metalCommandQueue", @[]) != nil);
            id executionStatus = Make(@"FigMetalExecutionStatus");
            Call(nativeContext, @"setExecutionStatus:", @[executionStatus]);
            NSMutableArray *gpuFailures = [NSMutableArray new];
            NSMutableArray *callOrder=[NSMutableArray new];report[@"native_call_order"]=callOrder;
            for (NSString *stage in @[@"setup", @"prewarm", @"prepareToProcess:", @"calculateStats", @"process"]) {
                [callOrder addObject:stage];
                report[@"stage"] = stage; Write(report, out);
                id<MTLCommandBuffer> explicitCommand = nil;
                if ([stage isEqual:@"calculateStats"] || [stage isEqual:@"process"]) {
                    explicitCommand = [queue commandBuffer];
                    explicitCommand.label = [@"Texture producer " stringByAppendingString:stage];
                    Call(nativeContext, @"setCommandBuffer:", @[explicitCommand]);
                }
                NSNumber *status = Call(effect, stage, [stage containsString:@":"] ? @[@0] : @[]);
                report[stage] = status; Write(report, out);
                if ([stage isEqual:@"setup"]) Call(effect, @"setMetalCommandQueue:", @[queue]);
                if ([stage isEqual:@"calculateStats"] || [stage isEqual:@"process"] || [stage isEqual:@"finishProcessing"]) {
                    Call(nativeContext, @"forceCommit", @[]);
                    Call(nativeContext, @"waitForIdle", @[]);
                }
                if (explicitCommand) {
                    if (explicitCommand.status == MTLCommandBufferStatusNotEnqueued || explicitCommand.status == MTLCommandBufferStatusEnqueued) [explicitCommand commit];
                    [explicitCommand waitUntilCompleted];
                    report[[stage stringByAppendingString:@"_command"]] = @{@"status": @(explicitCommand.status), @"error": explicitCommand.error.description ?: @""};
                }
                RecordGPUExecution(report, executionStatus, gpuFailures);
                Write(report, out);
                @try {
                    NSDictionary *states = [effect valueForKey:@"_perPersonIntermediatesAndStats"];
                    id state = states[Call(person,@"faceID",@[])];
                    if (state) {
                        NSValue *roi = [state valueForKey:@"_extendedFaceROI"];
                        NSMutableDictionary *details = [@{@"extended_face_roi": roi.description ?: @""} mutableCopy];
                        report[@"native_intermediates"] = details;
                    }
                } @catch (NSException *diagnostic) { report[@"intermediate_inspection_error"] = diagnostic.reason ?: @""; }
                report[stage] = status; report[@"native_output_person_stats"] = nativeStats; Write(report, out);
                if (status.intValue != 0) break;
            }
            BOOL productionCompleted=YES;
            for(NSString *stage in @[@"setup",@"prewarm",@"prepareToProcess:",@"calculateStats",@"process"])
                productionCompleted &= report[stage]!=nil && [report[stage] intValue]==0;
            if (productionCompleted && nativeStats.count) {
                id effectName = Call(NSClassFromString(@"CMITextureStylesProcessor"), @"effectTypeToEffectName:", @[report[@"native_effect_type"] ?: @3]);
                report[@"native_effect_name"] = effectName;
                Call(person, @"setImageStats:", @[@{effectName: nativeStats}]);
                id normalized = Call(person, @"normalizeRelativeToCropRect:", @[[NSValue valueWithRect:full]]);
                report[@"normalized_person_with_stats"] = PersonDictionary(normalized);
                report[@"unadjusted_native_normalized_person_dictionary"] = Call(normalized,@"dictionaryRepresentation",@[]);
                report[@"statistics_generation_completed"] = @YES;
            }
            // All statistics and rendering use the prepared effect before its
            // terminal finish. Fence both the native context and caller queue.
            Call(nativeContext,@"forceCommit",@[]);Call(nativeContext,@"waitForIdle",@[]);
            id<MTLCommandBuffer> fence=[queue commandBuffer];[fence commit];[fence waitUntilCompleted];
            report[@"terminal_queue_fence_status"]=@(fence.status);
            [callOrder addObject:@"finishProcessing"];report[@"stage"]=@"finishProcessing";Write(report,out);
            report[@"finishProcessing"]=Call(effect,@"finishProcessing",@[]);
            Call(nativeContext,@"forceCommit",@[]);Call(nativeContext,@"waitForIdle",@[]);
            RecordGPUExecution(report, executionStatus, gpuFailures);
            BOOL operationsCompleted=productionCompleted && [report[@"finishProcessing"] intValue]==0 && fence.status==MTLCommandBufferStatusCompleted;
            report[@"native_statistics_operations_completed"]=@(operationsCompleted);
            report[@"native_returned_empty_statistics"]=@(operationsCompleted && !nativeStats.count);
            report[@"stage"]=@"finished";
            CGImageRelease(image); CFRelease(source);
            CGColorSpaceRelease(srgb); CGColorSpaceRelease(working);
        } @catch (NSException *exception) {
            report[@"exception"] = @{@"name": exception.name, @"reason": exception.reason ?: @""};
        }
        Write(report, out);
        return report[@"exception"] ? 1 : 0;
    }
}
