#import "spi_call.h"
#import "matching.h"
#import "include/xdremux_texture.h"

static double Cross(CGPoint a, CGPoint b, CGPoint c) { return (b.x-a.x)*(c.y-a.y)-(b.y-a.y)*(c.x-a.x); }

static NSArray<NSValue *> *Hull(NSArray<NSValue *> *points) {
    NSArray *sorted = [points sortedArrayUsingComparator:^NSComparisonResult(NSValue *a, NSValue *b) {
        CGPoint p = a.pointValue, q = b.pointValue;
        return p.x < q.x || (p.x == q.x && p.y < q.y) ? NSOrderedAscending : p.x == q.x && p.y == q.y ? NSOrderedSame : NSOrderedDescending;
    }];
    if (sorted.count < 3) return @[];
    NSMutableArray *lower = [NSMutableArray new], *upper = [NSMutableArray new];
    for (NSValue *point in sorted) { while (lower.count >= 2 && Cross([lower[lower.count-2] pointValue], [lower.lastObject pointValue], point.pointValue) <= 0) [lower removeLastObject]; [lower addObject:point]; }
    for (NSValue *point in sorted.reverseObjectEnumerator) { while (upper.count >= 2 && Cross([upper[upper.count-2] pointValue], [upper.lastObject pointValue], point.pointValue) <= 0) [upper removeLastObject]; [upper addObject:point]; }
    [lower removeLastObject]; [upper removeLastObject]; [lower addObjectsFromArray:upper]; return lower;
}

static void FillHull(NSMutableData *mask, size_t width, size_t height, NSArray<NSValue *> *points) {
    NSArray<NSValue *> *hull = Hull(points); if (hull.count < 3) return;
    uint8_t *pixels = mask.mutableBytes;
    for (size_t y = 0; y < height; y++) for (size_t x = 0; x < width; x++) {
        CGPoint point = CGPointMake(x + 0.5, y + 0.5); BOOL inside = YES;
        for (NSUInteger i = 0; i < hull.count; i++) if (Cross(hull[i].pointValue, hull[(i+1)%hull.count].pointValue, point) < 0) { inside = NO; break; }
        if (inside) pixels[y * width + x] = 255;
    }
}

static NSDictionary *LandmarkMask(NSArray<VNFaceObservation *> *faces, NSArray<NSString *> *regions, size_t width, size_t height) {
    NSMutableData *data = [NSMutableData dataWithLength:width * height];
    for (VNFaceObservation *face in faces) for (NSString *region in regions) {
        VNFaceLandmarkRegion2D *landmarks = [face.landmarks valueForKey:region]; NSMutableArray *points = [NSMutableArray new];
        for (NSUInteger i = 0; i < landmarks.pointCount; i++) {
            CGPoint p = landmarks.normalizedPoints[i]; CGRect b = face.boundingBox;
            [points addObject:[NSValue valueWithPoint:CGPointMake((b.origin.x + p.x * b.size.width) * width, (1 - b.origin.y - p.y * b.size.height) * height)]];
        }
        FillHull(data, width, height, points);
    }
    return @{@"data": data, @"width": @(width), @"height": @(height)};
}

static NSDictionary *Mask(CVPixelBufferRef buffer, size_t targetWidth, size_t targetHeight) {
    size_t w = CVPixelBufferGetWidth(buffer), h = CVPixelBufferGetHeight(buffer), stride = CVPixelBufferGetBytesPerRow(buffer);
    OSType format = CVPixelBufferGetPixelFormatType(buffer);
    if (format != kCVPixelFormatType_OneComponent8 && format != kCVPixelFormatType_OneComponent32Float)
        @throw [NSException exceptionWithName:@"MaskFormat" reason:@(format).description userInfo:nil];
    NSMutableData *data = [NSMutableData dataWithLength:targetWidth * targetHeight];
    CVPixelBufferLockBaseAddress(buffer, kCVPixelBufferLock_ReadOnly);
    const uint8_t *base = CVPixelBufferGetBaseAddress(buffer);
    for (size_t ty = 0; ty < targetHeight; ty++) for (size_t tx = 0; tx < targetWidth; tx++) {
        size_t y = MIN(h-1, (ty * h + h/2) / targetHeight), x = MIN(w-1, (tx * w + w/2) / targetWidth);
        float v = format == kCVPixelFormatType_OneComponent8 ? base[y * stride + x] / 255.0f : ((const float *)(base + y * stride))[x];
        ((uint8_t *)data.mutableBytes)[ty * targetWidth + tx] = (uint8_t)lrintf(fminf(1, fmaxf(0, v)) * 255);
    }
    CVPixelBufferUnlockBaseAddress(buffer, kCVPixelBufferLock_ReadOnly);
    return @{@"data": data, @"width": @(targetWidth), @"height": @(targetHeight)};
}

