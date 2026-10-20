#import "spi_call.h"
#import "include/xdremux_texture.h"

int xdremux_texture_readback(const char *inputPath, const char *outputPath) {
    @autoreleasepool {
        NSMutableDictionary *report = [@{@"schema": @"texture-native-admission-v1", @"input_sha256": SHA([NSData dataWithContentsOfFile:@(inputPath)]), @"photos_UI_tested": @NO, @"capabilities_injected": @NO} mutableCopy];
        @try {
            CGImageSourceRef imageSource = CGImageSourceCreateWithURL((__bridge CFURLRef)[NSURL fileURLWithPath:@(inputPath)], NULL);
            CGImageRef decoded = imageSource ? CGImageSourceCreateImageAtIndex(imageSource, 0, NULL) : NULL;
            report[@"ImageIO_decoded"] = @(decoded != NULL);
            if (decoded) { report[@"decoded_width"] = @(CGImageGetWidth(decoded)); report[@"decoded_height"] = @(CGImageGetHeight(decoded)); CGImageRelease(decoded); }
            if (imageSource) {
                CFDictionaryRef imageProperties = CGImageSourceCopyPropertiesAtIndex(imageSource, 0, NULL);
                report[@"ImageIO_properties"] = imageProperties ? JSONCompatible((__bridge id)imageProperties) : @{};
                if (imageProperties) CFRelease(imageProperties);
                NSMutableDictionary *auxiliary = [NSMutableDictionary new];
                for (NSString *role in @[@"semanticpersonmatte", @"semanticskinmattev2", @"semanticfaceskinmatte", @"semanticteethmattev2", @"semanticglassesmattev2", @"semantichairmatte"]) {
                    CFDictionaryRef aux = CGImageSourceCopyAuxiliaryDataInfoAtIndex(imageSource, 0, (__bridge CFStringRef)[@"tag:apple.com,2026:photo:aux:" stringByAppendingString:role]);
                    auxiliary[role] = @{@"available": @(aux != NULL), @"facts": aux ? JSONCompatible((__bridge id)aux) : @{}};
                    if (aux) CFRelease(aux);
                }
                report[@"ImageIO_auxiliary"] = auxiliary; CFRelease(imageSource);
            }
            dlopen("/System/Library/PrivateFrameworks/NeutrinoCore.framework/NeutrinoCore", RTLD_NOW | RTLD_LOCAL);
            dlopen("/System/Library/PrivateFrameworks/PhotoImaging.framework/PhotoImaging", RTLD_NOW | RTLD_LOCAL);
            id source = Call(NSClassFromString(@"NUSourceDefinition"), @"imageSourceDefinitionWithURL:options:", @[[NSURL fileURLWithPath:@(inputPath)], @{}]);
            NSError *error = nil;
            id asset = Call(NSClassFromString(@"NUAssetLoader"), @"loadAssetWithSourceDefinition:options:error:", @[source, @{}, [NSValue valueWithPointer:&error]]);
            report[@"asset_loaded"] = @(asset != nil); report[@"asset_error"] = error.description ?: @"";
            if (asset) {
                report[@"asset_class"] = NSStringFromClass([asset class]);
                if ([asset respondsToSelector:NSSelectorFromString(@"loadWithOptions:error:")]) report[@"load"] = Call(asset, @"loadWithOptions:error:", @[@{}, [NSValue valueWithPointer:&error]]);
                report[@"capabilities"] = Call(asset, @"capabilities", @[]);
                report[@"PhotographicStyleV2"] = Call(asset, @"hasCapability:", @[@"PhotographicStyleV2"]);
                report[@"PhotographicStyle"] = Call(asset, @"hasCapability:", @[@"PhotographicStyle"]);
                if ([asset respondsToSelector:NSSelectorFromString(@"imageProperties")]) {
                    id properties = Call(asset, @"imageProperties", @[]);
                    report[@"native_image_properties"] = [properties description] ?: @"nil";
                    report[@"native_image_metadata"] = JSONCompatible(Call(properties, @"metadata", @[]));
                    id texture = Call(properties, @"textureStyleProperties", @[]);
                    id semantic = Call(properties, @"semanticStyleProperties", @[]);
                    report[@"texture_properties_present"] = @(texture != nil);
                    report[@"semantic_properties_present"] = @(semantic != nil);
                    if (texture) {
                        report[@"texture_person_count"] = Call(texture, @"numberOfPersons", @[]);
                        NSMutableArray *people = [NSMutableArray new],*unadjusted=[NSMutableArray new];
                        for (id person in Call(NSClassFromString(@"PITextureStyleProcessorKernel"), @"personInputDataFromStillProperties:", @[texture])) {
                            [people addObject:PersonDictionary(person)];[unadjusted addObject:Call(person,@"dictionaryRepresentation",@[])];
                        }
                        report[@"native_person_data_readback"] = people;
                        report[@"unadjusted_native_person_dictionaries"] = unadjusted;
                    }
                }
                id media = Call(asset, @"media", @[]);
                report[@"media_description"] = [media description] ?: @"";
                for (NSString *key in @[@"imageProperties", @"properties", @"textureStyleProperties", @"metadata", @"rawProperties"])
                    if ([media respondsToSelector:NSSelectorFromString(key)]) {
                        id value = Call(media, key, @[]); report[key] = JSONCompatible(value);
                        if ([key isEqual:@"metadata"] && [value respondsToSelector:NSSelectorFromString(@"cgImageProperties")]) report[@"native_cgImageProperties"] = JSONCompatible(Call(value, @"cgImageProperties", @[]));
                        if ([key isEqual:@"properties"] || [key isEqual:@"imageProperties"]) {
                            if ([value respondsToSelector:NSSelectorFromString(@"textureStyleProperties")]) {
                                id texture = Call(value, @"textureStyleProperties", @[]);
                                report[@"texture_properties_present"] = @(texture != nil);
                                if (texture) report[@"texture_person_count"] = Call(texture, @"numberOfPersons", @[]);
                            }
                        }
                    }
            }
        } @catch (NSException *exception) { report[@"exception"] = @{@"name": exception.name, @"reason": exception.reason ?: @""}; }
        Write(report, @(outputPath)); return report[@"exception"] ? 1 : 0;
    }
}
