//! A stenography engine in Rust.
//!
//! Most of the content was learnt from the [Plover] engine.
//!
//! Throughout this library, we denote time complexity of functions with [big O notation].
//!
//! [big O notation]: https://en.wikipedia.org/wiki/Big_O_notation
//! [Plover]: https://github.com/openstenoproject/plover

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod input;
pub mod model;
pub mod output;
pub mod util;

use std::sync::mpsc;

use crate::{
    input::plover_hid,
    model::{
        dictionary::Dictionaries,
        stroke::{self, LetterWithSide, StenoSystem},
    },
};

// TODO: Move to PloverHidSystem
pub const STENO_KEY_CHART: [Option<&str>; 64] = {
    let mut chart: [Option<&str>; 64] = [None; 64];

    // Bits 63..56
    chart[63] = Some("S1-");
    chart[62] = Some("T-");
    chart[61] = Some("K-");
    chart[60] = Some("P-");
    chart[59] = Some("W-");
    chart[58] = Some("H-");
    chart[57] = Some("R-");
    chart[56] = Some("A-");

    // Bits 55..48
    chart[55] = Some("O-");
    chart[54] = Some("*1");
    chart[53] = Some("-E");
    chart[52] = Some("-U");
    chart[51] = Some("-F");
    chart[50] = Some("-R");
    chart[49] = Some("-P");
    chart[48] = Some("-B");

    // Bits 47..40
    chart[47] = Some("-L");
    chart[46] = Some("-G");
    chart[45] = Some("-T");
    chart[44] = Some("-S");
    chart[43] = Some("-D");
    chart[42] = Some("-Z");
    chart[41] = Some("#1");
    chart[40] = Some("S2-");

    // Bits 39..32
    chart[39] = Some("*2");
    chart[38] = Some("*3");
    chart[37] = Some("*4");
    chart[36] = Some("#2");
    chart[35] = Some("#3");
    chart[34] = Some("#4");
    chart[33] = Some("#5");
    chart[32] = Some("#6");

    // Bits 31..24
    chart[31] = Some("#7");
    chart[30] = Some("#8");
    chart[29] = Some("#9");
    chart[28] = Some("#A");
    chart[27] = Some("#B");
    chart[26] = Some("#C");

    // Bits 25..0: X1-X26 (unused)
    chart
};

/// Maps physical key name from HID to canonical steno key name.
/// Returns None for keys we drop (X keys, framing/control bits).
pub fn hid_key_to_canonical(physical: &str) -> Option<LetterWithSide> {
    match physical {
        "S1-" | "S2-" => Some(LetterWithSide::parse("S-").unwrap()),
        "*1" | "*2" | "*3" | "*4" => Some(LetterWithSide::parse("*").unwrap()),
        "#1" | "#2" | "#3" | "#4" | "#5" | "#6" | "#7" | "#8" | "#9" | "#A" | "#B" | "#C" => {
            Some(LetterWithSide::parse("#").unwrap())
        }
        "T-" => Some(LetterWithSide::parse("T-").unwrap()),
        "K-" => Some(LetterWithSide::parse("K-").unwrap()),
        "P-" => Some(LetterWithSide::parse("P-").unwrap()),
        "W-" => Some(LetterWithSide::parse("W-").unwrap()),
        "H-" => Some(LetterWithSide::parse("H-").unwrap()),
        "R-" => Some(LetterWithSide::parse("R-").unwrap()),
        "A-" => Some(LetterWithSide::parse("A-").unwrap()),
        "O-" => Some(LetterWithSide::parse("O-").unwrap()),
        "-E" => Some(LetterWithSide::parse("-E").unwrap()),
        "-U" => Some(LetterWithSide::parse("-U").unwrap()),
        "-F" => Some(LetterWithSide::parse("-F").unwrap()),
        "-R" => Some(LetterWithSide::parse("-R").unwrap()),
        "-P" => Some(LetterWithSide::parse("-P").unwrap()),
        "-B" => Some(LetterWithSide::parse("-B").unwrap()),
        "-L" => Some(LetterWithSide::parse("-L").unwrap()),
        "-G" => Some(LetterWithSide::parse("-G").unwrap()),
        "-T" => Some(LetterWithSide::parse("-T").unwrap()),
        "-S" => Some(LetterWithSide::parse("-S").unwrap()),
        "-D" => Some(LetterWithSide::parse("-D").unwrap()),
        "-Z" => Some(LetterWithSide::parse("-Z").unwrap()),
        _ => None,
    }
}

