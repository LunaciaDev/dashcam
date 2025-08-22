use std::sync::{Arc, Barrier, mpsc};
use std::thread::{self, sleep};
use std::time::Duration;

use crate::ffmpeg_encoder::FrameInfo;

mod ffmpeg_encoder;
mod wl_capture;

#[derive(Default, Copy, Clone)]
pub(crate) struct ScreenDimension {
    pub height: i32,
    pub width: i32,
}

fn main() {
    // we want all thread to start together, so when they are producing data, other are also ready to read those.
    let start_barrier = Arc::new(Barrier::new(3));
    let screen_dimension: ScreenDimension;

    let (wlpacket_tx, wlpacket_rx) = mpsc::channel::<FrameInfo>();
    let (wlresponse_tx, wlresponse_rx) = mpsc::channel::<u8>();

    {
        let (screensize_tx, screensize_rx) = mpsc::channel::<ScreenDimension>();
        let thread_barrier = start_barrier.clone();
        thread::spawn(|| {
            wl_capture::start(screensize_tx, wlpacket_tx, wlresponse_rx, thread_barrier);
        });
        screen_dimension = screensize_rx.recv().unwrap();
        // after this, receiver is dropped, so the channel should close
    }

    {
        let screen_dimension_copy = screen_dimension;
        let thread_barrier = start_barrier.clone();
        thread::spawn(move || {
            ffmpeg_encoder::start(
                screen_dimension_copy.width,
                screen_dimension_copy.height,
                wlpacket_rx,
                wlresponse_tx,
                thread_barrier
            );
        });
    }

    start_barrier.wait();

    // [TODO]: Poll for keyboard event, if we receive an event, interrupt, close all thread and exit.
    loop {
        sleep(Duration::new(5, 0));
    }
}
