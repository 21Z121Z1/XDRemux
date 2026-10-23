#pragma once
#import "native_common.h"

// Calls only the explicitly named SPI below. NSInvocation preserves struct ABI.
static id Call(id target, NSString *name, NSArray *args) {
    NSMethodSignature *signature = [target methodSignatureForSelector:NSSelectorFromString(name)];
    if (!signature || signature.numberOfArguments != args.count + 2)
        @throw [NSException exceptionWithName:@"SPIABI" reason:name userInfo:nil];
    NSInvocation *call = [NSInvocation invocationWithMethodSignature:signature];
    call.target = target; call.selector = NSSelectorFromString(name);
    for (NSUInteger i = 0; i < args.count; i++) {
        const char *type = [signature getArgumentTypeAtIndex:i + 2];
        while (*type && strchr("rnNoORV", *type)) type++;
        id value = args[i];
        if (*type == '@') { __unsafe_unretained id item = value == NSNull.null ? nil : value; [call setArgument:&item atIndex:i + 2]; }
        else if (!strcmp(type, "Q")) { uint64_t item = [value unsignedLongLongValue]; [call setArgument:&item atIndex:i + 2]; }
        else if (!strcmp(type, "q")) { int64_t item = [value longLongValue]; [call setArgument:&item atIndex:i + 2]; }
        else if (!strcmp(type, "i")) { int item = [value intValue]; [call setArgument:&item atIndex:i + 2]; }
        else if (!strcmp(type, "I")) { unsigned item = [value unsignedIntValue]; [call setArgument:&item atIndex:i + 2]; }
        else if (!strcmp(type, "B")) { BOOL item = [value boolValue]; [call setArgument:&item atIndex:i + 2]; }
        else if (!strcmp(type, "f")) { float item = [value floatValue]; [call setArgument:&item atIndex:i + 2]; }
        else if (!strcmp(type, "d")) { double item = [value doubleValue]; [call setArgument:&item atIndex:i + 2]; }
        else if (*type == '^') { void *item = [value pointerValue]; [call setArgument:&item atIndex:i + 2]; }
        else if (!strcmp(type, @encode(CGPoint))) { CGPoint item = [value pointValue]; [call setArgument:&item atIndex:i + 2]; }
        else if (!strcmp(type, @encode(CGSize))) { CGSize item = [value sizeValue]; [call setArgument:&item atIndex:i + 2]; }
        else if (!strcmp(type, @encode(CGRect))) { CGRect item = [value rectValue]; [call setArgument:&item atIndex:i + 2]; }
        else @throw [NSException exceptionWithName:@"SPIABI" reason:[name stringByAppendingFormat:@" unsupported argument %s", type] userInfo:nil];
    }
    [call invoke];
    const char *type = signature.methodReturnType;
    if (*type == '@' || *type == '#') { __unsafe_unretained id result = nil; [call getReturnValue:&result]; return result; }
    if (*type == 'v') return NSNull.null;
    if (!strcmp(type, "i")) { int result; [call getReturnValue:&result]; return @(result); }
    if (!strcmp(type, "Q")) { uint64_t result; [call getReturnValue:&result]; return @(result); }
    if (!strcmp(type, "q")) { int64_t result; [call getReturnValue:&result]; return @(result); }
    if (!strcmp(type, "B")) { BOOL result; [call getReturnValue:&result]; return @(result); }
    if (!strcmp(type, "f")) { float result; [call getReturnValue:&result]; return @(result); }
    if (!strcmp(type, "d")) { double result; [call getReturnValue:&result]; return @(result); }
    if (*type == '{') { NSMutableData *bytes = [NSMutableData dataWithLength:signature.methodReturnLength]; [call getReturnValue:bytes.mutableBytes]; return [NSValue valueWithBytes:bytes.bytes objCType:type]; }
    @throw [NSException exceptionWithName:@"SPIABI" reason:[name stringByAppendingFormat:@" unsupported return %s", type] userInfo:nil];
}

static id Make(NSString *name) { return Call([NSClassFromString(name) alloc], @"init", @[]); }

// This OS's dictionaryRepresentation rounds positive angle floats and converts
// negative ones into huge positive values. Preserve the actual signed native
// getter values when serializing our newly generated context. Apple code stays
// unchanged; callers retain the original dictionary separately as evidence.
static NSMutableDictionary *PersonDictionary(id person) {
    NSMutableDictionary *dictionary=[Call(person,@"dictionaryRepresentation",@[]) mutableCopy];
    for(NSString *key in @[@"facePitch",@"faceRoll",@"faceYaw"]) {
        NSNumber *value=Call(person,key,@[]);
        if(!isfinite(value.doubleValue))@throw [NSException exceptionWithName:@"NativeAngle" reason:key userInfo:nil];
        dictionary[key]=value;
    }
    return dictionary;
}

static id PersonFromVisionFace(VNFaceObservation *face, size_t width, size_t height) {
    CGRect box = face.boundingBox;
    CGRect roi = CGRectMake(box.origin.x * width, (1-CGRectGetMaxY(box)) * height, box.size.width * width, box.size.height * height);
    id person = Make(@"CMITextureStylesPersonInputData");
    Call(person,@"setFaceID:",@[@1]); Call(person,@"setFaceROI:",@[[NSValue valueWithRect:roi]]);
    Call(person,@"setFaceSkinROI:",@[[NSValue valueWithRect:roi]]); Call(person,@"setInstanceROI:",@[[NSValue valueWithRect:CGRectMake(0,0,width,height)]]);
    Call(person,@"setFaceLandmarkType:",@[@1]); Call(person,@"setUnitOfAngle:",@[@0]);
    for (NSString *key in @[@"pitch",@"yaw",@"roll"]) {
        NSNumber *value = [face valueForKey:key];
        if (value) Call(person,[NSString stringWithFormat:@"setFace%@%@:",[[key substringToIndex:1] uppercaseString],[key substringFromIndex:1]],@[@(value.doubleValue * 180.0/M_PI)]);
    }
    NSMutableArray *landmarks = [NSMutableArray new]; VNFaceLandmarkRegion2D *points = face.landmarks.allPoints;
    for (NSUInteger i=0;i<points.pointCount;i++) {
        CGPoint p = points.normalizedPoints[i]; p = CGPointMake((box.origin.x+p.x*box.size.width)*width,(1-box.origin.y-p.y*box.size.height)*height);
        [landmarks addObject:Call([NSClassFromString(@"CMITextureStylesFaceLandmark") alloc],@"initWithPoint:error:",@[[NSValue valueWithPoint:p],@0])];
    }
    Call(person,@"setFaceLandmarks:",@[landmarks]); return person;
}
