use std::{
    num::NonZeroUsize,
    os::{
        fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd},
        raw::c_void,
    },
    ptr::NonNull,
};

use nix::{
    libc::{POLLIN, PROT_READ, PROT_WRITE, poll, pollfd},
    sys::{
        memfd::{MFdFlags, memfd_create},
        mman::{MapFlags, ProtFlags},
    },
    unistd::ftruncate,
};

pub fn create_fd(size: i32) -> OwnedFd {
    let file_descriptor = memfd_create("wl_buf", MFdFlags::empty()).unwrap();

    // truncate our file to the needed size
    ftruncate(file_descriptor.as_fd(), size as i64).unwrap();

    file_descriptor
}

pub fn poll_fd(fd: BorrowedFd) -> bool {
    unsafe {
        let mut fds = [pollfd {
            fd: fd.as_raw_fd(),
            events: POLLIN,
            revents: 0,
        }];

        let res = poll(fds.as_mut_ptr(), 1, 0);

        res > 0
    }
}

pub fn create_mmap(size: usize, fd: BorrowedFd) -> Option<NonNull<c_void>> {
    unsafe {
        let mmap_result = nix::sys::mman::mmap(
            None,
            NonZeroUsize::new(size).unwrap(),
            ProtFlags::from_bits_truncate(PROT_READ | PROT_WRITE),
            MapFlags::MAP_SHARED,
            fd.as_fd(),
            0,
        );

        mmap_result.ok()
    }
}

pub fn destroy_mmap(size: usize, ptr: NonNull<c_void>) {
    unsafe {
        nix::sys::mman::munmap(ptr, size).ok();
    }
}