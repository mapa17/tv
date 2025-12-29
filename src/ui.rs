
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, palette::tailwind};
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Table, Block, Borders, Row, TableState},
};
use tracing::{info, Level, debug, trace};

use crate::domain::TableConfig;
use crate::model::Model;

struct UIColors {
    buffer_bg: Color,
    header_bg: Color,
    header_fg: Color,
    row_fg: Color,
    selected_row_style_fg: Color,
    selected_column_style_fg: Color,
    selected_cell_style_fg: Color,
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
            selected_row_style_fg: color.c400,
            selected_column_style_fg: color.c400,
            selected_cell_style_fg: color.c600,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c900,
            footer_border_color: color.c400,
        }
    }
}

pub struct TableUI {
    colors: UIColors,
    table_state: TableState,
}

const PALETTES: [tailwind::Palette; 4] = [
    tailwind::BLUE,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::RED,
];

impl TableUI {
    pub fn new(_config: &TableConfig) -> Self {
        Self {
            colors: UIColors::new(&PALETTES[0]),
            table_state: TableState::default().with_selected(0),
        }
    }

    pub fn draw(&mut self, _table: &Model, frame: &mut Frame) {
        trace!("Drawing ui ...");
        let vertical = &Layout::vertical([Constraint::Min(5), Constraint::Length(4)]);
        let rects = vertical.split(frame.area());

        self.render_table(frame, rects[0]);

        self.render_cmdline(frame, rects[1]);
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
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
        let rows = [
            Row::new(vec!["Cell1", "Cell2"]),
            Row::new(vec!["Cell3", "Cell4"]),
        ];
        let widths = [Constraint::Length(5), Constraint::Length(5)];
        let table = Table::new(rows, widths);
        frame.render_stateful_widget(table, area, &mut self.table_state);
    }

    fn render_cmdline(&mut self, frame: &mut Frame, area: Rect) {
        let b = Block::default()
            .title("Cmd")
            .borders(Borders::ALL);
        frame.render_widget(b, area);
    }
}
