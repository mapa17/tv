use std::time::Instant;

use tracing::trace;

use crate::{domain::CMDMode, inputter::InputResult};

use super::{ColumnView, Model};

use crate::tui::{CMDLINE_HEIGH, SCROLLBAR_WIDTH, TABLE_HEADER_HEIGHT};

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
