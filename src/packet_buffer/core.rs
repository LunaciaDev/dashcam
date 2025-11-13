use std::{collections::VecDeque, sync::{mpsc::Receiver, Arc, Barrier}};

use crate::{ffmpeg_encoder::encoder_ffi::Packet, utils::{is_halt, set_halt}};

pub fn start(barrier: Arc<Barrier>, packet_rx: Receiver<Packet>) {
    // [TODO]: Figure out how many packet we need to reserve beforehand.
    // 32 sounds like a good start?
    let mut buffer: VecDeque<Packet> = VecDeque::with_capacity(32);

    barrier.wait();

    while !is_halt() {
        let packet = match packet_rx.recv() {
            Ok(r) => r,
            Err(_) => {
                set_halt();
                break;
            }
        };

        let last_pts = packet.pts;

        buffer.push_back(packet);

        // pts is 1/60 s
        while let Some(buf) = buffer.front() {
            if last_pts - buf.pts > 1800 {
                buffer.pop_front();
            }
        }
    }
}