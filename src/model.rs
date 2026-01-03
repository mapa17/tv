use std::collections::HashMap;
use std::path::{PathBuf, Path};
use std::fs;
use std::io::ErrorKind;
use std::time::Instant;
use polars::prelude::*;
use tracing::{info, debug, error, trace};
use rayon::prelude::*;

use crate::domain::{TVError, Message, TVConfig};

// A struct with different types
#[derive(Debug)]
enum FileType {
    CSV,
    PARQUET,
    XLSX,
}

// A struct with different types
#[derive(Debug, PartialEq)]
pub enum Status {
    EMPTY,
    READY,
    LOADING,
    PROCESSING,
    EXITING,
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
    width: usize, // q95 width
    width_max: usize,
    histogram: HashMap<String, usize>,
    render_width: usize,
    data: Vec<String>,
}

impl Column {
    pub fn as_string(&self) -> String {
        format!("{} \"{}\", {:?}, width: {}, width_max: {}, render_width: {}, # rows {}", 
        self.idx,
        self.name,
        self.status,
        self.width,
        self.width_max,
        self.render_width,
        self.data.len(),
    )
    }
}

pub struct ColumnView {
    pub name: String,
    pub width: usize,
    pub data: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub enum ColumnStatus {
    NORMAL,
    EXPANDED,
    COLLAPSED,
}

pub struct TableView {
    data_idx: usize, // Dataset index
    rows: Vec<usize>, // Mapping of TableView row index to data index
    visible_columns: Vec<usize>, // Idx of visible columns that are send to the UI for rendering.
    curser_row: usize,
    curser_column: usize,
    offset_row: usize,
    offset_column: usize,
    data: Vec<ColumnView>,
}

//#[derive(Debug)]
pub struct Model {
    file_info: FileInfo,
    config: TVConfig,
    frame: LazyFrame,
    pub status: Status,
    data: Vec<Vec<Column>>,
    tables: Vec<TableView>,
    current_table: usize,
    last_update: Instant,
    last_data_change: Instant,
    last_render: Instant,
    table_width: usize,
    table_heigh: usize,
}

impl Model {
    pub fn from_file(path: PathBuf, config: &TVConfig) -> Result<Self, TVError> {
        let file_info = Model::get_file_info(path)?;
        let frame = match file_info.file_type {
            FileType::CSV => Model::load_csv(&file_info.path)?,
            FileType::PARQUET => todo!(),
            FileType::XLSX => todo!(),
        };

        // Load dataframe using a different thread for each column
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
        let table = TableView {
            data_idx: 0,
            rows: (0..columns[0].data.len()).collect(),
            visible_columns: Vec::new(),
            curser_column: 0,
            curser_row: 0,
            offset_column: 0,
            offset_row: 0,
            data: Vec::new(),
        };

        Ok(
            Self {
                file_info,
                config: config.clone(),
                frame,
                status: Status::READY,
                data: vec![columns],
                tables: vec![table],
                current_table: 0,
                last_update: Instant::now() - std::time::Duration::from_secs(1),
                last_render: Instant::now() - std::time::Duration::from_secs(1),
                last_data_change: Instant::now(),
                table_heigh: 0,
                table_width: 0,
            })
    }

    fn detect_file_type(path: &Path) -> Result<FileType, TVError> {
        match path.extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_uppercase())
            .as_deref()
        {
            Some("CSV") => Ok(FileType::CSV),
            Some("PARQUET") => Ok(FileType::PARQUET),
            Some("XLSX") => Ok(FileType::XLSX),
            _ => Err(TVError::UnknownFileType),
        }
    }

    pub fn nrows(&self) -> usize {
        let table = &self.tables[self.current_table];
        table.rows.len()
    }

    pub fn ncols(&self) -> usize {
        let table = &self.tables[self.current_table];
        self.data[table.data_idx].len()
    }

    pub fn selected_row(&self) -> usize {
        let table = &self.tables[self.current_table];
        table.curser_row
    }

    pub fn selected_column(&self) -> usize {
        let table = &self.tables[self.current_table];
        table.curser_column
    }

    pub fn row_absolute(&self) -> usize {
        let table = &self.tables[self.current_table];
        table.offset_row + table.curser_row
    }

    pub fn column_absolute(&self) -> usize {
        let table = &self.tables[self.current_table];
        table.offset_column + table.curser_column
    }

    pub fn get_visible_columns(&self) -> &Vec<ColumnView> {

        let table = &self.tables[self.current_table];
        return &table.data;
    }

