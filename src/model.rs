use std::collections::HashMap;
use std::path::{PathBuf, Path};
use std::fs;
use std::io::ErrorKind;
use std::time::Instant;
use polars::prelude::*;
use ratatui::symbols::line::VERTICAL_RIGHT;
use tracing::{info, debug, error, warn, trace};
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

pub struct ColumnView<'a> {
    pub name: String,
    pub width: usize,
    pub data: Vec<&'a String>,
}

#[derive(Debug, PartialEq)]
pub enum ColumnStatus {
    NORMAL,
    EXPANDED,
    COLLAPSED,
}

pub struct TableView {
    rows: Vec<usize>,
    selected_row: usize,
    selected_column: usize,
}

//#[derive(Debug)]
pub struct Model {
    file_info: FileInfo,
    config: TVConfig,
    frame: LazyFrame,
    pub status: Status,
    schema: Schema,
    columns: Vec<Column>,
    tables: Vec<TableView>,
    current_table: usize,
    last_update: Instant,
    last_data_change: Instant,
    last_render: Instant,
    selected_column: usize,
    table_width: usize,
    table_heigh: usize,
}

impl Model {
    pub fn from_file(path: PathBuf, config: &TVConfig) -> Result<Self, TVError> {
        let file_info = Model::get_file_info(path)?;
        let mut frame = match file_info.file_type {
            FileType::CSV => Model::load_csv(&file_info.path)?,
            FileType::PARQUET => todo!(),
            FileType::XLSX => todo!(),
        };
        let schema = frame.collect_schema()?.as_ref().clone();

        // Load dataframe using a different thread for each column
        // This is a very intensive operation as the data is pre-processed.
        // The returned columns hold all data as Strings in memory.
        let start_time = Instant::now();
        let df = Arc::new(frame.clone().collect()?);
        let c_: Result<Vec<Column>, _> = df
            .get_column_names()
            .par_iter()
            .enumerate()
            .map(|(idx, name)| Self::process_column(&df, idx, name))
            .collect();
        let columns = c_?;
        let data_loading_duration = start_time.elapsed().as_millis();
        info!("Loading data took {data_loading_duration}ms ...");
        for c in columns.iter() {
            debug!("Column: {}", c.as_string());
        }
        let table = TableView {
            rows: (0..columns[0].data.len()).collect(),
            selected_column: 0,
            selected_row: 0,
        };

        Ok(
            Self {
                file_info: file_info,
                config: config.clone(),
                frame: frame,
                status: Status::READY,
                schema: schema,
                columns: columns,
                tables: vec![table],
                current_table: 0,
                last_update: Instant::now() - std::time::Duration::from_secs(1),
                last_render: Instant::now() - std::time::Duration::from_secs(1),
                last_data_change: Instant::now(),
                selected_column: 0,
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
        let mut nrows = 0;
        if !self.columns.is_empty() {
            nrows = self.columns[0].data.len();
        }
        nrows
    }

    pub fn ncols(&self) -> usize {
        self.columns.len()
    }

    pub fn get_selected_row(&self) -> usize {
        self.tables[self.current_table].selected_row
    }

    pub fn get_selected_column(&self) -> usize {
        self.selected_column
    }

    pub fn get_visible_columns<'a>(&'a self) -> Vec<ColumnView<'a>> {

        let table = &self.tables[self.current_table];
        let rbegin = table.selected_row.saturating_sub(self.table_heigh);
        let rend = (rbegin + self.table_heigh).min(table.rows.len());

        debug!("SR {}, SC {}, Rb {}, Re {}, th: {}, tw: {}", table.selected_row, self.selected_column, rbegin, rend, self.table_heigh, self.table_width);
        // table.rows[rbegin..rend]
        //     .iter()
        //     .map(|&ridx| &column.data[ridx])
        //     .collect()
 

        // Edge case
        // Selected Column is too wide to render on the full table
        /*
        if self.columns[self.selected_column].render_width >= self.table_width {
            let sel_col = &self.columns[self.selected_column];
            let available_name_length = std::cmp::max(sel_col.name.len()+2, self.table_width);

            if available_name_length < sel_col.name.len() {
                let name = sel_col.name[0..(available_name_length-3)].to_string();
                name += "...";
            } else {
                let name = sel_col.name;
            }
            visible_columns.push(ColumnView{
                name: name,
                width: available_name_length,
                data: 
            }) 
        } */

        // Get the selected column and all possible ones to its left side.
        let mut columns_idx = Vec::new();
        let mut width_budget = self.table_width;
        for cidx in (0..=self.selected_column).rev() {
            if self.columns[cidx].render_width <= width_budget {
                columns_idx.push(cidx);
                width_budget -= self.columns[cidx].render_width;
            }
            else {
                break;
            }
        }

        // If there is still space, fill it up with columns to the right
        if width_budget > 0 {
            for cidx in self.selected_column+1..self.columns.len() {
                if self.columns[cidx].render_width <= width_budget {
                    columns_idx.push(cidx);
                    width_budget -= self.columns[cidx].render_width;
                }
                else {
                    break;
                }
            }
        }
        // Sort to keep columns in the right order
        columns_idx.sort();

        let mut visible_columns = Vec::with_capacity(columns_idx.len());
        for &idx in &columns_idx {
            if let Some(column) = self.columns.get(idx) {
                let col_data = table.rows[rbegin..rend]
                    .iter()
                    .map(|&ridx| &column.data[ridx])
                    .collect();
                let name = Self::get_visible_name(column.name.clone(), column.render_width);
                let width = column.render_width;
                //trace!("Visible Column: \"{name}\", width: {width}");

                visible_columns.push(
                    ColumnView{
                        name: name,
                        width: width,
                        data: col_data
                    }
                );
            } else {
                error!("Trying to access column with unknown idx {idx}!");
            }
        }
  
        return visible_columns;
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
        return reduced_name;
    }

