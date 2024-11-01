use crate::editor::Size;
use std::io;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IOError occurred: {0:?}")]
    IOError(#[from] io::Error),

    // buffer.
    #[error("The print area size doesn't fit the buffer.")]
    PrintAreaSizeNotFit,
    #[error("Caret out of buffer height, caret y: {caret}, buffer height: {height}.")]
    CaretOutOfHeight { caret: usize, height: usize },
    #[error("Caret out of text len, caret x: {caret}, current line length: {len}.")]
    CaretOutOfLen { caret: usize, len: usize },
    #[error("End of buffer reached.")]
    EndOfFile,
    #[error("Deleting char at the very beginning of the buffer.")]
    DelAtBeginning,

    // edit area.
    #[error("Buffer size {buffer_size:?} exceeds the display area size {area_size:?}.")]
    BufferSizeExceeds { buffer_size: Size, area_size: Size },
}

pub type Result<T> = std::result::Result<T, Error>;