#ifndef __ENCODER_H__
#define __ENCODER_H__

#include <stdint.h>

void initialize_encoder(int width, int height);
void encode_frame(
    void   *frame_buffer,
    uint32_t frame_stride,
    uint32_t timestamp_sec_low,
    uint32_t timestamp_sec_high,
    uint32_t timestamp_ns
);

#endif