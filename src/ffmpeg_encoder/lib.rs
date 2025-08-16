#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
include!(concat!(env!("OUT_DIR"), "/libav_pixfmt.rs"));

use wayland_client::protocol::wl_shm::Format;

unsafe extern "C" {
    fn start_encoder();
}

pub fn test() {
    unsafe {
        start_encoder();
    }
}

/*
We need these stuffs:

- Height, Width of the capture frame
- Pointer to the shm pages where the frame reside
- Frame format
- Encoding codec
- Pointer to an IPC channel (Likely to be socket as we need bidirectional communication)
  - Signal to this library that which shm pages is readable
  - Send back that which shm pages has been read and can be reused

init(uint8_t height, uint8_t width, void*[] shm_pages, AVFormat format, char* codec, void* ipc_channel);
*/

fn format_conversion(format: Format) -> i32 {
    match format {
        Format::Argb8888 => AVPixelFormat_AV_PIX_FMT_ARGB,
        Format::Xrgb8888 => AVPixelFormat_AV_PIX_FMT_0RGB,
        Format::Rgb888 => AVPixelFormat_AV_PIX_FMT_RGB24,
        Format::Bgr888 => AVPixelFormat_AV_PIX_FMT_BGR24,
        Format::Xbgr8888 => AVPixelFormat_AV_PIX_FMT_0BGR,
        Format::Rgbx8888 => AVPixelFormat_AV_PIX_FMT_RGB0,
        Format::Bgrx8888 => AVPixelFormat_AV_PIX_FMT_BGR0,
        Format::Abgr8888 => AVPixelFormat_AV_PIX_FMT_ABGR,
        Format::Rgba8888 => AVPixelFormat_AV_PIX_FMT_RGBA,
        Format::Bgra8888 => AVPixelFormat_AV_PIX_FMT_BGRA,
        _ => {
            panic!("Unsupported format!");
        }
    }
}
