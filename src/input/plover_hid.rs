use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use hidapi::HidApi;

use crate::stroke::Stroke;

// Plover HID protocol on USB HID
const REPORT_ID: u8 = 0x50;

fn is_plover_hid(dev_info: &hidapi::DeviceInfo) -> bool {
    const USAGE_PAGE: u16 = 0xFF50;
    const USAGE: u16 = 0x4C56;
    dev_info.usage_page() == USAGE_PAGE && dev_info.usage() == USAGE
}

/// Chord detection mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChordMode {
    /// Emit chord when all keys are released.
    AllKeysUp,

    /// Emit chord as soon as any key is released.
    FirstUp,
}

/// Open the first Plover HID device found.
pub fn open_device(api: &HidApi) -> Option<hidapi::HidResult<hidapi::HidDevice>> {
    api.device_list().find_map(|dev_info| {
        if is_plover_hid(dev_info) {
            Some(dev_info.open_device(api))
        } else {
            None
        }
    })
}

/// Spawn an HID thread. Returns the receiver end of the channel.
pub fn spawn_hid_thread(device: hidapi::HidDevice, mode: ChordMode) -> mpsc::Receiver<HidEvent> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        if let Err(e) = hid_read_loop(&device, &tx, mode) {
            log::error!("HID read loop error: {}", e);
            let _ = tx.send(HidEvent::Disconnected);
        }
    });

    rx
}

/// Events from the HID thread.
#[derive(Debug)]
pub enum HidEvent {
    Stroke(Stroke),
    Disconnected,
}

/// Main HID read loop with chord detection.
fn hid_read_loop(
    device: &hidapi::HidDevice,
    tx: &mpsc::Sender<HidEvent>,
    mode: ChordMode,
) -> hidapi::HidResult<()> {
    let mut buf = [0u8; 64];
    let mut acc: usize = 0;
    let mut sent_first_up = false;

    loop {
        let n = device.read_timeout(&mut buf, 1000)?;

        if n < 9 || buf[0] != REPORT_ID {
            continue;
        }

        let current_stroke = usize::from_be_bytes([
            buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7], buf[8],
        ]);

        match mode {
            ChordMode::AllKeysUp => {
                // Accumulate pressed keys
                acc |= current_stroke;
                if current_stroke == 0 && acc != 0 {
                    // Released
                    let stroke = Stroke::new(acc);
                    acc = 0;
                    if tx.send(HidEvent::Stroke(stroke)).is_err() {
                        // nobody is listening
                        return Ok(());
                    }
                }
            }
            ChordMode::FirstUp => {
                if !sent_first_up {
                    // Handle release
                    if acc & !current_stroke != 0 {
                        let stroke = Stroke::new(acc);
                        sent_first_up = true;
                        if tx.send(HidEvent::Stroke(stroke)).is_err() {
                            return Ok(());
                        }
                    }
                }
                if current_stroke & !acc != 0 {
                    sent_first_up = false;
                }
                // Snapshot current state
                acc = current_stroke;
            }
        }
    }
}

/// Create a mock HID event sender for testing without hardware.
pub fn spawn_mock_hid_thread() -> mpsc::Receiver<HidEvent> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        // Just keep the sender alive until main thread drops receiver
        loop {
            thread::sleep(Duration::from_secs(1));
            if tx.send(HidEvent::Stroke(Stroke::new(0))).is_err() {
                break;
            }
        }
    });

    rx
}
