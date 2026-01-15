
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
    DataIndexingError(String),
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


#[derive(Debug, Clone)]
pub struct TVConfig {
    pub event_poll_time: usize,
    pub default_column_width: usize,
    pub column_margin: usize,
}

#[derive(PartialEq, Debug)]
pub enum Message {
    MoveUp,
    MovePageUp,
    MoveDown,
    MovePageDown,
    MoveLeft,
    MoveRight,
    MoveEnd,
    MoveBeginning,
    ShrinkColumn,
    GrowColumn,
    ToggleIndex,
    Resize(usize, usize,),
    CopyCell,
    CopyRow,
    Enter,
    Exit,
    Quit,
}

