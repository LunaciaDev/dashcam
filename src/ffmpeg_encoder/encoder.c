#include "encoder.h"

#include <asm-generic/errno-base.h>
#include <stdint.h>
#include <stdio.h>

#include "libavcodec/avcodec.h"
#include "libavcodec/codec.h"
#include "libavcodec/packet.h"
#include "libavfilter/avfilter.h"
#include "libavfilter/buffersink.h"
#include "libavfilter/buffersrc.h"
#include "libavformat/avformat.h"
#include "libavutil/error.h"
#include "libavutil/frame.h"
#include "libavutil/mem.h"
#include "libavutil/opt.h"
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

static AVFrame              *filtered_frame;
static AVFilterContext      *buffersink_context;
static AVFilterContext      *buffersource_context;
static AVFilterGraph        *filter_graph;

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

static void initialize_filter(enum AVPixelFormat format) {
    // prepare filter IO
    char            args[512];
    const AVFilter *buffersrc = avfilter_get_by_name("buffer");
    const AVFilter *buffersink = avfilter_get_by_name("buffersink");
    AVFilterInOut  *input = avfilter_inout_alloc();
    AVFilterInOut  *output = avfilter_inout_alloc();
    AVRational      time_base = video_codec_context->time_base;

    filter_graph = avfilter_graph_alloc();
    av_opt_set(
        filter_graph, "scale_sws_opts",
        "flags=fast_bilinear:src_range=1:dst_range=1", 0
    );

    if (output == NULL || input == NULL || filter_graph == NULL) {
        avfilter_inout_free(&input);
        avfilter_inout_free(&output);
        return;
    }

    // generate 'buffer' filter args
    // this effectively locks the video_size.
    snprintf(
        args, sizeof(args),
        "video_size=%dx%d:pix_fmt=%d:time_base=%d/%d:pixel_aspect=1/1",
        video_codec_context->width, video_codec_context->height, format,
        time_base.den, time_base.num
    );

    if (avfilter_graph_create_filter(
            &buffersource_context, buffersrc, "in", args, NULL, filter_graph
        ) < 0) {
        avfilter_inout_free(&input);
        avfilter_inout_free(&output);
        return;
    }

    if (avfilter_graph_create_filter(
            &buffersink_context, buffersink, "out", NULL, NULL, filter_graph
        ) < 0) {
        avfilter_inout_free(&input);
        avfilter_inout_free(&output);
        return;
    }

    // tell the sink what format we are expecting
    const enum AVPixelFormat chosen_pix_fmt[] = {
        AV_PIX_FMT_YUV420P, AV_PIX_FMT_NONE
    };

    av_opt_set_int_list(
        buffersink_context, "pix_fmts", chosen_pix_fmt, AV_PIX_FMT_NONE,
        AV_OPT_SEARCH_CHILDREN
    );

    // setup the pipeline
    output->name = av_strdup("in");
    output->filter_ctx = buffersource_context;
    output->pad_idx = 0;
    output->next = NULL;

    input->name = av_strdup("out");
    input->filter_ctx = buffersink_context;
    input->pad_idx = 0;
    input->next = NULL;

    if (avfilter_graph_parse_ptr(filter_graph, "format", &input, &output, NULL) <
        0) {
        avfilter_inout_free(&input);
        avfilter_inout_free(&output);
        return;
    }

    if (avfilter_graph_config(filter_graph, NULL) < 0) {
        avfilter_inout_free(&input);
        avfilter_inout_free(&output);
        return;
    }

    avfilter_inout_free(&input);
    avfilter_inout_free(&output);
}

// **** END UTILITIES ****

// we also need a function that create packet from buffer, send it to the Packet
// buffer another function that dump the buffer into decoder and somehow mixes
// both video and audio too.

