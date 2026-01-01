use std::time::Duration;
use tracing::trace;

use ratatui::crossterm::event::{self, Event, KeyCode};
use crate::domain::{TVConfig, TVError, Message};
use crate::model::Model;

pub struct Controller {
    event_poll_time: usize
}

impl Controller {
    pub fn new(cfg: &TVConfig) -> Self {
        Self {
            event_poll_time: cfg.event_poll_time,
        }
    }

    pub fn handle_event(&self, _model: &Model) -> Result<Option<Message>, TVError> {
        if event::poll(Duration::from_millis(self.event_poll_time as u64))?
            && let Event::Key(key) = event::read()?
                && key.kind == event::KeyEventKind::Press {
                    return Ok(self.handle_key(key));
                }
        Ok(None)
    }

    fn handle_key(&self, key: event::KeyEvent) -> Option<Message> {
        let message = match key.code {
            KeyCode::Char('q') => Some(Message::Quit),
            KeyCode::Char('h') => Some(Message::MoveLeft),
            KeyCode::Char('j') => Some(Message::MoveDown),
            KeyCode::Char('k') => Some(Message::MoveUp),
            KeyCode::Char('l') => Some(Message::MoveRight),
            _ => None,
        };
        trace!("Mapped: {key:?} => {message:?}");
        message
    }

}