int xdremux_texture_infer(const char *inputPath, const char *directoryPath) {
    @autoreleasepool {
        NSString *input = @(inputPath), *directory = @(directoryPath);
        [[NSFileManager defaultManager] createDirectoryAtPath:directory withIntermediateDirectories:YES attributes:nil error:NULL];
        NSString *reportPath = [directory stringByAppendingPathComponent:@"carrier-production.json"];
        NSMutableDictionary *report = [@{@"schema": @"texture-derived-carrier-v1", @"input_sha256": SHA([NSData dataWithContentsOfFile:input]), @"original_camera_provenance": @NO, @"apple_implementation_patched": @NO, @"stage": @"begin"} mutableCopy];
        Write(report, reportPath);
        @try {
            dlopen("/System/Library/PrivateFrameworks/CMImaging.framework/CMImaging", RTLD_NOW | RTLD_LOCAL);
            CGImageSourceRef source = CGImageSourceCreateWithURL((__bridge CFURLRef)[NSURL fileURLWithPath:input], NULL);
            CGImageRef image = CGImageSourceCreateThumbnailAtIndex(source, 0, (__bridge CFDictionaryRef)@{(__bridge id)kCGImageSourceCreateThumbnailFromImageAlways: @YES, (__bridge id)kCGImageSourceCreateThumbnailWithTransform: @YES, (__bridge id)kCGImageSourceThumbnailMaxPixelSize: @4096});
            if (!image) @throw [NSException exceptionWithName:@"Input" reason:@"upright decode failed" userInfo:nil];
            size_t w = CGImageGetWidth(image), h = CGImageGetHeight(image);
            report[@"width"] = @(w); report[@"height"] = @(h);
            VNImageRequestHandler *handler = [[VNImageRequestHandler alloc] initWithCGImage:image options:@{}];
            VNRequest *attributes = [NSClassFromString(@"VNGenerateHumanAttributesSegmentationRequest") new];
            VNRequest *glasses = [NSClassFromString(@"VNGenerateGlassesSegmentationRequest") new];
            VNGeneratePersonInstanceMaskRequest *instances = [VNGeneratePersonInstanceMaskRequest new];
            VNDetectFaceLandmarksRequest *faces = [VNDetectFaceLandmarksRequest new];
            VNRequest *faceSegments = [NSClassFromString(@"VNGenerateFaceSegmentsRequest") new];
            NSError *error = nil;
            if (![handler performRequests:@[attributes, glasses, instances, faceSegments] error:&error])
                @throw [NSException exceptionWithName:@"Inference" reason:error.description userInfo:nil];
            faces.inputFaceObservations = (NSArray<VNFaceObservation *> *)faceSegments.results;
            if (![handler performRequests:@[faces] error:&error]) @throw [NSException exceptionWithName:@"Inference" reason:error.description userInfo:nil];
            NSMutableDictionary *masks = [NSMutableDictionary new];
            NSMutableDictionary *individualMasks = [NSMutableDictionary new];
            report[@"instance_count"] = @0;
            NSDictionary *features = @{@"human_attribute_skin": @"semanticskinmattev2", @"human_attribute_hair": @"semantichairmatte", @"human_attribute_teeth": @"semanticteethmattev2"};
            for (VNPixelBufferObservation *observation in attributes.results) {
                NSString *role = features[[observation valueForKey:@"featureName"]];
                if (role) masks[role] = Mask(observation.pixelBuffer,w,h);
            }
            if (glasses.results.count == 1) masks[@"semanticglassesmattev2"] = Mask(((VNPixelBufferObservation *)glasses.results[0]).pixelBuffer,w,h);
            if (instances.results.count == 1) {
                VNInstanceMaskObservation *observation = instances.results[0];
                report[@"instance_count"] = @(observation.allInstances.count);
                if (observation.allInstances.count) {
                    CVPixelBufferRef mask = [observation generateScaledMaskForImageForInstances:observation.allInstances fromRequestHandler:handler error:&error];
                    if (!mask) @throw [NSException exceptionWithName:@"InstanceMask" reason:error.description userInfo:nil];
                    masks[@"semanticpersonmatte"] = Mask(mask,w,h); CVPixelBufferRelease(mask);
                    [observation.allInstances enumerateIndexesUsingBlock:^(NSUInteger index, BOOL *stop) {
                        (void)stop; NSError *instanceError = nil;
                        CVPixelBufferRef one = [observation generateScaledMaskForImageForInstances:[NSIndexSet indexSetWithIndex:index] fromRequestHandler:handler error:&instanceError];
                        if (!one) @throw [NSException exceptionWithName:@"InstanceMask" reason:instanceError.description userInfo:nil];
                        individualMasks[[NSString stringWithFormat:@"instance-%lu",(unsigned long)index]] = Mask(one,w,h);
                        CVPixelBufferRelease(one);
                    }];
                }
            }
            report[@"face_geometry"] = FaceFacts(faces.results);
            // Match each face to one distinct native instance using probability
            // mass in its measured bounds. Never reuse the aggregate as an instance.
            NSMutableArray *edges = [NSMutableArray new];
            for (NSUInteger f=0; f<faces.results.count; f++) {
                CGRect b = faces.results[f].boundingBox;
                for (NSString *role in individualMasks) {
                    const uint8_t *pixels = [individualMasks[role][@"data"] bytes];
                    double mass=0; NSUInteger count=0;
                    size_t x0=MAX(0,floor(b.origin.x*w)),x1=MIN(w,ceil(CGRectGetMaxX(b)*w));
                    size_t y0=MAX(0,floor((1-CGRectGetMaxY(b))*h)),y1=MIN(h,ceil((1-b.origin.y)*h));
                    for(size_t y=y0;y<y1;y++) for(size_t x=x0;x<x1;x++){mass+=pixels[y*w+x]/255.0;count++;}
                    [edges addObject:@{@"face_index":@(f),@"instance_role":role,@"face_probability_overlap":@(count?mass/count:0)}];
                }
            }
            NSDictionary *matched=SelectFaceInstances(edges,[[individualMasks allKeys] sortedArrayUsingSelector:@selector(compare:)],faces.results.count);
            report[@"matching_algorithm"]=@"maximum-total-overlap bipartite assignment with explicit unmatched columns";
            NSMutableArray *people=[NSMutableArray new],*unmatched=[NSMutableArray new],*instanceRows=[NSMutableArray new];
            for(NSUInteger f=0;f<faces.results.count;f++) {
                NSDictionary *edge=matched[@(f)];
                if(!edge){[unmatched addObject:@{@"face_index":@(f),@"reason":@"no distinct native person instance overlaps the detected face"}];continue;}
                id nativePerson=PersonFromVisionFace(faces.results[f],w,h);
                Call(nativePerson,@"setFaceID:",@[@(people.count+1)]);
                NSMutableDictionary *person=PersonDictionary(nativePerson);
                [person removeObjectForKey:@"faceROIAndLandmarksROIRelativeScalingROI"];
                NSString *key=[NSString stringWithFormat:@"DerivedVisionInstance%lu",(unsigned long)people.count+1];
                NSString *role=edge[@"instance_role"]; NSDictionary *mask=individualMasks[role];
                [people addObject:@{@"face_index":@(f),@"instance_role":role,@"instance_reference_key":key,@"face_probability_overlap":edge[@"face_probability_overlap"],@"native_person_without_stats":person,@"unadjusted_native_dictionary":Call(nativePerson,@"dictionaryRepresentation",@[])}];
                [instanceRows addObject:@{@"role":role,@"uri":@"tag:apple.com,2026:photo:aux:semanticpersoninstances",@"reference_key":key,@"file":[role stringByAppendingString:@".l8"],@"width":@(w),@"height":@(h),@"sha256":SHA(mask[@"data"]),@"size":@(w*h)}];
            }
            report[@"people"] = people; report[@"unmatched_faces"] = unmatched; report[@"matching_candidates"] = edges;
            report[@"instance_resources"] = instanceRows;
            if(people.count==1)report[@"native_person_without_stats"]=people[0][@"native_person_without_stats"];
            // Empty observations mean an empty derived editing context, not a
            // template person. Keep every scalar role explicitly dimensioned.
            NSMutableSet *zeroRoles=[NSMutableSet new];
            for(NSString *role in @[@"semanticskinmattev2",@"semantichairmatte",@"semanticteethmattev2",@"semanticglassesmattev2",@"semanticpersonmatte",@"semanticfaceskinmatte"])
                if(!masks[role]){masks[role]=@{@"data":[NSMutableData dataWithLength:w*h],@"width":@(w),@"height":@(h)};[zeroRoles addObject:role];}
            report[@"mask_resampling"]=@"native probability buffers are sampled at working-pixel centers with nearest-neighbor interpolation, clamped and quantized to L8 before producer/writer sharing";
            // Declared derived resource: real inferred skin intersected with the
            // measured face bounds; this is not presented as FSINC inference.
            NSDictionary *skin = masks[@"semanticskinmattev2"];
            if (skin && faces.results.count) {
                NSMutableData *data = [skin[@"data"] mutableCopy]; size_t mw = [skin[@"width"] unsignedLongValue], mh = [skin[@"height"] unsignedLongValue];
                for (size_t y = 0; y < mh; y++) for (size_t x = 0; x < mw; x++) {
                    BOOL inside = NO;
                    for (VNFaceObservation *face in faces.results) inside |= CGRectContainsPoint(face.boundingBox, CGPointMake((x + 0.5) / mw, 1 - (y + 0.5) / mh));
                    if (!inside) ((uint8_t *)data.mutableBytes)[y * mw + x] = 0;
                }
                masks[@"semanticfaceskinmatte"] = @{@"data": data, @"width": @(mw), @"height": @(mh)};
                [zeroRoles removeObject:@"semanticfaceskinmatte"];
                report[@"face_skin_source"] = @"inferred skin intersected with measured face bounds";
            }
            NSMutableDictionary *resourceSources = [NSMutableDictionary new];
            for (NSString *role in masks) resourceSources[role] = [role isEqual:@"semanticfaceskinmatte"] ? @"derived skin/face intersection" : @"native Vision segmentation";
            for(NSString *role in zeroRoles)resourceSources[role]=@"explicit empty context after no native observation; no detected-absence claim";
            report[@"zero_filled_roles_without_observation"]=[[zeroRoles allObjects] sortedArrayUsingSelector:@selector(compare:)];
            if (skin) {
                size_t mw = [skin[@"width"] unsignedLongValue], mh = [skin[@"height"] unsignedLongValue];
                NSData *faceData = masks[@"semanticfaceskinmatte"][@"data"];
                NSMutableData *other = [skin[@"data"] mutableCopy]; const uint8_t *facePixels = faceData.bytes;
                if (faceData.length) for (size_t i = 0; i < other.length; i++) ((uint8_t *)other.mutableBytes)[i] = MAX(0, ((uint8_t *)other.mutableBytes)[i] - facePixels[i]);
                masks[@"semanticnonfaceskinmatte"] = @{@"data": other, @"width": @(mw), @"height": @(mh)};
                resourceSources[@"semanticnonfaceskinmatte"] = @"inferred skin minus derived face skin";
                NSDictionary *regions = @{@"semanticnosematte": @[@"nose"], @"semanticlipsmatte": @[@"outerLips"], @"semanticeyebrowsmatte": @[@"leftEyebrow", @"rightEyebrow"]};
                for (NSString *role in regions) {
                    masks[role] = LandmarkMask(faces.results, regions[role], mw, mh);
                    resourceSources[role] = @"convex regions derived from the current image's native Vision face landmarks; not FSINC segmentation";
                }
                VNDetectHumanHandPoseRequest *hands = [VNDetectHumanHandPoseRequest new]; hands.maximumHandCount = 2;
                NSError *handError = nil; BOOL handSuccess = [handler performRequests:@[hands] error:&handError];
                report[@"hand_inference"] = @{@"success": @(handSuccess), @"count": @(hands.results.count), @"error": handError.description ?: @"", @"minimum_joint_confidence": @0.3};
                NSMutableData *handData = [NSMutableData dataWithLength:mw * mh];
                if (handSuccess) for (VNHumanHandPoseObservation *hand in hands.results) {
                    NSDictionary *joints = [hand recognizedPointsForGroupKey:VNHumanHandPoseObservationJointsGroupNameAll error:&handError];
                    NSMutableArray *points = [NSMutableArray new];
                    for (VNRecognizedPoint *point in joints.allValues) if (point.confidence >= 0.3) [points addObject:[NSValue valueWithPoint:CGPointMake(point.x * mw, (1 - point.y) * mh)]];
                    FillHull(handData, mw, mh, points);
                }
                masks[@"semantichandsmatte"] = @{@"data": handData, @"width": @(mw), @"height": @(mh)};
                resourceSources[@"semantichandsmatte"] = handSuccess ? @"current image's native hand-pose convex regions; empty when no hands detected" : @"optional hand protection disabled after an unavailable native request; no inference claimed";
                for (NSString *role in @[@"semanticearsmatte", @"semantictattoomatte"]) {
                    masks[role] = @{@"data": [NSMutableData dataWithLength:mw * mh], @"width": @(mw), @"height": @(mh)};
                    resourceSources[role] = @"optional per-region protection explicitly disabled in this derived edit context; no absence detection claimed";
                }
            }
            report[@"resource_sources"] = resourceSources;
            NSMutableArray *resourceRows = [NSMutableArray new];
            for (NSString *name in [[masks allKeys] sortedArrayUsingSelector:@selector(compare:)]) {
                NSDictionary *mask = masks[name]; NSData *bytes = mask[@"data"];
                NSString *file = [name stringByAppendingString:@".l8"];
                [bytes writeToFile:[directory stringByAppendingPathComponent:file] atomically:YES];
                NSString *uri = [name isEqual:@"semantichairmatte"] ? @"urn:com:apple:photo:2019:aux:semantichairmatte" : [@"tag:apple.com,2026:photo:aux:" stringByAppendingString:name];
                [resourceRows addObject:@{@"role": name, @"uri": uri, @"file": file, @"width": mask[@"width"], @"height": mask[@"height"], @"sha256": SHA(bytes), @"size": @(bytes.length)}];
            }
            report[@"resources"] = resourceRows;
            NSMutableArray *codecRows = [NSMutableArray new];
            NSMutableArray *codecResources=[resourceRows mutableCopy];[codecResources addObjectsFromArray:instanceRows];
            for(NSDictionary *row in instanceRows)[individualMasks[row[@"role"]][@"data"] writeToFile:[directory stringByAppendingPathComponent:row[@"file"]] atomically:YES];
            for (NSDictionary *row in codecResources) {
                // Public ImageIO only writes recognized auxiliary identifiers.
                // Encode each actual mask through its supported L8 carrier; the
                // assembler later assigns that distinct payload its final role.
                NSString *codecFile = [@"codec-" stringByAppendingFormat:@"%@.heic", row[@"role"]];
                NSString *path = [directory stringByAppendingPathComponent:codecFile];
                CGImageDestinationRef destination = CGImageDestinationCreateWithURL((__bridge CFURLRef)[NSURL fileURLWithPath:path], CFSTR("public.heic"), 1, NULL);
                CGImageDestinationAddImage(destination, image, (__bridge CFDictionaryRef)@{(__bridge id)kCGImageDestinationLossyCompressionQuality: @0.95});
                CGMutableImageMetadataRef metadata = CGImageMetadataCreateMutable(); CFErrorRef metadataError = NULL;
                CGImageMetadataRegisterNamespaceForPrefix(metadata, CFSTR("http://ns.apple.com/semanticSegmentationMatte/1.0/"), CFSTR("semanticSegmentationMatte"), &metadataError);
                CGImageMetadataSetValueWithPath(metadata, NULL, CFSTR("semanticSegmentationMatte:SemanticSegmentationMatteVersion"), CFSTR("65536"));
                NSDictionary *mask = masks[row[@"role"]] ?: individualMasks[row[@"role"]];
                NSDictionary *aux = @{(__bridge id)kCGImageAuxiliaryDataInfoData: mask[@"data"], (__bridge id)kCGImageAuxiliaryDataInfoDataDescription: @{(__bridge id)kCGImagePropertyWidth: row[@"width"], (__bridge id)kCGImagePropertyHeight: row[@"height"], (__bridge id)kCGImagePropertyBytesPerRow: row[@"width"], (__bridge id)kCGImagePropertyPixelFormat: @(kCVPixelFormatType_OneComponent8)}, (__bridge id)kCGImageAuxiliaryDataInfoMetadata: (__bridge id)metadata};
                CGImageDestinationAddAuxiliaryDataInfo(destination, kCGImageAuxiliaryDataTypeSemanticSegmentationSkinMatte, (__bridge CFDictionaryRef)aux);
                BOOL finalized = CGImageDestinationFinalize(destination); CFRelease(destination); CFRelease(metadata);
                CGImageSourceRef encoded = CGImageSourceCreateWithURL((__bridge CFURLRef)[NSURL fileURLWithPath:path], NULL);
                CFDictionaryRef read = encoded ? CGImageSourceCopyAuxiliaryDataInfoAtIndex(encoded, 0, kCGImageAuxiliaryDataTypeSemanticSegmentationSkinMatte) : NULL;
                [codecRows addObject:@{@"role": row[@"role"], @"file": codecFile, @"finalized": @(finalized), @"native_auxiliary_readback": @(read != NULL), @"readback": read ? JSONCompatible((__bridge id)read) : @{}}];
                if (read) CFRelease(read); if (encoded) CFRelease(encoded);
            }
            report[@"native_mask_codec_controls"] = codecRows;
            CGImageRelease(image); CFRelease(source);
            report[@"stage"] = @"finished";
        } @catch (NSException *exception) { report[@"exception"] = @{@"name": exception.name, @"reason": exception.reason ?: @""}; }
        Write(report, reportPath); return report[@"exception"] ? 1 : 0;
    }
}
