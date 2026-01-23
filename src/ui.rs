
use ratatui::layout::{Constraint, Layout, Margin, Position};
use ratatui::style::{Color, Style, palette::tailwind};
use ratatui::widgets::{Block, Borders, Row, ScrollbarState, Table, TableState, Scrollbar, ScrollbarOrientation, Cell, Paragraph};
use ratatui::{Frame, layout::Rect};
use tracing::{warn, trace};
use std::time::Instant;

use crate::domain::TVConfig;
use crate::model::{UIData, UILayout};
use crate::popup::Popup;

pub const INDEX_COLUMN_BORDER: usize = 2;
pub const SCROLLBAR_WIDTH: usize = 1;
pub const TABLE_HEADER_HEIGHT: usize = 1;
//pub const RECORD_HEADER_HEIGHT: usize = 1;
pub const CMDLINE_HEIGH: usize = 1;
pub const POPUP_HORIZONTAL_MARGIN: usize = 3;
pub const POPUP_VERTICAL_MARGIN: usize = 3;
pub const MAX_POPUP_CONTENT_WIDTH:usize = 60;
pub const STATUS_MESSAGE_DISPLAY_DURATION:std::time::Duration = std::time::Duration::new(2, 0);


#[derive(Clone)]
struct UIColors {
    buffer_bg: Color,
    header_bg: Color,
    header_fg: Color,
    row_fg: Color,
    row_bg: Color,
    selected_row_fg: Color,
    selected_row_bg: Color,
    selected_column_fg: Color,
    selected_cell_fg: Color,
    selected_cell_bg: Color,
    normal_row_color: Color,
    alt_row_color: Color,
    footer_border_color: Color,
}

impl UIColors {
    const fn new(color: &tailwind::Palette) -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            header_bg: color.c900,
            header_fg: tailwind::SLATE.c200,
            row_fg: tailwind::SLATE.c200,
            row_bg: tailwind::SLATE.c950,
            selected_row_fg: tailwind::YELLOW.c100,
            selected_row_bg: tailwind::YELLOW.c950,
            selected_column_fg: color.c400,
            selected_cell_fg: tailwind::BLUE.c600,
            selected_cell_bg: tailwind::BLUE.c50,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c900,
            footer_border_color: color.c400,
        }
    }
}
struct UIStyles {
    row: Style,
    selected_row: Style,
    header: Style,
    statusline: Style,
    selected_cell: Style,
    popup: Style,
}
impl UIStyles {
    const fn new(colors: &UIColors) -> Self {
        Self {
            row: Style::new().fg(colors.row_fg).bg(colors.row_bg),
            selected_row: Style::new()
                .fg(colors.selected_row_fg)
                .bg(colors.selected_row_bg),
            header: Style::new().fg(colors.header_fg).bg(colors.header_bg).bold().underlined(),
            statusline: Style::new().fg(colors.header_fg).bg(colors.header_bg),
            selected_cell: Style::new().fg(colors.selected_cell_fg).bg(colors.selected_cell_bg).bold().underlined(),
            popup: Style::new().fg(colors.header_fg).bg(colors.header_bg),
        }
    }
}


pub struct TableUI {
    colors: UIColors,
    styles: UIStyles,
    table_state: TableState,
    scrollbar_state: ScrollbarState,
    table_width: usize,
    table_heigh: usize,
    last_render: Instant,
    //headers: Vec<HeaderElement>,
    //visible_headers: Vec<usize>,
}

struct TableUILayout {
    table: Rect,
    statusline: Rect,
    index: Rect,
}

const PALETTES: [tailwind::Palette; 4] = [
    tailwind::BLUE,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::RED,
];


impl TableUI {
    pub fn new(_config: &TVConfig) -> Self {
        let colors = UIColors::new(&PALETTES[0]);
        let styles = UIStyles::new(&colors);

        Self {
            colors,
            styles,
            table_state: TableState::default(),
            scrollbar_state: ScrollbarState::new(1).position(0),
            table_width: 0,
            table_heigh: 0,
            last_render: Instant::now() - std::time::Duration::from_secs(1),
        }
    }

    fn create_layout(frame: &Frame, s: &UILayout) -> TableUILayout {
        let vertical = &Layout::vertical([Constraint::Length((s.table_height + TABLE_HEADER_HEIGHT) as u16), Constraint::Length(s.statusline_height as u16)]);
        let vsplit = vertical.split(frame.area());

        let mut index_width = s.index_width;
        if index_width > 0 {
            index_width += INDEX_COLUMN_BORDER;
        }
        let horizontal = &Layout::horizontal([Constraint::Length(index_width as u16), Constraint::Length((s.table_width + SCROLLBAR_WIDTH) as u16)]);
        let hsplit = horizontal.split(vsplit[0]);
        //trace!("Splitting table for index from total={}, index={}, table={}", vsplit[0].width, hsplit[0].width, hsplit[1].width);
        
        TableUILayout {
            table: hsplit[1],
            statusline: vsplit[1],
            index: hsplit[0],
        }
    }

    pub fn draw(&mut self, data: &UIData, frame: &mut Frame) {
        let layout = Self::create_layout(frame, &data.layout);

        self.render_table(data, frame, layout.table);
        self.render_index(data, frame, layout.index);
        self.render_statusline(data, frame, layout.statusline);

        if data.show_popup {
            self.render_popup(data, frame, layout.table);
        }          
        self.last_render = Instant::now();
    }

