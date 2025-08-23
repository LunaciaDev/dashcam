#![allow(
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    dead_code // we are okay with pulling the entirety of the pixfmt enum over, make it easier to maintain.
)]
include!(concat!(env!("OUT_DIR"), "/libav_pixfmt.rs"));

use std::{
    os::raw::c_void,
    ptr::NonNull,
    sync::{
        Arc, Barrier,
        mpsc::{Receiver, Sender},
    },
};
use wayland_client::protocol::wl_shm::Format;

use crate::utils::{is_halt, set_halt};

pub struct FrameInfo {
    pub frame_buffer: NonNull<c_void>,
    pub frame_stride: u32,
    pub frame_width: u32,
    pub frame_height: u32,
    pub frame_format: Format,
    pub tv_sec_hi: u32,
    pub tv_sec_lo: u32,
    pub tv_nsec: u32,
}

// sharing a raw ptr is not allowed, unless we explicitly say so.
// frame_buffer is not allowed to be written until this has released it
// and synced via message passing, so we should be fineeee... perhaps.
unsafe impl Send for FrameInfo {}

mod encoder_ffi {
    use std::os::raw::c_void;

    use crate::ffmpeg_encoder::core::AVPixelFormat;

    unsafe extern "C" {
        pub unsafe fn initialize_encoder(width: i32, height: i32);
        pub fn encode_frame(
            frame_buffer: *mut c_void,
            frame_stride: u32,
            frame_width: u32,
            frame_height: u32,
            frame_format: AVPixelFormat,
            timestamp_sec_low: u32,
            timestamp_sec_high: u32,
            timestamp_ns: u32,
        );
    }
}

pub fn encode_frame(frame_info: FrameInfo) {
    unsafe {
        encoder_ffi::encode_frame(
            frame_info.frame_buffer.as_ptr(),
            frame_info.frame_stride,
            frame_info.frame_width,
            frame_info.frame_height,
            format_conversion(frame_info.frame_format),
            frame_info.tv_sec_lo,
            frame_info.tv_sec_hi,
            frame_info.tv_nsec,
        );
    }
}

pub fn start(
    width: i32,
    height: i32,
    wlpacket_rx: Receiver<FrameInfo>,
    wlresponse_tx: Sender<u8>,
    barrier: Arc<Barrier>,
) {
    unsafe {
        encoder_ffi::initialize_encoder(width, height);
    }

    barrier.wait();

    while !is_halt() {
        let frame_info = match wlpacket_rx.recv() {
            Ok(f) => f,
            Err(_) => {
                // the capturing thread crashed, so..
                set_halt();
                return;
            }
        };

        encode_frame(frame_info);

        match wlresponse_tx.send(0x44) {
            Ok(f) => f,
            Err(_) => {
                // the capturing thread crashed, so..
                set_halt();
                return;
            }
        };
    }
}

fn format_conversion(format: Format) -> i32 {
    match format {
        Format::Argb8888 => AVPixelFormat_AV_PIX_FMT_BGRA,
        Format::Xrgb8888 => AVPixelFormat_AV_PIX_FMT_BGR0,
        Format::Rgb888 => AVPixelFormat_AV_PIX_FMT_BGR24,
        Format::Bgr888 => AVPixelFormat_AV_PIX_FMT_RGB24,
        Format::Xbgr8888 => AVPixelFormat_AV_PIX_FMT_RGB0,
        Format::Rgbx8888 => AVPixelFormat_AV_PIX_FMT_0BGR,
        Format::Bgrx8888 => AVPixelFormat_AV_PIX_FMT_0RGB,
        Format::Abgr8888 => AVPixelFormat_AV_PIX_FMT_RGBA,
        Format::Rgba8888 => AVPixelFormat_AV_PIX_FMT_ABGR,
        Format::Bgra8888 => AVPixelFormat_AV_PIX_FMT_ARGB,
        _ => {
            panic!("Unsupported format!");
        }
    }
}
