use std::{
    os::{fd::AsFd, raw::c_void},
    ptr::NonNull,
    thread::sleep,
    time::Duration,
};

use wayland_client::{
    Connection, Dispatch, QueueHandle, WEnum,
    protocol::{
        wl_buffer::WlBuffer,
        wl_output::WlOutput,
        wl_registry,
        wl_shm::{Format, WlShm},
        wl_shm_pool::WlShmPool,
    },
};

use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1::{self, ZwlrScreencopyFrameV1},
    zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

use crate::utils::{create_fd, create_mmap, extract_frames_from_mmap};

mod utils;

struct Data {
    zwlr_screencopy_manager: Option<ZwlrScreencopyManagerV1>,
    zwlr_screencopy_frame: Option<ZwlrScreencopyFrameV1>,

    wl_output: Option<WlOutput>,
    wl_shm: Option<WlShm>,
    wl_shm_pool: Option<WlShmPool>,

    result_buffer: Option<WlBuffer>,
    result_config: Option<BufferConfig>,
    result_raw_ptr: Option<NonNull<c_void>>,
}

impl Default for Data {
    fn default() -> Self {
        Data {
            zwlr_screencopy_manager: None,
            zwlr_screencopy_frame: None,
            wl_output: None,
            wl_shm: None,
            wl_shm_pool: None,
            result_raw_ptr: None,
            result_buffer: None,
            result_config: None,
        }
    }
}

struct BufferConfig {
    format: Format,
    width: u32,
    height: u32,
    stride: u32,
}

impl Dispatch<wl_registry::WlRegistry, ()> for Data {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        data: &(),
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
                    registry.bind::<ZwlrScreencopyManagerV1, _, _>(name, version, qhandle, *data),
                );
            }

            if interface == "wl_output" {
                state.wl_output =
                    Some(registry.bind::<WlOutput, _, _>(name, version, qhandle, *data));
            }

            if interface == "wl_shm" {
                state.wl_shm = Some(registry.bind::<WlShm, _, _>(name, version, qhandle, *data));
            }
        }
    }
}

impl Dispatch<WlOutput, ()> for Data {
    fn event(
        _state: &mut Self,
        _proxy: &WlOutput,
        _event: <WlOutput as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        // we are also not interested in any of their event yet.
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
        state: &mut Self,
        _proxy: &WlBuffer,
        event: <WlBuffer as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            wayland_client::protocol::wl_buffer::Event::Release => {
                state.result_buffer.as_ref().unwrap().destroy();
                state.result_buffer = None;
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
        data: &(),
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

                state.result_config = Some(BufferConfig {
                    format: selected_format.unwrap(),
                    width: width,
                    height: height,
                    stride: stride,
                });
            }

            zwlr_screencopy_frame_v1::Event::Flags { flags } => {
                println!("Flags event called");
            }

            zwlr_screencopy_frame_v1::Event::Damage {
                x,
                y,
                width,
                height,
            } => {
                println!("Unimplemented copy_with_damage")
            }

            zwlr_screencopy_frame_v1::Event::LinuxDmabuf {
                format,
                width,
                height,
            } => {
                // I have no idea about this...
                println!("Linux Dmabuf event called")
            }

            zwlr_screencopy_frame_v1::Event::BufferDone => {
                let buffer_cfg = state.result_config.as_ref().unwrap();

                if state.wl_shm_pool.is_none() {
                    let size = buffer_cfg.height as i32 * buffer_cfg.stride as i32;
                    let fd = create_fd(size);
                    let shm_pool = state.wl_shm.as_ref().unwrap().create_pool(
                        fd.as_fd(),
                        size,
                        qhandle,
                        *data,
                    );

                    state.result_raw_ptr = create_mmap(size as usize, fd.as_fd());

                    if state.result_raw_ptr.is_none() {
                        panic!("Failed to mmap data!");
                    }

                    state.wl_shm_pool = Some(shm_pool);
                }

                state.result_buffer = Some(state.wl_shm_pool.as_ref().unwrap().create_buffer(
                    0,
                    // [FIXME]: Potential overflow
                    // technically not, since these value are given
                    // to us, thus it should have been safe?
                    buffer_cfg.width as i32,
                    buffer_cfg.height as i32,
                    buffer_cfg.stride as i32,
                    buffer_cfg.format,
                    qhandle,
                    *data,
                ));

                state
                    .zwlr_screencopy_frame
                    .as_ref()
                    .unwrap()
                    .copy(&state.result_buffer.as_ref().unwrap());
            }

            zwlr_screencopy_frame_v1::Event::Ready {
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
            } => {
                // We are dealing with a raw c pointer.
                let ptr = state.result_raw_ptr.as_ref().unwrap();
                let frame_config = state.result_config.as_ref().unwrap();

                // [TODO]: Replace with emitting the decoded format for an encoder
                let pixel_vec = extract_frames_from_mmap(ptr, frame_config);

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

pub fn test() {
    let conn = Connection::connect_to_env().unwrap();
    let display = conn.display();
    let mut data: Data = Data::default();

    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();
    let _registry = display.get_registry(&qh, ());

    event_queue.roundtrip(&mut data).unwrap();

    if data.zwlr_screencopy_manager.is_none() {
        panic!("Support for wlr_screencopy is not announced. Exiting.");
    }

    if data.wl_output.is_none() {
        panic!("Support for wl_output is not announced. Exiting.");
    }

    if data.wl_shm.is_none() {
        panic!("Support for wl_shm is not announced. Exiting.");
    }

    loop {
        {
            let scrpy_mn = data.zwlr_screencopy_manager.as_ref().unwrap();
            let output = data.wl_output.as_ref().unwrap();
            data.zwlr_screencopy_frame = Some(scrpy_mn.capture_output(1, &output, &qh, ()));
        }

        event_queue.blocking_dispatch(&mut data).unwrap();
        sleep(Duration::new(1, 0));
    }
}
