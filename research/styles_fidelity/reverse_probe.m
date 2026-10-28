#import <Foundation/Foundation.h>
#import <objc/runtime.h>
#import <dlfcn.h>
#import <stdio.h>
#import <string.h>
#import "XDStylesNative.h"

// The current product primitive supplies all reverse operations. This caller
// contains the native abort policy solely to expose a failing precondition.
static void AssertionFailure(id self, SEL selector, id assertion) {
    NSString *reason = [assertion description] ?: @"Native assertion";
    fprintf(stderr, "NATIVE ASSERTION: %s\n", reason.UTF8String);
    fflush(stderr);
    @throw [NSException exceptionWithName:@"XDReverseNativeAssertion" reason:reason userInfo:nil];
}

int main(void) {
    @autoreleasepool {
        NSError *error = nil;
        NSData *input = [[NSFileHandle fileHandleWithStandardInput] readDataToEndOfFile];
        NSDictionary *request = [NSJSONSerialization JSONObjectWithData:input options:0 error:&error];
        if (![request isKindOfClass:NSDictionary.class] ||
            ![request[@"operation"] isEqual:@"styles-decode-reverse"] ||
            [request[@"schema_version"] intValue] != 2) return 2;
        dlopen("/System/Library/PrivateFrameworks/NeutrinoCore.framework/NeutrinoCore", RTLD_NOW | RTLD_LOCAL);
        Method policy = class_getInstanceMethod(NSClassFromString(@"NUAssertionPolicyAbort"),
                                               NSSelectorFromString(@"notifyAssertion:"));
        BOOL interposed = NO;
        if (policy) {
            NSMethodSignature *signature = [NSMethodSignature signatureWithObjCTypes:method_getTypeEncoding(policy)];
            if (signature.numberOfArguments == 3 && !strcmp(signature.methodReturnType, "v") &&
                !strcmp([signature getArgumentTypeAtIndex:2], "@")) {
                method_setImplementation(policy, (IMP)AssertionFailure);
                interposed = YES;
            }
        }
        NSDictionary *result = XDStylesPerform(request, &error);
        if (!result) {
            fprintf(stderr, "Native reverse failed (assertion containment=%d): %s\n",
                    interposed, (error.localizedDescription ?: @"No native result").UTF8String);
            return 2;
        }
        NSMutableDictionary *facts = [result mutableCopy];
        facts[@"diagnostic_assertion_policy_interposed"] = @(interposed);
        NSData *encoded = [NSJSONSerialization dataWithJSONObject:facts options:0 error:&error];
        if (!encoded) {
            fprintf(stderr, "Native reverse report encoding failed: %s\n", error.localizedDescription.UTF8String);
            return 2;
        }
        [[NSFileHandle fileHandleWithStandardOutput] writeData:encoded];
        return 0;
    }
}
