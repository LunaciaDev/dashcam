use std::os::raw::c_void;

use crate::ffmpeg_encoder::core::AVPixelFormat;

unsafe extern "C" {
    pub unsafe fn initialize_encoder(width: i32, height: i32);
    pub unsafe fn encode_frame(
        frame_buffer: *mut c_void,
        frame_stride: u32,
        frame_width: u32,
        frame_height: u32,
        frame_format: AVPixelFormat,
        timestamp_sec_low: u32,
        timestamp_sec_high: u32,
        timestamp_ns: u32,
    );
    pub unsafe fn finish_encode();
}

pub struct Packet {
    pub avpacket_ptr: *mut c_void,
    pub pts: i64,
}

unsafe impl Send for Packet {}