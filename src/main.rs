use std::error::Error;
use std::sync::{Arc, Barrier, mpsc};
use std::thread::{self, sleep};
use std::time::Duration;

use crate::ffmpeg_encoder::FrameInfo;
use crate::utils::{is_halt, HALT_FLAG};

mod ffmpeg_encoder;
mod utils;
mod wl_capture;

#[derive(Default, Copy, Clone)]
pub(crate) struct ScreenDimension {
    pub height: i32,
    pub width: i32,
}

fn launch() -> Result<(), Box<dyn Error>> {
    // we want all thread to start together, so when they are producing data, other are also ready to read those.
    let start_barrier = Arc::new(Barrier::new(3));
    let screen_dimension: ScreenDimension;
    let encode_thread_handle;
    let capture_thread_handle;

    let (wlpacket_tx, wlpacket_rx) = mpsc::channel::<FrameInfo>();
    let (wlresponse_tx, wlresponse_rx) = mpsc::channel::<u8>();

    // SIGINT handler to halt the program
    signal_hook::flag::register(signal_hook::consts::SIGINT, HALT_FLAG.clone())?;

    {
        let (screensize_tx, screensize_rx) = mpsc::channel::<ScreenDimension>();
        let thread_barrier = start_barrier.clone();
        capture_thread_handle = thread::spawn(|| {
            wl_capture::start(
                screensize_tx,
                wlpacket_tx,
                wlresponse_rx,
                thread_barrier,
            );
        });
        screen_dimension = match screensize_rx.recv() {
            Ok(s) => s,
            Err(_) => {
                // the capturing thread crashed
                return Ok(());
            },
        };
        // after this, receiver is dropped, so the channel should close
    }

    {
        let screen_dimension_copy = screen_dimension;
        let thread_barrier = start_barrier.clone();
        encode_thread_handle = thread::spawn(move || {
            ffmpeg_encoder::start(
                screen_dimension_copy.width,
                screen_dimension_copy.height,
                wlpacket_rx,
                wlresponse_tx,
                thread_barrier,
            );
        });
    }

    start_barrier.wait();

    loop {
        if is_halt() {
            break;
        }

        sleep(Duration::from_millis(100));
    }

    capture_thread_handle.join().unwrap();
    encode_thread_handle.join().unwrap();

    Ok(())
}

fn main() {
    // [TODO]: Create a CLI for this.

    match launch() {
        Ok(_) => {},
        Err(err) => {
            eprintln!("{err}");
        },
    }
}
