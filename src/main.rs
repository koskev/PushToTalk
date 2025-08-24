use clap::Parser;
use evdev::{Device, EventType, KeyCode};
use inotify::{EventMask, Inotify, WatchMask};
use std::collections::HashMap;
use std::io::ErrorKind;
use std::str::FromStr;

fn get_devices() -> Vec<String> {
    let paths = std::fs::read_dir("/dev/input").unwrap();
    let mut vec = std::vec::Vec::new();
    for path in paths {
        let path = path.unwrap().path();
        let filename = path
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();
        if filename.contains("event") {
            vec.push(filename.to_string());
        }
    }
    return vec;
}

struct PushToTalk {
    device: Device,
    push_to_talk_key: KeyCode,
}

impl PushToTalk {
    fn new(device: Device, key: KeyCode) -> Self {
        println!("Adding new listener for {}", device.name().unwrap());
        Self {
            device,
            push_to_talk_key: key,
        }
    }

    fn listen(&mut self) {
        let dev = &mut self.device;
        loop {
            let event_result = dev.fetch_events();
            match event_result {
                Ok(events) => {
                    for event in events {
                        if event.event_type() == EventType::KEY {
                            //println!("Got event val {} {}", event.value(), event.code());
                            let pressed_key = KeyCode::new(event.code());
                            PushToTalk::handle_key(
                                &self.push_to_talk_key,
                                &pressed_key,
                                event.value(),
                            );
                        }
                    }
                }

                Err(event) => {
                    println!("Failed to fetch events {}", event);
                    break;
                }
            }
        }
    }

    fn handle_key(ptt_key: &KeyCode, key: &KeyCode, value: i32) {
        //println!("Handling key {}, ppt key {}", key.0, PUSH_TO_TALK_KEY.0);
        if *key == *ptt_key {
            if value == 1 {
                PushToTalk::set_mute(false);
            } else if value == 0 {
                PushToTalk::set_mute(true);
            }
        }
    }

    fn set_mute(mute: bool) {
        std::process::Command::new("pactl")
            .args(["set-source-mute", "@DEFAULT_SOURCE@", &mute.to_string()])
            .output()
            .expect("Failed to run pactl");
    }
}

struct PushToTalkManager {
    listener: HashMap<String, std::thread::JoinHandle<PushToTalk>>,
    key: KeyCode,
}

impl PushToTalkManager {
    fn new(key: KeyCode) -> Self {
        Self {
            listener: HashMap::new(),
            key,
        }
    }
    fn on_new_device(&mut self, name: String) {
        let dev = Device::open(format!("/dev/input/{}", name));
        match dev {
            Ok(d) => {
                if !(d
                    .name()
                    .unwrap_or_default()
                    .to_lowercase()
                    .contains("keyboard"))
                {
                    return;
                }
                let mut ptt = PushToTalk::new(d, self.key);

                let thread = std::thread::spawn(move || {
                    ptt.listen();
                    ptt
                });
                self.listener.insert(name, thread);
            }
            Err(e) => println!("Could not open {}. Error: {}", name, e),
        }
    }

    fn on_delete_device(&mut self, name: String) {
        if self.listener.contains_key(&name) {
            self.listener.remove(&name);
        }
    }

    fn watch_inputs(&mut self) {
        // Setup inotify listener
        let mut inotify = Inotify::init().expect("Error while initializing inotify instance");
        inotify
            .add_watch("/dev/input", WatchMask::DELETE | WatchMask::ATTRIB)
            .expect("Failed to add file watch");

        let mut buffer = [0; 1024];
        loop {
            let events = loop {
                match inotify.read_events_blocking(&mut buffer) {
                    Ok(events) => break events,
                    Err(error) if error.kind() == ErrorKind::WouldBlock => continue,
                    _ => panic!("Error while reading events"),
                }
            };

            for event in events {
                let name = event.name.unwrap().to_str().unwrap().to_string();
                // XXX: CREATE is too fast. We need to wait for ATTRIB. If this bind already exists, it doesn't matter
                if event.mask == inotify::EventMask::ATTRIB {
                    println!("Attr changed on {}", name);
                    self.on_new_device(name);
                } else if event.mask == EventMask::DELETE {
                    self.on_delete_device(name);
                }
            }
        }
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Specify the key to use. Most keys have the form KEY_<NAME>
    #[arg(short, long, default_value = "KEY_CAPSLOCK")]
    key: String,
}

fn main() {
    let cli = Cli::parse();
    let key_res = KeyCode::from_str(&cli.key);
    match key_res {
        Ok(key) => {
            let mut manager = PushToTalkManager::new(key);
            println!("Starting global PTT with key {:?}", key);
            let device_names = get_devices();
            for device_name in device_names {
                manager.on_new_device(device_name);
            }
            manager.watch_inputs();
        }
        Err(e) => println!("Prodived an invalid key: {:?}", e),
    }
}
