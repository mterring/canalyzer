use anyhow::Result;
use crossterm::ExecutableCommand;
use ratatui::{prelude::*, widgets::{*, block::Position}};
use serde::Serialize;
use serde_with::{serde_as, TimestampMilliSeconds};
use std::cmp::Ordering;

mod canbus {
    use std::sync::mpsc::{channel, Receiver};

    pub struct Message {
        pub id: String,
        pub data: String,
        pub ts: std::time::SystemTime,
    }

    impl Message {
        fn new(id: String, data: String) -> Self {
            Self {
                id,
                data,
                ts: std::time::SystemTime::now(),
            }
        }
    }

    pub fn recv() -> Receiver<Message> {
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let lines = std::io::stdin().lines().map(|l| l.unwrap());
            for line in lines {
                if line == "sleep" {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                }
                let mut words = line.split(" ");
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
}

#[derive(Serialize)]
struct Message {
    id: String,
    values: Vec<Value>,
    ignored: bool,
    pinned: bool,
}

#[serde_as]
#[derive(Serialize, Default)]
struct Value {
    data: String,
    #[serde_as(as = "TimestampMilliSeconds")]
    ts: std::time::SystemTime,
}

impl From<canbus::Message> for Value {
    fn from(msg: canbus::Message) -> Self {
        Self {
            data: msg.data,
            ts: msg.ts,
        }
    }
}

impl Message {
    fn merge(&mut self, other: canbus::Message) {
        self.values.push(other.into());
    }
}

impl From<canbus::Message> for Message {
    fn from(other: canbus::Message) -> Self {
        Self {
            id: other.id.clone(),
            values: vec![other.into()],
            ignored: false,
            pinned: false,
        }
    }
}

impl <'a> From<&'a Value> for Cell<'a> {
    fn from(v: &'a Value) -> Self {
        Cell::new(a.data.as_str())
    }
}

impl<'a> From<&'a Message> for Row<'a> {
    fn from(msg: &'a Message) -> Self {
        let mut v = msg.values.iter().rev().fuse();
        let row = Row::new(vec![
            msg.id.as_str(),
            v.next().unwrap_or_default(),
            v.next().unwrap_or_default(),
        ]);
        if msg.ignored {
            row.dark_gray().crossed_out()
        } else {
            row
        }
    }
}

fn main() -> Result<()> {
    crossterm::terminal::enable_raw_mode()?;
    std::io::stdout().execute(crossterm::terminal::EnterAlternateScreen)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
    let rx = canbus::recv();
    let mut msgs = Vec::<Message>::new();
    let mut state = TableState::default();

    loop {
        if state.selected().is_none() {
            for m in rx.try_iter() {
                match msgs.iter_mut().find(|ref existing| existing.id == m.id) {
                    Some(existing) => existing.merge(m),
                    None => msgs.push(m.into()),
                }
            }
        }

        msgs.sort_by(|a, b| {
            if a.ignored && !b.ignored {
                Ordering::Greater
            } else if !a.ignored && b.ignored {
                Ordering::Less
            } else if !a.pinned && b.pinned {
                Ordering::Greater
            } else if a.pinned && !b.pinned {
                Ordering::Less
            } else {
                a.values.last().unwrap().ts.cmp(&b.values.last().unwrap().ts)
            }
        });

        terminal.draw(|f| {
            let block = if state.selected().is_none() {
                Block::new().title("canalyzer | F)ilter; Q)uit")
            } else {
                Block::new().title("canalyzer | I)gnore; P)in to top; Exit F)iltering")
            }
            .title_position(Position::Bottom).title_style(Style::new().yellow().on_blue());
            f.render_stateful_widget(
                Table::new(
                    msgs.iter().map(|m| m.into()),
                    &[
                        Constraint::Length(6),
                        Constraint::Percentage(50),
                        Constraint::Percentage(50),
                    ],
                )
                .highlight_symbol(">")
                .block(block),
                f.size(),
                &mut state,
            );
        })?;

        if crossterm::event::poll(std::time::Duration::from_millis(250))? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                if key.kind == crossterm::event::KeyEventKind::Press {
                    if key.code == crossterm::event::KeyCode::Char('q') {
                        break;
                    }
                    if key.code == crossterm::event::KeyCode::Char('f') {
                        state.select(match state.selected() {
                            None => Some(0),
                            Some(_) => None,
                        })
                    }
                    if let Some(row) = state.selected() {
                        if key.code == crossterm::event::KeyCode::Char('i') {
                            let msg = msgs.get_mut(row).unwrap();
                            msg.ignored = !msg.ignored;
                        }
                        if key.code == crossterm::event::KeyCode::Char('p') {
                            let msg = msgs.get_mut(row).unwrap();
                            msg.pinned = !msg.pinned;
                        }
                        if key.code == crossterm::event::KeyCode::Down {
                            state.select(Some((row + 1) % msgs.len()));
                        }
                        if key.code == crossterm::event::KeyCode::Up {
                            state.select(Some((row + msgs.len() - 1) % msgs.len()));
                        }
                    }
                }
            }
        }
    }

    std::io::stdout().execute(crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;
    serde_json::to_writer(std::io::stdout(), &msgs);
    println!();
    Ok(())
}
