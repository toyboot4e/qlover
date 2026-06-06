//! A stenography engine in Rust.
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

use crate::{input::plover_hid, model::stroke::Stroke};

// TODO: separate Plover HID protocol
pub struct Engine {
    /// Configuration for when we trigger translation.
    mode: plover_hid::ChordMode,
    /// Receiver.
    rx: Option<mpsc::Receiver<plover_hid::HidEvent>>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
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

    /// Process input events
    pub fn poll(&mut self) -> Vec<String> {
        // TODO: do not connect. do flat_map and fold
        let events = match &self.rx {
            Some(rx) => rx.try_iter().collect::<Vec<_>>(),
            None => return Vec::new(),
        };

        let mut results = Vec::new();

        for event in events {
            match event {
                plover_hid::HidEvent::Stroke(stroke) => {
                    if stroke.is_empty() {
                        continue;
                    }
                    if let Some(result) = self.process_stroke(stroke) {
                        results.push("TODO".to_string());
                    }
                }
                plover_hid::HidEvent::Disconnected => {
                    // TODO: handle disconnected
                }
            }
        }

        results
    }

    fn process_stroke(&self, stroke: Stroke) -> Option<String> {
        Some("TODO".to_string())
    }
}
