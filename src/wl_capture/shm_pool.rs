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
    sys::{memfd::{MFdFlags, memfd_create}, mman::{MapFlags, ProtFlags}}, unistd::ftruncate,
};
use wayland_client::{
    QueueHandle,
    protocol::{
        wl_buffer::WlBuffer,
        wl_shm::{Format, WlShm},
    },
};

use crate::wl_capture::dispatch::ApplicationState;

pub struct BufferPool {
    pub buffer_pool: Vec<Option<Buffer>>,
    pub size: usize,
    pub dirty_size: usize,
    pub current_ptr: usize,
}

pub struct Buffer {
    pub backing_data: NonNull<c_void>,
    pub wl_buffer: WlBuffer,
    pub format: Format,
    pub size: i32,
}

impl BufferPool {
    pub fn new(size: usize) -> BufferPool {
        let pool = Vec::with_capacity(size);

        BufferPool {
            buffer_pool: pool,
            size,
            dirty_size: 0,
            current_ptr: 0,
        }
    }

    pub fn has_free_buffer(&self) -> bool {
        self.dirty_size != self.size
    }

    pub fn bump_unavailable(&mut self, amount: usize) {
        self.dirty_size -= amount;
    }

    pub fn destroy(&self) {
        for buffer in self.buffer_pool.iter().flatten() {
            buffer.wl_buffer.destroy();
            destroy_mmap(buffer.size as usize, buffer.backing_data);
        }
    }

    pub fn get_buffer(
        &mut self,
        width: i32,
        height: i32,
        stride: i32,
        format: Format,
        wl_shm: &WlShm,
        qh: &QueueHandle<ApplicationState>,
    ) -> Result<(&WlBuffer, NonNull<c_void>), ()> {
        if !self.has_free_buffer() {
            return Err(());
        }

        let size = stride * height;
        let buffer = &self.buffer_pool[self.current_ptr];

        if buffer.is_none() {
            // Allocate a new buffer
            let new_buffer = create_buffer(width, height, stride, format, wl_shm, qh);
            self.buffer_pool[self.current_ptr] = Some(new_buffer);
        }
        else {
            let buffer = buffer.as_ref().expect("Buffer must be of Some<T>");

            if buffer.format != format || buffer.size != size {
                deallocate_buffer(buffer);

                let new_buffer = create_buffer(width, height, stride, format, wl_shm, qh);
                self.buffer_pool[self.current_ptr] = Some(new_buffer);
            }
        }

        let buffer = self.buffer_pool[self.current_ptr].as_ref().expect("Buffer must be allocated by now");

        Ok((&buffer.wl_buffer, buffer.backing_data))
    }
}

fn deallocate_buffer(buffer: &Buffer) {
    buffer.wl_buffer.destroy();
    destroy_mmap(buffer.size as usize, buffer.backing_data);
}

fn create_buffer(
    width: i32,
    height: i32,
    stride: i32,
    format: Format,
    wl_shm: &WlShm,
    qh: &QueueHandle<ApplicationState>,
) -> Buffer {
    let fd = create_fd(height * stride);
    let backing_data = create_mmap((height * stride).try_into().unwrap(), fd.as_fd());
    let shm_pool = wl_shm.create_pool(fd.as_fd(), height * stride, qh, ());
    let wl_buffer = shm_pool.create_buffer(0, width, height, stride, format, qh, ());

    Buffer { backing_data, wl_buffer, format, size: height * stride }
}

fn create_fd(size: i32) -> OwnedFd {
    let file_descriptor = memfd_create("wl_buf", MFdFlags::empty()).unwrap();

    ftruncate(file_descriptor.as_fd(), size as i64).unwrap();

    file_descriptor
}

fn create_mmap(size: usize, fd: BorrowedFd) -> NonNull<c_void> {
    unsafe {
        let mmap_result = nix::sys::mman::mmap(
            None,
            NonZeroUsize::new(size).unwrap(),
            ProtFlags::from_bits_truncate(PROT_READ | PROT_WRITE),
            MapFlags::MAP_SHARED,
            fd.as_fd(),
            0,
        );

        mmap_result.ok().unwrap()
    }
}

fn destroy_mmap(size: usize, ptr: NonNull<c_void>) {
    unsafe {
        nix::sys::mman::munmap(ptr, size).ok();
    }
}
