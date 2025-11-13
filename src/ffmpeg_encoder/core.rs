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

use crate::{ffmpeg_encoder::encoder_ffi, utils::{is_halt, set_halt}};

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
                // Sender thread might have crashed - we start wrapping up.
                set_halt();
                break;
            }
        };

        encode_frame(frame_info);

        match wlresponse_tx.send(0x44) {
            Ok(f) => f,
            Err(_) => {
                // Same deal, wrap up as sender thread might have crashed.
                set_halt();
                break;
            }
        };
    }

    // continue going until there is no data left.
    while let Ok(frame_info) = wlpacket_rx.try_recv() {
        encode_frame(frame_info);

        match wlresponse_tx.send(0x44) {
            Ok(f) => f,
            Err(_) => {
                // We ignore the send error, as we want to use up all remaining buffers.
            }
        };
    }

    unsafe { encoder_ffi::finish_encode() };
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
