use std::collections::HashMap;
use std::path::{PathBuf, Path};
use std::fs;
use std::io::ErrorKind;
use std::time::Instant;
use polars::prelude::*;
use tracing::{info, debug};
use rayon::prelude::*;

use crate::domain::{TVError, Message};

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
    width: usize, // q95 width
    width_max: usize,
    histogram: HashMap<String, usize>,
    data: Vec<String>,
}

pub struct ColumnInfo {
    pub idx: u16,
    pub name: String,
    pub width: usize,
}

impl Column {
    pub fn as_string(&self) -> String {
        format!("{} \"{}\", width: {}, width_max: {}, # rows {}", 
        self.idx,
        self.name,
        self.width,
        self.width_max,
        self.data.len(),
    )
    }
}

//#[derive(Debug)]
pub struct Model {
    file_info: FileInfo,
    frame: LazyFrame,
    pub status: Status,
    schema: Schema,
    columns: Vec<Column>,
    pub last_update: Instant,
}

impl Model {
    pub fn load(path: PathBuf) -> Result<Self, TVError> {
        let file_info = Model::get_file_info(path)?;
        let mut frame = match file_info.file_type {
            FileType::CSV => Model::load_csv(&file_info.path)?,
            FileType::PARQUET => todo!(),
            FileType::XLSX => todo!(),
        };
        let schema = frame.collect_schema()?.as_ref().clone();

        let start_time = Instant::now();
        // let columns = tokio::runtime::Runtime::new()
        //     .unwrap()
        //     .block_on(Self::load_columns(&frame))?;
        
        let df = Arc::new(frame.clone().collect()?);
        let c_: Result<Vec<Column>, _> = df
            .get_column_names()
            .par_iter()
            .enumerate()
            .map(|(idx, name)| Self::process_column(&df, idx, name))
            .collect();
        let columns = c_?;
        let data_loading_duration = start_time.elapsed().as_millis();
        info!("Loading data needed {data_loading_duration}ms ...");

        for c in columns.iter() {
            debug!("Column: {}", c.as_string());
        }

        Ok(
            Self {
            file_info,
            frame,
            status: Status::READY,
            schema,
            columns: columns,
            last_update: Instant::now(),
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
        return nrows;
    }

    pub fn ncols(&self) -> usize {
        return self.columns.len();
    }

    pub fn get_column_info(&self, idx: usize) -> Result<ColumnInfo, TVError> {
        let column = self.columns.get(idx).ok_or(TVError::DataIndexingError("Column index out of bounds".into()))?;
        
        Ok(ColumnInfo {
            idx: column.idx,
            name: column.name.clone(),
            width: column.width,
        })
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
       
        Ok(Column {
            idx: idx as u16,
            name: col_name.to_string(),
            width: q95_length,
            width_max,
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

    fn load_csv(path: &PathBuf) -> Result<LazyFrame, PolarsError> {
        LazyCsvReader::new(PlPath::Local(path.as_path().into())).with_has_header(true).finish()
    }

    pub fn get_path(&self) -> PathBuf {
        self.file_info.path.clone()
    }

    pub fn exit(&mut self){
        self.status = Status::EXITING;
    }

    pub fn update(&mut self, message: Message) -> Result<(), TVError> {
        match message {
            Message::Quit => {
                self.exit();
            }
        };
        Ok(())
    }

    pub fn get_headers(&self) -> impl Iterator<Item = &str> + '_ {
        self.schema.iter_names().map(|s| s.as_str())
    }

    pub fn get_column_data(&self, column_idx: usize, row_idxs: Vec<usize>) -> Result<Vec<&String>, TVError> {
        let column = self.columns.get(column_idx).ok_or(TVError::DataIndexingError("Column index out of bounds".into()))?;
    
        if row_idxs.is_empty() {
            // Return all rows
            Ok(column.data.iter().collect())
        } else {
            let mut result = Vec::with_capacity(row_idxs.len());
            for idx in row_idxs {
                let value = column.data
                    .get(idx)
                    .ok_or_else(|| TVError::DataIndexingError(format!("Row {} not found", idx)))?;
                result.push(value);
            }
            Ok(result)
        }
    }
}