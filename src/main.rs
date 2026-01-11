use std::process::ExitCode;
use std::path::PathBuf;

use tracing_subscriber::{self, EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_error::ErrorLayer;
use tracing::info;
use clap::{Parser};

mod model;
mod ui;
mod domain;
mod controller;


use domain::{TVConfig, TVError, Message};
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

pub fn initialize_logging(_cfg: &TVConfig, args: &TVArguments) -> Result<(), std::io::Error> {
    let log_path = args.log.clone();
    let log_file = std::fs::File::create(log_path)?;
    let log_level = match args.verbose {
        0 => "warning", 
        1 => "info", 
        2 => "debug", 
        3 => "trace", 
        _ => "trace",
    };
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level));

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


#[derive(Parser)]
#[command(name = "TV")]
#[command(version = "0.1")]
#[command(about = "TUI Table viewer", long_about = None)]
struct Cli {
    /// Location of file to open
    file: PathBuf,

    /// Sets location of log file
    #[arg(short, long, value_name = "LOG", default_value="./.tv.log")]
    log: PathBuf,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    //#[command(subcommand)]
    //command: Option<Commands>,
}

struct TVArguments {
    filepath: PathBuf,
    log: PathBuf,
    verbose: u8,
}

fn arg_parser() -> TVArguments {
    let cli = Cli::parse();

    TVArguments { filepath: cli.file, log: cli.log, verbose: cli.verbose }
}

fn run() -> Result<(), TVError> {
    let cfg = TVConfig{
        event_poll_time: 100,
        default_column_width: 10,
        column_margin: 1,
    };

    let args = arg_parser(); 
    initialize_logging(&cfg, &args)?;
    info!("Starting tv!");

     //let mut model = Model::load("tests/fixtures/testdata_01.csv".into())?; 
    let mut model = Model::from_file(args.filepath, &cfg)?; 
    
    let controller = Controller::new(&cfg);
    let mut terminal = ratatui::init();
    let mut ui = TableUI::new(&cfg);

    // Start by telling the model about the actual ui size
    let area = terminal.get_frame().area();
    let mut message= Some(Message::Resize(area.width as usize, area.height as usize)); 

    while model.status != Status::EXITING {
        // Handle events and map to a Message
        model.update(message)?;

        let uidata = model.get_uidata();
        if ui.needs_redrawing(uidata) {
            // Render the current view
            terminal.draw(|f| ui.draw(uidata, f))?;
        }

        message = controller.handle_event(&model)?; 
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