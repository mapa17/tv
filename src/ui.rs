use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style, palette::tailwind};
use ratatui::widgets::{Block, Borders, Row, ScrollbarState, Table, TableState, Scrollbar, ScrollbarOrientation};
use ratatui::{Frame, layout::Rect};
use tracing::{debug, info, trace};

use crate::domain::TableConfig;
use crate::model::Model;

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
            selected_cell_fg: color.c600,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c900,
            footer_border_color: color.c400,
        }
    }
}
struct UIStyles {
    row: Style,
    selected_row: Style,
}
impl UIStyles {
    const fn new(colors: &UIColors) -> Self {
        Self {
            row: Style::new().fg(colors.row_fg).bg(colors.row_bg),
            selected_row: Style::new()
                .fg(colors.selected_row_fg)
                .bg(colors.selected_row_bg),
        }
    }
}

struct HeaderElement {
    idx: u16,
    name: String,
    min_width: usize,
    width: usize,
    max_width: usize,
}

pub struct TableUI {
    colors: UIColors,
    styles: UIStyles,
    table_state: TableState,
    scrollbar_state: ScrollbarState,
}

const PALETTES: [tailwind::Palette; 4] = [
    tailwind::BLUE,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::RED,
];

impl TableUI {
    pub fn new(_config: &TableConfig) -> Self {
        let colors = UIColors::new(&PALETTES[0]);
        let styles = UIStyles::new(&colors);
        Self {
            colors: colors,
            styles: styles,
            table_state: TableState::default().with_selected(0),
            scrollbar_state: ScrollbarState::new(1).position(0),
        }
    }

    pub fn draw(&mut self, model: &Model, frame: &mut Frame) {
        //trace!("Drawing ui ...");
        let vertical = &Layout::vertical([Constraint::Min(5), Constraint::Length(4)]);
        let rects = vertical.split(frame.area());

        self.render_table(model, frame, rects[0]);

        self.render_cmdline(model, frame, rects[1]);
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


        let h = area.height;
        let w = area.width;
        info!("Table size: w:{w} h:{h}");
        let rows = [
            Row::new(vec!["Cell00", "Cell01", "Cell02"]),
            Row::new(vec!["Cell10", "Cell11", "Cell12"]),
            Row::new(vec![
                "Cell20",
                "Cell21----------------------------",
                "Cell22",
            ]),
        ];
        let widths = [
            Constraint::Length(20),
            Constraint::Length(5),
            Constraint::Length(5),
        ];
        let table = Table::new(rows, widths)
            .block(Block::new().title("Table"))
            .row_highlight_style(self.styles.selected_row)
            .highlight_symbol(">>");
        frame.render_stateful_widget(table, area, &mut self.table_state);

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        self.scrollbar_state = self.scrollbar_state.content_length(100).position(10); //.viewport_content_length(1);
        frame.render_stateful_widget(scrollbar, area, &mut self.scrollbar_state);
    }

    fn render_cmdline(&mut self, model: &Model, frame: &mut Frame, area: Rect) {
        let b = Block::default().title("Cmd").borders(Borders::ALL);
        frame.render_widget(b, area);
    }
}
