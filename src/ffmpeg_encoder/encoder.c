/*
 * Copyright (c) 2003 Fabrice Bellard
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL
 * THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
 * THE SOFTWARE.
 */

// Adapted from ffmpeg exmaple
// (https://github.com/FFmpeg/FFmpeg/blob/master/doc/examples/mux.c)

#include <stdio.h>

#include "libavcodec/avcodec.h"
#include "libavcodec/codec.h"
#include "libavcodec/codec_id.h"
#include "libavcodec/packet.h"
#include "libavformat/avformat.h"
#include "libavformat/avio.h"
#include "libavutil/error.h"
#include "libavutil/mem.h"
#include "libavutil/pixfmt.h"
#include "libavutil/rational.h"

typedef struct OutputStream {
    AVStream          *st;
    AVCodecContext    *enc;

    // pts of the next frame
    int64_t            next_pts;
    int                samples_count;

    AVFrame           *frame;

    AVPacket          *tmp_pkt;

    float              t, tincr, tincr2;
} OutputStream;

void add_stream(
    OutputStream        *output_stream,
    AVFormatContext     *context,
    const AVCodec      **codec,
    const enum AVCodecID id
) {
    AVCodecContext *codec_context;

    // find the equivalent encoder for the stream
    *codec = avcodec_find_encoder(id);
    if (*codec == NULL) {
        printf("Could not find encoder for %s\n", avcodec_get_name(id));
        exit(1);
    }

    output_stream->tmp_pkt = av_packet_alloc();
    if (output_stream->tmp_pkt == NULL) {
        printf("Could not allocate packet.");
        exit(1);
    }

    output_stream->st = avformat_new_stream(context, NULL);
    if (output_stream->st == NULL) {
        printf("Could not allocate AVStream\n");
        exit(1);
    }

    output_stream->st->id = context->nb_streams - 1;
    codec_context = avcodec_alloc_context3(*codec);
    if (codec_context == NULL) {
        printf("Could not allocate an encoding context\n");
        exit(1);
    }

    output_stream->enc = codec_context;

    // We only have video type for now!
    codec_context->codec_id = id;
    codec_context->bit_rate = 400000;  // arbitrary
    codec_context->width = 1280;
    codec_context->height = 720;

    // microsecond timebase - the smallest unit of defined time for the stream
    output_stream->st->time_base = (AVRational){1, 100};
    codec_context->time_base = output_stream->st->time_base;
    // how many frames per intra frame?
    codec_context->gop_size = 12;
    // pixel formats
    codec_context->pix_fmt = AV_PIX_FMT_YUV420P;

    // Some formats want stream headers to be separate.
    if (context->oformat->flags & AVFMT_GLOBALHEADER) {
        codec_context->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
    }
}

static AVFrame *
allocate_frame(enum AVPixelFormat pixel_format, int width, int height) {
    AVFrame *frame;
    int      ret;

    frame = av_frame_alloc();
    if (!frame) return NULL;

    frame->format = pixel_format;
    frame->width = width;
    frame->height = height;

    // allocate a buffer for this frame
    ret = av_frame_get_buffer(frame, 0);

    if (ret < 0) {
        printf("Could not allocate frame data.\n");
        exit(1);
    }

    return frame;
}

static void open_video(
    AVFormatContext *context,
    const AVCodec   *codec,
    OutputStream    *output_stream,
    AVDictionary    *opt_arg
) {
    int             ret;
    AVCodecContext *c = output_stream->enc;
    AVDictionary   *opt = NULL;

    av_dict_copy(&opt, opt_arg, 0);

    // [TODO]: figure out the codec setting too.

    /* open the codec */
    ret = avcodec_open2(c, codec, &opt);
    av_dict_free(&opt);

    if (ret < 0) {
        fprintf(stderr, "Could not open video codec: %s\n", av_err2str(ret));
        exit(1);
    }

    /* allocate and init a reusable frame */
    output_stream->frame = allocate_frame(c->pix_fmt, c->width, c->height);
    if (!output_stream->frame) {
        fprintf(stderr, "Could not allocate video frame\n");
        exit(1);
    }

    /* copy the stream parameters to the muxer */
    ret = avcodec_parameters_from_context(output_stream->st->codecpar, c);

    if (ret < 0) {
        fprintf(stderr, "Could not copy the stream parameters\n");
        exit(1);
    }
}

int write_stdout(void *opaque, const uint8_t *buf, int buf_size) {
    return fwrite(buf, 1, buf_size, stdout);
}

