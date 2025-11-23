use std::{
    error::Error,
    sync::{Arc, Barrier},
};

use wayland_client::Connection;
use wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_manager_v1::Options;

use crate::wl_capture::{dispatch::ApplicationState, utils::get_wl_object};

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
    while application_state.capture_config.is_none() {
        event_queue.blocking_dispatch(&mut application_state)?;
    }

    // Send the information back to main thread
    // [TODO]

    // Create the session
    {
        let source_manager = get_wl_object!(application_state.copy_capture_source_manager);
        let copy_manager = get_wl_object!(application_state.copy_capture_manager);
        let output = get_wl_object!(application_state.wl_output);

        let source = source_manager.create_source(output, &queue_handle, ());
        let session =
            copy_manager.create_session(&source, Options::PaintCursors, &queue_handle, ());

        application_state.copy_capture_session = Some(session);
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
        if application_state.copy_capture_frame.is_none() {
            let copy_session = get_wl_object!(application_state.copy_capture_session);

            let capture_frame = copy_session.create_frame(&queue_handle, ());
        }

        // Do something else
    }

    Ok(())
}

pub fn start(thread_barrier: Arc<Barrier>) {
    if let Ok(()) = main(thread_barrier) {}
}
