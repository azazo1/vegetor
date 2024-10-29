use crate::editor::terminal::Size;
use std::io;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IOError occurred: {0:?}")]
    IOError(#[from] io::Error),
    #[error("Buffer size {buffer_size:?} exceeds the display area size {area_size:?}.")]
    BufferSizeExceeds { buffer_size: Size, area_size: Size },
    #[error("Carpet out of range.")]
    CarpetOutOfRange,
}

pub type Result<T> = std::result::Result<T, Error>;