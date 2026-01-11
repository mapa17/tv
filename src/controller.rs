use std::time::Duration;
use tracing::trace;

use ratatui::crossterm::event::{self, Event, KeyCode, KeyModifiers};
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
        if event::poll(Duration::from_millis(self.event_poll_time as u64))? {
            match event::read()? {
                // Detect frame resize event
                event::Event::Resize(width, height) => {
                    trace!("Resized to {}x{}", width, height);
                    return Ok(Some(Message::Resize(width as usize, height as usize))); 
                }
                event::Event::Key(key) if key.kind == event::KeyEventKind::Press => {
                    return Ok(self.handle_key(key));
                }
                _ => {}
            }
        }
        Ok(None)
    }

    fn handle_key(&self, key: event::KeyEvent) -> Option<Message> {
        let message = match (key.code, key.modifiers) {
            (KeyCode::Char('q'), KeyModifiers::NONE) => Some(Message::Quit),
            (KeyCode::Char('h'), KeyModifiers::NONE) => Some(Message::MoveLeft),
            (KeyCode::Char('j'), KeyModifiers::NONE) => Some(Message::MoveDown),
            (KeyCode::Char('J'), KeyModifiers::SHIFT) => Some(Message::MovePageDown),
            (KeyCode::Char('k'), KeyModifiers::NONE) => Some(Message::MoveUp),
            (KeyCode::Char('K'), KeyModifiers::SHIFT) => Some(Message::MovePageUp),
            (KeyCode::Char('l'), KeyModifiers::NONE) => Some(Message::MoveRight),
            (KeyCode::Char('G'), KeyModifiers::SHIFT) => Some(Message::MoveEnd),
            (KeyCode::Char('g'), KeyModifiers::NONE) => Some(Message::MoveBeginning),
            (KeyCode::Char('-'), KeyModifiers::NONE) => Some(Message::ShrinkColumn),
            (KeyCode::Char('+'), KeyModifiers::NONE) => Some(Message::GrowColumn),
            (KeyCode::Char('n'), KeyModifiers::NONE) => Some(Message::ToggleIndex),
            _ => None,
        };
        trace!("Mapped: {key:?} => {message:?}");
        message
    }

}