    pub fn needs_redrawing(&self, data: &UIData) -> bool {
        data.last_update - self.last_render > std::time::Duration::ZERO
    }

    // pub fn get_table_size(&self, frame: &Frame, model: &Model) -> (usize, usize) {

    //     let mut layout = Self::create_layout(frame, false, 0);
    //     if let Some(index_column) = model.get_index_column() {
    //         layout = Self::create_layout(frame, true, index_column.width);
    //     }
    //     ((layout.table.width as usize).saturating_sub(SCROLLBAR_WIDTH), (layout.table.height as usize).saturating_sub(HEADER_HEIGHT)) // Subtract Column border and header size
    
    // }
    
    fn render_index(&mut self, data: &UIData, frame: &mut Frame, area: Rect) {
        if area.width > 0 {
            let column = &data.index;
            let rows = column.data.clone().into_iter().map(|row| Row::new(vec![row]).style(self.styles.row)).collect::<Vec<Row>>();
            let width = vec![Constraint::Length(column.width as u16)];

            let header = Row::new(vec![column.name.clone()])
                .style(self.styles.header);

            let table = Table::new(rows, width).header(header)
                .block(Block::default().borders(Borders::RIGHT).style(self.styles.row));
            frame.render_widget(table, area);
        }
    }

    fn render_popup(&mut self, data: &UIData, frame: &mut Frame, area: Rect) {

        let popup = Popup::default()
            .content(data.popup_message.clone())
            .style(self.styles.popup)
            .title("Key Bindings")
            .title_style(Style::new().white().bold())
            .border_style(Style::new().white().bold());
        let popup_vertical_margin = (area.height.saturating_sub(data.popup_message.matches("\n").count() as u16 + (POPUP_VERTICAL_MARGIN*2) as u16)) / 2;
        let popup_margin = area.width.saturating_sub(MAX_POPUP_CONTENT_WIDTH as u16) / 2;
        frame.render_widget(popup, area.inner(Margin {vertical: popup_vertical_margin as u16, horizontal: popup_margin as u16,}));
    }
    fn render_table(&mut self, data: &UIData, frame: &mut Frame, area: Rect) {
        let columns = &data.table;
        if columns.is_empty() {
            warn!("No visible columns!");
            return
        }
        let mut rows = Vec::new();
        let nrows = data.table[0].data.len(); // Assume there is always at least one column
        for ridx in 0..nrows {
            rows.push(Row::new(columns.iter().map(|c| c.data[ridx].clone()).collect::<Vec<String>>()).style(self.styles.row));
        }
        // Fill up the rest of the table with empty strings to have the empty part of the table render with the same style.
        for _ in nrows..data.layout.table_height {
            rows.push(Row::new([""].repeat(columns.len())).style(self.styles.row));
        }
        let widths = columns.iter().map(|c| Constraint::Length(c.width as u16)).collect::<Vec<Constraint>>();

        //trace!("num rows: {}, nrows {}", rows.len(), nrows);
        let header = Row::new(columns.iter().map(|c| Cell::from(c.name.clone())).collect::<Vec<Cell>>())
            .style(self.styles.header);
        
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        self.scrollbar_state = self.scrollbar_state.content_length(data.nrows).position(data.abs_selected_row); //.viewport_content_length(1);

        let table = Table::new(rows, widths)
            //.block(Block::new().title("Table"))
            .row_highlight_style(self.styles.selected_row)
            .cell_highlight_style(self.styles.selected_cell)
            .header(header);
            //.highlight_symbol(">>");
        //self.table_state.select_column(Some(model.get_selected_column()));
        //self.table_state.select(Some(model.get_selected_row()));
        self.table_state.select_cell(Some((data.selected_row, data.selected_column)));
        frame.render_stateful_widget(table, area, &mut self.table_state);

        // using an inner vertical margin of 1 unit makes the scrollbar inside the block
        frame.render_stateful_widget(scrollbar, area.inner(Margin {vertical: 1, horizontal: 0,}), &mut self.scrollbar_state);
    }

    fn render_statusline(&mut self, data: &UIData, frame: &mut Frame, area: Rect) {
        let mut render_curser = false; 
        let right = format!("{}/{}", data.abs_selected_row + 1, data.nrows);
        let left = if data.active_cmdinput {
            render_curser = true;
            format!(">{}", data.cmdinput.input)
        } else if (data.last_status_message_update + STATUS_MESSAGE_DISPLAY_DURATION) - Instant::now() > std::time::Duration::ZERO {
            data.status_message.clone()
        } else {
            data.name.clone()
        };
        
        // Use chars().count() instead of .len() to handle multi-byte characters correctly in TUI
        let total_width = area.width as usize;
        let right_len = right.chars().count();
        
        // This format string says: Left-align 'left' in a space of (total_width - right_len)
        let status_string = format!(
            "{:<width$}{}", 
            left, 
            right, 
            width = total_width.saturating_sub(right_len)
        );

        let status_bar = Paragraph::new(status_string)
            .style(self.styles.statusline);
            
        frame.render_widget(status_bar, area);

        if render_curser {
            let curser_pos = Position::new(area.x + data.cmdinput.curser_pos as u16 + 1, area.y);
            frame.set_cursor_position(curser_pos);
        }
    }
}
