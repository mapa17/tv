use arboard::Clipboard;
use polars::prelude::*;
use ratatui::crossterm::event::KeyEvent;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;
use tracing::{debug, error, info, trace};

use crate::domain::{CMDMode, HELP_TEXT, Message, TVConfig, TVError};
use crate::inputter::{InputResult, Inputter};
use crate::ui::{
    CMDLINE_HEIGH, COLUMN_WIDTH_COLLAPSED_COLUMN, COLUMN_WIDTH_MARGIN, SCROLLBAR_WIDTH,
    TABLE_HEADER_HEIGHT,
};

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
    EMPTY,
    READY,
    LOADING,
    PROCESSING,
    QUITTING,
}

#[derive(Debug)]
pub struct FileInfo {
    path: PathBuf,
    file_size: u64,
    file_type: FileType,
}

pub struct Column {
    idx: u16,
    name: String,
    status: ColumnStatus,
    max_width: usize,
    render_width: usize,
    data: Vec<String>,
    dtype: DataType,
}

impl Column {
    pub fn as_string(&self) -> String {
        format!(
            "{} \"{}\", {:?}, width_max: {}, render_width: {}, # rows {}",
            self.idx,
            self.name,
            self.status,
            self.max_width,
            self.render_width,
            self.data.len(),
        )
    }
}

#[derive(Clone)]
pub struct ColumnView {
    pub name: String,
    pub width: usize,
    pub data: Vec<String>,
}

