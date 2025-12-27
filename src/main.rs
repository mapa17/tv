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
mod table;
mod ui;

use table::Table;
use ui::{TableConfig, TableUI};

fn main() -> io::Result<()> {
    println!("Starting tv!");

    let table = Table::load("test.csv".into());
    let cfg = TableConfig{
        event_poll_time: 100
    };
    let mut ui = TableUI::new(cfg);

    let mut terminal = ratatui::init();
    let app_result = ui.run(&table, &mut terminal);
    
    ratatui::restore();
    app_result
}


// #[cfg(test)]
// mod tests {

//     use super::*;
//     use ratatui::style::Style;

//     #[test]
//     fn handle_key_event() -> io::Result<()> {
//         let mut app = App::default();
//         app.handle_key_event(KeyCode::Right.into());
//         assert_eq!(app.counter, 1);

//         app.handle_key_event(KeyCode::Left.into());
//         assert_eq!(app.counter, 0);

//         let mut app = App::default();
//         app.handle_key_event(KeyCode::Char('q').into());
//         assert!(app.exit);

//         Ok(())
//     }
// }