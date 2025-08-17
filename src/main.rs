use std::sync::{Arc, Barrier, mpsc};
use std::thread;

mod wl_capture;

#[derive(Default)]
pub(crate) struct ScreenDimension {
    pub height: i32,
    pub width: i32,
}

fn main() {
    // we want all thread to start together, so when they are producing data, other are also ready to read those.
    let start_barrier = Arc::new(Barrier::new(4));
    let screen_dimension: ScreenDimension;

    {
        let (screensize_tx, screensize_rx) = mpsc::channel::<ScreenDimension>();
        let thread_barrier = start_barrier.clone();
        thread::spawn(|| {
            wl_capture::start(screensize_tx, thread_barrier);
        });
        screen_dimension = screensize_rx.recv().unwrap();
        // after this, receiver is dropped, so the channel should close
    }

    // here for now.
    println!("{}", screen_dimension.height);

    start_barrier.wait();
}
