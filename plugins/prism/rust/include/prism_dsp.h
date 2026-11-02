#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct PrismDsp PrismDsp;

enum { PRISM_DSP_MAX_VOICES = 8 };

/** Packed prismizer parameters. */
typedef struct PrismDspParams {
    uint8_t key_semitone;               /* 0–11 */
    uint8_t scale;                      /* 0 chromatic, 1 major, 2 natural minor */
    float reference_hz;                 /* 400–480 */
    float retune_speed_ms;              /* 0–500 */
    float correction_amount;            /* 0–1 */
    float correction_range_semitones;   /* 0–12 (dryCorrect only) */
    float vibrato_preserve;             /* 0–1 */
    float formant_follow;               /* 0–1 */
    float quantize_dead_zone_cents;     /* 0–50 (dryCorrect only) */
    float mix;                          /* 0–1 MIDI voices level */
    float min_f0_hz;                    /* 50–200 detection floor / latency */
    float dry_level;                    /* 0–1 dry / dryCorrect underlay */
    uint8_t dry_correct;                /* 0/1 scale-correct the dry path */
} PrismDspParams;

/** Metering snapshot for the pitch→MIDI visualizer. */
typedef struct PrismDspMetering {
    float detected_hz;
    float dry_target_hz;
    uint32_t num_voices;
    float voice_midi[PRISM_DSP_MAX_VOICES];
    float voice_target_hz[PRISM_DSP_MAX_VOICES];
} PrismDspMetering;

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

void prism_dsp_note_on(PrismDsp* dsp, uint8_t midi_note, float velocity);
void prism_dsp_note_off(PrismDsp* dsp, uint8_t midi_note);

void prism_dsp_get_metering(const PrismDsp* dsp, PrismDspMetering* out);

/** Algorithmic latency in samples (≈ one period at minF0Hz). */
uint32_t prism_dsp_reported_latency_samples(const PrismDsp* dsp);

#ifdef __cplusplus
}
#endif
