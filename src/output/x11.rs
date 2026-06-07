//! X11 output support.
//!
//! - keycode (u8): physical key on the keyboard.
//! - keysym (u32): actually emitted key.

use std::collections::HashMap;

use thiserror::Error;
use x11rb::{
    connection::Connection,
    errors::{ConnectError, ConnectionError, ReplyError},
    protocol::{
        xproto::{self, ConnectionExt as _},
        xtest::ConnectionExt as XTestExt,
    },
    rust_connection::RustConnection,
};

use crate::output::{self, Key, Modifier, Output};

pub type Result<T> = std::result::Result<T, X11Error>;

#[derive(Debug, Error)]
pub enum X11Error {
    #[error("failed to connect to X11 server")]
    Connect(#[from] ConnectError),

    #[error("X11 connection error")]
    Connection(#[from] ConnectionError),

    #[error("X11 reply error")]
    Reply(#[from] ReplyError),
}

pub struct X11Output {
    conn: RustConnection,
    char_to_keycode: HashMap<char, (u8, bool)>,
    // modifiers:
    shift_keycode: u8,
    ctrl_keycode: u8,
    alt_keycode: u8,
    super_keycode: u8,
    // special (non-character) keys:
    backspace_keycode: u8,
    return_keycode: u8,
    tab_keycode: u8,
    escape_keycode: u8,
}

impl X11Output {
    pub fn new() -> Result<Self> {
        let (conn, _screen_num) = RustConnection::connect(None)?;

        let setup = conn.setup();
        let min_keycode: u8 = setup.min_keycode;
        let max_keycode: u8 = setup.max_keycode;

        // The X server's key mapping table
        let mapping = conn
            .get_keyboard_mapping(min_keycode, max_keycode - min_keycode + 1)?
            .reply()?;

        // each keycode has multiple _slots_ with different modifiers:
        let keysyms_per_keycode = mapping.keysyms_per_keycode as usize;
        let keysyms = &mapping.keysyms;

        let mut char_to_keycode = HashMap::new();

        // char -> keycode
        for keycode in min_keycode..=max_keycode {
            let keysym_idx = (keycode - min_keycode) as usize * keysyms_per_keycode;

            // slot 0: no modifiers
            if keysym_idx < keysyms.len() {
                let keysym = keysyms[keysym_idx];
                if let Some(ch) = keysym_to_char(keysym) {
                    char_to_keycode.entry(ch).or_insert((keycode, false));
                }
            }

            // slot 1: shift
            if keysym_idx + 1 < keysyms.len() {
                let keysym = keysyms[keysym_idx + 1];
                if let Some(ch) = keysym_to_char(keysym) {
                    char_to_keycode.entry(ch).or_insert((keycode, true));
                }
            }
        }

        // keysym -> keycode
        let find_keycode = |keysym: u32| -> u8 {
            for keycode in min_keycode..=max_keycode {
                let idx = (keycode - min_keycode) as usize * keysyms_per_keycode;
                for offset in 0..keysyms_per_keycode {
                    if idx + offset < keysyms.len() && keysyms[idx + offset] == keysym {
                        return keycode;
                    }
                }
            }
            0
        };

        Ok(X11Output {
            conn,
            char_to_keycode,
            // modifiers:
            shift_keycode: find_keycode(0xFFE1),
            ctrl_keycode: find_keycode(0xFFE3),
            alt_keycode: find_keycode(0xFFE9),
            super_keycode: find_keycode(0xFFEB),
            // special (non-character) keys:
            backspace_keycode: find_keycode(0xFF08),
            return_keycode: find_keycode(0xFF0D),
            tab_keycode: find_keycode(0xFF09),
            escape_keycode: find_keycode(0xFF1B),
        })
    }

    fn x11_key_event(&self, keycode: u8, press: bool) -> Result<()> {
        let event_type = if press {
            xproto::KEY_PRESS_EVENT
        } else {
            xproto::KEY_RELEASE_EVENT
        };
        self.conn
            .xtest_fake_input(event_type, keycode, 0, x11rb::CURRENT_TIME, 0, 0, 0)?;
        Ok(())
    }

    fn tap_key(&self, keycode: u8, shift: bool) -> Result<()> {
        if shift {
            self.x11_key_event(self.shift_keycode, true)?;
        }
        self.x11_key_event(keycode, true)?;
        self.x11_key_event(keycode, false)?;
        if shift {
            self.x11_key_event(self.shift_keycode, false)?;
        }
        Ok(())
    }

    fn keycode_for_modifier(&self, modifier: Modifier) -> u8 {
        match modifier {
            Modifier::Ctrl => self.ctrl_keycode,
            Modifier::Alt => self.alt_keycode,
            Modifier::Shift => self.shift_keycode,
            Modifier::Super => self.super_keycode,
        }
    }

    fn keycode_for_key(&self, key: Key) -> Option<(u8, bool)> {
        let keycode = match key {
            Key::Backspace => self.backspace_keycode,
            Key::Return => self.return_keycode,
            Key::Tab => self.tab_keycode,
            Key::Escape => self.escape_keycode,
            Key::Char(c) => return self.char_to_keycode.get(&c).cloned(),
            _ => return None,
        };
        Some((keycode, false))
    }
}

impl Output for X11Output {
    fn send_keys(&mut self, keys: &[Key], mods: &[Modifier]) -> output::Result<()> {
        // TODO: handle shift separately?
        for keycode in mods.iter().map(|m| self.keycode_for_modifier(*m)) {
            if keycode != 0 {
                self.x11_key_event(keycode, true)?;
            }
        }

        for key in keys {
            if let Some((keycode, shift)) = self.keycode_for_key(*key) {
                self.tap_key(keycode, shift)?;
            }
        }

        for keycode in mods.iter().rev().map(|m| self.keycode_for_modifier(*m)) {
            if keycode != 0 {
                self.x11_key_event(keycode, false)?;
            }
        }

        self.conn.flush().map_err(X11Error::from)?;
        Ok(())
    }
}

/// Convert an X11 keysym to a Unicode char.
fn keysym_to_char(keysym: u32) -> Option<char> {
    match keysym {
        0 => None,
        0x20..=0x7E | 0xA0..=0xFF => char::from_u32(keysym),
        _ if keysym > 0x01000000 => return char::from_u32(keysym - 0x01000000),
        _ => None,
    }
}
