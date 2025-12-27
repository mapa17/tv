use std::io;
use std::process::ExitCode;

mod table;
mod ui;
mod domain;


use table::Table;
use ui::{TableConfig, TableUI};
use domain::TVError;

fn main() -> ExitCode {
    println!("Starting tv!");

    let table = match Table::load("tests/fixtures/testdata_01.csv".into()) {
        Ok(frame) => frame,
        Err(e) => {
            eprintln!("Error: {:?}", e);
            return ExitCode::FAILURE;
        }
    };
    let cfg = TableConfig{
        event_poll_time: 100
    };
    let mut ui = TableUI::new(cfg);

    let mut terminal = ratatui::init();
    
    return match ui.run(&table, &mut terminal) {
        Ok(_) => {
            ratatui::restore();
            ExitCode::SUCCESS // Returns 0
        },
        Err(e) => {
            eprintln!("Error: {:?}", e);
            ExitCode::FAILURE  // Returns 1 
        }
    };
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