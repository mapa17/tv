
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style, palette::tailwind};
use ratatui::widgets::{Block, Borders, Row, ScrollbarState, Table, TableState, Scrollbar, ScrollbarOrientation, Cell};
use ratatui::{Frame, layout::Rect};
use tracing::warn;
use std::time::Instant;

use crate::domain::TVConfig;
use crate::model::{ColumnView, Model};

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

struct HeaderElement {
    idx: u16,
    name: String,
    width: usize,
    max_width: usize,
    collapsed: bool,
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
    index: Option<Rect>,
}

const PALETTES: [tailwind::Palette; 4] = [
    tailwind::BLUE,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::RED,
];

impl TableUI {
    pub fn new(_config: &TVConfig, frame: &Frame) -> Self {
        let colors = UIColors::new(&PALETTES[0]);
        let styles = UIStyles::new(&colors);

        let mut ui = Self {
            colors,
            styles,
            table_state: TableState::default(),
            scrollbar_state: ScrollbarState::new(1).position(0),
            table_width: 0,
            table_heigh: 0,
            last_render: Instant::now() - std::time::Duration::from_secs(1),
        };

        // Update table width as this is used by TVModel to calculate column widths
        let layout = Self::create_layout(frame, false, 0);
        ui.table_width = layout.table.width as usize;
        ui.table_heigh = layout.table.height as usize;
        ui
    }

    fn create_layout(frame: &Frame, show_index: bool, index_width: usize) -> TableUILayout {
        let vertical = &Layout::vertical([Constraint::Percentage(100), Constraint::Length(4)]);
        let vsplit = vertical.split(frame.area());
        if show_index {
            let horizontal = &Layout::horizontal([Constraint::Length(index_width as u16 + 2), Constraint::Percentage(100)]);
            let hsplit = horizontal.split(vsplit[0]);
            TableUILayout {
                table: hsplit[1],
                cmd: vsplit[1],
                index: Some(hsplit[0]),
            }
 
        } else {
            TableUILayout {
                table: vsplit[0],
                cmd: vsplit[1],
                index: None,
            }
        }
    }

    pub fn draw(&mut self, model: &Model, frame: &mut Frame) {

        //trace!("Drawing ui ...");
        if let Some(index_column) = model.get_index_column() {
            let layout = Self::create_layout(frame, true, index_column.width);
            self.render_table(model, frame, layout.table);
            self.render_index(index_column, frame, layout.index.unwrap());
            self.render_cmdline(model, frame, layout.cmd);
       } else {
            let layout = Self::create_layout(frame, false, 0);
            self.render_table(model, frame, layout.table);
            self.render_cmdline(model, frame, layout.cmd);
       }

       self.last_render = Instant::now();
    }

    pub fn needs_redrawing(&self, model: &Model) -> bool {
        model.get_last_render_update() - self.last_render > std::time::Duration::ZERO
    }

    pub fn get_table_size(&self) -> (usize, usize) {
        (self.table_width-2, self.table_heigh-1) // Subtract Column border and header size
    }
    
    fn render_index(&mut self, column: ColumnView, frame: &mut Frame, area: Rect) {
        let rows = column.data.into_iter().map(|row| Row::new(vec![row]).style(self.styles.row)).collect::<Vec<Row>>();
        let width = vec![Constraint::Length(column.width as u16)];

        let header = Row::new(vec![column.name.clone()])
            .style(self.styles.header);

        let table = Table::new(rows, width).header(header)
            .block(Block::default().borders(Borders::RIGHT).style(self.styles.row));
        frame.render_widget(table, area);
    }

    fn render_table(&mut self, model: &Model, frame: &mut Frame, area: Rect) {
        // let header_style = Style::default()
        //     .fg(self.colors.header_fg)
        //     .bg(self.colors.header_bg);
        // let selected_row_style = Style::default()
        //     .add_modifier(Modifier::REVERSED)
        //     .fg(self.colors.selected_row_style_fg);
        // let selected_col_style = Style::default().fg(self.colors.selected_column_style_fg);
        // let selected_cell_style = Style::default()
        //     .add_modifier(Modifier::REVERSED)
        //     .fg(self.colors.selected_cell_style_fg);

        // let header = ["Name", "Address", "Email"]
        //     .into_iter()
        //     .map(Cell::from)
        //     .collect::<Row>()
        //     .style(header_style)
        //     .height(1);
        // let items = vec![vec!["E00", "E01", "E02"], vec!["E10", "E01", "E02"]];
        // let rows = items.iter().enumerate().map(|(i, data)| {
        //     let color = match i % 2 {
        //         0 => self.colors.normal_row_color,
        //         _ => self.colors.alt_row_color,
        //     };

        //     data.into_iter()
        //         .map(|content| Cell::from(Text::from(format!("\n{content}\n"))))
        //         .collect::<Row>()
        //         .style(Style::new().fg(self.colors.row_fg).bg(color))
        //         .height(4)
        // });
        // let bar = " █ ";

        // let t = Table::new(
        //     rows,
        //     [
        //         // + 1 is for padding.
        //         Constraint::Length(self.longest_item_lens.0 + 1),
        //         Constraint::Min(self.longest_item_lens.1 + 1),
        //         Constraint::Min(self.longest_item_lens.2),
        //     ],
        // )
        // .header(header)
        // .row_highlight_style(selected_row_style)
        // .column_highlight_style(selected_col_style)
        // .cell_highlight_style(selected_cell_style)
        // .highlight_symbol(Text::from(vec![
        //     "".into(),
        //     bar.into(),
        //     bar.into(),
        //     "".into(),
        // ]))
        // .bg(self.colors.buffer_bg)
        // .highlight_spacing(HighlightSpacing::Always);

        //let headers = model.get_headers().collect();

       // Update table width as this is used by TVModel to calculate column widths
        self.table_width = area.width as usize;
        self.table_heigh = area.height as usize;

        let _h = area.height;
        let _w = area.width;
        //trace!("Table size: w:{w} h:{h}");
        let columns = model.get_visible_columns(); 
        if columns.is_empty() {
            warn!("No visible columns!");
            return
        }
        let mut rows = Vec::new();
        let nrows = columns[0].data.len();
        for ridx in 0..nrows {
            rows.push(Row::new(columns.iter().map(|c| c.data[ridx].clone()).collect::<Vec<String>>()).style(self.styles.row));
        }
        let widths = columns.iter().map(|c| Constraint::Length(c.width as u16)).collect::<Vec<Constraint>>();

        let header = Row::new(columns.iter().map(|c| Cell::from(c.name.clone())).collect::<Vec<Cell>>())
            .style(self.styles.header);
        
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        self.scrollbar_state = self.scrollbar_state.content_length(model.nrows()).position(model.row_absolute()); //.viewport_content_length(1);
        frame.render_stateful_widget(scrollbar, area, &mut self.scrollbar_state);

        let table = Table::new(rows, widths)
            //.block(Block::new().title("Table"))
            .row_highlight_style(self.styles.selected_row)
            .cell_highlight_style(self.styles.selected_cell)
            .header(header);
            //.highlight_symbol(">>");
        //self.table_state.select_column(Some(model.get_selected_column()));
        //self.table_state.select(Some(model.get_selected_row()));
        self.table_state.select_cell(Some((model.selected_row(), model.selected_column())));
        frame.render_stateful_widget(table, area, &mut self.table_state);

        }

    fn render_cmdline(&mut self, _model: &Model, frame: &mut Frame, area: Rect) {
        let b = Block::default().title("Cmd").borders(Borders::ALL);
        frame.render_widget(b, area);
    }
}
