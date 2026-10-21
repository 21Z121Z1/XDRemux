#define _GNU_SOURCE
#include <dlfcn.h>
#include <fcntl.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

/* Audit-only one-shot scheduling point before the first owned PVT member. */
static int (*real_open)(const char *, int, ...);
static int (*real_open64)(const char *, int, ...);
static int fired;
__attribute__((constructor)) static void initialize(void) {
    real_open = dlsym(RTLD_NEXT, "open");
    real_open64 = dlsym(RTLD_NEXT, "open64");
}
static void before_open(const char *path, int flags) {
    if (!(flags & O_CREAT) || !strstr(path, "/.xdremux-live-photo-pvt-") ||
        !strstr(path, "/capture.heic") || !getenv("PVT_READY") ||
        !__sync_bool_compare_and_swap(&fired, 0, 1)) return;
    int fd = real_open(getenv("PVT_READY"), O_WRONLY | O_CREAT | O_EXCL, 0600);
    if (fd < 0 || write(fd, path, strlen(path)) != (ssize_t)strlen(path)) _exit(90);
    close(fd);
    for (int n = 0; n < 30000; n++) {
        if (access(getenv("PVT_RELEASE"), F_OK) == 0) return;
        usleep(1000);
    }
    _exit(91);
}
int open(const char *path, int flags, ...) {
    mode_t mode = 0;
    if (flags & O_CREAT) { va_list ap; va_start(ap, flags); mode = va_arg(ap, int); va_end(ap); }
    before_open(path, flags);
    return real_open(path, flags, mode);
}
int open64(const char *path, int flags, ...) {
    mode_t mode = 0;
    if (flags & O_CREAT) { va_list ap; va_start(ap, flags); mode = va_arg(ap, int); va_end(ap); }
    before_open(path, flags);
    return real_open64(path, flags, mode);
}
