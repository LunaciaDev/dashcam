#include "encoder.h"
#include <libavutil/avutil.h>
#include <stdio.h>

void hello_world() {
    printf("Hello World!\n");
    printf("libav version: %s\n", av_version_info());
}