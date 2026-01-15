
use ratatui::layout::{Constraint, Layout, Margin};
use ratatui::style::{Color, Style, palette::tailwind};
use ratatui::widgets::{Block, Borders, Row, ScrollbarState, Table, TableState, Scrollbar, ScrollbarOrientation, Cell};
use ratatui::{Frame, layout::Rect};
use tracing::{warn, trace};
use std::time::Instant;

use crate::domain::TVConfig;
use crate::model::{UIData, UILayout};

pub const INDEX_COLUMN_BORDER: usize = 2;
pub const SCROLLBAR_WIDTH: usize = 1;
pub const TABLE_HEADER_HEIGHT: usize = 1;
pub const RECORD_HEADER_HEIGHT: usize = 1;

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
    selected_cell: Style,
}
impl UIStyles {
    const fn new(colors: &UIColors) -> Self {
        Self {
            row: Style::new().fg(colors.row_fg).bg(colors.row_bg),
            selected_row: Style::new()
                .fg(colors.selected_row_fg)
                .bg(colors.selected_row_bg),
            header: Style::new().fg(colors.header_fg).bg(colors.header_bg).bold().underlined(),
            selected_cell: Style::new().fg(colors.selected_cell_fg).bg(colors.selected_cell_bg).bold().underlined(),
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
    cmd: Rect,
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
        let vertical = &Layout::vertical([Constraint::Length((s.table_height + TABLE_HEADER_HEIGHT) as u16), Constraint::Length(s.cmdline_height as u16)]);
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
            cmd: vsplit[1],
            index: hsplit[0],
        }
    }

    pub fn draw(&mut self, data: &UIData, frame: &mut Frame) {
        trace!{"Drawing ..."};
        let layout = Self::create_layout(frame, &data.layout);
        
        self.render_table(data, frame, layout.table);

        self.render_index(data, frame, layout.index);

        self.render_cmdline(data, frame, layout.cmd);

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
        let widths = columns.iter().map(|c| Constraint::Length(c.width as u16)).collect::<Vec<Constraint>>();

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

    fn render_cmdline(&mut self, _data: &UIData, frame: &mut Frame, area: Rect) {
        let b = Block::default().title("Cmd").borders(Borders::ALL);
        frame.render_widget(b, area);
    }
}
