use crossterm::event::{KeyCode, KeyEvent};

use super::{Chord, Mods};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Code {
    Char(char),
    Esc,
    Enter,
    Tab,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    Up,
    Down,
    Left,
    Right,
    F(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key {
    code: Code,
    modifiers: Mods,
}

impl Key {
    pub const fn esc() -> Self {
        Self::plain(Code::Esc)
    }

    pub const fn enter() -> Self {
        Self::plain(Code::Enter)
    }

    pub const fn tab() -> Self {
        Self::plain(Code::Tab)
    }

    pub const fn backtab() -> Self {
        Self::modified_code(Code::Tab, Mods::SHIFT)
    }

    pub const fn backspace() -> Self {
        Self::plain(Code::Backspace)
    }

    pub const fn delete() -> Self {
        Self::plain(Code::Delete)
    }

    pub const fn insert() -> Self {
        Self::plain(Code::Insert)
    }

    pub const fn home() -> Self {
        Self::plain(Code::Home)
    }

    pub const fn end() -> Self {
        Self::plain(Code::End)
    }

    pub const fn page_up() -> Self {
        Self::plain(Code::PageUp)
    }

    pub const fn page_down() -> Self {
        Self::plain(Code::PageDown)
    }

    pub const fn up() -> Self {
        Self::plain(Code::Up)
    }

    pub const fn down() -> Self {
        Self::plain(Code::Down)
    }

    pub const fn left() -> Self {
        Self::plain(Code::Left)
    }

    pub const fn right() -> Self {
        Self::plain(Code::Right)
    }

    pub const fn f(n: u8) -> Self {
        Self::plain(Code::F(n))
    }

    pub fn ctrl(c: char) -> Self {
        Self::modified_code(Code::Char(c.to_ascii_lowercase()), Mods::CTRL)
    }

    pub fn alt(c: char) -> Self {
        Self::modified_code(Code::Char(c.to_ascii_lowercase()), Mods::ALT)
    }

    pub fn shift(key: impl Into<Key>) -> Self {
        Self::modified(key, Mods::SHIFT)
    }

    pub fn modified(key: impl Into<Key>, modifiers: Mods) -> Self {
        let mut key = key.into();
        key.modifiers |= modifiers;
        key.normalize()
    }

    pub fn with_ctrl(self) -> Self {
        Self::modified(self, Mods::CTRL)
    }

    pub fn with_alt(self) -> Self {
        Self::modified(self, Mods::ALT)
    }

    pub fn with_shift(self) -> Self {
        Self::modified(self, Mods::SHIFT)
    }

    pub const fn modifiers(self) -> Mods {
        self.modifiers
    }

    pub fn then(self, next: impl Into<Key>) -> Chord {
        Chord::from(self).then(next)
    }

    pub fn chord<I, K>(keys: I) -> Chord
    where
        I: IntoIterator<Item = K>,
        K: Into<Key>,
    {
        Chord::from_keys(keys)
    }

    pub fn from_event(event: KeyEvent) -> Option<Self> {
        let modifiers = Mods::from_modifiers(event.modifiers)?;
        match event.code {
            KeyCode::Char(c) => Some(Self::modified_code(Code::Char(c), modifiers).normalize()),
            KeyCode::Esc => Some(Self::modified_code(Code::Esc, modifiers)),
            KeyCode::Enter => Some(Self::modified_code(Code::Enter, modifiers)),
            KeyCode::Tab => Some(Self::modified_code(Code::Tab, modifiers)),
            KeyCode::BackTab => Some(Self::modified_code(Code::Tab, modifiers | Mods::SHIFT)),
            KeyCode::Backspace => Some(Self::modified_code(Code::Backspace, modifiers)),
            KeyCode::Delete => Some(Self::modified_code(Code::Delete, modifiers)),
            KeyCode::Insert => Some(Self::modified_code(Code::Insert, modifiers)),
            KeyCode::Home => Some(Self::modified_code(Code::Home, modifiers)),
            KeyCode::End => Some(Self::modified_code(Code::End, modifiers)),
            KeyCode::PageUp => Some(Self::modified_code(Code::PageUp, modifiers)),
            KeyCode::PageDown => Some(Self::modified_code(Code::PageDown, modifiers)),
            KeyCode::Up => Some(Self::modified_code(Code::Up, modifiers)),
            KeyCode::Down => Some(Self::modified_code(Code::Down, modifiers)),
            KeyCode::Left => Some(Self::modified_code(Code::Left, modifiers)),
            KeyCode::Right => Some(Self::modified_code(Code::Right, modifiers)),
            KeyCode::F(n) => Some(Self::modified_code(Code::F(n), modifiers)),
            _ => None,
        }
    }

    pub fn label(&self) -> String {
        let base = match self.code {
            Code::Char(' ') => "Space".to_string(),
            Code::Char(c) if self.modifiers.is_empty() => c.to_string(),
            Code::Char(c) => c.to_ascii_uppercase().to_string(),
            Code::Esc => "Esc".to_string(),
            Code::Enter => "Enter".to_string(),
            Code::Tab => "Tab".to_string(),
            Code::Backspace => "Backspace".to_string(),
            Code::Delete => "Delete".to_string(),
            Code::Insert => "Insert".to_string(),
            Code::Home => "Home".to_string(),
            Code::End => "End".to_string(),
            Code::PageUp => "PageUp".to_string(),
            Code::PageDown => "PageDown".to_string(),
            Code::Up => "Up".to_string(),
            Code::Down => "Down".to_string(),
            Code::Left => "Left".to_string(),
            Code::Right => "Right".to_string(),
            Code::F(n) => format!("F{n}"),
        };

        if self.modifiers.is_empty() {
            return base;
        }

        let mut parts = Vec::with_capacity(4);
        if self.modifiers.contains(Mods::CTRL) {
            parts.push("Ctrl".to_string());
        }
        if self.modifiers.contains(Mods::ALT) {
            parts.push("Alt".to_string());
        }
        if self.modifiers.contains(Mods::SHIFT) {
            parts.push("Shift".to_string());
        }
        parts.push(base);
        parts.join("+")
    }

    const fn plain(code: Code) -> Self {
        Self {
            code,
            modifiers: Mods::NONE,
        }
    }

    const fn modified_code(code: Code, modifiers: Mods) -> Self {
        Self { code, modifiers }
    }

    fn normalize(mut self) -> Self {
        if let Code::Char(c) = self.code
            && c.is_ascii_uppercase()
        {
            self.code = Code::Char(c.to_ascii_lowercase());
            self.modifiers |= Mods::SHIFT;
        } else if let Code::Char(c) = self.code
            && !c.is_ascii_alphabetic()
        {
            self.modifiers.remove(Mods::SHIFT);
        }
        self
    }
}

impl From<char> for Key {
    fn from(c: char) -> Self {
        Self::modified_code(Code::Char(c), Mods::NONE).normalize()
    }
}
