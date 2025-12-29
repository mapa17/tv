use std::process::ExitCode;

mod model;
mod ui;
mod domain;
mod controller;


use domain::{TableConfig, TVError};
use model::{Model, Status};
use ui::TableUI;
use controller::Controller;

fn main() -> ExitCode {
    match run() {
        Err(e) => {
            eprintln!("Error: {:?}", e);
            ExitCode::FAILURE
        }
        Ok(_) => {
            ratatui::restore();
            ExitCode::SUCCESS
        }
    }
}

fn run() -> Result<(), TVError> {
    println!("Starting tv!");

    let mut model = Model::load("tests/fixtures/testdata_01.csv".into())?; 
    
    let cfg = TableConfig{
        event_poll_time: 100
    };
    let mut ui = TableUI::new(&cfg);

    let mut controller = Controller::new(&cfg);

    let mut terminal = ratatui::init();

    while model.status != Status::EXITING {
        // Render the current view
        terminal.draw(|f| ui.draw(&model, f))?;
        
        // Handle events and map to a Message
        if let Some(message) = controller.handle_event(&model)? {
            model.update(message)?;
        };
    };

    Ok(())
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