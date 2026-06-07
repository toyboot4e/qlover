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

use crate::output::{self, Key, Modifiers, Output};

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
        let keysyms = &mapping.keysyms;
        let keysyms_per_keycode = mapping.keysyms_per_keycode as usize;

        let char_to_keycode = {
            let mut char_to_keycode = HashMap::new();

            // each keycode has multiple _slots_ with different modifiers:
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

            char_to_keycode
        };

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

    fn keycode_for_modifier_bit(&self, bit: Modifiers) -> u8 {
        if bit == Modifiers::CTRL {
            self.ctrl_keycode
        } else if bit == Modifiers::ALT {
            self.alt_keycode
        } else if bit == Modifiers::SHIFT {
            self.shift_keycode
        } else if bit == Modifiers::SUPER {
            self.super_keycode
        } else {
            0
        }
    }

    fn on_modifier_change(&self, from: Modifiers, to: Modifiers) -> Result<()> {
        // release
        for bit in (from - to).iter() {
            let kc = self.keycode_for_modifier_bit(bit);
            if kc != 0 {
                self.x11_key_event(kc, false)?;
            }
        }

        // press
        for bit in (to - from).iter() {
            let kc = self.keycode_for_modifier_bit(bit);
            if kc != 0 {
                self.x11_key_event(kc, true)?;
            }
        }

        Ok(())
    }

    fn keycode_for_key(&self, key: Key) -> Option<(u8, bool)> {
        let keycode = match key {
            Key::Backspace => self.backspace_keycode,
            Key::Return => self.return_keycode,
            Key::Tab => self.tab_keycode,
            Key::Escape => self.escape_keycode,
            Key::Char(c) => return self.char_to_keycode.get(&c).cloned(),
        };
        Some((keycode, false))
    }
}

impl Output for X11Output {
    fn send_keys(&mut self, strokes: &[(Key, Modifiers)]) -> output::Result<()> {
        let held = strokes
            .iter()
            .cloned()
            .try_fold(Modifiers::empty(), |held, (key, mods)| {
                let Some((keycode, shift)) = self.keycode_for_key(key) else {
                    return Ok(held);
                };
                let mods = if shift { mods | Modifiers::SHIFT } else { mods };
                self.on_modifier_change(held, mods)?;
                self.x11_key_event(keycode, true)?;
                self.x11_key_event(keycode, false)?;
                Ok::<_, X11Error>(mods)
            })?;

        self.on_modifier_change(held, Modifiers::empty())?;
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
