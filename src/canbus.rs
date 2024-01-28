use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;
use std::{
    sync::mpsc::{channel, Receiver},
    time::{Duration, SystemTime},
};

pub struct Message {
    pub id: String,
    pub data: String,
    pub ts: SystemTime,
}

impl Message {
    fn new(id: String, data: String) -> Self {
        Self {
            id,
            data,
            ts: SystemTime::now(),
        }
    }
}

pub fn recv() -> Receiver<Message> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let serial = File::open("/dev/ttyACM0").unwrap();
        let lines = BufReader::new(serial).lines().map(|l| l.unwrap_or_default());
        for line in lines {
            if line == "sleep" {
                std::thread::sleep(Duration::from_secs(1));
            }
            let mut words = line.split(' ');
            if words.next() == Some("ID:") {
                if let Some(id) = words.next() {
                    let data = words.nth(1).unwrap_or_default();
                    tx.send(Message::new(id.to_string(), data.to_string()))
                        .unwrap();
                }
            }
        }
    });
    rx
}
