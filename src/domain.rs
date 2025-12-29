
use std::io::Error;
use polars::error::PolarsError;

// This is a custom error type that we will be using in `parse_pos_nonzero()`.
#[derive(Debug)]
pub enum TVError {
    IoError(Error),
    PolarsError(PolarsError),
    LoadingFailed(String),
    FileNotFound,
    PermissionDenied,
    UnknownFileType,
}


impl From<Error> for TVError {
    fn from(err: Error) -> Self {
        TVError::IoError(err)
    } 
}

impl From<PolarsError> for TVError {
    fn from(err: PolarsError) -> Self {
        TVError::PolarsError(err)
    } 
}


#[derive(Debug)]
pub struct TableConfig {
    pub event_poll_time: u64,
}

#[derive(PartialEq)]
pub enum Message {
    // Increment,
    // Decrement,
    // Reset,
    Quit,
}

