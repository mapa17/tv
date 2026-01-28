use std::time::Duration;
use tracing::trace;

use ratatui::crossterm::event::{self, KeyCode, KeyModifiers};
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

    pub fn handle_event(&self, model: &Model) -> Result<Option<Message>, TVError> {
        if event::poll(Duration::from_millis(self.event_poll_time as u64))? {
            match event::read()? {
                // Detect frame resize event
                event::Event::Resize(width, height) => {
                    trace!("Resized to {}x{}", width, height);
                    return Ok(Some(Message::Resize(width as usize, height as usize))); 
                }
                event::Event::Key(key) if key.kind == event::KeyEventKind::Press => {
                    if model.raw_keyevents() {
                        return Ok(Some(Message::RawKey(key)));
                    }
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
            (KeyCode::Left, KeyModifiers::NONE) => Some(Message::MoveLeft),
            (KeyCode::Char('j'), KeyModifiers::NONE) => Some(Message::MoveDown),
            (KeyCode::Down, KeyModifiers::NONE) => Some(Message::MoveDown),
            (KeyCode::Char('J'), KeyModifiers::SHIFT) => Some(Message::MovePageDown),
            (KeyCode::Down, KeyModifiers::SHIFT) => Some(Message::MovePageDown),
            (KeyCode::Char('k'), KeyModifiers::NONE) => Some(Message::MoveUp),
            (KeyCode::Up, KeyModifiers::NONE) => Some(Message::MoveUp),
            (KeyCode::Char('K'), KeyModifiers::SHIFT) => Some(Message::MovePageUp),
            (KeyCode::Up, KeyModifiers::SHIFT) => Some(Message::MovePageUp),
            (KeyCode::Char('l'), KeyModifiers::NONE) => Some(Message::MoveRight),
            (KeyCode::Right, KeyModifiers::NONE) => Some(Message::MoveRight),
            (KeyCode::Char('G'), KeyModifiers::SHIFT) => Some(Message::MoveEnd),
            (KeyCode::End, KeyModifiers::CONTROL) => Some(Message::MoveEnd),
            (KeyCode::Down, KeyModifiers::CONTROL) => Some(Message::MoveEnd),
            (KeyCode::Char('g'), KeyModifiers::NONE) => Some(Message::MoveBeginning),
            (KeyCode::Home, KeyModifiers::CONTROL) => Some(Message::MoveBeginning),
            (KeyCode::Up, KeyModifiers::CONTROL) => Some(Message::MoveBeginning),
            (KeyCode::Char('v'), KeyModifiers::NONE) => Some(Message::ToggleIndex),
            (KeyCode::Tab, KeyModifiers::NONE) => Some(Message::ToggleColumnState),
            (KeyCode::Char('y'), KeyModifiers::NONE) => Some(Message::CopyCell),
            (KeyCode::Char('Y'), KeyModifiers::SHIFT) => Some(Message::CopyRow),
            (KeyCode::Char('?'), KeyModifiers::NONE) => Some(Message::Help),
            (KeyCode::Char(':'), KeyModifiers::NONE) => Some(Message::EnterCommand),
            (KeyCode::Char('/'), KeyModifiers::NONE) => Some(Message::Find),
            (KeyCode::Char('|'), KeyModifiers::NONE) => Some(Message::Filter),
            (KeyCode::Char('F'), KeyModifiers::SHIFT) => Some(Message::Histogram),
            (KeyCode::Char('n'), KeyModifiers::NONE) => Some(Message::SearchNext),
            (KeyCode::Char('p'), KeyModifiers::NONE) => Some(Message::SearchPrev),
            (KeyCode::Char('0'), KeyModifiers::NONE) => Some(Message::MoveToFirstColumn),
            (KeyCode::Left, KeyModifiers::SHIFT) => Some(Message::MoveToFirstColumn),
            (KeyCode::Home, KeyModifiers::NONE) => Some(Message::MoveToFirstColumn),
            (KeyCode::Char('$'), KeyModifiers::NONE) => Some(Message::MoveToLastColumn),
            (KeyCode::Right, KeyModifiers::SHIFT) => Some(Message::MoveToLastColumn),
            (KeyCode::End, KeyModifiers::NONE) => Some(Message::MoveToLastColumn),
            (KeyCode::Enter, KeyModifiers::NONE) => Some(Message::Enter),
            (KeyCode::Esc, KeyModifiers::NONE) => Some(Message::Exit),
            _ => None,
        };
        trace!("Mapped: {key:?} => {message:?}");
        message
    }

}