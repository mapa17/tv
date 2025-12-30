use std::time::Duration;
use tracing::trace;

use ratatui::crossterm::event::{self, Event, KeyCode};
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

    pub fn handle_event(&self, _model: &Model) -> Result<Option<Message>, TVError> {
        if event::poll(Duration::from_millis(self.event_poll_time))?
            && let Event::Key(key) = event::read()?
                && key.kind == event::KeyEventKind::Press {
                    return Ok(self.handle_key(key));
                }
        Ok(None)
    }

    fn handle_key(&self, key: event::KeyEvent) -> Option<Message> {
        let message = match key.code {
            KeyCode::Char('q') => Some(Message::Quit),
            _ => None,
        };
        trace!("Mapped: {key:?} => {message:?}");
        message
    }

}