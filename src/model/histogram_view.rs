use std::{collections::HashMap, time::Instant};

use tracing::trace;

use crate::model::{Column, TableView, UIData};

use super::ColumnView;

pub struct HistogramView {
    pub value_data: Vec<String>,
    pub column_histograms: HashMap<usize, (Vec<usize>, Vec<String>)>,
    pub value_width: usize,
    pub value_view: ColumnView,
    pub count_data: Vec<String>, // Count in absolute and relative values
    pub count_width: usize,
    pub count_view: ColumnView,
    pub column_idx: usize, // Index in Model.data[0][XXX]
    pub curser_row: usize,
    pub curser_offset: usize,
    pub last_update: Instant,
    pub last_column_idx: usize,
    pub height: usize, // UI height
    pub width: usize,  // UI Width
}

impl HistogramView {
    pub fn empty() -> Self {
        HistogramView {
            value_data: Vec::new(),
            column_histograms: HashMap::new(),
            value_width: 0,
            value_view: ColumnView::empty(),
            count_data: Vec::new(),
            count_width: 0,
            count_view: ColumnView::empty(),
            column_idx: 0,
            curser_row: 0,
            curser_offset: 0,
            last_update: Instant::now(),
            last_column_idx: 99999,
            height: 0,
            width: 0,
        }
    }

    fn calculate_column_histogram(
        &mut self,
        column_idx: usize,
        data: &mut Vec<Column>,
        table: &TableView,
    ) {
        trace!("Calculate histogram for column {}", column_idx);
        self.column_histograms.entry(column_idx).or_insert_with(|| {
            let column_data = &data[column_idx].data;

            let mut counts: HashMap<String, usize> = HashMap::new();
            for &ridx in table.rows.iter() {
                *counts.entry(column_data[ridx].clone()).or_insert(0) += 1;
            }
            let mut sorted: Vec<(usize, String)> =
                counts.iter().map(|(k, v)| (*v, k.clone())).collect();
            sorted.sort_unstable();
            sorted.reverse();
            let (counts, values): (Vec<usize>, Vec<String>) = sorted.into_iter().unzip();
            (counts, values)
        });
    }

    pub fn move_selection_up(
        &mut self,
        size: usize,
        data: &mut Vec<Column>,
        table: &TableView,
        uidata: &mut UIData,
    ) {
        if self.curser_row > 0 {
            // Curser somewhere in the middle
            self.curser_row = self.curser_row.saturating_sub(size);
        } else {
            // Curser at the top
            if self.curser_offset > 0 {
                // Shift table up
                self.curser_offset = self.curser_offset.saturating_sub(size);
            }
        }
        self.update(self.last_column_idx, data, table, uidata);
    }

    pub fn move_selection_down(
        &mut self,
        size: usize,
        data: &mut Vec<Column>,
        table: &TableView,
        uidata: &mut UIData,
    ) {
        if self.curser_row + self.curser_offset < (self.value_data.len() - 1) {
            // Somewhere in the middle
            if self.curser_row < self.height - 1 {
                // Somewhere in the middle of the table
                self.curser_row =
                    std::cmp::min(self.curser_row + size, self.value_view.data.len() - 1);
            } else {
                // At the bottom of the table, need to shift table down
                self.curser_offset =
                    std::cmp::min(self.curser_offset + size, self.value_data.len() - 1);
                self.curser_row =
                    std::cmp::min(self.height - 1, self.value_data.len() - self.curser_offset);
            }
            self.update(self.last_column_idx, data, table, uidata);
        }
    }

    pub fn update(
        &mut self,
        column_idx: usize,
        data: &mut Vec<Column>,
        table: &TableView,
        uidata: &mut UIData,
    ) {
        if self.last_column_idx != column_idx {
            self.calculate_column_histogram(column_idx, data, table);
            let counts = &self.column_histograms[&column_idx];
            self.curser_offset = 0;
            self.curser_row = 0;
            self.column_idx = column_idx;
            self.height = table.heigh;
            self.width = table.width;

            let nrecords = table.rows.len();
            self.count_data = counts
                .0
                .iter()
                .map(|&c| format!("{:.0}% {}", c as f64 * 100.0 / nrecords as f64, c))
                .collect();
            self.value_data = counts.1.clone();
        }

        let rbegin = self.curser_offset;
        let rend = std::cmp::min(rbegin + self.height, self.value_data.len());

        self.count_width = self.count_data[0].len();
        self.count_view = ColumnView {
            name: "Counts".to_string(),
            data: self.count_data[rbegin..rend].to_vec(),
            width: self.count_width,
        };

        self.value_width = self.width - self.count_width;
        self.value_view = ColumnView {
            name: "Values".to_string(),
            data: self.value_data[rbegin..rend].to_vec(),
            width: self.value_width,
        };

        self.last_column_idx = column_idx;
        self.update_uidata(&table.name, uidata);
    }

    pub fn update_uidata(&self, table_name: &str, uidata: &mut UIData) {
        uidata.name = format!("H[{}]", table_name);
        uidata.table = vec![self.count_view.clone(), self.value_view.clone()];
        uidata.selected_column = 1;
        uidata.nrows = self.value_data.len();
        uidata.selected_row = self.curser_row;
        uidata.abs_selected_row = self.curser_row + self.curser_offset;
        uidata.last_update = Instant::now();
    }
}
