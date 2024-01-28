use anyhow::Result;
use crossterm::ExecutableCommand;
use ratatui::{
    prelude::*,
    widgets::{block::Position, *},
};
use serde::Serialize;
use serde_with::{serde_as, TimestampMilliSeconds};
use std::{cmp::Ordering, iter, time::SystemTime};

mod canbus;

#[derive(Serialize)]
struct Message {
    id: String,
    values: Vec<Value>,
    ignored: bool,
    pinned: bool,
}

#[serde_as]
#[derive(Serialize)]
struct Value {
    data: String,
    #[serde_as(as = "TimestampMilliSeconds")]
    ts: SystemTime,
}

impl Value {
    fn bg_color(&self) -> Color {
        match self.ts.elapsed() {
            Ok(d) if d.as_secs() < 1 => Color::Rgb(255, 155, 53),
            Ok(d) if d.as_secs() < 2 => Color::Rgb(189, 55, 10),
            Ok(d) if d.as_secs() < 3 => Color::Rgb(94, 0, 0),
            _ => Color::Black,
        }
    }

    fn diff(&self, other: Option<&Self>) -> Cell {
        let mut diff = Line::default();
        for i in 0..self.data.len() {
            let c = self.data.get(i..i + 1).unwrap();
            let color = if let Some(other) = other {
                if other.data.get(i..i + 1) == Some(c) {
                    Color::White
                } else {
                    Color::LightCyan
                }
            } else {
                Color::White
            };
            diff.spans.push(Span::styled(c, Style::default().fg(color)))
        }
        diff.patch_style(Style::default().bg(self.bg_color()));
        diff.into()
    }
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

    fn as_row(&self, cols: usize) -> Row {
        let row = Row::new(
            iter::once(self.id.as_str().into())
                .chain(
                    self.values
                        .iter()
                        .rev()
                        .zip(
                            self.values
                                .iter()
                                .rev()
                                .skip(1)
                                .map(Some)
                                .chain(iter::repeat(None)),
                        )
                        .map(|(a, b)| a.diff(b))
                        .take(cols),
                )
                .collect::<Vec<Cell>>(),
        );
        if self.ignored {
            row.dark_gray().crossed_out()
        } else {
            row
        }
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
                match msgs.iter_mut().find(|existing| existing.id == m.id) {
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
                b.values
                    .last()
                    .unwrap()
                    .ts
                    .cmp(&a.values.last().unwrap().ts)
            }
        });

        terminal.draw(|f| {
            let block = if state.selected().is_none() {
                Block::new().title("canalyzer | F)ilter; Q)uit")
            } else {
                Block::new().title("canalyzer | I)gnore; P)in to top; Exit F)iltering")
            }
            .title_position(Position::Bottom)
            .title_style(Style::new().yellow().on_blue());
            let cols = f.size().width as usize / 17;
            f.render_stateful_widget(
                Table::new(
                    msgs.iter().map(|m| m.as_row(cols)),
                    iter::once(Constraint::Length(6))
                        .chain(iter::repeat(Constraint::Length(16)).take(cols)),
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
    let _ = serde_json::to_writer(std::io::stdout(), &msgs);
    println!();
    Ok(())
}
