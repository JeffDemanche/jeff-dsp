#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct PrismDsp PrismDsp;

/** Packed corrector parameters (matches Vamp LL / crates/pitch CorrectorParams). */
typedef struct PrismDspParams {
    uint8_t key_semitone;               /* 0–11 */
    uint8_t scale;                      /* 0 chromatic, 1 major, 2 natural minor */
    float reference_hz;                 /* 400–480 */
    float retune_speed_ms;              /* 0–500 */
    float correction_amount;            /* 0–1 */
    float correction_range_semitones;   /* 0–12 */
    float vibrato_preserve;             /* 0–1 */
    float formant_follow;               /* 0–1 */
    float quantize_dead_zone_cents;     /* 0–50 */
    float mix;                          /* 0–1 */
    float min_f0_hz;                    /* 80–200 */
} PrismDspParams;

PrismDsp* prism_dsp_create(void);
void prism_dsp_destroy(PrismDsp* dsp);

void prism_dsp_prepare(PrismDsp* dsp, double sample_rate, uint32_t max_block_size);
void prism_dsp_reset(PrismDsp* dsp);

/** Process planar float buffers in-place (channels x samples). Expects 1–2 channels. */
void prism_dsp_process(PrismDsp* dsp,
                       float* const* channels,
                       uint32_t num_channels,
                       uint32_t num_samples);

void prism_dsp_set_params(PrismDsp* dsp, const PrismDspParams* params);

/** Algorithmic latency in samples (≈ one period at minF0Hz). */
uint32_t prism_dsp_reported_latency_samples(const PrismDsp* dsp);

#ifdef __cplusplus
}
#endif
