use std::{
    error::Error,
    sync::{
        Arc, Barrier,
        mpsc::{Receiver, Sender},
    },
};

use crate::{ScreenDimension, ffmpeg_encoder::FrameInfo, utils::set_halt};

use wayland_client::Connection;
// We switched from zwlr_screencopy to ext_image_screencopy_manager as the former has been deprecated.
use wayland_protocols::ext::image_copy_capture::v1::client;

fn main(
    screensize_tx: Sender<ScreenDimension>,
    wlpacket_tx: Sender<FrameInfo>,
    wlresponse_rx: Receiver<u8>,
    thread_barrier: Arc<Barrier>,
) -> Result<(), Box<dyn Error>> {
    let compositor_connection = Connection::connect_to_env()?;
    let display = compositor_connection.display();

    Ok(())
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
