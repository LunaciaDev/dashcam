use wayland_client::{
    Connection, Dispatch, QueueHandle,
    protocol::{wl_buffer::WlBuffer, wl_output::WlOutput, wl_registry},
};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1::{self, ZwlrScreencopyFrameV1},
    zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

struct Data {
    wlr_screencopy_manager: Option<ZwlrScreencopyManagerV1>,
    wl_output: Option<WlOutput>,
    screencopy_frame: Option<ZwlrScreencopyFrameV1>,
    screencopy_buffer: Option<WlBuffer>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for Data {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        data: &(),
        _: &Connection,
        queue_handle: &QueueHandle<Data>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            // we depend directly on v3 of screencopy
            if interface == "zwlr_screencopy_manager_v1" && version == 3 {
                state.wlr_screencopy_manager =
                    Some(registry.bind::<ZwlrScreencopyManagerV1, _, _>(
                        name,
                        version,
                        queue_handle,
                        *data,
                    ));
            }

            if interface == "wl_output" {
                state.wl_output =
                    Some(registry.bind::<WlOutput, _, _>(name, version, queue_handle, *data));
            }
        }
    }
}

impl Dispatch<WlOutput, ()> for Data {
    fn event(
        _: &mut Self,
        _: &WlOutput,
        _: <WlOutput as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // we are also not interested in any of their event yet.
    }
}

impl Dispatch<ZwlrScreencopyManagerV1, ()> for Data {
    fn event(
        _: &mut Self,
        _: &ZwlrScreencopyManagerV1,
        _: <ZwlrScreencopyManagerV1 as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // this interface has to event to handle.
    }
}

impl Dispatch<ZwlrScreencopyFrameV1, ()> for Data {
    fn event(
        state: &mut Self,
        proxy: &ZwlrScreencopyFrameV1,
        event: <ZwlrScreencopyFrameV1 as wayland_client::Proxy>::Event,
        data: &(),
        conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_screencopy_frame_v1::Event::Buffer {
                format,
                width,
                height,
                stride,
            } => {
                println!("Buffer event called");
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
                println!("Damage event called");
            }

            zwlr_screencopy_frame_v1::Event::LinuxDmabuf {
                format,
                width,
                height,
            } => {
                println!("LinuxDmabuf event called");
            }

            zwlr_screencopy_frame_v1::Event::BufferDone => {
                println!("Bufferdone event called")
            }

            zwlr_screencopy_frame_v1::Event::Ready {
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
            } => {
                println!("Ready event called");
            }

            zwlr_screencopy_frame_v1::Event::Failed => {
                println!("Failed event called");
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
    let mut data: Data = Data {
        wlr_screencopy_manager: None,
        wl_output: None,
        screencopy_frame: None,
        screencopy_buffer: None,
    };

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

    loop {
        let scrpy_mn = data.wlr_screencopy_manager.as_ref().unwrap();
        let output = data.wl_output.as_ref().unwrap();

        data.screencopy_frame = Some(scrpy_mn.capture_output(1, &output, &qh, ()));


        // [TODO]: Figure out the event loop.
        // Roundtrip is synchronized with the compositor, but this is slower
        // than async calls.
        event_queue.roundtrip(&mut data).unwrap();
    }
}
