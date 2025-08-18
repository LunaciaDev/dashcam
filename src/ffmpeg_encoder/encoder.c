#include "encoder.h"
#include <stdio.h>

#include "libavcodec/avcodec.h"
#include "libavcodec/codec.h"
#include "libavcodec/packet.h"
#include "libavformat/avformat.h"
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

// **** END GLOBAL ****

// we also need a function that create packet from buffer, send it to the Packet buffer
// another function that dump the buffer into decoder
// and somehow mixes both video and audio too.

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
    //video_codec_context->framerate = (AVRational){60, 1};

    video_codec_context->gop_size = 10;
    video_codec_context->max_b_frames = 1;
    // Also allow setting this!
    video_codec_context->pix_fmt = AV_PIX_FMT_RGB24;

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