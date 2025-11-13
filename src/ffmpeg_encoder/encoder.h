#ifndef __ENCODER_H__
#define __ENCODER_H__

#include <stdint.h>
#include "libavutil/pixfmt.h"

void initialize_encoder(int width, int height);
void encode_frame(
    void   *frame_buffer,
    uint32_t frame_stride,
    uint32_t frame_width,
    uint32_t frame_height,
    enum AVPixelFormat frame_format,

    uint32_t timestamp_sec_low,
    uint32_t timestamp_sec_high,
    uint32_t timestamp_ns
);
void finish_encode();

struct Packet {
    void* avpacket_ptr;
    int64_t pts;
};

#endif