    fn update_frame_data(&mut self) {
        let table = &mut self.tables[self.current_table];
        let columns = &self.data[table.data_idx];
        //let rbegin = table.selected_row.saturating_sub(self.table_heigh);
        let rbegin = table.offset_row;
        let rend = (rbegin + self.table_heigh).min(table.rows.len());

        trace!("Cr {}, Cc {}, Or {}, Oc {}, Rb {}, Re {}, th: {}, tw: {}", table.curser_row, table.curser_column, table.offset_row, table.offset_column, rbegin, rend, self.table_heigh, self.table_width);

        /*
        // Get the selected column and all possible ones to its left side.
        let mut columns_idx = Vec::new();
        let mut width_budget = self.table_width;
        for cidx in (0..=selected_column).rev() {
            if data[cidx].render_width <= width_budget {
                columns_idx.push(cidx);
                width_budget -= data[cidx].render_width;
            }
            else {
                break;
            }
        }

        // If there is still space, fill it up with columns to the right
        if width_budget > 0 {
            for cidx in selected_column+1..data.len() {
                if data[cidx].render_width <= width_budget {
                    columns_idx.push(cidx);
                    width_budget -= data[cidx].render_width;
                }
                else {
                    break;
                }
            }
        }
        // Sort to keep columns in the right order
        columns_idx.sort();
        trace!("Visible columns: {columns_idx:?}");
        */
        let mut visible_columns_idx = Vec::new();
        let mut width_budget = self.table_width;
        for cidx in table.offset_column..columns.len() {
            if columns[cidx].render_width <= width_budget {
                visible_columns_idx.push(cidx);
                width_budget -= columns[cidx].render_width;
            }
            else {
                break;
            }
        }

        // Create ColumnViews for visible columns
        table.data.clear();
        table.data = Vec::with_capacity(visible_columns_idx.len());
        for idx in visible_columns_idx {
            if let Some(column) = columns.get(idx) {
                let col_data = table.rows[rbegin..rend]
                    .iter()
                    .map(|&ridx| column.data[ridx].clone())
                    .collect();
                let name = Self::get_visible_name(column.name.clone(), column.render_width);
                let width = column.render_width;
                //trace!("Visible Column: \"{name}\", width: {width}");

                table.data.push(
                    ColumnView{
                        name,
                        width,
                        data: col_data
                    }
                );
            } else {
                error!("Trying to access column with unknown idx {idx}!");
            }
        }
    }

    fn get_visible_name(name: String, width: usize) -> String {
        let mut reduced_name = name.clone();
        if width < 3 {
            return "".to_string();
        }
        if reduced_name.len() > width {
            reduced_name = reduced_name[0..width-3].to_string();
            reduced_name.push_str("...");
        }
        reduced_name
    }

    async fn load_frame(frame: &LazyFrame) -> Result<Vec<Column>, TVError> {
        // Collect once - shared cost
        let df = frame.clone().collect()?;
        let df = Arc::new(df);  // Share DataFrame across threads
        
        let mut tasks = Vec::new();
        
        for (idx, col_name) in df.get_column_names().iter().enumerate() {
            let df_clone = Arc::clone(&df);
            let col_name = col_name.to_string();
            
            let task = tokio::spawn(async move {
                Self::load_columns(&df_clone, idx, &col_name)
            });
            tasks.push(task);
        }
        
        // Wait for all tasks
        let mut columns = Vec::new();
        for task in tasks {
            let result = task.await
                .map_err(|e| TVError::LoadingFailed(format!("Loading column data failed: {}", e)))??;
            columns.push(result);
        }
        
        Ok(columns)
    }

    fn load_columns(df: &DataFrame, idx: usize, col_name: &str) -> Result<Column, PolarsError> {
        let col = df.column(col_name)?.cast(&DataType::String)?;
        let series = col.str()?;
        let mut lengths = Vec::with_capacity(series.len());
        let mut counts: HashMap<String, usize> = HashMap::new();
        let mut data = Vec::with_capacity(series.len());

        for value in series.into_iter() {
            let ss = match value {
                Some(s) => s.to_string(),
                None => String::from("∅"),
            };

            lengths.push(ss.len());
            *counts.entry(ss.clone()).or_insert(0) += 1;
            data.push(ss);
        } 

        lengths.sort_unstable();
        let q95_idx = ((lengths.len() as f64 * 0.95).ceil() as usize).min(lengths.len());
        let q95_length = lengths.get(q95_idx.saturating_sub(1)).copied().unwrap_or(col_name.len());
        let width_max = lengths.last().copied().unwrap_or(q95_length);
        //let render_width: min(width_max)
       
        Ok(Column {
            idx: idx as u16,
            name: col_name.to_string(),
            status: ColumnStatus::NORMAL,
            width: q95_length,
            width_max,
            render_width: 0, // Will be set later
            histogram: counts,
            data,
        })
    }

