use polars::prelude::DataType;

pub struct Column {
    pub idx: u16,
    pub name: String,
    pub status: ColumnStatus,
    pub max_width: usize,
    pub render_width: usize,
    pub data: Vec<String>,
    pub dtype: DataType,
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

    // Return mask index positions of rows in the column that match given term
    pub fn search(&self, term: &str, mask: &[usize]) -> Vec<usize> {
        let mut matches = Vec::new();
        for (midx, &m) in mask.iter().enumerate() {
            if self.data[m].contains(term) {
                matches.push(midx)
            }
        }
        matches
    }
}

#[derive(Clone)]
pub struct ColumnView {
    pub name: String,
    pub width: usize,
    pub data: Vec<String>,
}

impl ColumnView {
    pub fn empty() -> Self {
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
