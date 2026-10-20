#pragma once
#include <stdint.h>
// Apple framework primitives only. Reports and codec resources are written to
// caller-owned staging paths. The Rust runtime owns policy and publication.
int xdremux_texture_prepare(const char *input, const char *directory, uint32_t max_dimension);
int xdremux_texture_infer(const char *input, const char *directory);
int xdremux_texture_statistics(const char *input, const char *output, const char *directory,
                               uint32_t person_index, const char *effect, const char *tuning_json);
int xdremux_texture_readback(const char *input, const char *output);
