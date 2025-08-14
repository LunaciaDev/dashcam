use std::os::fd::{AsFd, OwnedFd};

use nix::{sys::memfd::{memfd_create, MFdFlags}, unistd::ftruncate};

pub fn create_fd(size: i32) -> OwnedFd {
    let file_descriptor = memfd_create("wl_buf", MFdFlags::empty()).unwrap();

    // truncate our file to the needed size
    ftruncate(file_descriptor.as_fd(), size as i64).unwrap();

    file_descriptor
}