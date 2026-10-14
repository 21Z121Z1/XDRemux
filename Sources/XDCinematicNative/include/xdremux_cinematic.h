#ifndef XDREMUX_CINEMATIC_H
#define XDREMUX_CINEMATIC_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

int xdremux_cinematic_generate(const char *input_path,
                               const char *output_path,
                               float focal_length_35mm,
                               float aperture,
                               uint64_t *processed_frames,
                               char *error_buffer,
                               size_t error_buffer_size);

#ifdef __cplusplus
}
#endif

#endif
