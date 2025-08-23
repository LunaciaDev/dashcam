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
#include "libavformat/avio.h"
#include "libavutil/error.h"
#include "libavutil/frame.h"
#include "libavutil/mem.h"
#include "libavutil/opt.h"
#include "libavutil/pixfmt.h"
#include "libavutil/rational.h"

// **** GLOBALS ****

// OUTPUT STREAM
static AVFormatContext      *output_context;
static const AVOutputFormat *output_format;
static AVStream             *video_stream;

// VIDEO STREAM
static const AVCodec     *video_codec;
static AVCodecContext    *video_codec_context;
static AVPacket          *packet;
static enum AVPixelFormat pixel_format;

static AVFilterContext   *buffersink_context;
static AVFilterContext   *buffersource_context;
static AVFilterGraph     *filter_graph;

static int64_t            base_timestamp = -1;

// **** END GLOBAL ****

// **** UTILITIES ****

/*
    Given a timestamp, calcualte the pts.

    Return -1 if the pts cannot be calculated due to overflow.
*/
static int64_t
timestamp_to_pts(uint32_t sec_low, uint32_t sec_high, uint32_t ns) {
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
    if (sec_high >> 25 != 0) {
        return -1;
    }

    int64_t second = ((uint64_t)sec_high << 32) | sec_low;

    if (base_timestamp == -1) {
        base_timestamp = second * 1000000000 + ns;
        return 0;
    }

    return ((second * 1000000000 + ns) - base_timestamp) * 60 /
           (int64_t)1000000000;
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

    if (avfilter_graph_parse_ptr(
            filter_graph, "format", &input, &output, NULL
        ) < 0) {
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

static void internal_encode_frame(
    void              *frame_buffer,
    uint32_t           frame_stride,
    uint32_t           frame_width,
    uint32_t           frame_height,
    enum AVPixelFormat frame_format,

    AVFrame           *frame,
    AVFrame           *filtered_frame,

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
    frame->data[0] = frame_buffer;
    frame->linesize[0] = frame_stride;
    frame->pts = timestamp_to_pts(ts_sec_low, ts_sec_high, ts_ns);

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

            av_packet_rescale_ts(
                packet, video_codec_context->time_base, video_stream->time_base
            );
            packet->stream_index = video_stream->index;
            av_interleaved_write_frame(output_context, packet);

            // interleaved_write_frame reset our packet, so no unref necessary.
            // av_packet_unref(packet);
        }
    }
}

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
    AVFrame *frame = av_frame_alloc();
    if (frame == NULL) {
        printf("Cannot allocate frame.\n");
        return;
    }

    AVFrame *filtered_frame = av_frame_alloc();
    if (filtered_frame == NULL) {
        printf("Cannot allocate frame.\n");
        return;
    }

    internal_encode_frame(
        frame_buffer, frame_stride, frame_width, frame_height, frame_format,
        frame, filtered_frame, ts_sec_low, ts_sec_high, ts_ns
    );

    av_frame_free(&frame);
    av_frame_free(&filtered_frame);
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

    video_codec_context->gop_size = 12;
    // Also allow setting this!
    video_codec_context->pix_fmt = AV_PIX_FMT_YUV420P;
    pixel_format = AV_PIX_FMT_YUV420P;

    if (avcodec_open2(video_codec_context, video_codec, NULL) < 0) {
        printf("Cannot open codec.\n");
        return;
    }

    // output context

    avformat_alloc_output_context2(&output_context, NULL, NULL, "output.mkv");
    if (output_context == NULL) {
        printf("Unrecognized container format, using matroska as fallback.");
        avformat_alloc_output_context2(
            &output_context, NULL, "matroska", "output.mkv"
        );
    }
    if (output_context == NULL) {
        return;
    }

    output_context->video_codec = video_codec;
    output_format = output_context->oformat;

    video_stream = avformat_new_stream(output_context, video_codec);
    if (video_stream == NULL) {
        printf("Cannot allocate stream");
        return;
    }
    video_stream->id = output_context->nb_streams - 1;

    avcodec_parameters_from_context(
        video_stream->codecpar, video_codec_context
    );

    if (output_context->oformat->flags & AVFMT_GLOBALHEADER) {
        video_codec_context->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
    }

    video_stream->time_base = video_codec_context->time_base;

    if (!(output_format->flags & AVFMT_NOFILE)) {
        if (avio_open(&output_context->pb, "output.mkv", AVIO_FLAG_WRITE) < 0) {
            printf("Cannot open AVIO context");
            return;
        }
    }

    if (avformat_write_header(output_context, NULL) < 0) {
        printf("Cannot write header");
        return;
    }
}