use std::{
    num::NonZeroUsize,
    os::{
        fd::{AsFd, BorrowedFd, OwnedFd},
        raw::c_void,
    },
    ptr::NonNull,
};

use nix::{
    libc::{PROT_READ, PROT_WRITE},
    sys::{
        memfd::{MFdFlags, memfd_create},
        mman::{MapFlags, ProtFlags},
    },
    unistd::ftruncate,
};
use wayland_client::protocol::wl_shm::Format;

use crate::BufferConfig;

pub fn create_fd(size: i32) -> OwnedFd {
    let file_descriptor = memfd_create("wl_buf", MFdFlags::empty()).unwrap();

    // truncate our file to the needed size
    ftruncate(file_descriptor.as_fd(), size as i64).unwrap();

    file_descriptor
}

pub fn create_mmap(size: usize, fd: BorrowedFd) -> Option<NonNull<c_void>> {
    unsafe {
        let mmap_result = nix::sys::mman::mmap(
            None,
            NonZeroUsize::new(size as usize).unwrap(),
            ProtFlags::from_bits_truncate(PROT_READ | PROT_WRITE),
            MapFlags::MAP_SHARED,
            fd.as_fd(),
            0,
        );

        match mmap_result {
            Ok(ptr) => Some(ptr),
            Err(_) => None,
        }
    }
}

#[derive(Debug)]
pub struct RGBAPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

pub fn extract_frames_from_mmap(
    ptr: &NonNull<c_void>,
    frame_config: &BufferConfig,
) -> Vec<Vec<RGBAPixel>> {
    let format = frame_config.format;
    let width = frame_config.width as usize;
    let height = frame_config.height as usize;
    let stride = frame_config.stride as usize;
    let ptr: *const u8 = ptr.as_ptr() as *const u8;

    let mut pixel_vec: Vec<Vec<RGBAPixel>> = Vec::new();

    unsafe {
        for y in 0..height {
            pixel_vec.push(Vec::new());

            let row_ptr = ptr.add(stride * y);
            let row_vec = &mut pixel_vec[y];

            for x in 0..width {
                // Check out the enum definition for the byte layout
                match format {
                    Format::Argb8888 => {
                        let col_ptr = row_ptr.add(x * 4);

                        row_vec.push(RGBAPixel {
                            a: *(col_ptr.add(3)),
                            r: *(col_ptr.add(2)),
                            g: *(col_ptr.add(1)),
                            b: *(col_ptr),
                        });
                    }

                    Format::Xrgb8888 => {
                        let col_ptr = row_ptr.add(x * 4);

                        row_vec.push(RGBAPixel {
                            a: 0xFF,
                            r: *(col_ptr.add(2)),
                            g: *(col_ptr.add(1)),
                            b: *(col_ptr),
                        });
                    }

                    Format::Xbgr8888 => {
                        let col_ptr = row_ptr.add(x * 4);

                        row_vec.push(RGBAPixel {
                            a: 0xFF,
                            b: *(col_ptr.add(2)),
                            g: *(col_ptr.add(1)),
                            r: *(col_ptr),
                        });
                    }

                    _ => {
                        panic!("Unsupported format!");
                    }
                };
            }
        }
    }

    return pixel_vec;
}
