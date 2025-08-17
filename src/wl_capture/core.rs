use crate::{ScreenDimension, wl_capture::events::Data};
use std::sync::{Arc, Barrier, mpsc::Sender};
use wayland_client::Connection;

pub fn start(screensize_tx: Sender<ScreenDimension>, thread_barrier: Arc<Barrier>) {
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

    screensize_tx
        .send(ScreenDimension {
            height: data.screen_height,
            width: data.screen_width,
        })
        .unwrap();

    thread_barrier.wait();

    loop {
        {
            let scrpy_mn = data.zwlr_screencopy_manager.as_ref().unwrap();
            let output = data.wl_output.as_ref().unwrap();
            data.zwlr_screencopy_frame = Some(scrpy_mn.capture_output(1, output, &qh, ()));
        }

        event_queue.blocking_dispatch(&mut data).unwrap();
    }
}
