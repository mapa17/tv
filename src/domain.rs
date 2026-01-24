
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
    MoveToFirstColumn,
    MoveToLastColumn,
    MoveBeginning,
    ShrinkColumn,
    GrowColumn,
    ToggleIndex,
    Resize(usize, usize,),
    CopyCell,
    CopyRow,
    Help,
    EnterCommand,
    Find,
    Filter,
    Histogram,
    Enter,
    Exit,
    Quit,
    SearchNext,
    SearchPrev,
    RawKey(event::KeyEvent),
}


pub const HELP_TEXT: &str = "
    q       : Quit

            == Table View ==
    Enter   : Enter Record view for selected cell.
    TAB     : Show Row Index
    h       : Move selection to the left.
    j       : Move selection to the down.
    k       : Move selection to the up.
    l       : Move selection to the right.
    J       : Jump page down
    K       : Jump page up
    g       : Jump to the first row
    G       : Jump to the last row
    0       : Jump to the first column
    $       : Jump to the last column
    y       : Copy cell value
    Y       : Copy row
    -       : Shrink column
    +       : Expand column
    /       : Search in complete table
    n       : Jump to next search result
    p       : Jump to previous search result
    |       : Filter table on matches in the current column
    F       : Show histogram of current column


            == Record View ==
    ESC     : Return to Table view
    h       : Show previous row record.
    j       : Move selection to the down.
    k       : Move selection to the up.
    l       : Show next row record.

            == Histogram View ==
    ESC     : Return to Table view
    y       : Copy selection
    j       : Move selection to the down.
    k       : Move selection to the up.
    ENTER   : Filter table for selected value


    Question? Write to contact@pasieka.ai
";