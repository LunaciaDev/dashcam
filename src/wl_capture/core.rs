use std::{
    error::Error,
    sync::{Arc, Barrier},
};

use wayland_client::Connection;

use crate::wl_capture::wayland_event_handler::ApplicationState;

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

    // Do a quick check if we do have all object needed
    // This check is for initialization - Always check the object, as it may get removed during runtime.

    if application_state.wl_output.is_none() {
        panic!();
    }

    if application_state.wl_shm.is_none() {
        panic!();
    }

    if application_state.copy_capture_manager.is_none() {
        panic!();
    }

    // Now we wait to receive information about the screen size.
    while application_state.frame_height.is_none() || application_state.frame_width.is_none() {
        event_queue.blocking_dispatch(&mut application_state)?;
    }

    // Send the information back to main thread
    // [TODO]

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

        // Do something else
    }

    Ok(())
}

pub fn start(thread_barrier: Arc<Barrier>) {
    if let Ok(()) = main(thread_barrier) {}
}
