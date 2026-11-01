#pragma once

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct PrismDsp PrismDsp;

PrismDsp* prism_dsp_create(void);
void prism_dsp_destroy(PrismDsp* dsp);

void prism_dsp_prepare(PrismDsp* dsp, double sample_rate, uint32_t max_block_size);
void prism_dsp_reset(PrismDsp* dsp);

/** Process interleaved planar float buffers in-place (channels x samples). */
void prism_dsp_process(PrismDsp* dsp,
                       float* const* channels,
                       uint32_t num_channels,
                       uint32_t num_samples);

void prism_dsp_set_gain(PrismDsp* dsp, float gain_linear);

#ifdef __cplusplus
} // extern "C"
#endif
