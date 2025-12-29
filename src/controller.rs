use std::{time::Duration, io};

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, poll};
use crate::domain::{TableConfig, TVError, Message};
use crate::model::Model;

pub struct Controller {
    event_poll_time: u64
}

impl Controller {
    pub fn new(cfg: &TableConfig) -> Self {
        Self {
            event_poll_time: cfg.event_poll_time,
        }
    }

    pub fn handle_event(&self, model: &Model) -> Result<Option<Message>, TVError> {
        if event::poll(Duration::from_millis(self.event_poll_time))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    return Ok(self.handle_key(key));
                }
            }
        }
        Ok(None)
    }

    fn handle_key(&self, key: event::KeyEvent) -> Option<Message> {
        match key.code {
            // KeyCode::Char('j') => Some(Message::Increment),
            // KeyCode::Char('k') => Some(Message::Decrement),
            KeyCode::Char('q') => Some(Message::Quit),
            _ => None,
        }
    }

    fn update(&self, model: &mut Model, msg: Message) -> Option<Message> {
        match msg {
            // Message::Increment => {
            //     model.counter += 1;
            //     if model.counter > 50 {
            //         return Some(Message::Reset);
            //     }
            // }
            // Message::Decrement => {
            //     model.counter -= 1;
            //     if model.counter < -50 {
            //         return Some(Message::Reset);
            //     }
            // }
            // Message::Reset => model.counter = 0,
            Message::Quit => {
                // You can handle cleanup and exit here
                model.exit();
            }
        };
        None
    }
}