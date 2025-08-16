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
                // We will be supporting 8-bit RGBA and variants only for now.
                // [TODO]: Check each Format for their equivalent in libav
                // There are matches - we need not do parsing if it is the case,
                // or be able to find "close enough" formats to coerce to
                // to reduce the parsing cost.
                match format {
                    // 64-bit RGBA/X 16:16:16:16 FP
                    Format::Xrgb16161616f => todo!(),
                    Format::Xbgr16161616f => todo!(),
                    Format::Argb16161616f => todo!(),
                    Format::Abgr16161616f => todo!(),

                    // 64-bit RGBA/X 16:16:16:16
                    Format::Xrgb16161616 => todo!(),
                    Format::Xbgr16161616 => todo!(),
                    Format::Argb16161616 => todo!(),
                    Format::Abgr16161616 => todo!(),

                    // 32-bit RGBA/X 8:8:8:8
                    Format::Rgbx8888 => {
                        let col_ptr = row_ptr.add(x * 4);

                        row_vec.push(RGBAPixel {
                            r: *(col_ptr.add(3)),
                            g: *(col_ptr.add(2)),
                            b: *(col_ptr.add(1)),
                            a: 0xFF,
                        });
                    }
                    Format::Bgrx8888 => {
                        let col_ptr = row_ptr.add(x * 4);

                        row_vec.push(RGBAPixel {
                            b: *(col_ptr.add(3)),
                            g: *(col_ptr.add(2)),
                            r: *(col_ptr.add(1)),
                            a: 0xFF,
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

                    Format::Rgba8888 => {
                        let col_ptr = row_ptr.add(x * 4);

                        row_vec.push(RGBAPixel {
                            r: *(col_ptr.add(3)),
                            g: *(col_ptr.add(2)),
                            b: *(col_ptr.add(1)),
                            a: *(col_ptr),
                        });
                    }
                    Format::Bgra8888 => {
                        let col_ptr = row_ptr.add(x * 4);

                        row_vec.push(RGBAPixel {
                            b: *(col_ptr.add(3)),
                            g: *(col_ptr.add(2)),
                            r: *(col_ptr.add(1)),
                            a: *(col_ptr),
                        });
                    }
                    Format::Argb8888 => {
                        let col_ptr = row_ptr.add(x * 4);

                        row_vec.push(RGBAPixel {
                            a: *(col_ptr.add(3)),
                            r: *(col_ptr.add(2)),
                            g: *(col_ptr.add(1)),
                            b: *(col_ptr),
                        });
                    }
                    Format::Abgr8888 => {
                        let col_ptr = row_ptr.add(x * 4);

                        row_vec.push(RGBAPixel {
                            a: *(col_ptr.add(3)),
                            b: *(col_ptr.add(2)),
                            g: *(col_ptr.add(1)),
                            r: *(col_ptr),
                        });
                    }

                    // 32-bit RGBA/X 10:10:10:2
                    Format::Xrgb2101010 => todo!(),
                    Format::Xbgr2101010 => todo!(),
                    Format::Rgbx1010102 => todo!(),
                    Format::Bgrx1010102 => todo!(),

                    Format::Argb2101010 => todo!(),
                    Format::Abgr2101010 => todo!(),
                    Format::Rgba1010102 => todo!(),
                    Format::Bgra1010102 => todo!(),

                    // 24-bit RGB 8:8:8
                    Format::Rgb888 => {
                        let col_ptr = row_ptr.add(x * 3);

                        row_vec.push(RGBAPixel {
                            r: *(col_ptr.add(2)),
                            g: *(col_ptr.add(1)),
                            b: *(col_ptr),
                            a: 0xFF,
                        });
                    }
                    Format::Bgr888 => {
                        let col_ptr = row_ptr.add(x * 3);

                        row_vec.push(RGBAPixel {
                            b: *(col_ptr.add(2)),
                            g: *(col_ptr.add(1)),
                            r: *(col_ptr),
                            a: 0xFF,
                        });
                    }

                    // 16-bit RGBA/X 4:4:4:4
                    Format::Rgbx4444 => todo!(),
                    Format::Bgrx4444 => todo!(),
                    Format::Xrgb4444 => todo!(),
                    Format::Xbgr4444 => todo!(),

                    Format::Rgba4444 => todo!(),
                    Format::Bgra4444 => todo!(),
                    Format::Argb4444 => todo!(),
                    Format::Abgr4444 => todo!(),

                    // 16-bit RGBA/X 5:5:5:1
                    Format::Rgbx5551 => todo!(),
                    Format::Bgrx5551 => todo!(),
                    Format::Xrgb1555 => todo!(),
                    Format::Xbgr1555 => todo!(),

                    Format::Rgba5551 => todo!(),
                    Format::Bgra5551 => todo!(),
                    Format::Argb1555 => todo!(),
                    Format::Abgr1555 => todo!(),

                    // 8-bit RGB 3:3:2
                    Format::Rgb332 => todo!(),
                    Format::Bgr233 => todo!(),

                    // 8-bit RGB 5:6:5
                    Format::Rgb565 => todo!(),
                    Format::Bgr565 => todo!(),

                    _ => {
                        panic!("Unsupported format!");
                    }
                };
            }
        }
    }

    return pixel_vec;
}
