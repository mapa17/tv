use rayon::prelude::*;
use std::{sync::Arc, time::Instant};

use tracing::{error, trace};

use crate::{
    model::{Column, Model, UIData, UILayout, column_view::ColumnStatus},
    tui::{COLUMN_WIDTH_COLLAPSED_COLUMN, COLUMN_WIDTH_MARGIN},
};

use super::ColumnView;

pub struct TableView {
    pub name: String,
    pub rows: Arc<Vec<usize>>, // Mapping of TableView row index to data index. Wrap in arc to allow multi threaded access
    pub visible_columns: Vec<usize>, // Idx of visible columns that are send to the UI for rendering.
    pub visible_width: usize,
    pub curser_row: usize,
    pub curser_column: usize,
    pub offset_row: usize,
    pub offset_column: usize,
    pub data: Vec<ColumnView>, // Currently visible part of the table
    pub search_results: Vec<(usize, usize)>,
    pub search_idx: usize,
    pub show_index: bool,
    pub index: ColumnView,
    pub heigh: usize,
    pub width: usize,
}

impl TableView {
    pub fn empty() -> Self {
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
            heigh: 0,
            width: 0,
        }
    }

    // Helper function to wrap cell content to ensure csv valid format
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

    pub fn get_current_row(&self, data: &Vec<Column>) -> String {
        let row = self.rows[self.offset_row + self.curser_row];

        let content = data
            .iter()
            .map(|c| Self::wrap_cell_content(&c.data[row]))
            .collect::<Vec<String>>();
        let row_content = content.join(",");
        return row_content;
    }

    pub fn get_current_cell(&self, data: &Vec<Column>) -> String {
        let row = self.rows[self.offset_row + self.curser_row];
        let column = self.offset_column + self.curser_column;
        let cell = data[column].data[row].clone();
        return cell;
    }

    pub fn toggle_column_status(&mut self, data: &mut Vec<Column>, toggle_to_expand: bool) {
        let new_status = if toggle_to_expand {
            match data[self.visible_columns[self.curser_column]].status {
                ColumnStatus::COLLAPSED => ColumnStatus::EXPANDED,
                ColumnStatus::NORMAL => ColumnStatus::EXPANDED,
                ColumnStatus::EXPANDED => ColumnStatus::COLLAPSED,
            }
        } else {
            match data[self.visible_columns[self.curser_column]].status {
                ColumnStatus::COLLAPSED => ColumnStatus::NORMAL,
                ColumnStatus::NORMAL => ColumnStatus::COLLAPSED,
                ColumnStatus::EXPANDED => ColumnStatus::COLLAPSED,
            }
        };
        data[self.visible_columns[self.curser_column]].status = new_status;
    }

    pub fn move_selection_beginning(
        &mut self,
        data: &mut Vec<Column>,
        layout: &UILayout,
        uidata: &mut UIData,
    ) {
        self.curser_row = 0;
        self.offset_row = 0;
        self.update(data, layout, uidata);
    }

    pub fn move_selection_end(
        &mut self,
        data: &mut Vec<Column>,
        layout: &UILayout,
        uidata: &mut UIData,
    ) {
        if self.rows.len() < layout.table_height {
            self.offset_row = 0;
            self.curser_row = self.rows.len() - 1;
        } else {
            self.offset_row = self.rows.len() - layout.table_height;
            self.curser_row = layout.table_height - 1;
        }
        self.update(data, layout, uidata);
    }

    pub fn move_selection_up(
        &mut self,
        size: usize,
        data: &mut Vec<Column>,
        layout: &UILayout,
        uidata: &mut UIData,
    ) {
        if self.curser_row > 0 {
            // Curser somewhere in the middle
            self.curser_row = self.curser_row.saturating_sub(size);
        } else {
            // Curser at the top
            if self.offset_row > 0 {
                // Shift table up
                self.offset_row = self.offset_row.saturating_sub(size);
            }
        }
        self.update(data, layout, uidata);
    }

    pub fn move_selection_down(
        &mut self,
        size: usize,
        data: &mut Vec<Column>,
        layout: &UILayout,
        uidata: &mut UIData,
    ) {
        if self.curser_row + self.offset_row < (self.rows.len() - 1) {
            // Somewhere in the Frame
            if self.curser_row < layout.table_height - 1 {
                // Somewhere in the middle of the table
                self.curser_row =
                    std::cmp::min(self.curser_row + size, self.data[0].data.len() - 1);
            } else {
                // At the bottom of the table, need to shift table down
                self.offset_row = std::cmp::min(self.offset_row + size, self.rows.len() - 1);
                self.curser_row = std::cmp::min(
                    layout.table_height - 1,
                    self.rows.len() - self.offset_row - 1,
                );
            }
            self.update(data, layout, uidata);
        }
    }

    pub fn move_selection_left(
        &mut self,
        data: &mut Vec<Column>,
        layout: &UILayout,
        uidata: &mut UIData,
    ) {
        if self.curser_column > 0 {
            self.curser_column = self.curser_column.saturating_sub(1);
        } else if self.offset_column > 0 {
            self.offset_column = self.offset_column.saturating_sub(1);
        }
        self.update(data, layout, uidata);
    }

    pub fn move_selection_right(
        &mut self,
        data: &mut Vec<Column>,
        layout: &UILayout,
        uidata: &mut UIData,
    ) {
        if self.curser_column + self.offset_column < (data.len() - 1) {
            // Somewhere before the last column
            if self.curser_column < (self.visible_columns.len() - 1) {
                // In the middle
                self.curser_column += 1;
            } else {
                // At the end of the screen
                self.offset_column += 1;
            }
            self.update(data, layout, uidata);
        } else {
            // At the last visible column (which could be wider then the screen)
            if self.visible_width > self.width && self.offset_column < (data.len() - 1) {
                self.offset_column += 1;
                self.update(data, layout, uidata);
            }
        }
    }

    pub fn select_cell(
        &mut self,
        row: usize,
        column: usize,
        data: &mut Vec<Column>,
        layout: &UILayout,
        uidata: &mut UIData,
    ) {
        // If relevant column is already visible, only select the right row, otherwise move the view.
        if self.visible_columns.contains(&column) {
            self.curser_column = self
                .visible_columns
                .iter()
                .position(|&c| c == column)
                .unwrap_or(0);
        } else {
            self.offset_column = column;
            self.curser_column = 0;
        }

        if row >= self.offset_row && row < self.offset_row + self.heigh {
            self.curser_row = row - self.offset_row;
        } else {
            self.curser_row = 0;
            self.offset_row = row;
        }

        self.update(data, layout, uidata);
    }

    pub fn search(
        &mut self,
        term: &str,
        current_column_only: bool,
        data: &mut Vec<Column>,
        layout: &UILayout,
        uidata: &mut UIData,
    ) -> usize {
        let start_time = Instant::now();

        let mask = Arc::clone(&self.rows);
        let search_term = term.to_string();

        let matching_rows: Vec<(usize, usize)> = if current_column_only {
            let col_idx = self.curser_column + self.offset_column;
            data[col_idx]
                .search(&search_term, &mask)
                .into_iter()
                .map(|row_idx| (row_idx, col_idx))
                .collect()
        } else {
            let columns = &data;
            columns
                .par_iter()
                .enumerate()
                .flat_map(|(col_idx, column)| {
                    column
                        .search(&search_term, &mask)
                        .into_iter()
                        .map(|row_idx| (row_idx, col_idx))
                        .collect::<Vec<_>>()
                })
                .collect()
        };

        let search_duration = start_time.elapsed().as_millis();

        if matching_rows.is_empty() {
            self.search_results.clear();
            return 0;
        } else {
            // Sort by rows
            self.search_results = matching_rows.into_iter().collect();
            self.search_results.sort_unstable();

            // Set the search index to the first match that is after the cursor
            let curser_ridx = self.offset_row + self.curser_row;
            self.search_idx = self
                .search_results
                .iter()
                .position(|&(row, _col)| row >= curser_ridx)
                .unwrap_or(0);

            trace!(
                "Search found {} matching rows in {}ms",
                self.search_results.len(),
                search_duration
            );

            self.search_next(0, data, layout, uidata);
            return self.search_results.len();
        }
    }

    pub fn search_next(
        &mut self,
        step: i32,
        data: &mut Vec<Column>,
        layout: &UILayout,
        uidata: &mut UIData,
    ) -> usize {
        // Note: step has to be -1, 0, 1
        let mut next_match: Option<(usize, usize)> = None;
        let mut next_match_idx = 0;
        let total_matches = self.search_results.len();
        if total_matches > 0 {
            if step >= 0 {
                let s = step as usize;
                if self.search_idx + s >= total_matches {
                    self.search_idx = 0;
                } else {
                    self.search_idx += s;
                }
            } else if self.search_idx as i32 + step < 0 {
                self.search_idx = self.search_results.len() - 1;
            } else {
                self.search_idx = (self.search_idx as i32 + step) as usize;
            }
            next_match = Some((
                self.search_results[self.search_idx].0,
                self.search_results[self.search_idx].1,
            ));
            next_match_idx = self.search_idx;
        }

        if let Some((row, column)) = next_match {
            self.select_cell(row, column, data, layout, uidata);
            trace!(
                "Selecting next search result {}/{}, pos {}:{}",
                next_match_idx + 1,
                total_matches,
                row,
                column
            );
        }
        return next_match_idx;
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

    fn get_collapsed_column(nrows: usize) -> ColumnView {
        let data = vec!["⋮".to_string(); nrows];
        ColumnView {
            name: "...".to_string(),
            width: 3,
            data,
        }
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

    fn calculate_column_width(column: &Column, max_column_width: usize) -> usize {
        let width = std::cmp::max(column.name.len(), column.max_width) + COLUMN_WIDTH_MARGIN;
        match column.status {
            ColumnStatus::COLLAPSED => COLUMN_WIDTH_COLLAPSED_COLUMN,
            ColumnStatus::NORMAL => std::cmp::min(width, max_column_width),
            ColumnStatus::EXPANDED => width,
        }
    }

    pub fn update(&mut self, data: &mut Vec<Column>, layout: &UILayout, uidata: &mut UIData) {
        self.width = layout.table_width;
        self.heigh = layout.table_height;

        let rbegin = self.offset_row;
        let rend = std::cmp::min(rbegin + self.heigh, self.rows.len());

        trace!(
            "Table: I:{}, Cr {}, Cc {}, Or {}, Oc {}, Rb {}, Re {}, tw: {}, th:{}, uiw: {}, uih: {}",
            self.show_index,
            self.curser_row,
            self.curser_column,
            self.offset_row,
            self.offset_column,
            rbegin,
            rend,
            self.width,
            self.heigh,
            layout.width,
            layout.height
        );

        self.visible_columns = Vec::new();
        let mut visible_width = 0;

        // Calculate current render with for each column
        // This could change because a column was expanded or collapsed
        for column in data.iter_mut() {
            column.render_width = Self::calculate_column_width(column, 25);
        }

        // Create a list of columns that fit in the table
        for (cidx, column) in data[self.offset_column..].iter_mut().enumerate() {
            if visible_width + (column.render_width + 1) <= layout.table_width {
                //if (column.render_width+1) <= width_budget {
                self.visible_columns.push(cidx + self.offset_column);
                //width_budget -= column.render_width + 1; // Rendered with and 1 spacer character
                visible_width += column.render_width + 1;
            } else {
                // Add the last partial visible column
                if visible_width < layout.table_width {
                    let remaining_width = layout.table_width - visible_width;
                    self.visible_columns.push(cidx + self.offset_column);
                    visible_width += remaining_width;
                    column.render_width = remaining_width;
                }
                break;
            }
        }
        // Store how wide the table would be in its full rendering to know the most right column is only partially rendered
        self.visible_width = visible_width;

        // Growing columns can reduce the number of visible columns. Make sure the column curser is at most the last visible column
        self.curser_column = std::cmp::min(self.curser_column, self.visible_columns.len() - 1);

        // Create ColumnViews for visible columns

        self.data.clear();
        self.data = Vec::with_capacity(self.visible_columns.len());
        for idx in self.visible_columns.iter() {
            if let Some(column) = data.get(*idx) {
                if column.status == ColumnStatus::COLLAPSED {
                    self.data.push(Self::get_collapsed_column(rend - rbegin));
                } else {
                    let col_data = self.rows[rbegin..rend]
                        .iter()
                        .map(|&ridx| column.data[ridx].clone())
                        .collect();
                    let name = Self::get_visible_name(column.name.clone(), column.render_width);
                    let width = column.render_width;
                    //trace!("Visible Column: \"{name}\", width: {width}");

                    self.data.push(ColumnView {
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
        uidata.layout = layout.clone();
        self.build_index();

        self.update_uidata(uidata);
    }

    pub fn update_uidata(&self, uidata: &mut UIData) {
        uidata.name = self.name.clone();
        uidata.table = self.data.clone();
        uidata.index = self.index.clone();
        uidata.selected_column = self.curser_column;
        uidata.selected_row = self.curser_row;
        uidata.nrows = self.rows.len();
        uidata.abs_selected_row = self.offset_row + self.curser_row;
        uidata.last_update = Instant::now();
    }
}
