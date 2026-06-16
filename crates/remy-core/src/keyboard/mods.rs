use std::ops::{BitOr, BitOrAssign};

use crossterm::event::KeyModifiers;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Mods(u8);

impl Mods {
    pub const NONE: Self = Self(0);
    pub const CTRL: Self = Self(1 << 0);
    pub const ALT: Self = Self(1 << 1);
    pub const SHIFT: Self = Self(1 << 2);

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub fn remove(&mut self, other: Self) {
        self.0 &= !other.0;
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub(crate) fn from_modifiers(modifiers: KeyModifiers) -> Option<Self> {
        if !(modifiers - KeyModifiers::SHIFT - KeyModifiers::CONTROL - KeyModifiers::ALT).is_empty()
        {
            return None;
        }

        let mut key_mods = Self::NONE;
        if modifiers.contains(KeyModifiers::CONTROL) {
            key_mods |= Self::CTRL;
        }
        if modifiers.contains(KeyModifiers::ALT) {
            key_mods |= Self::ALT;
        }
        if modifiers.contains(KeyModifiers::SHIFT) {
            key_mods |= Self::SHIFT;
        }
        Some(key_mods)
    }
}

impl BitOr for Mods {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for Mods {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}