static int write_frame(
    AVFormatContext *format_context,
    AVCodecContext  *codec_context,
    AVStream        *stream,
    AVFrame         *frame,
    AVPacket        *packet
) {
    int return_code;

    // send the frame to the encoder
    return_code = avcodec_send_frame(codec_context, frame);
    if (return_code < 0) {
        fprintf(
            stderr, "Error sending a frame to the encoder: %s\n",
            av_err2str(return_code)
        );
        exit(1);
    }

    while (return_code >= 0) {
        return_code = avcodec_receive_packet(codec_context, packet);
        if (return_code == AVERROR(EAGAIN) || return_code == AVERROR_EOF)
            break;
        else if (return_code < 0) {
            fprintf(
                stderr, "Error encoding a frame: %s\n", av_err2str(return_code)
            );
            exit(1);
        }

        /* rescale output packet timestamp values from codec to stream timebase
         */
        av_packet_rescale_ts(
            packet, codec_context->time_base, stream->time_base
        );
        packet->stream_index = stream->index;

        /* Write the compressed frame to the media file. */
        return_code = av_interleaved_write_frame(format_context, packet);
        /* pkt is now blank (av_interleaved_write_frame() takes ownership of
         * its contents and resets pkt), so that no unreferencing is necessary.
         * This would be different if one used av_write_frame(). */
        if (return_code < 0) {
            fprintf(
                stderr, "Error while writing output packet: %s\n",
                av_err2str(return_code)
            );
            exit(1);
        }
    }

    return return_code == AVERROR_EOF ? 1 : 0;
}

static void
fill_image(AVFrame *frame, int64_t frame_index, int width, int height) {
    int x, y, i;

    i = frame_index;

    /* Y */
    for (y = 0; y < height; y++)
        for (x = 0; x < width; x++)
            frame->data[0][y * frame->linesize[0] + x] = x + y + i * 3;

    /* Cb and Cr */
    for (y = 0; y < height / 2; y++) {
        for (x = 0; x < width / 2; x++) {
            frame->data[1][y * frame->linesize[1] + x] = 128 + y + i * 2;
            frame->data[2][y * frame->linesize[2] + x] = 64 + x + i * 5;
        }
    }
}

static AVFrame *get_video_frame(OutputStream *stream) {
    AVCodecContext *c = stream->enc;

    /* check if we want to generate more frames */
    if (av_compare_ts(stream->next_pts, c->time_base, 25, (AVRational){1, 1}) >
        0)
        return NULL;

    /* when we pass a frame to the encoder, it may keep a reference to it
     * internally; make sure we do not overwrite it here */
    if (av_frame_make_writable(stream->frame) < 0) exit(1);

    // you may need to convert the image to correct type before filling.
    // we have a static frame, so should be fineee.
    fill_image(stream->frame, stream->next_pts, c->width, c->height);

    stream->frame->pts = stream->next_pts++;

    return stream->frame;
}

static int
write_video_frame(AVFormatContext *output_context, OutputStream *stream) {
    return write_frame(
        output_context, stream->enc, stream->st, get_video_frame(stream),
        stream->tmp_pkt
    );
}

static void close_stream(AVFormatContext *oc, OutputStream *stream) {
    avcodec_free_context(&stream->enc);
    av_frame_free(&stream->frame);
    av_packet_free(&stream->tmp_pkt);
}

int start_encoder() {
    OutputStream          video_stream = {0};
    const AVOutputFormat *output_format;
    AVFormatContext      *output_context;
    const AVCodec        *video_codec;

    // allocate the output context
    avformat_alloc_output_context2(&output_context, NULL, "mpegts", NULL);

    if (output_context == NULL) {
        return 1;
    }

    output_format = output_context->oformat;

    // if there is no default codec
    if (output_format->video_codec == AV_CODEC_ID_NONE) {
        return 1;
    }

    add_stream(
        &video_stream, output_context, &video_codec, output_format->video_codec
    );

    open_video(output_context, video_codec, &video_stream, NULL);

    av_dump_format(output_context, 0, "output_stream", 1);

    // create an avio context
    void        *io_buffer = av_malloc(4096);
    AVIOContext *io_context =
        avio_alloc_context(io_buffer, 4096, 1, NULL, NULL, &write_stdout, NULL);
    output_context->pb = io_context;

    // write the header, if needed.
    if (avformat_write_header(output_context, NULL) < 0) {
        printf("Error occured");
        return 1;
    }

    while (1) {
        if (write_video_frame(output_context, &video_stream) < 0) {
            break;
        }
    }

    close_stream(output_context, &video_stream);

    // free the IO context
    avio_context_free(&io_context);

    // free the output context
    avformat_free_context(output_context);
    return 0;
}