    async fn load_columns(frame: &LazyFrame) -> Result<Vec<Column>, TVError> {
        // Collect once - shared cost
        let df = frame.clone().collect()?;
        let df = Arc::new(df);  // Share DataFrame across threads
        
        let mut tasks = Vec::new();
        
        for (idx, col_name) in df.get_column_names().iter().enumerate() {
            let df_clone = Arc::clone(&df);
            let col_name = col_name.to_string();
            
            let task = tokio::spawn(async move {
                Self::process_column(&df_clone, idx, &col_name)
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

    fn process_column(df: &DataFrame, idx: usize, col_name: &str) -> Result<Column, PolarsError> {
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
            width_max: width_max,
            render_width: 0, // Will be set later
            histogram: counts,
            data: data,
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
            for column in self.columns.iter_mut() {
                let render_width = Self::calculate_column_width(&column, self.config.default_column_width);
                column.render_width = render_width;
            }
        }

        if let Some(msg) = message {
            match msg {
                Message::Quit => self.exit(),
                Message::MoveDown => self.move_selection_down(),
                Message::MoveLeft => self.move_selection_left(),
                Message::MoveRight => self.move_selection_right(),
                Message::MoveUp => self.move_selection_up(),
            }
        }

        self.last_update = Instant::now();
        Ok(())
    }


    fn move_selection_up(&mut self) {
        self.tables[self.current_table].selected_row = self.tables[self.current_table].selected_row.saturating_sub(1);
    }

    fn move_selection_down(&mut self) {
        if self.tables[self.current_table].selected_row < self.tables[self.current_table].rows.len() {
            self.tables[self.current_table].selected_row += 1;
        }
    }

    fn move_selection_left(&mut self) {
        self.tables[self.current_table].selected_column = self.tables[self.current_table].selected_column.saturating_sub(1);
    }

    fn move_selection_right(&mut self) {
        if self.selected_column < (self.ncols()-1){
            self.selected_column += 1;
        }
    }

    // pub fn get_headers(&self) -> impl Iterator<Item = &str> + '_ {
    //     self.schema.iter_names().map(|s| s.as_str())
    // }

    // pub fn get_column_data(&self, column_idx: usize, row_idxs: Vec<usize>) -> Result<Vec<&String>, TVError> {
    //     let column = self.columns.get(column_idx).ok_or(TVError::DataIndexingError("Column index out of bounds".into()))?;
    
    //     if row_idxs.is_empty() {
    //         // Return all rows
    //         Ok(column.data.iter().collect())
    //     } else {
    //         let mut result = Vec::with_capacity(row_idxs.len());
    //         for idx in row_idxs {
    //             let value = column.data
    //                 .get(idx)
    //                 .ok_or_else(|| TVError::DataIndexingError(format!("Row {} not found", idx)))?;
    //             result.push(value);
    //         }
    //         Ok(result)
    //     }
    // }
}