/// Convert a 64-bit HID key state bitmask into a canonical Stroke.
pub fn hid_to_keys(bits: u64) -> Vec<LetterWithSide> {
    // TODO: use a bit iterator over bits
    (0..64)
        .into_iter()
        .filter_map(|i| {
            if bits & (1u64 << i) == 0 {
                return None;
            }
            let physical = STENO_KEY_CHART[i]?;
            let canonical = hid_key_to_canonical(physical)?;
            Some(canonical)
        })
        .collect::<Vec<_>>()
}

fn english_system_keys() -> Vec<LetterWithSide> {
    let s = r##"
        #
        S- T- K- P- W- H- R-
        A- O-
        *
        -E -U
        -F -R -P -B -L -G -T -S -D -Z
"##;

    stroke::parse_keys(s).unwrap()
}

// TODO: data-driven steno system
fn english_system() -> StenoSystem {
    StenoSystem::new(&english_system_keys(), Some(8..13)).unwrap()
}

#[derive(Debug, Clone)]
pub enum EngineCommand {
    Translated(String),
    NotTranslated(String),
}

// TODO: separate Plover HID protocol
pub struct Engine {
    /// Dictionaries
    pub dicts: Dictionaries,
    pub system: StenoSystem,
    /// Configuration for when we trigger translation.
    mode: plover_hid::ChordMode,
    /// Receiver.
    rx: Option<mpsc::Receiver<plover_hid::HidEvent>>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            dicts: Dictionaries::default(),
            system: english_system(),
            mode: plover_hid::ChordMode::FirstUp,
            rx: None,
        }
    }

    /// Try to open and attach to a Plover HID device. Note that it does not refresh the device
    /// list of the HID API. Be sure to refresh it if needed.
    pub fn connect(&mut self, api: &hidapi::HidApi) -> bool {
        if let Some(device) = plover_hid::open_device(api) {
            match device {
                Ok(device) => {
                    self.rx = Some(plover_hid::spawn_hid_thread(device, self.mode));
                    true
                }
                Err(err) => {
                    log::info!("failed to open device: {err}");
                    false
                }
            }
        } else {
            false
        }
    }

    /// Try to open and attach to a Plover HID device, until it succeeds.
    pub fn connect_loop(&mut self, api: &mut hidapi::HidApi) {
        loop {
            if let Err(e) = api.refresh_devices() {
                log::warn!("failed to refresh HID devices: {e}");
            }
            if self.connect(api) {
                return;
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }

    /// Process input events.
    // TODO: better to treat it as an iterator?
    pub fn poll(&mut self) -> Vec<EngineCommand> {
        // TODO: flat_map?
        let events = match &self.rx {
            Some(rx) => rx.try_iter().collect::<Vec<_>>(),
            None => return Vec::new(),
        };

        let mut commands = Vec::new();

        for event in events {
            match event {
                plover_hid::HidEvent::StrokeBits(bits) => {
                    if bits.0 == 0 {
                        continue;
                    }
                    let keys = hid_to_keys(bits.0);
                    if let Some(_stroke) = self.system.stroke_from_keys(&keys) {
                        // TODO: show stroke
                        // TODO: translate the outline
                        commands.push(EngineCommand::NotTranslated(format!(
                            "{:?}",
                            keys.iter().map(|k| k.to_string()).collect::<Vec<_>>()
                        )));
                    } else {
                        // invalid stroke
                        commands.push(EngineCommand::NotTranslated(format!(
                            "<{}>",
                            bits.0.to_string()
                        )));
                    }
                }
                plover_hid::HidEvent::Disconnected => {
                    // TODO: handle disconnected
                }
            }
        }

        commands
    }
}
