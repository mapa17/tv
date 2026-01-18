use tracing::trace;

use ratatui::crossterm::event::{self, KeyCode, KeyModifiers};

#[derive(Default)]
pub struct Inputter {
    pub current_input: String,
    pub curser_pos: usize,
    pub input_width: usize,
}

#[derive(Default, Clone)]
pub struct InputResult {
    pub input: String,
    pub finished: bool,
    pub canceled: bool,
    pub curser_pos: usize,
}

impl Inputter {
    pub fn read(&mut self, key: event::KeyEvent) -> InputResult {
        match (key.code, key.modifiers) {
            (KeyCode::Enter, KeyModifiers::NONE) => self.enter(),
            (KeyCode::Esc, KeyModifiers::NONE) => self.escape(),
            (KeyCode::Backspace, KeyModifiers::NONE) => self.backspace(),
            (KeyCode::Left, KeyModifiers::NONE) => self.left(),
            (KeyCode::Right, KeyModifiers::NONE) => self.right(),
            (kc, km) => self.key(kc, km),
        }
    }

    fn enter(&mut self) -> InputResult {
        let input = self.current_input.clone();
        self.current_input.clear();
        InputResult {
            canceled: false,
            finished: true,
            input: input,
            curser_pos: 0,
        }
    }

    fn escape(&mut self) -> InputResult {
        let input = self.current_input.clone();
        self.current_input.clear();
        InputResult {
            canceled: true,
            finished: true,
            input: String::new(),
            curser_pos: 0,
        }
    }

    fn backspace(&mut self) -> InputResult {
        InputResult::default()
    }

    fn left(&mut self) -> InputResult {
        InputResult::default()
    }

    fn right(&mut self) -> InputResult {
        InputResult::default()
    }

    fn key(&mut self, code: KeyCode, modifier: KeyModifiers) -> InputResult {
        if let Some(chr) = code.as_char() {
            self.current_input.push(chr);
            self.curser_pos +=1;
        }
        InputResult {
            canceled: false,
            finished: false,
            input: self.current_input.clone(),
            curser_pos: self.curser_pos,
        }
    }
}