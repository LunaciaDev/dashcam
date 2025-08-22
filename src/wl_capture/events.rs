use std::{os::raw::c_void, ptr::NonNull, sync::mpsc::Sender};

use wayland_client::{
    Connection, Dispatch, QueueHandle, WEnum,
    protocol::{
        wl_buffer::WlBuffer,
        wl_output::{self, Mode, WlOutput},
        wl_registry,
        wl_shm::{Format, WlShm},
        wl_shm_pool::WlShmPool,
    },
};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1::{self, ZwlrScreencopyFrameV1},
    zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

use crate::{ffmpeg_encoder::FrameInfo, wl_capture::shm_pool::ManagedBufferPool};

#[derive(Default)]
pub struct Data {
    pub screen_width: i32,
    pub screen_height: i32,
    pub zwlr_screencopy_manager: Option<ZwlrScreencopyManagerV1>,
    pub zwlr_screencopy_frame: Option<ZwlrScreencopyFrameV1>,

    pub wl_output: Option<WlOutput>,
    pub wl_shm: Option<WlShm>,

    pub buffer_pool: ManagedBufferPool,
    pub buffer_config: Option<BufferConfig>,
    pub buffer_raw_ptr: Option<NonNull<c_void>>,
    pub buffer_send_channel: Option<Sender<FrameInfo>>,
}

pub struct BufferConfig {
    pub format: Format,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
}

impl Dispatch<wl_registry::WlRegistry, ()> for Data {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _conn: &Connection,
        qhandle: &QueueHandle<Data>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            // we depend directly on v3 of screencopy
            if interface == "zwlr_screencopy_manager_v1" && version == 3 {
                state.zwlr_screencopy_manager = Some(
                    registry.bind::<ZwlrScreencopyManagerV1, _, _>(name, version, qhandle, ()),
                );
            }

            if interface == "wl_output" {
                state.wl_output = Some(registry.bind::<WlOutput, _, _>(name, version, qhandle, ()));
            }

            if interface == "wl_shm" {
                state.wl_shm = Some(registry.bind::<WlShm, _, _>(name, version, qhandle, ()));
            }
        }
    }
}

impl Dispatch<WlOutput, ()> for Data {
    fn event(
        state: &mut Self,
        _proxy: &WlOutput,
        event: <WlOutput as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if let wl_output::Event::Mode {
            flags,
            width,
            height,
            refresh: _,
        } = event
            && let WEnum::Value(Mode::Current) = flags
        {
            state.screen_width = width;
            state.screen_height = height;
        }
    }
}

impl Dispatch<ZwlrScreencopyManagerV1, ()> for Data {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrScreencopyManagerV1,
        _event: <ZwlrScreencopyManagerV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        // this interface has to event to handle.
    }
}

impl Dispatch<WlShm, ()> for Data {
    fn event(
        _state: &mut Self,
        _proxy: &WlShm,
        _event: <WlShm as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        // This has an event that tell us the supported pixel format
        // Might be useful later
    }
}

impl Dispatch<WlShmPool, ()> for Data {
    fn event(
        _state: &mut Self,
        _proxy: &WlShmPool,
        _event: <WlShmPool as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        // No event emitted.
    }
}

impl Dispatch<WlBuffer, ()> for Data {
    fn event(
        _state: &mut Self,
        _proxy: &WlBuffer,
        event: <WlBuffer as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            wayland_client::protocol::wl_buffer::Event::Release => {
                // println!("Buffer release event called")
            }
            _ => {
                panic!("Unimplemented event!");
            }
        }
    }
}

impl Dispatch<ZwlrScreencopyFrameV1, ()> for Data {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrScreencopyFrameV1,
        event: <ZwlrScreencopyFrameV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_screencopy_frame_v1::Event::Buffer {
                format,
                width,
                height,
                stride,
            } => {
                // [TODO]: Is there a way to request a certain type of format?
                let selected_format = match format {
                    WEnum::Value(value) => match value {
                        Format::Argb8888 | Format::Xrgb8888 | Format::Xbgr8888 => Some(value),
                        _ => None,
                    },
                    WEnum::Unknown(_) => None,
                };

                state.buffer_config = Some(BufferConfig {
                    format: selected_format.unwrap(),
                    width,
                    height,
                    stride,
                });
            }

            zwlr_screencopy_frame_v1::Event::Flags { flags } => {
                // println!("Flags event called");
            }

            zwlr_screencopy_frame_v1::Event::Damage {
                x,
                y,
                width,
                height,
            } => {
                // println!("Unimplemented copy_with_damage")
            }

            zwlr_screencopy_frame_v1::Event::LinuxDmabuf {
                format,
                width,
                height,
            } => {
                // I have no idea about this...
                // println!("Linux Dmabuf event called")
            }

            zwlr_screencopy_frame_v1::Event::BufferDone => {
                let buffer_cfg = state.buffer_config.as_ref().unwrap();
                let (buffer, ptr) = state.buffer_pool.get_buffer(
                    buffer_cfg.width as i32,
                    buffer_cfg.height as i32,
                    buffer_cfg.stride as i32,
                    buffer_cfg.format,
                    state.wl_shm.as_ref().unwrap(),
                    qhandle,
                ).unwrap();

                state.buffer_raw_ptr = Some(ptr);
                state
                    .zwlr_screencopy_frame
                    .as_ref()
                    .unwrap()
                    .copy(buffer);
            }

            zwlr_screencopy_frame_v1::Event::Ready {
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
            } => {
                // construct our packet
                let frame_config = state.buffer_config.as_ref().unwrap();

                let packet = FrameInfo {
                    frame_buffer: *state.buffer_raw_ptr.as_ref().unwrap(),
                    frame_stride: frame_config.stride,
                    frame_width: frame_config.width,
                    frame_height: frame_config.height,
                    frame_format: frame_config.format,
                    tv_sec_hi,
                    tv_sec_lo,
                    tv_nsec,
                };

                state
                    .buffer_send_channel
                    .as_ref()
                    .unwrap()
                    .send(packet)
                    .unwrap();

                state.zwlr_screencopy_frame.as_ref().unwrap().destroy();
                state.zwlr_screencopy_frame = None;
            }

            zwlr_screencopy_frame_v1::Event::Failed => {
                state.zwlr_screencopy_frame.as_ref().unwrap().destroy();
                state.zwlr_screencopy_frame = None;
            }

            // Just in case if a new events is added without a version bump
            // This should not happen!
            _ => {
                panic!("Unhandled event!");
            }
        }
    }
}
