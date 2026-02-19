use std::time::Instant;

use tracing::trace;

use crate::model::{UIData, table_view::TableView};

use super::{Column, ColumnView};

pub struct RecordView {
    pub header_data: Vec<String>,
    pub header_width: usize,
    pub header_view: ColumnView,
    pub row_data: Vec<String>, // Add row values
    pub row_width: usize,
    pub row_view: ColumnView,
    pub last_record_idx: usize, // Index in TableView.rows[XXX]
    pub curser_row: usize,
    pub curser_offset: usize,
    pub last_update: Instant,
    pub height: usize, // UI height
    pub width: usize,  // UI Width
}

impl RecordView {
    pub fn empty() -> Self {
        RecordView {
            header_data: Vec::new(),
            header_width: 0,
            header_view: ColumnView::empty(),
            row_data: Vec::new(),
            row_width: 0,
            row_view: ColumnView::empty(),
            last_record_idx: 0,
            curser_row: 0,
            curser_offset: 0,
            last_update: Instant::now(),
            height: 0,
            width: 0,
        }
    }

    pub fn new(
        table: &TableView,
        data: &Vec<Column>,
        uidata: &mut UIData,
        max_column_width: usize,
    ) -> Self {
        let mut record = RecordView::empty();
        // Get header names
        record.header_data = data
            .iter()
            .map(|c| c.name.chars().take(max_column_width).collect::<String>())
            .collect::<Vec<String>>();

        record.curser_offset = 0;
        record.curser_row = 0;
        record.last_record_idx = 999999; // Hack in order to self.update() to actually set self.row_data
        record.height = uidata.layout.table_height;
        record.width = uidata.layout.table_width;

        record.header_width = record
            .header_data
            .iter()
            .map(|h| h.len())
            .max()
            .unwrap_or(0);
        record.row_width = record.width - record.header_width;
        record.update(table.curser_row + table.offset_row, table, data, uidata);
        record
    }

    pub fn next_record(&mut self, table: &TableView, data: &mut Vec<Column>, uidata: &mut UIData) {
        if self.last_record_idx < table.rows.len() - 1 {
            self.update(self.last_record_idx + 1, table, data, uidata);
        }
    }

    pub fn previous_record(
        &mut self,
        table: &TableView,
        data: &mut Vec<Column>,
        uidata: &mut UIData,
    ) {
        if self.last_record_idx > 0 {
            self.update(self.last_record_idx - 1, table, data, uidata);
        }
    }

    pub fn move_selection_up(
        &mut self,
        size: usize,
        table: &TableView,
        data: &mut Vec<Column>,
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
        self.update(self.last_record_idx, table, data, uidata);
    }

    pub fn move_selection_down(
        &mut self,
        size: usize,
        table: &TableView,
        data: &mut Vec<Column>,
        uidata: &mut UIData,
    ) {
        if self.curser_row + self.curser_offset < (self.row_data.len() - 1) {
            // Somewhere in the middle
            if self.curser_row < self.height - 1 {
                // Somewhere in the middle of the table
                self.curser_row =
                    std::cmp::min(self.curser_row + size, self.row_view.data.len() - 1);
            } else {
                // At the bottom of the table, need to shift table down
                self.curser_offset =
                    std::cmp::min(self.curser_offset + size, self.row_data.len() - 1);
                self.curser_row =
                    std::cmp::min(self.height - 1, self.row_data.len() - self.curser_offset);
            }
            self.update(self.last_record_idx, table, data, uidata);
        }
    }

    pub fn update(
        &mut self,
        record_idx: usize,
        table: &TableView,
        data: &Vec<Column>,
        uidata: &mut UIData,
    ) {
        if self.last_record_idx != record_idx {
            self.row_data = data
                .iter()
                .map(|c| c.data[table.rows[record_idx]].clone())
                .collect::<Vec<String>>();
        }

        let rbegin = self.curser_offset;
        let rend = std::cmp::min(rbegin + self.height, self.row_data.len());

        trace!(
            "Record: rIdx {}, rb {}, re {}, rows {}",
            record_idx,
            rbegin,
            rend,
            self.row_data.len()
        );
        self.header_view = ColumnView {
            name: "Headers".to_string(),
            data: self.header_data[rbegin..rend].to_vec(),
            width: self.header_width,
        };

        self.row_view = ColumnView {
            name: "Values".to_string(),
            data: self.row_data[rbegin..rend].to_vec(),
            width: self.row_width,
        };
        self.last_record_idx = record_idx;
        self.last_update = Instant::now();
        self.update_uidata(table, uidata);
    }

    fn update_uidata(&mut self, table: &TableView, uidata: &mut UIData) {
        uidata.name = format!("R[{}]", table.name);
        uidata.table = vec![self.header_view.clone(), self.row_view.clone()];
        uidata.selected_row = self.curser_row;
        uidata.selected_column = 1;
        uidata.nrows = table.rows.len();
        uidata.abs_selected_row = self.last_record_idx; // In the record view, show which record we are looking at instead of line in record view.
        uidata.last_update = Instant::now();
    }
}
