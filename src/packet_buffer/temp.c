#include "libavformat/avformat.h"
#include "libavutil/error.h"

AVFormatContext *output_context;
AVStream        *video_stream;

void             initialize_muxer(AVCodec *video_codec) {
    // Allocate the output context
    avformat_alloc_output_context2(&output_context, NULL, NULL, "output.mkv");
    if (output_context == NULL) {
        fprintf(
            stderr, "Unrecognized container format, using matroska as fallback."
        );
        avformat_alloc_output_context2(
            &output_context, NULL, "matroska", "output.mkv"
        );
    }
    if (output_context == NULL) {
        return;
    }

    output_context->video_codec = video_codec;

    // create a new stream and assigning it to the output context

    video_stream = avformat_new_stream(output_context, video_codec);
    if (video_stream == NULL) {
        fprintf(stderr, "Cannot allocate stream");
        return;
    }
    video_stream->id = output_context->nb_streams - 1;

    // video_codec_context has to be a shared object...
    // [TODO]: Figure out how to deal with that.
    // we need a copy of the video_codec_context to set the codecpar of video stream.
}