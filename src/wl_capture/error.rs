use std::{error::Error, fmt::Display};

use wayland_client::protocol::wl_shm::Format;

#[derive(Debug)]
pub enum WlObjectError {
    MissingWlrScreencopy,
    MissingWlOutput,
    MissingWlShm,
    MismatchedWlrScreencopyVersion(u32),
}

#[derive(Debug)]
pub enum FormatError {
    UnsupportedFormat(Format),
    UndetectedFormat(u32)
}

impl Display for WlObjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WlObjectError::MissingWlrScreencopy => {
                write!(f, "The compositor does not support wlr_screencopy.")
            },
            WlObjectError::MissingWlOutput => {
                write!(f, "The compositor does not support wl_output.")
            },
            WlObjectError::MissingWlShm => {
                write!(f, "The compositor does not support wl_shm")
            },
            WlObjectError::MismatchedWlrScreencopyVersion(version) => {
                write!(f, "Expected wlr_screencopy version 3, got version: {version}.")
            }
        }
    }
}

impl Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatError::UnsupportedFormat(format) => {
                write!(f, "The pixel format {format:?} is not yet supported.")
            }
            FormatError::UndetectedFormat(id) => {
                write!(f, "The pixel format id {id} is not recognized.")
            }
        }
    }
}

impl Error for WlObjectError {}
impl Error for FormatError {}