void encode_frame(
    void              *frame_buffer,
    uint32_t           frame_stride,
    uint32_t           frame_width,
    uint32_t           frame_height,
    enum AVPixelFormat frame_format,

    uint32_t           ts_sec_low,
    uint32_t           ts_sec_high,
    uint32_t           ts_ns
) {
    if (buffersource_context == NULL) {
        // initialize the filter
        // we assume the frame_format does NOT change!
        initialize_filter(frame_format);
    }

    frame->width = frame_width;
    frame->height = frame_height;
    frame->format = frame_format;
    
    if (av_frame_get_buffer(frame, 0) < 0) {
        printf("Cannot allocate frame.\n");
        return;
    }

    // [TODO]: Add recasting for each depth type
    {
        uint8_t *casted_buffer = frame_buffer;

        // copying is forced here as we need to reuse the frame buffer later
        for (int y = 0; y < video_codec_context->height; y++) {
            int frame_row_offset = y * frame_stride;

            for (int x = 0; x < video_codec_context->width; x++) {
                int frame_col = frame_row_offset + x * 4;
                // we use linesize instead of width as the buffer may be padded
                // to align.
                //"libavutil/pixdesc.h" Use this to figure out how to fill data.
                int pixel_location = y * frame->linesize[0] + x * 4;

                // BGRABGRABGRA
                frame->data[0][pixel_location] =
                    casted_buffer[frame_col];
                frame->data[0][pixel_location + 1] =
                    casted_buffer[frame_col + 1];
                frame->data[0][pixel_location + 2] =
                    casted_buffer[frame_col + 2];
                frame->data[0][pixel_location + 3] =
                    casted_buffer[frame_col + 3];
            }
        }
    }

    frame->pts = timestamp_to_pts(ts_sec_low, ts_sec_high, ts_ns);

    // now we have a complete av_frame, now make it compatiable with the codec.

    // feed the frame into the filtergraph
    if (av_buffersrc_add_frame_flags(buffersource_context, frame, 0) < 0) {
        printf("Failed to push frame to filters.\n");
        return;
    }

    // pull out all frame from the filtergraph
    while (1) {
        int buffersink_ret =
            av_buffersink_get_frame(buffersink_context, filtered_frame);

        if (buffersink_ret == AVERROR(EAGAIN) ||
            buffersink_ret == AVERROR_EOF) {
            // we cannot get more frame from the sink. Stop pulling.
            break;
        } else if (buffersink_ret < 0) {
            // filter error!
            printf("Error encountered while filtering.\n");
            return;
        }

        // we now have a frame filtered. Send it to the encoder.

        if (avcodec_send_frame(video_codec_context, filtered_frame) < 0) {
            printf("Failed to send frame.\n");
            return;
        }

        // frame is sent, now we receive packets
        int codec_ret = 0;

        while (codec_ret >= 0) {
            codec_ret = avcodec_receive_packet(video_codec_context, packet);

            if (codec_ret == AVERROR(EAGAIN) || codec_ret == AVERROR_EOF) {
                // output has been fully read or no input received.
                return;
            } else if (codec_ret < 0) {
                // actual errors
                printf("Error encountered while encoding.\n");
                return;
            }

            // now we have a packet.
            // [TODO]: Send the packet to the queue, and allocate a new one.
            printf("Packet emitted.\n");

            av_packet_unref(packet);
        }

        // reset filtered frame, prepares for next loop.
        av_frame_unref(filtered_frame);
    }

    av_frame_unref(frame);
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

    video_codec_context->gop_size = 5;
    // Also allow setting this!
    video_codec_context->pix_fmt = AV_PIX_FMT_YUV420P;
    pixel_format = AV_PIX_FMT_YUV420P;

    if (avcodec_open2(video_codec_context, video_codec, NULL) < 0) {
        printf("Cannot open codec.\n");
        return;
    }

    frame = av_frame_alloc();
    if (frame == NULL) {
        printf("Cannot allocate frame.\n");
        return;
    }

    filtered_frame = av_frame_alloc();
    if (filtered_frame == NULL) {
        printf("Cannot allocate frame.\n");
        return;
    }
}