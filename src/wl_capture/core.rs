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

    thread_barrier.wait();

    Ok(())
}

pub fn start(thread_barrier: Arc<Barrier>) {
    if let Ok(()) = main(thread_barrier) {}
}
