//! X11 output support.

use std::collections::HashMap;

use log::debug;
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

use crate::output::{self, KeyboardOutput};

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
    backspace_keycode: u8,
    shift_keycode: u8,
    ctrl_keycode: u8,
    alt_keycode: u8,
    super_keycode: u8,
    return_keycode: u8,
    tab_keycode: u8,
    escape_keycode: u8,
}

impl X11Output {
    pub fn new() -> Result<Self> {
        let (conn, _screen_num) = RustConnection::connect(None)?;

        let setup = conn.setup();
        let min_keycode = setup.min_keycode;
        let max_keycode = setup.max_keycode;

        let mapping = conn
            .get_keyboard_mapping(min_keycode, max_keycode - min_keycode + 1)?
            .reply()?;

        let keysyms_per_keycode = mapping.keysyms_per_keycode as usize;
        let keysyms = &mapping.keysyms;

        let mut char_to_keycode = HashMap::new();

        for keycode in min_keycode..=max_keycode {
            let idx = (keycode - min_keycode) as usize * keysyms_per_keycode;

            if idx < keysyms.len() {
                let keysym = keysyms[idx];
                if let Some(ch) = keysym_to_char(keysym) {
                    char_to_keycode.entry(ch).or_insert((keycode, false));
                }
            }

            if idx + 1 < keysyms.len() {
                let keysym = keysyms[idx + 1];
                if let Some(ch) = keysym_to_char(keysym) {
                    char_to_keycode.entry(ch).or_insert((keycode, true));
                }
            }
        }

        let find_keycode = |target_keysym: u32| -> u8 {
            for keycode in min_keycode..=max_keycode {
                let idx = (keycode - min_keycode) as usize * keysyms_per_keycode;
                for offset in 0..keysyms_per_keycode {
                    if idx + offset < keysyms.len() && keysyms[idx + offset] == target_keysym {
                        return keycode;
                    }
                }
            }
            0
        };

        Ok(X11Output {
            backspace_keycode: find_keycode(0xFF08),
            shift_keycode: find_keycode(0xFFE1),
            ctrl_keycode: find_keycode(0xFFE3),
            alt_keycode: find_keycode(0xFFE9),
            super_keycode: find_keycode(0xFFEB),
            return_keycode: find_keycode(0xFF0D),
            tab_keycode: find_keycode(0xFF09),
            escape_keycode: find_keycode(0xFF1B),
            conn,
            char_to_keycode,
        })
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

    fn modifier_keycode(&self, name: &str) -> u8 {
        match name {
            "ctrl" | "control" => self.ctrl_keycode,
            "alt" => self.alt_keycode,
            "shift" => self.shift_keycode,
            "super" | "win" => self.super_keycode,
            _ => 0,
        }
    }

    fn named_keycode(&self, name: &str) -> Option<u8> {
        let kc = match name {
            "return" | "enter" => self.return_keycode,
            "tab" => self.tab_keycode,
            "escape" | "esc" => self.escape_keycode,
            "backspace" => self.backspace_keycode,
            _ => return None,
        };
        if kc == 0 {
            None
        } else {
            Some(kc)
        }
    }
}

impl KeyboardOutput for X11Output {
    fn send_backspaces(&mut self, count: usize) -> output::Result<()> {
        for _ in 0..count {
            self.tap_key(self.backspace_keycode, false)?;
        }
        self.conn.flush().map_err(X11Error::from)?;
        Ok(())
    }

    fn send_string(&mut self, s: &str) -> output::Result<()> {
        for ch in s.chars() {
            if let Some(&(keycode, shift)) = self.char_to_keycode.get(&ch) {
                self.tap_key(keycode, shift)?;
            } else {
                debug!("No keycode for character: {:?}", ch);
            }
        }
        self.conn.flush().map_err(X11Error::from)?;
        Ok(())
    }

    fn send_key_combination(&mut self, key: &str, modifiers: &[&str]) -> output::Result<()> {
        for m in modifiers {
            let keycode = self.modifier_keycode(m);
            if keycode != 0 {
                self.x11_key_event(keycode, true)?;
            }
        }

        if let Some(keycode) = self.named_keycode(key) {
            self.x11_key_event(keycode, true)?;
            self.x11_key_event(keycode, false)?;
        } else if let Some(ch) = key.chars().next() {
            if key.chars().count() == 1 {
                if let Some(&(keycode, shift)) = self.char_to_keycode.get(&ch) {
                    if shift {
                        self.x11_key_event(self.shift_keycode, true)?;
                    }
                    self.x11_key_event(keycode, true)?;
                    self.x11_key_event(keycode, false)?;
                    if shift {
                        self.x11_key_event(self.shift_keycode, false)?;
                    }
                }
            }
        }

        for m in modifiers.iter().rev() {
            let keycode = self.modifier_keycode(m);
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
