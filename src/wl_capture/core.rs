use std::{
    error::Error,
    sync::{Arc, Barrier},
};

use wayland_client::{Connection, QueueHandle};
use wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_manager_v1::Options;

use crate::wl_capture::{dispatch::ApplicationState, shm_pool::BufferPool, utils::get_wl_object};

fn start_capture_frame(
    application_state: &mut ApplicationState,
    qh: &QueueHandle<ApplicationState>,
    buffer_pool: &mut BufferPool,
) {
    // Request to capture a new frame
    let capture_session = get_wl_object!(application_state.copy_capture_session);
    let capture_frame = capture_session.create_frame(qh, ());
    let buffer_width = *get_wl_object!(application_state.buffer_width) as i32;
    let buffer_height = *get_wl_object!(application_state.buffer_height) as i32;
    // [TODO]: Figure out how to calculate stride since the protocol does not send that information anymore
    let buffer_stride = buffer_width * 4;
    let buffer_format = get_wl_object!(application_state.buffer_type);
    let wl_shm = get_wl_object!(application_state.wl_shm);

    let buffer = buffer_pool
        .get_buffer(
            buffer_width,
            buffer_height,
            buffer_stride,
            *buffer_format,
            wl_shm,
            qh,
        )
        .unwrap();

    application_state.buffer_raw = Some(buffer.1);
    capture_frame.attach_buffer(buffer.0);
}

fn main(thread_barrier: Arc<Barrier>) -> Result<(), Box<dyn Error>> {
    let mut application_state = ApplicationState::default();

    // Connect to the compositor and get the associated display
    let compositor_connection = Connection::connect_to_env()?;
    let display = compositor_connection.display();
    let mut buffer_pool = BufferPool::new(8);

    // Create the event queue
    let mut event_queue = compositor_connection.new_event_queue();
    let queue_handle = event_queue.handle();
    let _registry = display.get_registry(&queue_handle, ());

    event_queue.roundtrip(&mut application_state)?;

    // Now we wait to receive information about the screen size.
    while application_state.capture_config.is_none() {
        event_queue.blocking_dispatch(&mut application_state)?;
    }

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

    event_queue.roundtrip(&mut application_state)?;

    // Send the information back to main thread
    // [TODO]

    // Wait until all system are ready.
    thread_barrier.wait();

    // Main event loop.
    loop {
        // Send all message and dispatch callbacks for all received messages
        event_queue.flush()?;
        event_queue.dispatch_pending(&mut application_state)?;

        // Check if we needed to capture a new frame
        if application_state.copy_capture_frame.is_none() && buffer_pool.has_free_buffer() {
            start_capture_frame(&mut application_state, &queue_handle, &mut buffer_pool);
        }
    }

    Ok(())
}

pub fn start(thread_barrier: Arc<Barrier>) {
    if let Ok(()) = main(thread_barrier) {}
}
