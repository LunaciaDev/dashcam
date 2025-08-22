use crate::{ffmpeg_encoder::FrameInfo, wl_capture::{events::Data, utils::poll_fd}, ScreenDimension};
use std::{os::fd::AsFd, sync::{
    mpsc::{Receiver, Sender}, Arc, Barrier
}};
use wayland_client::Connection;

pub fn start(
    screensize_tx: Sender<ScreenDimension>,
    wlpacket_tx: Sender<FrameInfo>,
    wlresponse_rx: Receiver<u8>,
    thread_barrier: Arc<Barrier>,
) {
    let conn = Connection::connect_to_env().unwrap();
    let display = conn.display();
    let mut data: Data = Data {
        buffer_send_channel: Some(wlpacket_tx),
        screen_height: -1,
        screen_width: -1,
        ..Default::default()
    };

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
        // we hold the initialization loop until we learned about the screen dimensions.
        event_queue.blocking_dispatch(&mut data).unwrap();
        if data.screen_height != -1 && data.screen_width != -1 {
            break;
        }
    }

    screensize_tx
        .send(ScreenDimension {
            height: data.screen_height,
            width: data.screen_width,
        })
        .unwrap();

    thread_barrier.wait();

    loop {
        // Send all queued event to the compositor.
        event_queue.flush().unwrap();

        let read_guard = event_queue.prepare_read().unwrap();
        let read_fd = read_guard.connection_fd();

        let wl_socket_ready = poll_fd(read_fd.as_fd());

        if wl_socket_ready {
            // consume the read guard
            read_guard.read().unwrap();
            event_queue.dispatch_pending(&mut data).unwrap();
        }
        else {
            drop(read_guard);
        }

        // now that we have checked the Wayland Event Loop, we can do other stuff.

        // Receive all data from the other thread.
        // For each received packet, release the buffer associated
        while wlresponse_rx.try_recv().is_ok() {
            data.buffer_pool.bump_unavailable();
        }

        // if we are ready to capture something new
        if data.zwlr_screencopy_frame.is_none() && data.buffer_pool.has_free_buffer() {
            let screencopy_manager = data.zwlr_screencopy_manager.as_ref().unwrap();
            let output = data.wl_output.as_ref().unwrap();
            data.zwlr_screencopy_frame =
                Some(screencopy_manager.capture_output(1, output, &qh, ()));
        }
    }
}
