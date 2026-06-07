//! A stenography engine in Rust.

use qlover::{output, Engine, EngineCommand};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let mut hid = hidapi::HidApi::new().unwrap_or_else(|e| panic!("unable to initialize HID: {e}"));
    let mut engine = Engine::new();
    let _output = output::new().unwrap();

    // Test
    let dict = serde_json::from_str(r#"{ "a": "abc"}"#).unwrap();
    engine.dicts.stack.push(dict);

    loop {
        println!("connecting to a new device..");
        engine.connect_loop(&mut hid);
        println!("..connected!");

        loop {
            for cmd in engine.poll() {
                match cmd {
                    EngineCommand::Translated(s) => {
                        println!("{}", s);
                        // output.send_string(&s).unwrap();
                    }
                    EngineCommand::NotTranslated(s) => {
                        println!("{}", s);
                        // output.send_string(&s).unwrap();
                    }
                }
            }
        }
    }
}