    fn get_file_info(path: PathBuf) -> Result<FileInfo, TVError> {

        let metadata = fs::metadata(&path)
            .map_err(|e| match e.kind() {
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

    fn calculate_column_width(column: &Column, default_width: usize) -> usize {
        match column.status {
            ColumnStatus::COLLAPSED => 3,
            ColumnStatus::NORMAL => std::cmp::max(column.name.len(), std::cmp::min(default_width, column.width)),
            ColumnStatus::EXPANDED => std::cmp::max(column.name.len(), column.width),
        }
    }

    fn load_csv(path: &PathBuf) -> Result<LazyFrame, PolarsError> {
        LazyCsvReader::new(PlPath::Local(path.as_path().into())).with_has_header(true).finish()
    }

    pub fn get_path(&self) -> PathBuf {
        self.file_info.path.clone()
    }

    pub fn exit(&mut self){
        self.status = Status::EXITING;
    }

    pub fn update(&mut self, message: Option<Message>, table_width: usize, table_heigh: usize) -> Result<(), TVError> {
        self.table_width = table_width;
        self.table_heigh = table_heigh;

        if self.last_data_change - self.last_update > std::time::Duration::ZERO {
            debug!("Underlying data has changed! Updating infos ...");
            let table = &self.tables[self.current_table];
            for column in self.data[table.data_idx].iter_mut() {
                let render_width = Self::calculate_column_width(column, self.config.default_column_width);
                column.render_width = render_width;
            }
            self.update_frame_data();
        }

        if let Some(msg) = message {
            match msg {
                Message::Quit => self.exit(),
                Message::MoveDown => self.move_selection_down(1),
                Message::MoveLeft => self.move_selection_left(),
                Message::MoveRight => self.move_selection_right(),
                Message::MoveUp => self.move_selection_up(1),
            }
        }

        self.last_update = Instant::now();
        Ok(())
    }


    fn move_selection_up(&mut self, size: usize) {

        let table = &mut self.tables[self.current_table];
        if table.curser_row > 0 {
            // Curser somewhere in the middle
            table.curser_row = table.curser_row.saturating_sub(size);
            self.update_frame_data();
        } else {
            // Curser at the top
            if table.offset_row > 0 {
                // Shift table up
                table.offset_row = table.offset_row.saturating_sub(size);
                self.update_frame_data();
            }
        }
        //self.tables[self.current_table].selected_row = self.tables[self.current_table].selected_row.saturating_sub(1);
    }

    fn move_selection_down(&mut self, size: usize) {

        let table = &mut self.tables[self.current_table];
        if table.curser_row + table.offset_row < (table.rows.len()-1) {
            // Somewhere in the Frame
            if table.curser_row < self.table_heigh-1 {
                // Somewhere in the middle of the table
                table.curser_row = std::cmp::min(table.curser_row + size, self.table_heigh-1);
                self.update_frame_data();
            } else {
                // At the bottom of the table, need to shift table down
                table.offset_row = std::cmp::min(table.offset_row + size, table.rows.len()-1);
                table.curser_row = std::cmp::min(self.table_heigh-1, table.rows.len()-table.offset_row);
                self.update_frame_data();
            }
        } 
        //if self.tables[self.current_table].selected_row < self.tables[self.current_table].rows.len() {
        //    self.tables[self.current_table].selected_row += 1;
        //}
    }

    fn move_selection_left(&mut self) {
        let table = &mut self.tables[self.current_table];
        if table.curser_column > 0 {
            table.curser_column = table.curser_column.saturating_sub(1);
            self.update_frame_data();
        } else {
            if table.offset_column > 0 {
                table.offset_column = table.offset_column.saturating_sub(1);
                self.update_frame_data();
            }
        }
        //self.tables[self.current_table].selected_column = self.tables[self.current_table].selected_column.saturating_sub(1);
    }

    fn move_selection_right(&mut self) {
        let table = &mut self.tables[self.current_table];

        if table.curser_column + table.offset_column < (self.data[table.data_idx].len()-1){
            // TODO: this needs to be handled!
            table.curser_column += 1;
            self.update_frame_data();
        }

        //if self.selected_column < (self.ncols()-1){
        //    self.selected_column += 1;
        //}
    }


}