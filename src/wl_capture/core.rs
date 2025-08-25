use crate::{
    ScreenDimension,
    ffmpeg_encoder::FrameInfo,
    utils::{is_halt, set_halt},
    wl_capture::{error::WlObjectError, events::Data, utils::poll_fd},
};
use std::{
    error::Error,
    os::fd::AsFd,
    sync::{
        Arc, Barrier,
        mpsc::{Receiver, Sender},
    },
    thread::sleep,
    time::Duration,
};
use wayland_client::Connection;

pub fn main(
    screensize_tx: Sender<ScreenDimension>,
    wlpacket_tx: Sender<FrameInfo>,
    wlresponse_rx: Receiver<u8>,
    thread_barrier: Arc<Barrier>,
) -> Result<(), Box<dyn Error>> {
    let conn = Connection::connect_to_env()?;
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

    event_queue.roundtrip(&mut data)?;

    if data.zwlr_screencopy_manager.is_none() {
        return Err(Box::new(WlObjectError::MissingWlrScreencopy));
    }

    if data.wl_output.is_none() {
        return Err(Box::new(WlObjectError::MissingWlOutput));
    }

    if data.wl_shm.is_none() {
        return Err(Box::new(WlObjectError::MissingWlShm));
    }

    loop {
        // we hold the initialization loop until we learned about the screen dimensions.
        event_queue.blocking_dispatch(&mut data)?;

        if data.screen_height != -1 && data.screen_width != -1 {
            break;
        }
    }

    screensize_tx
        .send(ScreenDimension {
            height: data.screen_height,
            width: data.screen_width,
        })
        .expect("The receiver on main thread only deconstruct this after receiving the info.");

    thread_barrier.wait();

    while !is_halt() {
        // Send all queued event to the compositor.
        event_queue.flush()?;

        let read_guard = match event_queue.prepare_read() {
            Some(s) => s,
            None => {
                // call dispatch_pending before invoking again.
                event_queue.dispatch_pending(&mut data)?;
                continue;
            }
        };
        let read_fd = read_guard.connection_fd();
        let wl_socket_ready = poll_fd(read_fd.as_fd());

        if wl_socket_ready {
            // consume the read guard
            read_guard.read()?;
            event_queue.dispatch_pending(&mut data)?;
        } else {
            drop(read_guard);
        }

        // now that we have checked the Wayland Event Loop, we can do other stuff.

        // Receive all data from the other thread.
        // For each received packet, release the buffer associated
        while wlresponse_rx.try_recv().is_ok() {
            data.buffer_pool.bump_unavailable(1);
        }

        if data.zwlr_screencopy_frame.is_some() || !data.buffer_pool.has_free_buffer() {
            sleep(Duration::from_millis(1));
            continue;
        }

        //we are ready to capture something new
        let screencopy_manager = data
            .zwlr_screencopy_manager
            .as_ref()
            .expect("The zwlr_screencopy_manager object cannot be None.");
        let output = data
            .wl_output
            .as_ref()
            .expect("The wl_output object cannot be None.");
        data.zwlr_screencopy_frame = Some(screencopy_manager.capture_output(1, output, &qh, ()));
    }

    drop(data.buffer_send_channel.expect("The send channel cannot be None."));
    data.wl_output
        .expect("The wl_output object cannot be None")
        .release();
    data.wl_shm
        .expect("The wl_shm object cannot be None")
        .release();
    data.zwlr_screencopy_manager
        .expect("The zwlr_screencopy_manager object cannot be None")
        .destroy();
    data.zwlr_screencopy_frame.inspect(|frame| {
        frame.destroy();
    });

    // wait until the other thread has released all buffer
    loop {
        match wlresponse_rx.try_recv() {
            Ok(_) => data.buffer_pool.bump_unavailable(1),
            Err(e) => match e {
                std::sync::mpsc::TryRecvError::Empty => {},
                // if this trigger, we can be certain that the other thread will not be touching shared memory.
                std::sync::mpsc::TryRecvError::Disconnected => break,
            },
        }

        sleep(Duration::from_millis(50));
    }

    data.buffer_pool.destroy();

    // everything is cleaned up!

    // we might have an error from the event system.
    match data.event_error {
        Some(err) => Err(err),
        None => Ok(()),
    }
}

pub fn start(
    screensize_tx: Sender<ScreenDimension>,
    wlpacket_tx: Sender<FrameInfo>,
    wlresponse_rx: Receiver<u8>,
    thread_barrier: Arc<Barrier>,
) {
    match main(screensize_tx, wlpacket_tx, wlresponse_rx, thread_barrier) {
        Ok(_) => {}
        Err(error) => {
            eprintln!("{}", error);
            set_halt();
        }
    }
}
