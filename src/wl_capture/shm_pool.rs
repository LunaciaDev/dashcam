use std::{
    os::{fd::AsFd, raw::c_void},
    ptr::NonNull,
};

use wayland_client::{
    QueueHandle,
    protocol::{
        wl_buffer::WlBuffer,
        wl_shm::{Format, WlShm},
    },
};

use crate::wl_capture::{
    events::Data,
    utils::{create_fd, create_mmap, destroy_mmap},
};

pub struct ManagedBufferPool {
    buffers: Vec<Buffer>,

    size: usize,
    available_ptr: usize,
    unavailable_ptr: usize,
    initialized: bool,
}

impl Default for ManagedBufferPool {
    fn default() -> Self {
        ManagedBufferPool::new(8)
    }
}

#[derive(Default)]
pub struct Buffer {
    pub raw_data: Option<NonNull<c_void>>,
    pub wl_buffer: Option<WlBuffer>,
    pub format: Option<Format>,
    pub size: i32,
    pub height: i32,
    pub width: i32,
    pub stride: i32,
}

impl ManagedBufferPool {
    pub fn new(size: usize) -> ManagedBufferPool {
        let mut vec = Vec::with_capacity(size);
        vec.resize_with(size, Buffer::default);

        ManagedBufferPool {
            buffers: vec,
            available_ptr: 0,
            unavailable_ptr: 0,
            size,
            initialized: false,
        }
    }

    pub fn has_free_buffer(&self) -> bool {
        self.available_ptr != self.unavailable_ptr || !self.initialized
    }

    pub fn bump_unavailable(&mut self) {
        self.unavailable_ptr = (self.unavailable_ptr + 1) % self.size;
    }

    pub fn get_buffer(
        &mut self,
        frame_width: i32,
        frame_height: i32,
        frame_stride: i32,
        frame_format: Format,
        wl_shm: &WlShm,
        qhandle: &QueueHandle<Data>,
    ) -> Result<(&WlBuffer, NonNull<c_void>), i32> {
        if !self.has_free_buffer() {
            return Err(0);
        }

        let buffer = &mut self.buffers[self.available_ptr];

        // if the buffer has yet to be allocated, the format is not matching or the size is not matching, allocate.
        if buffer.wl_buffer.is_none()
            || buffer.format.unwrap() != frame_format
            || buffer.size != frame_stride * frame_height
        {
            let size = frame_stride * frame_height;
            let fd = create_fd(size);
            let pool = wl_shm.create_pool(fd.as_fd(), size, qhandle, ());

            if buffer.wl_buffer.is_some() {
                // we already allocated something, so need to do proper deallocation.
                buffer.wl_buffer.as_ref().unwrap().destroy();
                destroy_mmap(buffer.size as usize, buffer.raw_data.unwrap());
            }

            buffer.wl_buffer = Some(pool.create_buffer(
                0,
                frame_width,
                frame_height,
                frame_stride,
                frame_format,
                qhandle,
                (),
            ));
            buffer.raw_data = Some(create_mmap(size as usize, fd.as_fd()).unwrap());
            buffer.format = Some(frame_format);
            buffer.height = frame_height;
            buffer.width = frame_width;
            buffer.stride = frame_stride;
            buffer.size = size;

            pool.destroy();
        }

        if !self.initialized {
            self.initialized = true;
        }

        self.available_ptr = (self.available_ptr + 1) % self.size;

        Ok((
            buffer.wl_buffer.as_ref().unwrap(),
            *buffer.raw_data.as_ref().unwrap(),
        ))
    }
}
