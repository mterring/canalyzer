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
        let lines = std::io::stdin().lines().map(|l| l.unwrap());
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
