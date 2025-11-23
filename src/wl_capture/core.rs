use std::{
    error::Error,
    sync::{Arc, Barrier},
};

use wayland_client::Connection;
use wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_manager_v1::Options;

use crate::wl_capture::dispatch::ApplicationState;

fn main(thread_barrier: Arc<Barrier>) -> Result<(), Box<dyn Error>> {
    let mut application_state = ApplicationState::default();

    // Connect to the compositor and get the associated display
    let compositor_connection = Connection::connect_to_env()?;
    let display = compositor_connection.display();

    // Create the event queue
    let mut event_queue = compositor_connection.new_event_queue();
    let queue_handle = event_queue.handle();
    let _registry = display.get_registry(&queue_handle, ());

    event_queue.roundtrip(&mut application_state)?;

    // Now we wait to receive information about the screen size.
    while application_state.capture_height.is_none() || application_state.capture_width.is_none() {
        event_queue.blocking_dispatch(&mut application_state)?;
    }

    // Send the information back to main thread
    // [TODO]

    // Create the session
    {
        let source_manager = match application_state.copy_capture_source_manager.as_ref() {
            Some(t) => t,
            None => panic!(),
        };

        let copy_manager = match application_state.copy_capture_manager.as_ref() {
            Some(t) => t,
            None => panic!(),
        };

        let output = match application_state.wl_output.as_ref() {
            Some(t) => t,
            None => panic!(),
        };

        let source = source_manager.create_source(output, &queue_handle, ());
        let session =
            copy_manager.create_session(&source, Options::PaintCursors, &queue_handle, ());
        let frame = session.create_frame(&queue_handle, ());

        application_state.copy_capture_session = Some(session);
        application_state.copy_capture_frame = Some(frame);
    }

    // Wait until all system are ready.
    thread_barrier.wait();

    // Main event loop.
    loop {
        // Flush remaining outgoing events to the server
        event_queue.flush()?;

        // Check if we have wayland events to process.
        if let Some(read_guard) = event_queue.prepare_read() {
            // poll until the socket is ready
            // [TODO]

            read_guard.read()?;
            event_queue.dispatch_pending(&mut application_state)?;

            // if it's not ready, drop the guard
            //std::mem::drop(read_guard);
        }

        // Capture an image if needed
        {}

        // Do something else
    }

    Ok(())
}

pub fn start(thread_barrier: Arc<Barrier>) {
    if let Ok(()) = main(thread_barrier) {}
}
