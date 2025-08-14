use std::{os::fd::AsFd, thread::sleep, time::Duration};

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

use crate::utils::create_fd;

mod utils;

struct Data {
    wlr_screencopy_manager: Option<ZwlrScreencopyManagerV1>,
    wl_output: Option<WlOutput>,
    wl_shm: Option<WlShm>,
    wl_shm_pool: Option<WlShmPool>,
    screencopy_frame: Option<ZwlrScreencopyFrameV1>,
    screencopy_buffer: Option<WlBuffer>,
    screencopy_buffer_config: Option<BufferConfig>,
}

impl Default for Data {
    fn default() -> Self {
        Data {
            wlr_screencopy_manager: None,
            wl_output: None,
            wl_shm: None,
            wl_shm_pool: None,
            screencopy_frame: None,
            screencopy_buffer: None,
            screencopy_buffer_config: None,
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
                state.wlr_screencopy_manager = Some(
                    registry.bind::<ZwlrScreencopyManagerV1, _, _>(name, version, qhandle, *data),
                );
            }

            if interface == "wl_output" {
                state.wl_output =
                    Some(registry.bind::<WlOutput, _, _>(name, version, qhandle, *data));
            }

            println!("{}, version {}", interface, version);

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
                state.screencopy_buffer.as_ref().unwrap().destroy();
                state.screencopy_buffer = None;
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
                // [TODO]: Dig into this format system more?
                // We might want to write some kind of dispatcher to handle more format type
                // but Argb8888 is well supported.
                let selected_format = match format {
                    WEnum::Value(value) => {
                        match value {
                            Format::Argb8888 => Some(value),
                            Format::Xrgb8888 => Some(value),
                            _ => None,
                        }
                    },
                    WEnum::Unknown(_) => None,
                };

                state.screencopy_buffer_config = Some(BufferConfig {
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
                panic!("Unimplemented copy_with_damage")
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
                let buffer_cfg = state.screencopy_buffer_config.as_ref().unwrap();

                if state.wl_shm_pool.is_none() {
                    let size = buffer_cfg.height as i32 * buffer_cfg.stride as i32;
                    let fd = create_fd(size);
                    let shm_pool = state.wl_shm.as_ref().unwrap().create_pool(
                        fd.as_fd(),
                        size,
                        qhandle,
                        *data,
                    );

                    // [TODO]: Create a mem mapped reference of shm pool
                    // so we can access the shared data too?

                    state.wl_shm_pool = Some(shm_pool);
                }

                state.screencopy_buffer = Some(state.wl_shm_pool.as_ref().unwrap().create_buffer(
                    0,

                    // [FIXME]: Potential overflow
                    buffer_cfg.width as i32,
                    buffer_cfg.height as i32,
                    buffer_cfg.stride as i32,
    
                    buffer_cfg.format,
                    qhandle,
                    *data,
                ));

                state.screencopy_frame.as_ref().unwrap().copy(&state.screencopy_buffer.as_ref().unwrap());
            }

            zwlr_screencopy_frame_v1::Event::Ready {
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
            } => {
                println!("Ready event called");

                // do something

                state.screencopy_frame.as_ref().unwrap().destroy();
                state.screencopy_frame = None;
            }

            zwlr_screencopy_frame_v1::Event::Failed => {
                state.screencopy_frame.as_ref().unwrap().destroy();
                state.screencopy_frame = None;
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

    if data.wlr_screencopy_manager.is_none() {
        panic!("Support for wlr_screencopy is not announced. Exiting.");
    }

    if data.wl_output.is_none() {
        panic!("Support for wl_output is not announced. Exiting.");
    }

    if data.wl_shm.is_none() {
        panic!("Support for wl_shm is not announced. Exiting.");
    }

    let scrpy_mn = data.wlr_screencopy_manager.as_ref().unwrap();
    let output = data.wl_output.as_ref().unwrap();

    data.screencopy_frame = Some(scrpy_mn.capture_output(1, &output, &qh, ()));

    loop {
        sleep(Duration::new(1, 0));
        event_queue.blocking_dispatch(&mut data).unwrap();
    }
}
