use std::{time::Duration, io};

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, poll};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
    DefaultTerminal, Frame,
};

use crate::table::Table;

#[derive(Debug)]
pub struct TableConfig {
    pub event_poll_time: u64,
}

#[derive(Debug)]
pub struct TableUI {
    config: TableConfig,
    exit: bool,
}

struct TableView<'a> {
    ui: &'a TableUI,
    table: &'a Table,
}

impl TableUI {
    pub fn new(config: TableConfig) -> Self {
        Self {
            config: config,
            exit: false
        }
    }

    pub fn run(&mut self, table: &Table, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| {
                frame.render_widget(TableView { ui: self, table }, frame.area());
            })?;
            self.handle_events()?;
        }
        Ok(())
    }


    fn handle_events(&mut self) -> io::Result<()> {
        if poll(Duration::from_millis(self.config.event_poll_time)).is_ok() {
            match event::read()? {
                // it's important to check that the event is a key press event as
                // crossterm also emits key release and repeat events on Windows.
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    self.handle_key_event(key_event)
                }
                _ => {}
            };
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            //KeyCode::Left => self.decrement_counter(),
            //KeyCode::Right => self.increment_counter(),
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

impl Widget for TableView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Access both self.ui and self.table here

        let title = Line::from(" Table viewer ".bold());
        let instructions = Line::from(vec![
            " Decrement ".into(),
            "<Left>".blue().bold(),
            " Increment ".into(),
            "<Right>".blue().bold(),
            " Quit ".into(),
            "<Q> ".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let path = self.table.get_path();
        let loc = path.to_string_lossy().yellow();
        let counter_text = Text::from(vec![Line::from(vec![
            "location: ".into(),
            loc,
        ])]);

        Paragraph::new(counter_text)
            .centered()
            .block(block)
            .render(area, buf);
    }
}