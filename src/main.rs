use std::{thread, sync::{Arc, Barrier}};

mod wl_capture;

fn main() {
    let encode_thread_handle;
    let launch_barrier = Arc::new(Barrier::new(2));

    // Launch wl screen captures
    {
        let thread_launch_barrier = launch_barrier.clone();
        encode_thread_handle = thread::spawn(|| {
            wl_capture::start(thread_launch_barrier);
        });
    }

    launch_barrier.wait();

    encode_thread_handle.join().unwrap();
}