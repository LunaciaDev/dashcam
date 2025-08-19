#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
include!(concat!(env!("OUT_DIR"), "/libav_pixfmt.rs"));

use std::{os::raw::c_void, ptr::NonNull};

use wayland_client::protocol::wl_shm::Format;

mod encoder_ffi {
    use std::os::raw::c_void;

    unsafe extern "C" {
        pub unsafe fn initialize_encoder(width: i32, height: i32);
        pub unsafe fn encode_frame(
            frame_buffer: *mut c_void,
            // the pointer to the buffer we have in Rust is &NonNull<c_void>
            // the C function take a void*
            timestamp_sec_low: u32,
            timestamp_sec_high: u32,
            timestamp_ns: u32,
        );
    }
}

pub fn encode_frame(frame_buffer: &NonNull<c_void>, tv_sec_hi: u32, tv_sec_lo: u32, tv_nsec: u32) {
    unsafe {
        encoder_ffi::encode_frame(frame_buffer.as_ptr(), tv_sec_hi, tv_sec_lo, tv_nsec);
    }
}

pub fn start(width: i32, height: i32) {
    unsafe {
        encoder_ffi::initialize_encoder(width, height);
    }

    /*
    we need a way to receive the pointers to the buffers
    and a channel to signal which buffer is ready.

    A channel, where the first couple value sent are pointers
    while later, send signals?

    Also we need to scale up wl_capture to use multiple buffers.
    */
}

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
