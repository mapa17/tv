use ratatui::crossterm::event::{self, KeyCode, KeyModifiers};
use tracing::trace;

#[derive(Default)]
pub struct Inputter {
    current_input: String,
    curser_pos: usize,
    input_width: usize,
    finished: bool,
    canceled: bool,
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

    pub fn set(&mut self, s: &str) {
        self.current_input = s.to_string();
        self.curser_pos += s.to_ascii_lowercase().len();
    }

    pub fn get(&self) -> InputResult {
        InputResult {
            canceled: self.canceled,
            finished: self.finished,
            input: self.current_input.clone(),
            curser_pos: self.curser_pos,
        }
    }

    pub fn set_width(&mut self, width: usize) {
        self.input_width = width;
    }

    pub fn clear(&mut self) {
        self.canceled = false;
        self.finished = false;
        self.current_input.clear();
        self.curser_pos = 0;
    }

    fn enter(&mut self) -> InputResult {
        self.finished = true;
        self.get()
    }

    fn escape(&mut self) -> InputResult {
        self.clear();
        self.canceled = true;
        self.finished = true;
        self.get()
    }

    fn backspace(&mut self) -> InputResult {
        if self.curser_pos > 0 {
            self.current_input.pop();
            self.curser_pos -= 1;
        }
        self.get()
    }

    fn left(&mut self) -> InputResult {
        self.curser_pos = self.curser_pos.saturating_sub(1);
        self.get()
    }

    fn right(&mut self) -> InputResult {
        if self.curser_pos < self.current_input.len() {
            self.curser_pos += 1;
        }
        self.get()
    }

    fn key(&mut self, code: KeyCode, _modifier: KeyModifiers) -> InputResult {
        if let Some(chr) = code.as_char() {
            self.current_input.insert(self.getbytepos(), chr);
            self.curser_pos += 1;
        }
        self.get()
    }

    fn getbytepos(&self) -> usize {
        self.current_input
            .char_indices()
            .nth(self.curser_pos)
            .map(|(byte_idx, _)| byte_idx)
            .unwrap_or(self.current_input.len())
    }
}
