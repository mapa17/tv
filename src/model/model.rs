use arboard::Clipboard;
use polars::prelude::*;
use ratatui::crossterm::event::KeyEvent;
use rayon::prelude::*;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::{debug, error, info, trace};

use crate::domain::{CMDMode, HELP_TEXT, Message, TVConfig, TVError};
use crate::inputter::{InputResult, Inputter};

use super::{Column, ColumnStatus, HistogramView, RecordView, TableView, UIData, UILayout};

// A struct with different types
#[derive(Debug)]
enum FileType {
    CSV,
    PARQUET,
    XLSX,
    ARROW,
}

// A struct with different types
#[derive(Debug, PartialEq)]
pub enum Status {
    READY,
    QUITTING,
}

#[derive(Debug)]
pub struct FileInfo {
    path: PathBuf,
    file_size: u64,
    file_type: FileType,
}

#[derive(Debug, Clone, Copy)]
enum Modus {
    TABLE,
    RECORD,
    POPUP,
    CMDINPUT,
    HISTOGRAM,
}

//#[derive(Debug)]
pub struct Model {
    file_info: Option<FileInfo>,
    config: TVConfig,
    pub status: Status,
    modus: Modus,
    previous_modus: Modus,
    data: Vec<Column>,
    pub tables: Vec<TableView>,
    record_view: RecordView,
    histogram_view: HistogramView,
    //current_table: usize,
    last_update: Instant,
    last_data_change: Instant,
    pub uilayout: UILayout,
    uidata: UIData,
    clipboard: Clipboard,
    input: Inputter,
    cmd_mode: Option<CMDMode>,
    last_input: InputResult,
    active_cmdinput: bool,
    status_message: String,
    last_status_message_update: Instant,
}

impl Model {
    pub fn init(config: &TVConfig, ui_width: usize, ui_height: usize) -> Result<Self, TVError> {
        let mut model = Self {
            file_info: None,
            config: config.clone(),
            modus: Modus::TABLE,
            previous_modus: Modus::TABLE,
            status: Status::READY,
            data: Vec::new(),
            tables: Vec::new(),
            record_view: RecordView::empty(),
            histogram_view: HistogramView::empty(),
            last_update: Instant::now() - std::time::Duration::from_secs(1),
            last_data_change: Instant::now(),
            uilayout: UILayout::from_values(0, ui_width, ui_height),
            uidata: UIData::empty(), // TODO: find out how to do this better. How can i in a factory function create an object that relies on self to exit?
            clipboard: Clipboard::new().unwrap(),
            input: Inputter::default(),
            cmd_mode: None,
            last_input: InputResult::default(),
            active_cmdinput: false,
            status_message: "Started tv!".to_string(),
            last_status_message_update: Instant::now(),
        };

        model.uidata.layout = model.uilayout.clone();
        model.set_status_message("Loading ...".to_string());
        Ok(model)
    }

    pub fn load_data_file(&mut self, path: PathBuf) -> Result<bool, TVError> {
        let file_info = Model::get_file_info(path)?;
        let frame = match file_info.file_type {
            FileType::CSV => Model::load_csv(&file_info.path)?,
            FileType::PARQUET => Model::load_parquet(&file_info.path)?,
            FileType::XLSX => todo!(),
            FileType::ARROW => Model::load_arrow(&file_info.path)?,
        };

        // Load dataframe using rayon with data parallelism.
        // Each column is loaded in its own thread.
        // This is a very intensive operation as the data is pre-processed.
        // The returned columns hold all data as Strings in memory.
        let start_time = Instant::now();

        let df = Arc::new(frame);
        let c_: Result<Vec<Column>, _> = df
            .get_column_names()
            .par_iter()
            .enumerate()
            .map(|(idx, name)| Self::load_columns(&df, idx, name))
            .collect();
        let columns = c_?;

        let data_loading_duration = start_time.elapsed().as_millis();
        info!("Loading data took {data_loading_duration}ms ...");
        for c in columns.iter() {
            debug!("Column: {}", c.as_string());
        }
        let mut table = TableView::empty();
        // set default row mapping
        table.rows = Arc::new((0..columns[0].data.len()).collect());
        table.name = file_info
            .path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("???")
            .to_string();

        self.tables.push(table);
        self.data = columns;
        self.update_table_data();
        self.set_status_message(format!("Loaded data in {}ms ...", data_loading_duration));

        Ok(true)
    }

