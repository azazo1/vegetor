use crate::editor::Size;
use std::io;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IOError occurred: {0:?}")]
    IOError(#[from] io::Error),

    // buffer.
    #[error("The print area size doesn't fit the buffer.")]
    PrintAreaSizeNotFit,
    #[error("Carpet out of buffer height, carpet y: {carpet}, buffer height: {height}.")]
    CarpetOutOfHeight { carpet: usize, height: usize },
    #[error("Carpet out of text len, carpet x: {carpet}, current line length: {len}.")]
    CarpetOutOfLen { carpet: usize, len: usize },

    // edit area.
    #[error("Buffer size {buffer_size:?} exceeds the display area size {area_size:?}.")]
    BufferSizeExceeds { buffer_size: Size, area_size: Size },
    #[error("Carpet out of range.")]
    CarpetOutOfRange,
}

pub type Result<T> = std::result::Result<T, Error>;