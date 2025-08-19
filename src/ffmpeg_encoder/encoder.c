#include "encoder.h"

#include <asm-generic/errno-base.h>
#include <stdint.h>
#include <stdio.h>

#include "libavcodec/avcodec.h"
#include "libavcodec/codec.h"
#include "libavcodec/packet.h"
#include "libavformat/avformat.h"
#include "libavutil/error.h"
#include "libavutil/frame.h"
#include "libavutil/pixfmt.h"
#include "libavutil/rational.h"

// **** GLOBALS ****

static const AVOutputFormat *output_format;
static AVFormatContext      *output_context;
static const AVCodec        *video_codec;
static AVCodecContext       *video_codec_context;
static AVFrame              *frame;
static AVPacket             *packet;
static enum AVPixelFormat    pixel_format;

static uint32_t              base_sec_low;
static uint32_t              base_sec_high;
static uint32_t base_ns = 0x34342E35;  // magic number if this is unset

// **** END GLOBAL ****

// **** UTILITIES ****

/*
    Given a timestamp, calcualte the pts.

    Return -1 if the pts cannot be calculated due to overflow.
*/
static int64_t
timestamp_to_pts(uint32_t sec_low, uint32_t sec_high, uint32_t ns) {
    if (base_ns == 0x34342E35) {
        base_sec_high = sec_high;
        base_sec_low = sec_low;
        base_ns = ns;
        return 0;
    }

    int64_t result = sec_high - base_sec_high;

    /*
    high_diff store bit 33-64 of the seconds part of the timestamp.
    We want to multiply it by the divisor of the time_base to figure out the
    amount of step.
    Problem: signed integer overflow is undefined.

    It take floor(log_2(a * x)) + 1 bits to store the result.
    = floor(log_2(a) + log_2(x)) + 1
    <= floor(log_2(a)) + 1 + floor(log_2(x)) + 1

    so we need at most floor(log_2(x)) + 1 more bit to store the result

    This should not happen, as so far wlr seems to use uptime as the timestamp,
    but there is no guarantee in the protocol description.
    2^32 seconds ~= 10 years up-time.
    */

    // overflow case
    // [TODO]: adjust based on time_base
    if (result >> 25 != 0) {
        return -1;
    }

    // sec_high component
    result = (result << 32) * 60;

    if (ns < base_ns) {
        sec_low -= 1;
        ns += 1000000000;
    }

    // sec_low component
    result += (base_sec_low - sec_low) * 60;

    // ns component
    // [TODO]: adjust based on time_base
    result += (ns - base_ns) * 60 / 1000000000;

    return result;
}

// **** END UTILITIES ****

// we also need a function that create packet from buffer, send it to the Packet
// buffer another function that dump the buffer into decoder and somehow mixes
// both video and audio too.

void encode_frame(
    void    *frame_buffer,
    uint32_t frame_stride,

    uint32_t ts_sec_low,
    uint32_t ts_sec_high,
    uint32_t ts_ns
) {
    // Make the frame writable, if needed.
    if (av_frame_make_writable(frame) < 0) {
        printf("Cannot make frame writable\n");
        return;
    }

    // [TODO]: Add recasting for each depth type
    {
        uint8_t *casted_buffer = frame_buffer;

        // copying is forced here as we need to reuse the frame buffer later
        for (int y = 0; y < video_codec_context->height; y++) {
            int frame_row_offset = y * frame_stride;

            for (int x = 0; x < video_codec_context->width; x++) {
                int frame_col = frame_row_offset + 4 * x;
                // we use linesize instead of width as the buffer may be padded
                // to align.
                //"libavutil/pixdesc.h" Use this to figure out how to fill data.
                int pixel_location = y * frame->linesize[0] + x;

                // BGRABGRABGRA
                frame->data[0][pixel_location] =
                    ((uint8_t *)frame_buffer)[frame_col];
                frame->data[0][pixel_location + 1] =
                    ((uint8_t *)frame_buffer)[frame_col + 1];
                frame->data[0][pixel_location + 2] =
                    ((uint8_t *)frame_buffer)[frame_col + 2];
                frame->data[0][pixel_location + 3] =
                    ((uint8_t *)frame_buffer)[frame_col + 3];
            }
        }
    }

    frame->pts = timestamp_to_pts(ts_sec_low, ts_sec_high, ts_ns);

    // now we have a complete av_frame
    // [TODO]: Signal back that the buffer can be reused.

    if (avcodec_send_frame(video_codec_context, frame) < 0) {
        printf("Failed to send frame.\n");
        return;
    }

    // frame is sent, now we receive packets
    int ret = 0;

    while (ret >= 0) {
        ret = avcodec_receive_packet(video_codec_context, packet);

        if (ret == AVERROR(EAGAIN) || ret == AVERROR_EOF) {
            // output has been fully read or no input received.
            return;
        } else if (ret < 0) {
            // actual errors
            printf("Error encountered while encoding.\n");
            return;
        }

        // now we have a packet.
        // [TODO]: Send the packet to the queue, and allocate a new one.

        // for now
        av_packet_unref(packet);
    }
}

void initialize_encoder(int width, int height) {
    width -= width % 2;
    height -= height % 2;

    // [TODO]: Allow customizing codec
    video_codec = avcodec_find_encoder_by_name("libvpx-vp9");
    if (video_codec == NULL) {
        printf("Cannot find codec.\n");
        return;
    }

    video_codec_context = avcodec_alloc_context3(video_codec);
    if (video_codec_context == NULL) {
        printf("Cannot allocate codec.\n");
        return;
    }

    packet = av_packet_alloc();
    if (packet == NULL) {
        printf("Cannot allocate packet");
        return;
    }

    // Arbitrary value?
    video_codec_context->bit_rate = 400000;
    video_codec_context->width = width;
    video_codec_context->height = height;
    video_codec_context->time_base = (AVRational){1, 60};
    // We do not have a constant frame rate here
    // if we need CFR, set this.
    // video_codec_context->framerate = (AVRational){60, 1};

    video_codec_context->gop_size = 10;
    video_codec_context->max_b_frames = 1;
    // Also allow setting this!
    video_codec_context->pix_fmt = AV_PIX_FMT_BGRA;
    pixel_format = AV_PIX_FMT_BGRA;

    if (avcodec_open2(video_codec_context, video_codec, NULL) < 0) {
        printf("Cannot open codec.\n");
        return;
    }

    frame = av_frame_alloc();
    if (frame == NULL) {
        printf("Cannot allocate frame.\n");
        return;
    }
}