    fn detect_file_type(path: &Path) -> Result<FileType, TVError> {
        match path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_uppercase())
            .as_deref()
        {
            Some("CSV") => Ok(FileType::CSV),
            Some("PARQUET") | Some("PQ") => Ok(FileType::PARQUET),
            Some("XLSX") => Ok(FileType::XLSX),
            Some("ARROW") | Some("IPC") | Some("FEATHER") => Ok(FileType::ARROW),
            _ => Err(TVError::UnknownFileType),
        }
    }

    fn set_status_message(&mut self, message: impl Into<String>) {
        self.status_message = message.into();
        self.last_status_message_update = Instant::now();
        self.uidata.status_message = self.status_message.clone();
        self.uidata.last_status_message_update = self.last_status_message_update;
        self.uidata.last_update = Instant::now();
    }

    pub fn get_uidata(&self) -> &UIData {
        &self.uidata
    }

    fn update_histogram(&mut self) {
        let hist = &mut self.histogram_view;
        let table = self.tables.last().unwrap();
        let column_idx = table.offset_column + table.curser_column;
        self.uidata.layout = self.uilayout.clone();
        hist.update(column_idx, &mut self.data, table, &mut self.uidata)
    }

    fn update_table_data(&mut self) {
        // If the model is empty, there is nothing to do.
        if self.tables.is_empty() || self.data.is_empty() {
            // Does self.uidata need some work?
        } else {
            let table = self.tables.last_mut().unwrap();
            table.update(&mut self.data, &self.uilayout, &mut self.uidata);
        }
    }

    fn is_numeric_type(dtype: &DataType) -> bool {
        matches!(
            dtype,
            DataType::Int8
                | DataType::Int16
                | DataType::Int32
                | DataType::Int64
                | DataType::UInt8
                | DataType::UInt16
                | DataType::UInt32
                | DataType::UInt64
                | DataType::Float32
                | DataType::Float64
        )
    }

    fn load_columns(df: &DataFrame, idx: usize, col_name: &str) -> Result<Column, PolarsError> {
        let original_dtype = df.column(col_name)?.dtype().clone();

        let col = df.column(col_name)?.cast(&DataType::String)?;
        let series = col.str()?;
        let mut data = Vec::with_capacity(series.len());

        let mut max_width = 0;
        for value in series.into_iter() {
            let ss = match value {
                Some(s) => s.to_string().replace("\r\n", " ↵ ").replace("\n", " ↵ "),
                None => String::from("∅"),
            };
            if ss.len() > max_width {
                max_width = ss.len();
            }
            data.push(ss);
        }

        Ok(Column {
            idx: idx as u16,
            name: col_name.to_string(),
            status: ColumnStatus::NORMAL,
            max_width,
            render_width: 0, // Will be set later
            data,
            dtype: original_dtype,
        })
    }

    fn get_file_info(path: PathBuf) -> Result<FileInfo, TVError> {
        let metadata = fs::metadata(&path).map_err(|e| match e.kind() {
            ErrorKind::NotFound => TVError::FileNotFound,
            ErrorKind::PermissionDenied => TVError::PermissionDenied,
            _ => TVError::IoError(e),
        })?;
        if !metadata.is_file() {
            return Err(TVError::LoadingFailed("Not a file!".into()));
        }

        let file_size = metadata.len();

        let file_type = Model::detect_file_type(&path)?;

        Ok(FileInfo {
            path,
            file_size,
            file_type,
        })
    }

    fn load_csv(path: &PathBuf) -> Result<DataFrame, PolarsError> {
        CsvReadOptions::default()
            .with_has_header(true)
            .try_into_reader_with_file_path(Some(path.into()))?
            .finish()
            .or_else(|_| {
                error!("Loading CSV failed! Fallback, trying to load in UTF8 lossy mode.");
                // Fallback: read as bytes and replace invalid UTF-8
                let bytes = std::fs::read(path).map_err(|e| PolarsError::IO {
                    error: e.into(),
                    msg: None,
                })?;
                let content = String::from_utf8_lossy(&bytes);
                let cursor = std::io::Cursor::new(content.as_bytes());

                let mut options = CsvReadOptions::default();
                options.has_header = true;
                CsvReader::new(cursor).with_options(options).finish()
            })
    }

    fn load_parquet(path: &PathBuf) -> Result<DataFrame, PolarsError> {
        let file = std::fs::File::open(path)?;
        ParquetReader::new(file).finish()
    }

    fn load_arrow(path: &PathBuf) -> Result<DataFrame, PolarsError> {
        let file = std::fs::File::open(path)?;
        IpcReader::new(file).finish()
    }

    pub fn raw_keyevents(&self) -> bool {
        self.active_cmdinput
    }

    pub fn quit(&mut self) {
        self.status = Status::QUITTING;
    }

    fn ui_resize(&mut self, width: usize, height: usize) {
        trace!(
            "UI was resized! w:{}->{}, h:{}->{}",
            self.uilayout.width, width, self.uilayout.height, height
        );
        self.uilayout = UILayout::from_model(self, width, height);
        self.input.set_width(self.uilayout.statusline_width);
        match self.modus {
            Modus::TABLE => self.update_table_data(),
            Modus::RECORD => {
                self.update_table_data();
                let table = self.tables.last().unwrap();
                self.record_view = RecordView::new(
                    table,
                    &self.data,
                    &mut self.uidata,
                    self.config.max_column_width,
                )
            }
            Modus::HISTOGRAM => self.update_histogram(),
            Modus::POPUP => {}
            Modus::CMDINPUT => {}
        }
    }

    pub fn update(&mut self, message: Option<Message>) -> Result<(), TVError> {
        if self.last_data_change - self.last_update > std::time::Duration::ZERO {
            self.update_table_data();
        }

        //trace!("Update: Modus {:?}, Message {:?}", self.modus, message);
        if let Some(msg) = message {
            if self.data.is_empty()
                || self.data[0].data.is_empty()
                || self.tables.is_empty()
                || self.tables.last().unwrap().rows.is_empty()
            {
                info!("Empty Table, switch to minimal mode!");
                self.set_status_message("Empty table!".to_string());
                if let Modus::TABLE = self.modus {
                    match msg {
                        Message::Quit => self.quit(),
                        Message::Resize(width, height) => self.ui_resize(width, height),
                        Message::Help => self.show_help(),
                        Message::EnterCommand => self.enter_cmd_mode(CMDMode::Raw),
                        Message::Exit => self.exit(),
                        _ => (),
                    }
                }
            } else {
                match self.modus {
                    Modus::TABLE => match msg {
                        Message::Quit => self.quit(),
                        Message::MoveDown => self.move_table_selection_down(1),
                        Message::MoveLeft => self.move_table_selection_left(),
                        Message::MoveRight => self.move_table_selection_right(),
                        Message::MoveUp => self.move_table_selection_up(1),
                        Message::MovePageUp => {
                            self.move_table_selection_up(self.uilayout.table_height + 1)
                        }
                        Message::MovePageDown => {
                            self.move_table_selection_down(self.uilayout.table_height + 1)
                        }
                        Message::MoveBeginning => self.move_table_selection_beginning(),
                        Message::MoveEnd => self.move_table_selection_end(),
                        Message::ToggleColumnState => self.toggle_column_status(false),
                        Message::ToggleExpandColumnState => self.toggle_column_status(true),
                        Message::ToggleIndex => self.toggle_table_index(),
                        Message::Resize(width, height) => self.ui_resize(width, height),
                        Message::CopyCell => self.copy_table_cell(),
                        Message::CopyRow => self.copy_table_row(),
                        Message::Help => self.show_help(),
                        Message::EnterCommand => self.enter_cmd_mode(CMDMode::Raw),
                        Message::Search => self.enter_cmd_mode(CMDMode::SearchTable),
                        Message::Filter => self.enter_cmd_mode(CMDMode::FilterByColumn),
                        Message::SearchInColumn => self.enter_cmd_mode(CMDMode::SearchInColumn),
                        Message::Enter => self.enter(),
                        Message::Exit => self.exit(),
                        Message::Histogram => {
                            self.previous_modus = self.modus;
                            self.modus = Modus::HISTOGRAM;
                            self.update_histogram()
                        }
                        Message::SearchNext => self.search_next(1),
                        Message::SearchPrev => self.search_next(-1),
                        Message::SortAscending => self.sort_current_column(true),
                        Message::SortDescending => self.sort_current_column(false),
                        Message::MoveToFirstColumn => {
                            self.select_cell(
                                self.tables.last().unwrap().curser_row
                                    + self.tables.last().unwrap().offset_row,
                                0,
                            );
                        }
                        Message::MoveToLastColumn => {
                            let table = self.tables.last().unwrap();
                            self.select_cell(
                                table.curser_row + table.offset_row,
                                self.data.len() - 1,
                            );
                        }
                        _ => (),
                    },
                    Modus::RECORD => match msg {
                        Message::Quit => self.quit(),
                        Message::MoveDown => self.move_record_selection_down(1),
                        Message::MoveLeft => self.previous_record(),
                        Message::MoveRight => self.next_record(),
                        Message::MoveUp => self.move_record_selection_up(1),
                        Message::MovePageUp => self.move_record_selection_up(10),
                        Message::MovePageDown => self.move_record_selection_down(10),
                        Message::Resize(width, height) => self.ui_resize(width, height),
                        Message::CopyCell => self.copy_record_cell(),
                        Message::Help => self.show_help(),
                        Message::Enter => self.enter(),
                        Message::Exit => self.exit(),
                        _ => (),
                    },
                    Modus::HISTOGRAM => match msg {
                        Message::Quit => self.quit(),
                        Message::MoveDown => self.move_histogram_selection_down(1),
                        Message::MoveUp => self.move_histogram_selection_up(1),
                        Message::MovePageUp => self.move_histogram_selection_up(10),
                        Message::MovePageDown => self.move_histogram_selection_down(10),
                        Message::Resize(width, height) => self.ui_resize(width, height),
                        Message::Help => self.show_help(),
                        Message::Enter => self.enter(),
                        Message::Exit => self.exit(),
                        _ => (),
                    },

                    Modus::POPUP => match msg {
                        Message::Quit => self.quit(),
                        Message::Resize(width, height) => self.ui_resize(width, height),
                        Message::Exit => self.exit(),
                        _ => (),
                    },
                    Modus::CMDINPUT => {
                        if let Message::RawKey(key) = msg {
                            self.raw_input(key)
                        }
                    }
                }
            }
        }

        self.last_update = Instant::now();
        Ok(())
    }

    // -------------------- Control handling functions ---------------------- //

    fn enter(&mut self) {
        match self.modus {
            Modus::TABLE => {
                let table = self.tables.last_mut().unwrap();
                table.show_index = false;
                let table = self.tables.last().unwrap();
                // Disabling the index will change the ui layout. Recalculate it
                self.uilayout =
                    UILayout::from_model(self, self.uilayout.width, self.uilayout.height);
                //self.build_record_view(record_idx);
                self.record_view = RecordView::new(
                    table,
                    &self.data,
                    &mut self.uidata,
                    self.config.max_column_width,
                );
                self.modus = Modus::RECORD;
                self.previous_modus = Modus::TABLE;
            }
            Modus::RECORD => {}
            Modus::HISTOGRAM => {
                let hist = &self.histogram_view;
                let table = self.tables.last().unwrap();
                let term = hist.value_data[hist.curser_offset + hist.curser_row].clone();
                let matches = self.data[hist.column_idx].search(&term, &table.rows);
                self.filter_table(matches);
                self.modus = Modus::TABLE;
                self.previous_modus = Modus::HISTOGRAM;
            }
            Modus::POPUP => {}
            Modus::CMDINPUT => {}
        }
    }

    fn exit(&mut self) {
        match self.modus {
            Modus::TABLE => {
                // Nothing todo, there is no exit from table, only quit
                if self.tables.len() > 1 {
                    self.tables.pop();
                    self.update_table_data();
                }
            }
            Modus::RECORD => {
                // Switch back to table mode
                self.previous_modus = Modus::RECORD;
                self.modus = Modus::TABLE;
                self.update_table_data();
            }
            Modus::POPUP => {
                trace!("Close popup ...");
                self.modus = self.previous_modus;
                self.previous_modus = Modus::POPUP;
                self.uidata.show_popup = false;
                self.uidata.last_update = Instant::now();
            }
            Modus::CMDINPUT => {}
            Modus::HISTOGRAM => {
                // Switch back to table mode
                self.previous_modus = Modus::HISTOGRAM;
                self.modus = Modus::TABLE;
                self.update_table_data();
            }
        }
    }

    fn show_help(&mut self) {
        self.previous_modus = self.modus;
        self.modus = Modus::POPUP;
        self.uidata.popup_message = HELP_TEXT.to_string();
        self.uidata.show_popup = true;
        self.uidata.last_update = Instant::now();
    }

    fn raw_input(&mut self, key: KeyEvent) {
        if self.active_cmdinput {
            self.last_input = self.input.read(key);
            if self.last_input.finished {
                self.handle_cmd_input();
            }
            self.uidata.cmdinput = self.last_input.clone();
            self.uidata.cmd_mode = self.cmd_mode;
            self.uidata.last_update = Instant::now();
        }
    }

    fn enter_cmd_mode(&mut self, mode: CMDMode) {
        trace!("Entering command mode ...");
        self.previous_modus = self.modus;
        self.modus = Modus::CMDINPUT;
        self.cmd_mode = Some(mode);

        self.active_cmdinput = true;
        self.input.clear();
        self.last_input = self.input.get();

        self.last_input = self.input.get();
        self.uidata.cmdinput = self.last_input.clone();
        self.uidata.active_cmdinput = self.active_cmdinput;
        self.uidata.last_update = Instant::now();
        self.uidata.cmd_mode = self.cmd_mode;
    }

    fn handle_cmd_input(&mut self) {
        // TODO: process self.last_input
        trace!("Handle cmd input {}", self.last_input.input);

        self.active_cmdinput = false;
        self.modus = self.previous_modus;
        self.previous_modus = Modus::CMDINPUT;

        self.uidata.active_cmdinput = self.active_cmdinput;
        self.last_update = Instant::now();

        let cmd_input = self.last_input.input.clone();
        match self.cmd_mode {
            Some(CMDMode::SearchTable) => {
                self.search(&cmd_input, false);
            }
            Some(CMDMode::FilterByColumn) => {
                self.filter(&cmd_input);
            }
            Some(CMDMode::SearchInColumn) => {
                self.search(&cmd_input, true);
            }
            Some(CMDMode::Raw) => {
                info!("Raw cmd mode {cmd_input}")
            }
            None => {
                info!("Cmd mode is none!")
            }
        }

        self.cmd_mode = None;
    }

    fn search(&mut self, term: &str, current_column_only: bool) {
        trace!("Starting search for {} ...", term);
        let table = self.tables.last_mut().unwrap();
        let num_matches = table.search(
            term,
            current_column_only,
            &mut self.data,
            &self.uilayout,
            &mut self.uidata,
        );
        if num_matches == 0 {
            self.set_status_message("Found no matches!".to_string());
        } else {
            self.set_status_message(format!("Found {} results", num_matches));
        }
    }

    // Sets the curser to the next search result
    fn search_next(&mut self, step: i32) {
        let table = self.tables.last_mut().unwrap();
        if table.search_results.is_empty() {
            self.set_status_message("Empty search results!".to_string());
        } else {
            let next_match_idx =
                table.search_next(step, &mut self.data, &self.uilayout, &mut self.uidata);

            let table = self.tables.last().unwrap();
            self.set_status_message(format!(
                "Search result {}/{}",
                next_match_idx + 1,
                table.search_results.len()
            ));
        }
    }

    fn sort_current_column(&mut self, ascending: bool) {
        let table = self.tables.last_mut().unwrap();
        let data = &(self.data[table.curser_column + table.offset_column]).data;
        let is_numeric =
            Model::is_numeric_type(&self.data[table.curser_column + table.offset_column].dtype);

        // Create a vector of (original_index, value) pairs
        let mut indexed_rows: Vec<(usize, &String)> = table
            .rows
            .iter()
            .map(|&row_idx| (row_idx, &data[row_idx]))
            .collect();

        //indexed_rows.sort_unstable_by_key(|(idx, &data)| data);

        // Sort by the data values
        if is_numeric {
            // If the column originally was a numeric, try to convert each value to a float representation and compare it.
            // LLM generated matches will order partial float conversion, giving order preference to successful converted floats
            // Falling back to string sorting if nothing can be converted
            indexed_rows.sort_by(|(_, a), (_, b)| {
                let a_val: Result<f64, _> = a.parse();
                let b_val: Result<f64, _> = b.parse();

                match (a_val, b_val) {
                    (Ok(a_float), Ok(b_float)) => {
                        if ascending {
                            a_float
                                .partial_cmp(&b_float)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        } else {
                            b_float
                                .partial_cmp(&a_float)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        }
                    }
                    (Ok(_), Err(_)) => std::cmp::Ordering::Less, // Valid numbers come first
                    (Err(_), Ok(_)) => std::cmp::Ordering::Greater, // Invalid strings come last
                    (Err(_), Err(_)) => {
                        // Both invalid, sort as strings
                        if ascending { a.cmp(b) } else { b.cmp(a) }
                    }
                }
            });
        } else {
            // Sort as Strings
            if ascending {
                indexed_rows.sort_by(|(_, a), (_, b)| a.cmp(b));
            } else {
                indexed_rows.sort_by(|(_, a), (_, b)| b.cmp(a));
            }
        }

        // Overwrite the table rows with the new ordered index
        table.rows = Arc::new(indexed_rows.into_iter().map(|(i, _)| i).collect());
        self.update_table_data();
    }

    fn select_cell(&mut self, row: usize, column: usize) {
        let table = self.tables.last_mut().unwrap();
        table.select_cell(
            row,
            column,
            &mut self.data,
            &self.uilayout,
            &mut self.uidata,
        );
    }

    fn filter(&mut self, term: &str) {
        trace!("Starting filter for {} ...", term);
        let table = self.tables.last_mut().unwrap();
        let start_time = Instant::now();

        let matches =
            self.data[table.offset_column + table.curser_column].search(term, &table.rows);

        let search_duration = start_time.elapsed().as_millis();

        trace!(
            "Search found {} matching rows in {}ms",
            matches.len(),
            search_duration
        );
        if matches.is_empty() {
            self.set_status_message("Empty table!".to_string());
        }
        self.filter_table(matches);
    }

    fn filter_table(&mut self, indices: Vec<usize>) {
        let table = self.tables.last().unwrap();
        let mut new_table = TableView::empty();
        new_table.name = format!("F[{}]", table.name);
        let resolved_indices: Vec<usize> = indices.iter().map(|&midx| table.rows[midx]).collect();
        new_table.rows = Arc::new(resolved_indices);
        self.tables.push(new_table);
        self.update_table_data();
    }

    fn toggle_table_index(&mut self) {
        let table = self.tables.last_mut().unwrap();
        table.show_index = !table.show_index;

        // Update ui layout and the underlying data
        self.uilayout = UILayout::from_model(self, self.uilayout.width, self.uilayout.height);
        self.update_table_data();
    }

    fn copy_table_cell(&mut self) {
        let table = self.tables.last().unwrap();
        let cell = table.get_current_cell(&self.data);

        match self.clipboard.set_text(cell) {
            Ok(_) => self.set_status_message("Copied cell to clipboard!"),
            Err(e) => self.set_status_message(format!("Copying to clipboard failed! {e}")),
        }
    }

    fn copy_table_row(&mut self) {
        let table = self.tables.last().unwrap();
        let row_content = table.get_current_row(&self.data);

        match self.clipboard.set_text(row_content) {
            Ok(_) => self.set_status_message("Copied row to clipboard!"),
            Err(e) => self.set_status_message(format!("Copying to clipboard failed! {e}")),
        }
    }

    fn toggle_column_status(&mut self, toggle_to_expand: bool) {
        let table = self.tables.last_mut().unwrap();
        table.toggle_column_status(&mut self.data, toggle_to_expand);
        self.update_table_data();
    }

    fn move_table_selection_beginning(&mut self) {
        let table = self.tables.last_mut().unwrap();
        table.move_selection_beginning(&mut self.data, &self.uilayout, &mut self.uidata);
    }

    fn move_table_selection_end(&mut self) {
        let table = self.tables.last_mut().unwrap();
        table.move_selection_end(&mut self.data, &self.uilayout, &mut self.uidata);
    }

    fn move_table_selection_up(&mut self, size: usize) {
        let table = self.tables.last_mut().unwrap();
        table.move_selection_up(size, &mut self.data, &self.uilayout, &mut self.uidata);
    }

    fn move_table_selection_down(&mut self, size: usize) {
        let table = self.tables.last_mut().unwrap();
        table.move_selection_down(size, &mut self.data, &self.uilayout, &mut self.uidata);
    }

    fn move_table_selection_left(&mut self) {
        let table = self.tables.last_mut().unwrap();
        table.move_selection_left(&mut self.data, &self.uilayout, &mut self.uidata);
    }

    fn move_table_selection_right(&mut self) {
        let table = self.tables.last_mut().unwrap();
        table.move_selection_right(&mut self.data, &self.uilayout, &mut self.uidata);
    }

    fn copy_record_cell(&mut self) {
        let record = &self.record_view;
        let cell = record.row_data[record.curser_offset + record.curser_row].clone();
        trace!("Cell content: {}", cell);

        match self.clipboard.set_text(cell) {
            Ok(_) => trace!("Copied cell content to clipboard."),
            Err(e) => trace!("Error copying to clipboard: {:?}", e),
        }
    }

    fn move_record_selection_up(&mut self, size: usize) {
        let record = &mut self.record_view;
        let table = self.tables.last().unwrap();
        record.move_selection_up(size, table, &mut self.data, &mut self.uidata);
    }

    fn move_histogram_selection_up(&mut self, size: usize) {
        let hist = &mut self.histogram_view;
        hist.move_selection_up(
            size,
            &mut self.data,
            self.tables.last().unwrap(),
            &mut self.uidata,
        );
    }

    fn move_record_selection_down(&mut self, size: usize) {
        let record = &mut self.record_view;
        let table = self.tables.last().unwrap();
        record.move_selection_down(size, table, &mut self.data, &mut self.uidata);
    }

    fn move_histogram_selection_down(&mut self, size: usize) {
        let hist = &mut self.histogram_view;
        hist.move_selection_down(
            size,
            &mut self.data,
            self.tables.last().unwrap(),
            &mut self.uidata,
        )
    }

    fn previous_record(&mut self) {
        let record = &mut self.record_view;
        let table = self.tables.last().unwrap();
        record.previous_record(table, &mut self.data, &mut self.uidata);
    }

    fn next_record(&mut self) {
        let record = &mut self.record_view;
        let table = self.tables.last().unwrap();
        record.next_record(table, &mut self.data, &mut self.uidata);
    }
}
