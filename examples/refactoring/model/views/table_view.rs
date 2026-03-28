use std::collections::HashMap;
use std::sync::Arc;

use super::ColumnView;

/// Represents the state and data for rendering a table view.
/// 
/// This struct maintains the current view state including cursor position,
/// visible columns, search results, and column histograms.
pub struct TableView {
    pub(crate) name: String,
    pub(crate) rows: Arc<Vec<usize>>, // Mapping of TableView row index to data index
    pub(crate) visible_columns: Vec<usize>, // Idx of visible columns sent to UI
    pub(crate) visible_width: usize,
    pub(crate) curser_row: usize,
    pub(crate) curser_column: usize,
    pub(crate) offset_row: usize,
    pub(crate) offset_column: usize,
    pub(crate) data: Vec<ColumnView>,
    pub(crate) search_results: Vec<(usize, usize)>,
    pub(crate) search_idx: usize,
    pub(crate) show_index: bool,
    pub(crate) index: ColumnView,
    pub(crate) column_histograms: HashMap<usize, (Vec<usize>, Vec<String>)>,
    pub(crate) heigh: usize,
    pub(crate) width: usize,
}

impl TableView {
    /// Creates an empty TableView with default values.
    pub(crate) fn empty() -> Self {
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

    /// Builds the index column based on current visible rows.
    /// 
    /// This generates row numbers for the visible portion of the table,
    /// adjusting for the current offset.
    pub(crate) fn build_index(&mut self) {
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
