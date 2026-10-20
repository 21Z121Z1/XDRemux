#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <fcntl.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

/* Audit-only scheduling/fault injection into the real CLI; no product changes. */
static int (*next_open)(const char *, int, ...);
static int (*next_open64)(const char *, int, ...);
static int (*next_link)(const char *, const char *);
static int (*next_linkat)(int, const char *, int, const char *, int);
static int (*next_rename)(const char *, const char *);
static int fired;

__attribute__((constructor)) static void initialize(void) {
    next_open = dlsym(RTLD_NEXT, "open");
    next_open64 = dlsym(RTLD_NEXT, "open64");
    next_link = dlsym(RTLD_NEXT, "link");
    next_linkat = dlsym(RTLD_NEXT, "linkat");
    next_rename = dlsym(RTLD_NEXT, "rename");
}

static int ends(const char *path, const char *suffix) {
    size_t a = strlen(path), b = strlen(suffix);
    return a >= b && strcmp(path + a - b, suffix) == 0;
}

static void put(const char *path, const char *bytes) {
    int fd = next_open(path, O_WRONLY | O_CREAT | O_EXCL, 0600);
    if (fd < 0) { perror("audit put"); _exit(90); }
    if (write(fd, bytes, strlen(bytes)) != (ssize_t)strlen(bytes)) _exit(91);
    fsync(fd);
    close(fd);
}

static void copy(const char *source, const char *destination) {
    int input = next_open(source, O_RDONLY);
    int output = next_open(destination, O_WRONLY | O_CREAT | O_EXCL, 0600);
    if (input < 0 || output < 0) _exit(92);
    char buffer[65536];
    ssize_t count;
    while ((count = read(input, buffer, sizeof(buffer))) > 0) {
        ssize_t written = 0;
        while (written < count) {
            ssize_t n = write(output, buffer + written, count - written);
            if (n <= 0) _exit(93);
            written += n;
        }
    }
    if (count < 0) _exit(94);
    fsync(output);
    close(input);
    close(output);
}

static void pause_point(const char *path) {
    put(getenv("XDREMUX_AUDIT_READY"), path);
    fprintf(stderr, "audit: paused before filesystem operation: %s\n", path);
    for (int n = 0; n < 30000; n++) {
        if (access(getenv("XDREMUX_AUDIT_RELEASE"), F_OK) == 0) return;
        usleep(1000);
    }
    _exit(95);
}

static void before_open(const char *path) {
    const char *mode = getenv("XDREMUX_AUDIT_MODE");
    if (!mode || !strstr(path, "/.xdremux-live-photo-")) return;
    int image = ends(path, "/capture.heic");
    int movie = ends(path, "/capture.mov");
    int eligible = (image && (strcmp(mode, "late-competitor") == 0 ||
                             strcmp(mode, "late-image-only") == 0 ||
                             strcmp(mode, "pause-before-image") == 0)) ||
                   (movie && strcmp(mode, "pause-before-movie") == 0);
    if (!eligible || !__sync_bool_compare_and_swap(&fired, 0, 1)) return;
    if (strcmp(mode, "late-competitor") == 0 || strcmp(mode, "late-image-only") == 0) {
        put(getenv("XDREMUX_AUDIT_IMAGE"), "COMPETITOR STILL\n");
        if (strcmp(mode, "late-competitor") == 0)
            put(getenv("XDREMUX_AUDIT_MOVIE"), "COMPETITOR MOVIE\n");
        fprintf(stderr, "audit: competitor entries installed after preflight\n");
        return;
    }
    pause_point(path);
}

int open(const char *path, int flags, ...) {
    mode_t mode = 0;
    if (flags & O_CREAT) {
        va_list ap; va_start(ap, flags); mode = va_arg(ap, int); va_end(ap);
    }
    before_open(path);
    return next_open(path, flags, mode);
}

int open64(const char *path, int flags, ...) {
    mode_t mode = 0;
    if (flags & O_CREAT) {
        va_list ap; va_start(ap, flags); mode = va_arg(ap, int); va_end(ap);
    }
    before_open(path);
    return next_open64(path, flags, mode);
}

static int fail_link(const char *source, const char *destination) {
    const char *mode = getenv("XDREMUX_AUDIT_MODE");
    const char *image = getenv("XDREMUX_AUDIT_IMAGE");
    const char *movie = getenv("XDREMUX_AUDIT_MOVIE");
    if (!mode || !image || !movie) return 0;
    if (strcmp(mode, "unsupported-movie-link") == 0 && strcmp(destination, movie) == 0 &&
        __sync_bool_compare_and_swap(&fired, 0, 1)) {
        fprintf(stderr, "audit: injected EOPNOTSUPP at first hard link\n");
        errno = EOPNOTSUPP;
        return 1;
    }
    if (strcmp(destination, image) != 0 || !strstr(source, "/.xdremux-live-photo-")) return 0;
    if (strcmp(mode, "pause-fail-still-link") == 0 &&
        __sync_bool_compare_and_swap(&fired, 0, 1)) {
        pause_point(source);
        errno = EIO;
        return 1;
    }
    if (strcmp(mode, "competitor-before-failed-still-link") == 0 &&
        __sync_bool_compare_and_swap(&fired, 0, 1)) {
        char image_stage[4096], movie_stage[4096];
        snprintf(image_stage, sizeof(image_stage), "%s.audit-peer", image);
        snprintf(movie_stage, sizeof(movie_stage), "%s.audit-peer", movie);
        copy(getenv("XDREMUX_AUDIT_SEED_IMAGE"), image_stage);
        copy(getenv("XDREMUX_AUDIT_SEED_MOVIE"), movie_stage);
        if (next_rename(image_stage, image) || next_rename(movie_stage, movie)) _exit(96);
        fprintf(stderr, "audit: non-cooperating writer replaced finals with valid pair; injected EIO at still link\n");
        errno = EIO;
        return 1;
    }
    return 0;
}

int link(const char *source, const char *destination) {
    if (fail_link(source, destination)) return -1;
    return next_link(source, destination);
}

int linkat(int oldfd, const char *source, int newfd, const char *destination, int flags) {
    if (fail_link(source, destination)) return -1;
    return next_linkat(oldfd, source, newfd, destination, flags);
}
