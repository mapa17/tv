
use std::io::Error;
use polars::error::PolarsError;
use ratatui::crossterm::event;

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
    Help,
    EnterCommand,
    Enter,
    Exit,
    Quit,
    RawKey(event::KeyEvent),
}


pub const HELP_TEXT: &str = "
TV - Table Viewer Key bindings

        == General ==
q       : Quit

        == Table View ==
Enter   : Enter Record view for selected cell.
h       : Move selection to the left.
j       : Move selection to the down.
k       : Move selection to the up.
l       : Move selection to the right.
J       : Jump page down
K       : Jump page up
g       : Jump to the first row
G       : Jump to the last row
-       : Shrink column
+       : Expand column


        == Record View ==
ESC     : Return to Table view
h       : Show previous row record.
j       : Move selection to the down.
k       : Move selection to the up.
l       : Show next row record.


Question? Write to contact@pasieka.ai
";