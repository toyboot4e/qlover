//! A stenography engine in Rust.

use qlover::{engine::Engine, output};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let mut hid = hidapi::HidApi::new().unwrap_or_else(|e| panic!("unable to initialize HID: {e}"));
    let mut engine = Engine::new();
    let mut output = output::create_output().unwrap();

    loop {
        println!("connecting to a new device..");
        engine.connect_loop(&mut hid);
        println!("..connected!");

        loop {
            for cmd in engine.poll() {
                // TODO: handle any command
                output.send_string(&cmd);
            }
        }

        // TODO: handle input
        break;
    }
}
