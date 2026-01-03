use std::process::ExitCode;
use std::path::PathBuf;

use tracing_subscriber::{self, EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_error::ErrorLayer;
use tracing::info;

mod model;
mod ui;
mod domain;
mod controller;


use domain::{TVConfig, TVError};
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

pub fn initialize_logging(_cfg: &TVConfig) -> Result<(), std::io::Error> {
    let log_path = PathBuf::from("./.tv.log");
    let log_file = std::fs::File::create(log_path)?;
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let file_subscriber = tracing_subscriber::fmt::layer()
    .with_file(true)
    .with_line_number(true)
    .with_writer(log_file)
    .with_target(false)
    .with_ansi(false)
    .with_filter(filter);

    tracing_subscriber::registry().with(file_subscriber).with(ErrorLayer::default()).init();
    Ok(())
}

fn run() -> Result<(), TVError> {
    let cfg = TVConfig{
        event_poll_time: 100,
        default_column_width: 10,
        column_margin: 1,
    };
 
    initialize_logging(&cfg)?;
    info!("Starting tv!");

     //let mut model = Model::load("tests/fixtures/testdata_01.csv".into())?; 
    let mut model = Model::from_file("tests/fixtures/testdata_02.csv".into(), &cfg)?; 
    
    let controller = Controller::new(&cfg);
    let mut terminal = ratatui::init();
    let mut ui = TableUI::new(&cfg, &terminal.get_frame());
   
    while model.status != Status::EXITING {
        let tsize = ui.get_table_size();

        // Handle events and map to a Message
        let message = controller.handle_event(&model)?; 
        model.update(message, tsize.0, tsize.1)?;

        // Render the current view
        terminal.draw(|f| ui.draw(&model, f))?;
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