impl ColumnView {
    fn empty() -> Self {
        ColumnView {
            name: "".to_string(),
            width: 0,
            data: Vec::new(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ColumnStatus {
    NORMAL,
    EXPANDED,
    COLLAPSED,
}

#[derive(Debug, Clone, Copy)]
enum Modus {
    TABLE,
    RECORD,
    POPUP,
    CMDINPUT,
    HISTOGRAM,
}

pub struct TableView {
    name: String,
    rows: Arc<Vec<usize>>, // Mapping of TableView row index to data index. Wrap in arc to allow multi threaded access
    visible_columns: Vec<usize>, // Idx of visible columns that are send to the UI for rendering.
    visible_width: usize,
    curser_row: usize,
    curser_column: usize,
    offset_row: usize,
    offset_column: usize,
    data: Vec<ColumnView>,
    search_results: Vec<(usize, usize)>,
    search_idx: usize,
    show_index: bool,
    index: ColumnView,
    column_histograms: HashMap<usize, (Vec<usize>, Vec<String>)>,
    heigh: usize,
    width: usize,
}

impl TableView {
    fn empty() -> Self {
        TableView {
            name: String::new(),
            rows: Arc::new(Vec::new()),
            visible_columns: Vec::new(),
            visible_width: 0,
            curser_column: 0,
            curser_row: 0,
            offset_column: 0,
            offset_row: 0,
            data: Vec::new(),
            search_results: Vec::new(),
            search_idx: 0,
            show_index: false,
            index: ColumnView::empty(),
            column_histograms: HashMap::new(),
            heigh: 0,
            width: 0,
        }
    }

    fn build_index(&mut self) {
        let rbegin = self.offset_row;
        let rend = std::cmp::min(rbegin + self.heigh, self.rows.len());

        let data = self.rows[rbegin..rend]
            .iter()
            .map(|idx| (idx + 1).to_string())
            .collect::<Vec<String>>();
        let width = data.last().map(|s| s.len()).unwrap_or(3);
        self.index = ColumnView {
            name: "".to_string(),
            width,
            data,
        }
    }
}

struct RecordView {
    table_idx: usize, // Table view index
    header_data: Vec<String>,
    header_width: usize,
    header_view: ColumnView,
    row_data: Vec<String>, // Add row values
    row_width: usize,
    row_view: ColumnView,
    record_idx: usize, // Index in TableView.rows[XXX]
    curser_row: usize,
    curser_offset: usize,
    last_update: Instant,
    height: usize, // UI height
    width: usize,  // UI Width
}

impl RecordView {
    fn empty() -> Self {
        RecordView {
            table_idx: 0,
            header_data: Vec::new(),
            header_width: 0,
            header_view: ColumnView::empty(),
            row_data: Vec::new(),
            row_width: 0,
            row_view: ColumnView::empty(),
            record_idx: 0,
            curser_row: 0,
            curser_offset: 0,
            last_update: Instant::now(),
            height: 0,
            width: 0,
        }
    }
}

struct HistogramView {
    table_idx: usize, // Table view index
    value_data: Vec<String>,
    value_width: usize,
    value_view: ColumnView,
    count_data: Vec<String>, // Count in absolute and relative values
    count_width: usize,
    count_view: ColumnView,
    column_idx: usize, // Index in Model.data[0][XXX]
    curser_row: usize,
    curser_offset: usize,
    last_update: Instant,
    height: usize, // UI height
    width: usize,  // UI Width
}

impl HistogramView {
    fn empty() -> Self {
        HistogramView {
            table_idx: 0,
            value_data: Vec::new(),
            value_width: 0,
            value_view: ColumnView::empty(),
            count_data: Vec::new(),
            count_width: 0,
            count_view: ColumnView::empty(),
            column_idx: 0,
            curser_row: 0,
            curser_offset: 0,
            last_update: Instant::now(),
            height: 0,
            width: 0,
        }
    }
}

pub struct UIData {
    pub name: String,
    pub table: Vec<ColumnView>,
    pub index: ColumnView,
    pub nrows: usize, // Total number of raws in this View
    pub selected_row: usize,
    pub selected_column: usize,
    pub abs_selected_row: usize,
    pub show_popup: bool,
    pub popup_message: String,
    pub layout: UILayout,
    pub last_update: Instant,
    pub cmdinput: InputResult,
    pub cmd_mode: Option<CMDMode>,
    pub active_cmdinput: bool,
    pub status_message: String,
    pub last_status_message_update: Instant,
}

impl UIData {
    pub fn empty() -> Self {
        UIData {
            name: String::new(),
            table: Vec::new(),
            index: ColumnView {
                name: "".to_string(),
                width: 0,
                data: Vec::new(),
            },
            nrows: 0,
            selected_row: 0,
            selected_column: 0,
            abs_selected_row: 0,
            show_popup: false,
            popup_message: String::new(),
            layout: UILayout::default(),
            last_update: Instant::now(),
            cmdinput: InputResult::default(),
            cmd_mode: None,
            active_cmdinput: false,
            status_message: String::new(),
            last_status_message_update: Instant::now(),
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct UILayout {
    pub width: usize,
    pub height: usize,
    pub table_width: usize,
    pub table_height: usize,
    pub index_width: usize,
    pub index_height: usize,
    pub statusline_width: usize,
    pub statusline_height: usize,
}

impl UILayout {
    pub fn from_model(model: &Model, ui_width: usize, ui_height: usize) -> Self {
        let table = model.tables.last().unwrap();
        let mut index_width = 0;
        if table.show_index {
            index_width = table.index.width;
        }
        UILayout::from_values(index_width, ui_width, ui_height)
    }

    pub fn from_values(index_width: usize, ui_width: usize, ui_height: usize) -> Self {
        let cmdline_heigth = CMDLINE_HEIGH;
        let cmdline_width = ui_width;

        let table_width = ui_width - SCROLLBAR_WIDTH - index_width;
        let table_height = ui_height - cmdline_heigth - TABLE_HEADER_HEIGHT;
        let index_height = table_height;

        let layout = UILayout {
            width: ui_width,
            height: ui_height,
            table_width,
            table_height,
            index_width,
            index_height,
            statusline_width: cmdline_width,
            statusline_height: cmdline_heigth,
        };
        trace!("Build UILayout: {:?}", layout);
        layout
    }
}

//#[derive(Debug)]
pub struct Model {
    file_info: Option<FileInfo>,
    config: TVConfig,
    pub status: Status,
    modus: Modus,
    previous_modus: Modus,
    data: Vec<Column>,
    tables: Vec<TableView>,
    record_view: RecordView,
    histogram_view: HistogramView,
    //current_table: usize,
    last_update: Instant,
    last_data_change: Instant,
    uilayout: UILayout,
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
        //model.update_table_data();
        model.update_uidata_for_table();
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

        let df = Arc::new(frame.clone().collect()?);
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

    fn update_uidata_for_record(&mut self) {
        let table = &mut self.tables.last().unwrap();
        let record = &self.record_view;
        self.uidata = UIData {
            name: format!("R[{}]", table.name),
            table: vec![record.header_view.clone(), record.row_view.clone()],
            index: table.index.clone(),
            nrows: table.rows.len(),
            selected_row: record.curser_row,
            selected_column: 1,
            show_popup: false,
            popup_message: String::new(),
            abs_selected_row: record.record_idx, // In the record view, show which record we are looking at instead of line in record view.
            layout: self.uilayout.clone(),
            cmdinput: self.last_input.clone(),
            cmd_mode: self.uidata.cmd_mode,
            active_cmdinput: self.active_cmdinput,
            last_update: Instant::now(),
            status_message: self.status_message.clone(),
            last_status_message_update: self.last_status_message_update,
        }
    }

    fn update_uidata_for_table(&mut self) {
        if self.tables.is_empty() {
            self.uidata = UIData::empty();
            self.uidata.layout = self.uilayout.clone(); // Set layout for ui rendering
        } else {
            let table = &mut self.tables.last().unwrap();

            self.uidata = UIData {
                name: table.name.clone(),
                table: table.data.clone(),
                index: table.index.clone(),
                nrows: table.rows.len(),
                selected_row: table.curser_row,
                selected_column: table.curser_column,
                abs_selected_row: table.offset_row + table.curser_row,
                show_popup: false,
                popup_message: String::new(),
                layout: self.uilayout.clone(),
                cmdinput: self.last_input.clone(),
                cmd_mode: self.uidata.cmd_mode,
                active_cmdinput: self.active_cmdinput,
                last_update: Instant::now(),
                status_message: self.status_message.clone(),
                last_status_message_update: self.last_status_message_update,
            }
        }
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

    fn get_collapsed_column(nrows: usize) -> ColumnView {
        let data = vec!["⋮".to_string(); nrows];
        ColumnView {
            name: "...".to_string(),
            width: 3,
            data,
        }
    }

    fn calculate_column_histogram(&mut self, column_idx: usize) {
        trace!("Calculate histogram for column {}", column_idx);
        let table = self.tables.last_mut().unwrap();
        if !table.column_histograms.contains_key(&column_idx) {
            let column_data = &self.data[column_idx].data;

            let mut counts: HashMap<String, usize> = HashMap::new();
            for &ridx in table.rows.iter() {
                *counts.entry(column_data[ridx].clone()).or_insert(0) += 1;
            }
            let mut sorted: Vec<(usize, String)> =
                counts.iter().map(|(k, v)| (*v, k.clone())).collect();
            sorted.sort_unstable();
            sorted.reverse();
            let (counts, values): (Vec<usize>, Vec<String>) = sorted.into_iter().unzip();
            table.column_histograms.insert(column_idx, (counts, values));
        }
    }

    fn build_histogram_view(&mut self) {
        self.modus = Modus::HISTOGRAM;
        let current_column = {
            let table = self.tables.last().unwrap();
            table.offset_column + table.curser_column
        };
        self.calculate_column_histogram(current_column);

        // Disable rendering of index
        let table = self.tables.last_mut().unwrap();
        table.show_index = false;
        let table = self.tables.last().unwrap();

        // Update histogram data
        let counts = &table.column_histograms[&current_column];
        let hist = &mut self.histogram_view;
        hist.curser_offset = 0;
        hist.curser_row = 0;
        hist.column_idx = current_column;
        hist.height = table.heigh;
        hist.width = table.width;

        let nrecords = table.rows.len();
        hist.count_data = counts
            .0
            .iter()
            .map(|&c| format!("{:.0}% {}", c as f64 * 100.0 / nrecords as f64, c))
            .collect();
        hist.value_data = counts.1.clone();

        self.update_histogram_view();
    }

    fn update_histogram_view(&mut self) {
        self.uilayout = UILayout::from_model(self, self.uilayout.width, self.uilayout.height);
        let hist = &mut self.histogram_view;
        let rbegin = hist.curser_offset;
        let rend = std::cmp::min(rbegin + hist.height, hist.value_data.len());

        hist.count_width = hist.count_data[0].len();
        hist.count_view = ColumnView {
            name: "Counts".to_string(),
            data: hist.count_data[rbegin..rend].to_vec(),
            width: hist.count_width,
        };

        hist.value_width = hist.width - hist.count_width;
        hist.value_view = ColumnView {
            name: "Values".to_string(),
            data: hist.value_data[rbegin..rend].to_vec(),
            width: hist.value_width,
        };

        self.update_uidata_for_histogram();
    }

    fn update_uidata_for_histogram(&mut self) {
        let hist = &self.histogram_view;
        let uidata = &mut self.uidata;
        let table = self.tables.last().unwrap();

        uidata.name = format!("H[{}]", table.name);
        uidata.table = vec![hist.count_view.clone(), hist.value_view.clone()];
        uidata.selected_column = 1;
        uidata.nrows = hist.value_data.len();
        uidata.selected_row = hist.curser_row;
        uidata.abs_selected_row = hist.curser_row + hist.curser_offset;
        uidata.last_update = Instant::now();
    }

    fn build_record_view(&mut self, record_idx: usize) {
        trace!("Building record view ...");
        let table = self.tables.last().unwrap();
        let record = &mut self.record_view;
        // Get header names
        record.header_data = self
            .data
            .iter()
            .map(|c| {
                c.name
                    .chars()
                    .take(self.config.max_column_width)
                    .collect::<String>()
            })
            .collect::<Vec<String>>();

        record.curser_offset = 0;
        record.curser_row = 0;
        record.record_idx = record_idx;
        record.height = table.heigh;
        record.width = table.width;

        record.header_width = record
            .header_data
            .iter()
            .map(|h| h.len())
            .max()
            .unwrap_or(0);
        record.row_width = record.width - record.header_width;

        self.update_record_data();
    }

    fn update_record_data(&mut self) {
        let table = self.tables.last().unwrap();
        let record = &mut self.record_view;

        record.row_data = self
            .data
            .iter()
            .map(|c| c.data[table.rows[record.record_idx]].clone())
            .collect::<Vec<String>>();

        let rbegin = record.curser_offset;
        let rend = std::cmp::min(rbegin + record.height, record.row_data.len());

        trace!(
            "Record: rIdx {}, rb {}, re {}, rows {}",
            record.record_idx,
            rbegin,
            rend,
            record.row_data.len()
        );
        record.header_view = ColumnView {
            name: "Headers".to_string(),
            data: record.header_data[rbegin..rend].to_vec(),
            width: record.header_width,
        };

        record.row_view = ColumnView {
            name: "Values".to_string(),
            data: record.row_data[rbegin..rend].to_vec(),
            width: record.row_width,
        };
        self.record_view.last_update = Instant::now();

        self.update_uidata_for_record();
    }

    fn update_table_data(&mut self) {
        // If the model is empty, there is nothing to do.
        if self.tables.is_empty() || self.data.is_empty() {
            return;
        } else {
            let table = self.tables.last_mut().unwrap();

            table.width = self.uilayout.table_width;
            table.heigh = self.uilayout.table_height;

            let rbegin = table.offset_row;
            let rend = std::cmp::min(rbegin + table.heigh, table.rows.len());

            trace!(
                "Table: I:{}, Cr {}, Cc {}, Or {}, Oc {}, Rb {}, Re {}, tw: {}, th:{}, uiw: {}, uih: {}",
                table.show_index,
                table.curser_row,
                table.curser_column,
                table.offset_row,
                table.offset_column,
                rbegin,
                rend,
                table.width,
                table.heigh,
                self.uilayout.width,
                self.uilayout.height
            );

            table.visible_columns = Vec::new();
            let mut visible_width = 0;

            // Calculate current render with for each column
            // This could change because a column was expanded or collapsed
            for column in self.data.iter_mut() {
                column.render_width =
                    Self::calculate_column_width(column, self.config.max_column_width);
            }

            // Create a list of columns that fit in the table
            for (cidx, column) in self.data[table.offset_column..].iter_mut().enumerate() {
                if visible_width + (column.render_width + 1) <= self.uilayout.table_width {
                    //if (column.render_width+1) <= width_budget {
                    table.visible_columns.push(cidx + table.offset_column);
                    //width_budget -= column.render_width + 1; // Rendered with and 1 spacer character
                    visible_width += column.render_width + 1;
                } else {
                    // Add the last partial visible column
                    if visible_width < self.uilayout.table_width {
                        let remaining_width = self.uilayout.table_width - visible_width;
                        table.visible_columns.push(cidx + table.offset_column);
                        visible_width += remaining_width;
                        column.render_width = remaining_width;
                    }
                    break;
                }
            }
            // Store how wide the table would be in its full rendering to know the most right column is only partially rendered
            table.visible_width = visible_width;

            // Growing columns can reduce the number of visible columns. Make sure the column curser is at most the last visible column
            table.curser_column =
                std::cmp::min(table.curser_column, table.visible_columns.len() - 1);

            // Create ColumnViews for visible columns

            table.data.clear();
            table.data = Vec::with_capacity(table.visible_columns.len());
            for idx in table.visible_columns.iter() {
                if let Some(column) = self.data.get(*idx) {
                    if column.status == ColumnStatus::COLLAPSED {
                        table.data.push(Self::get_collapsed_column(rend - rbegin));
                    } else {
                        let col_data = table.rows[rbegin..rend]
                            .iter()
                            .map(|&ridx| column.data[ridx].clone())
                            .collect();
                        let name = Self::get_visible_name(column.name.clone(), column.render_width);
                        let width = column.render_width;
                        //trace!("Visible Column: \"{name}\", width: {width}");

                        table.data.push(ColumnView {
                            name,
                            width,
                            data: col_data,
                        });
                    }
                } else {
                    error!("Trying to access column with unknown idx {idx}!");
                }
            }

            // Update the index
            table.build_index();
        }
        self.update_uidata_for_table();
    }

    fn get_visible_name(name: String, width: usize) -> String {
        let mut reduced_name = name.clone();
        if width < 3 {
            return "".to_string();
        }
        if reduced_name.len() > width {
            reduced_name = reduced_name[0..width - 3].to_string();
            reduced_name.push_str("...");
        }
        reduced_name
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

    fn calculate_column_width(column: &Column, max_column_width: usize) -> usize {
        let width = std::cmp::max(column.name.len(), column.max_width) + COLUMN_WIDTH_MARGIN;
        match column.status {
            ColumnStatus::COLLAPSED => COLUMN_WIDTH_COLLAPSED_COLUMN,
            ColumnStatus::NORMAL => std::cmp::min(width, max_column_width),
            ColumnStatus::EXPANDED => width,
        }
    }

    fn load_csv(path: &PathBuf) -> Result<LazyFrame, PolarsError> {
        LazyCsvReader::new(PlPath::Local(path.as_path().into()))
            .with_has_header(true)
            .finish()
    }

    fn load_parquet(path: &PathBuf) -> Result<LazyFrame, PolarsError> {
        LazyFrame::scan_parquet(
            PlPath::Local(path.as_path().into()),
            ScanArgsParquet::default(),
        )
    }

    fn load_arrow(path: &PathBuf) -> Result<LazyFrame, PolarsError> {
        LazyFrame::scan_ipc(
            PlPath::Local(path.as_path().into()),
            polars::io::ipc::IpcScanOptions,
            UnifiedScanArgs::default(),
        )
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
            Modus::RECORD => self.update_record_data(),
            Modus::HISTOGRAM => self.update_histogram_view(),
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
                    Message::Histogram => self.build_histogram_view(),
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
                        self.select_cell(table.curser_row + table.offset_row, self.data.len() - 1);
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

        self.last_update = Instant::now();
        Ok(())
    }

    // -------------------- Control handling functions ---------------------- //

    fn enter(&mut self) {
        match self.modus {
            Modus::TABLE => {
                let record_idx = {
                    let table = self.tables.last_mut().unwrap();
                    table.show_index = false;
                    table.offset_row + table.curser_row
                };
                // Disabling the index will change the ui layout. Recalculate it
                self.uilayout =
                    UILayout::from_model(self, self.uilayout.width, self.uilayout.height);
                self.build_record_view(record_idx);
                self.modus = Modus::RECORD;
                self.previous_modus = Modus::TABLE;
            }
            Modus::RECORD => {}
            Modus::HISTOGRAM => {
                let hist = &self.histogram_view;
                let table = self.tables.last().unwrap();
                let term = hist.value_data[hist.curser_offset + hist.curser_row].clone();
                let matches = Self::search_column(&term, &self.data[hist.column_idx], &table.rows);
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
                // TODO: Implement search in current column
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

    // Return mask index positions of rows in the column that match given term
    fn search_column(term: &str, column: &Column, mask: &[usize]) -> Vec<usize> {
        let mut matches = Vec::new();
        for (midx, &m) in mask.iter().enumerate() {
            if column.data[m].contains(term) {
                matches.push(midx)
            }
        }
        matches
    }

    fn search(&mut self, term: &str, current_column_only: bool) {
        trace!("Starting search for {} ...", term);
        let table = self.tables.last_mut().unwrap();
        let start_time = Instant::now();

        let mask = Arc::clone(&table.rows);
        let search_term = term.to_string();

        let matching_rows: Vec<(usize, usize)> = if current_column_only {
            let current_col_idx = table.curser_column + table.offset_column;
            Self::search_column(&search_term, &self.data[current_col_idx], &mask)
                .into_iter()
                .map(|row_idx| (row_idx, current_col_idx))
                .collect()
        } else {
            let columns = &self.data;
            columns
                .par_iter()
                .enumerate()
                .flat_map(|(col_idx, column)| {
                    Self::search_column(&search_term, column, &mask)
                        .into_iter()
                        .map(move |row_idx| (row_idx, col_idx))
                        .collect::<Vec<_>>()
                })
                .collect()
        };

        let search_duration = start_time.elapsed().as_millis();

        if matching_rows.len() == 0 {
            table.search_results.clear();
            self.set_status_message(format!("Found no matches!"));
        } else {
            // Sort by rows
            table.search_results = matching_rows.into_iter().collect();
            table.search_results.sort_unstable();

            // Set the search index to the first match that is after the cursor
            let curser_ridx = table.offset_row + table.curser_row;
            table.search_idx = table
                .search_results
                .iter()
                .position(|&(row, _col)| row >= curser_ridx)
                .unwrap_or(0);

            trace!(
                "Search found {} matching rows in {}ms",
                table.search_results.len(),
                search_duration
            );

            trace!("Matches {:?}", table.search_results);

            self.search_next(0);

            let table = self.tables.last().unwrap();
            self.set_status_message(format!("Found {} results", table.search_results.len()));
        }
    }

    // Sets the curser to the next search result
    fn search_next(&mut self, step: i32) {
        // Note: step has to be -1, 0, 1
        let mut next_match: Option<(usize, usize)> = None;
        let mut next_match_idx = 0;
        let table = self.tables.last_mut().unwrap();
        let total_matches = table.search_results.len();
        if total_matches > 0 {
            if step >= 0 {
                let s = step as usize;
                if table.search_idx + s >= total_matches {
                    table.search_idx = 0;
                } else {
                    table.search_idx += s;
                }
            } else if table.search_idx as i32 + step < 0 {
                table.search_idx = table.search_results.len() - 1;
            } else {
                table.search_idx = (table.search_idx as i32 + step) as usize;
            }
            next_match = Some((
                table.search_results[table.search_idx].0,
                table.search_results[table.search_idx].1,
            ));
            next_match_idx = table.search_idx;
        }

        if let Some((row, column)) = next_match {
            self.select_cell(row, column);
            self.set_status_message(format!(
                "Search result {}/{}",
                next_match_idx + 1,
                total_matches
            ));
            trace!(
                "Selecting next search result {}/{}, pos {}:{}",
                next_match_idx + 1,
                total_matches,
                row,
                column
            );
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
            // LLM generated matches will order partial float converation, giving order preference to successfull converted floats
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
        trace!("Select record {}:{}", row, column);

        // If relevant column is already visible, only select the right row, otherwise move the view.
        if table.visible_columns.contains(&column) {
            table.curser_column = table
                .visible_columns
                .iter()
                .position(|&c| c == column)
                .unwrap_or(0);
        } else {
            table.offset_column = column;
            table.curser_column = 0;
        }

        if row >= table.offset_row && row < table.offset_row + table.heigh {
            table.curser_row = row - table.offset_row;
        } else {
            table.curser_row = 0;
            table.offset_row = row;
        }

        self.update_table_data();
    }

    fn filter(&mut self, term: &str) {
        trace!("Starting filter for {} ...", term);
        let table = self.tables.last_mut().unwrap();
        let start_time = Instant::now();

        let matches = Self::search_column(
            term,
            &self.data[table.offset_column + table.curser_column],
            &table.rows,
        );

        let search_duration = start_time.elapsed().as_millis();

        trace!(
            "Search found {} matching rows in {}ms",
            matches.len(),
            search_duration
        );

        self.filter_table(matches);
    }

    fn filter_table(&mut self, indices: Vec<usize>) {
        let mut new_table = TableView::empty();
        let table = self.tables.last().unwrap();
        new_table.name = format!("F[{}]", self.tables.last().unwrap().name);
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
        let row = table.rows[table.offset_row + table.curser_row];
        let column = table.offset_column + table.curser_column;
        let cell = self.data[column].data[row].clone();
        trace!("Cell content: {}", cell);

        match self.clipboard.set_text(cell) {
            Ok(_) => trace!("Copied cell content to clipboard."),
            Err(e) => trace!("Error copying to clipboard: {:?}", e),
        }
    }

    fn wrap_cell_content(c: &String) -> String {
        let needs_escaping = c.chars().any(|c| c == '"');
        let needs_wrapping = c.chars().any(|c| c == ' ' || c == '\t' || c == ',');
        let mut out = String::from(c);

        if needs_escaping {
            out = out.replace("\"", "\"\"");
        }
        if needs_wrapping {
            out = format!("\"{out}\"");
        }
        out
    }

    fn copy_table_row(&mut self) {
        let table = self.tables.last().unwrap();
        let row = table.rows[table.offset_row + table.curser_row];

        let content = self
            .data
            .iter()
            .map(|c| Model::wrap_cell_content(&c.data[row]))
            .collect::<Vec<String>>();
        let row_content = content.join(",");

        match self.clipboard.set_text(row_content) {
            Ok(_) => trace!("Copied cell content to clipboard."),
            Err(e) => trace!("Error copying to clipboard: {:?}", e),
        }
    }

    fn toggle_column_status(&mut self, toggle_to_expand: bool) {
        let table = self.tables.last_mut().unwrap();
        let new_status = if toggle_to_expand {
            match self.data[table.visible_columns[table.curser_column]].status {
                ColumnStatus::COLLAPSED => ColumnStatus::EXPANDED,
                ColumnStatus::NORMAL => ColumnStatus::EXPANDED,
                ColumnStatus::EXPANDED => ColumnStatus::COLLAPSED,
            }
        } else {
            match self.data[table.visible_columns[table.curser_column]].status {
                ColumnStatus::COLLAPSED => ColumnStatus::NORMAL,
                ColumnStatus::NORMAL => ColumnStatus::COLLAPSED,
                ColumnStatus::EXPANDED => ColumnStatus::COLLAPSED,
            }
        };
        self.data[table.visible_columns[table.curser_column]].status = new_status;
        self.update_table_data();
    }

    fn move_table_selection_beginning(&mut self) {
        let table = self.tables.last_mut().unwrap();
        table.curser_row = 0;
        table.offset_row = 0;
        self.update_table_data();
    }

    fn move_table_selection_end(&mut self) {
        let table = self.tables.last_mut().unwrap();
        if table.rows.len() < self.uilayout.table_height {
            table.offset_row = 0;
            table.curser_row = table.rows.len() - 1;
        } else {
            table.offset_row = table.rows.len() - self.uilayout.table_height;
            table.curser_row = self.uilayout.table_height - 1;
        }
        self.update_table_data();
    }

    fn move_table_selection_up(&mut self, size: usize) {
        let table = self.tables.last_mut().unwrap();
        if table.curser_row > 0 {
            // Curser somewhere in the middle
            table.curser_row = table.curser_row.saturating_sub(size);
        } else {
            // Curser at the top
            if table.offset_row > 0 {
                // Shift table up
                table.offset_row = table.offset_row.saturating_sub(size);
            }
        }
        self.update_table_data();
    }

    fn move_table_selection_down(&mut self, size: usize) {
        let table = self.tables.last_mut().unwrap();
        if table.curser_row + table.offset_row < (table.rows.len() - 1) {
            // Somewhere in the Frame
            if table.curser_row < self.uilayout.table_height - 1 {
                // Somewhere in the middle of the table
                table.curser_row =
                    std::cmp::min(table.curser_row + size, table.data[0].data.len() - 1);
            } else {
                // At the bottom of the table, need to shift table down
                table.offset_row = std::cmp::min(table.offset_row + size, table.rows.len() - 1);
                table.curser_row = std::cmp::min(
                    self.uilayout.table_height - 1,
                    table.rows.len() - table.offset_row - 1,
                );
            }
            self.update_table_data();
        }
    }

    fn move_table_selection_left(&mut self) {
        let table = self.tables.last_mut().unwrap();
        if table.curser_column > 0 {
            table.curser_column = table.curser_column.saturating_sub(1);
        } else if table.offset_column > 0 {
            table.offset_column = table.offset_column.saturating_sub(1);
        }
        self.update_table_data();
    }

    fn move_table_selection_right(&mut self) {
        let table = self.tables.last_mut().unwrap();

        if table.curser_column + table.offset_column < (self.data.len() - 1) {
            // Somewhere before the last column
            if table.curser_column < (table.visible_columns.len() - 1) {
                // In the middle
                table.curser_column += 1;
            } else {
                // At the end of the screen
                table.offset_column += 1;
            }
            self.update_table_data();
        } else {
            // At the last visible column (which could be wider then the screen)
            if table.visible_width > table.width && table.offset_column < (self.data.len() - 1) {
                table.offset_column += 1;
                self.update_table_data();
            }
        }
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
        if record.curser_row > 0 {
            // Curser somewhere in the middle
            record.curser_row = record.curser_row.saturating_sub(size);
        } else {
            // Curser at the top
            if record.curser_offset > 0 {
                // Shift table up
                record.curser_offset = record.curser_offset.saturating_sub(size);
            }
        }
        self.update_record_data();
    }

    fn move_histogram_selection_up(&mut self, size: usize) {
        let hist = &mut self.histogram_view;
        if hist.curser_row > 0 {
            // Curser somewhere in the middle
            hist.curser_row = hist.curser_row.saturating_sub(size);
        } else {
            // Curser at the top
            if hist.curser_offset > 0 {
                // Shift table up
                hist.curser_offset = hist.curser_offset.saturating_sub(size);
            }
        }
        self.update_histogram_view();
    }

    fn move_record_selection_down(&mut self, size: usize) {
        let record = &mut self.record_view;
        if record.curser_row + record.curser_offset < (record.row_data.len() - 1) {
            // Somewhere in the middle
            if record.curser_row < record.height - 1 {
                // Somewhere in the middle of the table
                record.curser_row =
                    std::cmp::min(record.curser_row + size, record.row_view.data.len() - 1);
            } else {
                // At the bottom of the table, need to shift table down
                record.curser_offset =
                    std::cmp::min(record.curser_offset + size, record.row_data.len() - 1);
                record.curser_row = std::cmp::min(
                    record.height - 1,
                    record.row_data.len() - record.curser_offset,
                );
            }
            self.update_record_data();
        }
    }

    fn move_histogram_selection_down(&mut self, size: usize) {
        let hist = &mut self.histogram_view;
        if hist.curser_row + hist.curser_offset < (hist.value_data.len() - 1) {
            // Somewhere in the middle
            if hist.curser_row < hist.height - 1 {
                // Somewhere in the middle of the table
                hist.curser_row =
                    std::cmp::min(hist.curser_row + size, hist.value_view.data.len() - 1);
            } else {
                // At the bottom of the table, need to shift table down
                hist.curser_offset =
                    std::cmp::min(hist.curser_offset + size, hist.value_data.len() - 1);
                hist.curser_row =
                    std::cmp::min(hist.height - 1, hist.value_data.len() - hist.curser_offset);
            }
            self.update_histogram_view();
        }
    }

    fn previous_record(&mut self) {
        let record = &mut self.record_view;
        if record.record_idx > 0 {
            record.record_idx = record.record_idx.saturating_sub(1);
        }
        self.update_record_data();
    }

    fn next_record(&mut self) {
        let record = &mut self.record_view;
        let table = self.tables.last().unwrap();
        if record.record_idx < table.rows.len() - 1 {
            record.record_idx += 1;
        }
        self.update_record_data();
    }
}
